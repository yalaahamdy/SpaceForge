//! Duplicate detection (spec §19): same-size → partial hash → full hash,
//! verified by byte comparison so only proven-identical content groups.
//! Hard links share a file identity and never count as redundant copies.

use crate::models::{CoreError, CoreResult, DuplicateGroup, DuplicateMember, KeepStrategy};
use crate::util::paths::name_of;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A candidate file for duplicate analysis.
#[derive(Debug, Clone)]
pub struct DuplicateCandidate {
    pub path: String,
    pub size: u64,
    pub modified: Option<i64>,
    /// File identity (windows file index / unix inode). Files sharing this
    /// identity are hard links, not independent copies.
    pub file_id: Option<u64>,
}

pub type ProgressFn = Arc<dyn Fn(&str, u64, u64) + Send + Sync>;

pub struct DuplicateOptions {
    pub min_size: u64,
    pub keep_strategy: KeepStrategy,
    pub preferred_folder: Option<String>,
    /// Progress callback: (stage, done, total).
    pub on_progress: ProgressFn,
    /// Cancellation flag.
    pub cancelled: Arc<AtomicBool>,
}

impl Default for DuplicateOptions {
    fn default() -> Self {
        Self {
            min_size: 1024 * 1024, // 1 MB — sub-1MB duplicates rarely matter
            keep_strategy: KeepStrategy::KeepNewest,
            preferred_folder: None,
            on_progress: Arc::new(|_, _, _| {}),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

const PARTIAL_LEN: u64 = 64 * 1024;

/// Find duplicate groups. Only files with established content equality end
/// up grouped (spec §19). Returns groups sorted by wasted space.
pub fn find_duplicates(
    candidates: Vec<DuplicateCandidate>,
    opts: &DuplicateOptions,
) -> CoreResult<Vec<DuplicateGroup>> {
    use rayon::prelude::*;

    let cancelled = opts.cancelled.clone();

    // Stage 1 — same size.
    (opts.on_progress)("size", 0, candidates.len() as u64);
    let mut by_size: HashMap<u64, Vec<DuplicateCandidate>> = HashMap::new();
    for c in candidates {
        if c.size < opts.min_size {
            continue;
        }
        by_size.entry(c.size).or_default().push(c);
    }
    let size_groups: Vec<(u64, Vec<DuplicateCandidate>)> = by_size
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(s, v)| (s, v))
        .collect();
    let total_candidates: u64 = size_groups.iter().map(|(_, g)| g.len() as u64).sum();
    (opts.on_progress)("partial", 0, total_candidates);

    // Stage 2 — partial content hash (head + tail 64KB) in parallel.
    let partial_groups: Vec<(u64, Vec<DuplicateCandidate>)> = size_groups
        .into_par_iter()
        .filter_map(|(size, group)| {
            if cancelled.load(Ordering::Relaxed) {
                return None;
            }
            let mut by_partial: HashMap<u64, Vec<DuplicateCandidate>> = HashMap::new();
            for c in group {
                let h = match partial_hash(Path::new(&c.path), c.size) {
                    Some(h) => h,
                    None => continue,
                };
                by_partial.entry(h).or_default().push(c);
            }
            let groups: Vec<Vec<DuplicateCandidate>> = by_partial
                .into_iter()
                .filter(|(_, v)| v.len() > 1)
                .map(|(_, v)| v)
                .collect();
            if groups.is_empty() {
                None
            } else {
                let key = size ^ xxhash_rust::xxh3::xxh3_64(groups[0][0].path.as_bytes());
                Some((key, groups.into_iter().flatten().collect()))
            }
        })
        .collect();
    if cancelled.load(Ordering::Relaxed) {
        return Err(CoreError::Cancelled);
    }

    // Stage 3 — full hash, then byte-compare verification.
    let mut groups: Vec<DuplicateGroup> = Vec::new();
    let mut done: u64 = 0;
    let full_total: u64 = partial_groups.iter().map(|(_, g)| g.len() as u64).sum();
    (opts.on_progress)("full", 0, full_total);

    for (_, group) in partial_groups {
        if cancelled.load(Ordering::Relaxed) {
            return Err(CoreError::Cancelled);
        }
        let mut by_full: HashMap<u64, Vec<DuplicateCandidate>> = HashMap::new();
        for c in group {
            let h = match full_hash(Path::new(&c.path), c.size) {
                Some(h) => h,
                None => continue,
            };
            by_full.entry(h).or_default().push(c);
            done += 1;
            (opts.on_progress)("full", done, full_total);
        }
        for (_, same_hash) in by_full {
            if same_hash.len() < 2 {
                continue;
            }
            // Byte-compare every member against the first — this *proves*
            // content equality regardless of hash collisions.
            let reference = &same_hash[0];
            let mut verified: Vec<DuplicateCandidate> = vec![reference.clone()];
            for c in &same_hash[1..] {
                if files_equal(Path::new(&reference.path), Path::new(&c.path)) {
                    verified.push(c.clone());
                }
            }
            if verified.len() < 2 {
                continue;
            }
            if let Some(g) = build_group(verified, opts) {
                groups.push(g);
            }
        }
    }

    groups.sort_by(|a, b| b.wasted.cmp(&a.wasted));
    Ok(groups)
}

/// Pick the keeper and order members so index 0 is the keeper.
fn build_group(members: Vec<DuplicateCandidate>, opts: &DuplicateOptions) -> Option<DuplicateGroup> {
    if members.is_empty() {
        return None;
    }
    let size_each = members[0].size;
    let mut ms: Vec<DuplicateMember> = members
        .into_iter()
        .map(|c| DuplicateMember { path: c.path, modified: c.modified.map(ts), file_id: c.file_id })
        .collect();

    // Hard links: identical file ids are one logical copy.
    let mut logical: Vec<DuplicateMember> = Vec::new();
    let mut seen_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for m in ms {
        if let Some(id) = m.file_id {
            if !seen_ids.insert(id) {
                continue;
            }
        }
        logical.push(m);
    }
    ms = logical;
    if ms.len() < 2 {
        return None;
    }

    let keep_idx = match opts.keep_strategy {
        KeepStrategy::KeepNewest => ms
            .iter()
            .enumerate()
            .max_by_key(|(_, m)| m.modified)
            .map(|(i, _)| i)
            .unwrap_or(0),
        KeepStrategy::KeepOldest => ms
            .iter()
            .enumerate()
            .min_by_key(|(_, m)| m.modified)
            .map(|(i, _)| i)
            .unwrap_or(0),
        KeepStrategy::KeepShortestPath => ms
            .iter()
            .enumerate()
            .min_by_key(|(_, m)| (m.path.len(), m.path.clone()))
            .map(|(i, _)| i)
            .unwrap_or(0),
        KeepStrategy::KeepPreferredFolder => ms
            .iter()
            .enumerate()
            .find(|(_, m)| {
                opts.preferred_folder
                    .as_ref()
                    .map(|p| m.path.to_lowercase().starts_with(&p.to_lowercase()))
                    .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .unwrap_or_else(|| best_guess_idx(&ms)),
        KeepStrategy::KeepFirst => best_guess_idx(&ms),
    };
    if keep_idx != 0 {
        ms.swap(0, keep_idx);
    }
    let copies = ms.len() as u32;
    let wasted = size_each * (copies as u64 - 1);
    let id = format!("dup-{:016x}", xxhash_rust::xxh3::xxh3_64(ms[0].path.as_bytes()));
    Some(DuplicateGroup { id, size_each, copies, wasted, keep: ms[0].path.clone(), paths: ms })
}

/// Without a preference, prefer keepers in user-visible data over caches:
/// shortest path heuristic is a reasonable deterministic default.
fn best_guess_idx(ms: &[DuplicateMember]) -> usize {
    ms.iter()
        .enumerate()
        .min_by_key(|(_, m)| (m.path.len(), m.path.clone()))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn partial_hash(p: &Path, size: u64) -> Option<u64> {
    let mut f = std::fs::File::open(p).ok()?;
    let mut buf = vec![0u8; PARTIAL_LEN as usize];
    let mut h = xxhash_rust::xxh3::Xxh3::new();
    let head = buf.as_mut_slice();
    let n = read_up_to(&mut f, head)?;
    h.update(&head[..n]);
    if size > PARTIAL_LEN {
        let tail_pos = size.saturating_sub(PARTIAL_LEN);
        use std::io::Seek;
        f.seek(std::io::SeekFrom::Start(tail_pos)).ok()?;
        let n = read_up_to(&mut f, head)?;
        h.update(&head[..n]);
    }
    Some(h.digest())
}

fn full_hash(p: &Path, size: u64) -> Option<u64> {
    let mut f = std::fs::File::open(p).ok()?;
    let mut h = xxhash_rust::xxh3::Xxh3::new();
    h.update(&size.to_le_bytes());
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = read_up_to(&mut f, &mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Some(h.digest())
}

fn read_up_to(f: &mut std::fs::File, buf: &mut [u8]) -> Option<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match f.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
    Some(filled)
}

/// Definitive content equality: streaming byte comparison.
fn files_equal(a: &Path, b: &Path) -> bool {
    let (mut fa, mut fb) = match (std::fs::File::open(a), std::fs::File::open(b)) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return false,
    };
    let mut ba = vec![0u8; 1024 * 1024];
    let mut bb = vec![0u8; 1024 * 1024];
    loop {
        let na = read_up_to(&mut fa, &mut ba).unwrap_or(0);
        let nb = read_up_to(&mut fb, &mut bb).unwrap_or(0);
        if na != nb {
            return false;
        }
        if na == 0 {
            return true;
        }
        if ba[..na] != bb[..nb] {
            return false;
        }
    }
}

fn ts(secs: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(secs, 0).unwrap_or_else(|| {
        use chrono::TimeZone;
        chrono::Utc.timestamp_opt(0, 0).single().unwrap()
    })
}

pub fn group_display_name(g: &DuplicateGroup) -> String {
    name_of(&g.keep)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, data: &[u8]) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, data).unwrap();
    }

    fn opts() -> DuplicateOptions {
        DuplicateOptions { min_size: 10, ..Default::default() }
    }

    #[test]
    fn finds_true_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a").join("same.bin");
        let b = dir.path().join("b").join("same.bin");
        let c = dir.path().join("b").join("diff.bin");
        let payload: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        write(&a, &payload);
        write(&b, &payload);
        write(&c, &payload[..100_000]);
        let cands = vec![
            DuplicateCandidate { path: a.to_string_lossy().into(), size: payload.len() as u64, modified: None, file_id: None },
            DuplicateCandidate { path: b.to_string_lossy().into(), size: payload.len() as u64, modified: None, file_id: None },
            DuplicateCandidate { path: c.to_string_lossy().into(), size: (payload.len() / 2) as u64, modified: None, file_id: None },
        ];
        let groups = find_duplicates(cands, &opts()).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].copies, 2);
        assert_eq!(groups[0].wasted, payload.len() as u64);
    }

    #[test]
    fn different_content_same_size_not_grouped() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("x.bin");
        let b = dir.path().join("y.bin");
        let p1: Vec<u8> = (0..50_000u32).map(|i| (i % 7) as u8).collect();
        let p2: Vec<u8> = (0..50_000u32).map(|i| ((i + 3) % 7) as u8).collect();
        write(&a, &p1);
        write(&b, &p2);
        let cands = vec![
            DuplicateCandidate { path: a.to_string_lossy().into(), size: 50_000, modified: None, file_id: None },
            DuplicateCandidate { path: b.to_string_lossy().into(), size: 50_000, modified: None, file_id: None },
        ];
        let groups = find_duplicates(cands, &opts()).unwrap();
        assert!(groups.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn hardlinks_are_not_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("x.dat");
        let b = dir.path().join("y.dat");
        write(&a, b"0123456789abcdef0123456789abcdef");
        std::fs::hard_link(&a, &b).unwrap();
        let ino = {
            #[cfg(target_os = "linux")]
            use std::os::linux::fs::MetadataExt;
            #[cfg(not(target_os = "linux"))]
            use std::os::unix::fs::MetadataExt;
            std::fs::metadata(&a).unwrap().st_ino()
        };
        let cands = vec![
            DuplicateCandidate { path: a.to_string_lossy().into(), size: 32, modified: None, file_id: Some(ino) },
            DuplicateCandidate { path: b.to_string_lossy().into(), size: 32, modified: None, file_id: Some(ino) },
        ];
        let groups = find_duplicates(cands, &opts()).unwrap();
        assert!(groups.is_empty(), "hard links must not be redundant copies");
    }

    #[test]
    fn keep_strategies_pick_expected_member() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("old.bin");
        let new = dir.path().join("new.bin");
        write(&old, b"abcdefghijklmnopqrst");
        write(&new, b"abcdefghijklmnopqrst");
        let cands = vec![
            DuplicateCandidate { path: old.to_string_lossy().into(), size: 20, modified: Some(1000), file_id: None },
            DuplicateCandidate { path: new.to_string_lossy().into(), size: 20, modified: Some(2000), file_id: None },
        ];
        let mut o = opts();
        o.keep_strategy = KeepStrategy::KeepOldest;
        let g = find_duplicates(cands.clone(), &o).unwrap();
        assert_eq!(g[0].keep, old.to_string_lossy());
        o.keep_strategy = KeepStrategy::KeepNewest;
        let g = find_duplicates(cands, &o).unwrap();
        assert_eq!(g[0].keep, new.to_string_lossy());
    }

    #[test]
    fn cancellation_stops() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.bin");
        let b = dir.path().join("b.bin");
        write(&a, b"0123456789abcdef0123456789abcdef");
        write(&b, b"0123456789abcdef0123456789abcdef");
        let cands = vec![
            DuplicateCandidate { path: a.to_string_lossy().into(), size: 32, modified: None, file_id: None },
            DuplicateCandidate { path: b.to_string_lossy().into(), size: 32, modified: None, file_id: None },
        ];
        let mut o = opts();
        o.cancelled = Arc::new(AtomicBool::new(true));
        assert!(find_duplicates(cands, &o).is_err());
    }
}
