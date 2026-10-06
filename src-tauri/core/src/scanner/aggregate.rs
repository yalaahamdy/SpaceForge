//! Aggregation: turn raw walk rows into the [`DirNode`] tree, rollups and
//! top-N lists (spec §14, §15, §16). Hard-linked content is counted once
//! per scan (spec §10). All computations are streaming and bounded.

use crate::models::{DirNode, FileCategory, FileEntry, ScanSummary, UnreadableEntry};
use crate::util::paths::{canon, name_of, parent_of};
use std::collections::{HashMap, HashSet};

/// One raw file row collected during the walk.
#[derive(Debug, Clone)]
pub struct FileRow {
    pub path: String,
    pub size: u64,
    pub allocated: u64,
    pub modified: Option<i64>,
    pub created: Option<i64>,
    pub category: FileCategory,
    pub extension: String,
    /// Identity for hard-link collapsing (windows file id / unix inode),
    /// present only when the filesystem exposes it.
    pub link_id: Option<u64>,
    pub sparse: bool,
}

#[derive(Debug, Clone)]
pub struct DirRow {
    pub path: String,
    pub modified: Option<i64>,
    pub unreadable_child: Option<UnreadableEntry>,
}

pub struct AggregateInput {
    pub volume: String,
    pub index_threshold: u64,
    /// Cap on individually retained files (memory bound, spec §54).
    pub max_indexed_files: usize,
}

pub struct AggregateOutput {
    pub root: DirNode,
    pub large_files: Vec<FileEntry>,
    pub extensions: Vec<(String, u64, u64)>,
    pub categories: Vec<(FileCategory, u64, u64)>,
    pub total_files: u64,
    pub total_dirs: u64,
    pub hard_links_collapsed: u64,
}

/// Aggregate rows into the final tree. `files` must contain *all* files
/// (the function applies the index threshold itself); `dirs` every walked
/// directory.
pub fn aggregate(
    input: &AggregateInput,
    files: &[FileRow],
    dirs: Vec<DirRow>,
    hard_links_collapsed: u64,
) -> AggregateOutput {
    let mut nodes: HashMap<String, DirNode> = HashMap::with_capacity(dirs.len() + 16);
    let mut small: HashMap<String, (u64, u64)> = HashMap::new();
    let mut ext_map: HashMap<String, (u64, u64)> = HashMap::new();
    let mut cat_map: HashMap<FileCategory, (u64, u64)> = HashMap::new();

    let mut total_files = 0u64;
    let mut seen_links: HashSet<u64> = HashSet::new();
    let mut heap: std::collections::BinaryHeap<std::cmp::Reverse<(u64, usize)>> =
        std::collections::BinaryHeap::new();

    // ---- pass 1: totals, rollups, top-N selection ------------------------
    for (i, f) in files.iter().enumerate() {
        if let Some(id) = f.link_id {
            if !seen_links.insert(id) {
                continue; // hard-linked content counted once (spec §10)
            }
        }
        total_files += 1;
        let parent = canon(&parent_of(&f.path));
        if f.size < input.index_threshold {
            let e = small.entry(parent).or_insert((0, 0));
            e.0 += f.size;
            e.1 += 1;
        } else if heap.len() < input.max_indexed_files {
            heap.push(std::cmp::Reverse((f.size, i)));
        } else if let Some(&std::cmp::Reverse((smallest, _))) = heap.peek() {
            if f.size > smallest {
                heap.pop();
                heap.push(std::cmp::Reverse((f.size, i)));
            }
        }
        let e = ext_map.entry(f.extension.clone()).or_insert((0, 0));
        e.0 += f.size;
        e.1 += 1;
        let c = cat_map.entry(f.category).or_insert((0, 0));
        c.0 += f.size;
        c.1 += 1;
    }

    let large_files: Vec<FileEntry> = {
        let mut idxs: Vec<usize> = heap.into_iter().map(|std::cmp::Reverse((_, i))| i).collect();
        idxs.sort_by(|a, b| files[*b].size.cmp(&files[*a].size));
        idxs.iter()
            .map(|&i| {
                let f = &files[i];
                FileEntry {
                    path: f.path.clone(),
                    name: name_of(&f.path),
                    extension: f.extension.clone(),
                    size: f.size,
                    allocated: f.allocated,
                    modified: f.modified.map(ts_to_datetime),
                    created: f.created.map(ts_to_datetime),
                    category: f.category,
                    file_id: f.link_id,
                    link_count: None,
                    sparse: f.sparse,
                }
            })
            .collect()
    };

    // ---- pass 2: directory nodes -----------------------------------------
    for d in &dirs {
        let key = canon(&d.path);
        let entry = nodes.entry(key.clone()).or_insert_with(|| DirNode {
            name: name_of(&d.path),
            path: key.clone(),
            modified: d.modified.map(ts_to_datetime),
            ..Default::default()
        });
        if let Some(u) = &d.unreadable_child {
            entry.unreadable.push(u.clone());
        }
    }

    // Ensure parent chains exist for file parents missing from dir rows.
    for parent in small.keys().cloned().collect::<Vec<_>>() {
        let mut cur = parent;
        loop {
            if cur.is_empty() || nodes.contains_key(&cur) {
                break;
            }
            nodes.insert(
                cur.clone(),
                DirNode {
                    name: name_of(&cur),
                    path: cur.clone(),
                    ..Default::default()
                },
            );
            let up = canon(&parent_of(&cur));
            if up == cur || up.is_empty() {
                break;
            }
            cur = up;
        }
    }

    // Fill per-node direct file lists (retained files only).
    let mut counted: HashSet<u64> = HashSet::new();
    for f in files {
        if let Some(id) = f.link_id {
            if !counted.insert(id) {
                continue;
            }
        }
        if f.size < input.index_threshold {
            continue;
        }
        let parent = canon(&parent_of(&f.path));
        if let Some(n) = nodes.get_mut(&parent) {
            n.files.push(FileEntry {
                path: f.path.clone(),
                name: name_of(&f.path),
                extension: f.extension.clone(),
                size: f.size,
                allocated: f.allocated,
                modified: f.modified.map(ts_to_datetime),
                created: f.created.map(ts_to_datetime),
                category: f.category,
                file_id: f.link_id,
                link_count: None,
                sparse: f.sparse,
            });
        }
    }

    // ---- pass 3: child links + bottom-up sums -----------------------------
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    for path in nodes.keys() {
        let parent = canon(&parent_of(path));
        if parent != *path && nodes.contains_key(&parent) {
            children_map.entry(parent).or_default().push(path.clone());
        }
    }

    let mut order: Vec<String> = nodes.keys().cloned().collect();
    // deepest first for accumulation
    order.sort_by(|a, b| depth_of(b).cmp(&depth_of(a)).then_with(|| a.cmp(b)));
    for path in &order {
        let (mut size, mut alloc) = (0u64, 0u64);
        let (mut fc, mut dc, mut un_bytes, mut small_size, mut small_count) =
            (0u64, 0u64, 0u64, 0u64, 0u64);
        if let Some(n) = nodes.get(path) {
            size = n.files.iter().map(|f| f.size).sum();
            alloc = n.files.iter().map(|f| f.allocated).sum();
            fc = n.files.len() as u64;
            un_bytes = n.unreadable.len() as u64;
        }
        if let Some((s, c)) = small.get(path) {
            small_size = *s;
            small_count = *c;
        }
        if let Some(children) = children_map.get(path) {
            for c in children {
                if let Some(cn) = nodes.get(c) {
                    size += cn.size;
                    alloc += cn.allocated;
                    fc += cn.file_count;
                    dc += cn.dir_count + 1;
                    un_bytes += cn.unreadable.len() as u64;
                }
            }
        }
        if let Some(n) = nodes.get_mut(path) {
            n.size = size + small_size;
            n.allocated = alloc;
            n.file_count = fc + small_count;
            n.dir_count = dc;
            n.small_files_size = small_size;
            n.small_files_count = small_count;
            n.unreadable_bytes = un_bytes;
        }
    }

    // Attach children deepest-first: a node's own subtree is complete
    // before it is moved into its parent.
    let mut attach_order: Vec<String> = nodes.keys().cloned().collect();
    attach_order.sort_by(|a, b| depth_of(b).cmp(&depth_of(a)).then_with(|| b.cmp(a)));
    for path in &attach_order {
        let children = children_map
            .get(path)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|c| nodes.remove(&c))
            .collect::<Vec<DirNode>>();
        if let Some(n) = nodes.get_mut(path) {
            let mut ch = children;
            ch.sort_by(|a, b| b.size.cmp(&a.size));
            n.children = ch;
        }
    }

    // Root = shallowest remaining node.
    let root_path = nodes
        .keys()
        .min_by_key(|p| depth_of(p))
        .cloned()
        .unwrap_or_else(|| canon(&input.volume));
    let root = nodes.remove(&root_path).unwrap_or_default();

    let mut extensions: Vec<(String, u64, u64)> = ext_map.into_iter().map(|(k, (s, c))| (k, s, c)).collect();
    extensions.sort_by(|a, b| b.1.cmp(&a.1));
    let mut categories: Vec<(FileCategory, u64, u64)> = cat_map.into_iter().map(|(k, (s, c))| (k, s, c)).collect();
    categories.sort_by(|a, b| b.1.cmp(&a.1));

    AggregateOutput {
        root,
        large_files,
        extensions,
        categories,
        total_files,
        total_dirs: dirs.len() as u64,
        hard_links_collapsed,
    }
}

fn depth_of(p: &str) -> usize {
    p.split(['\\', '/']).filter(|s| !s.is_empty()).count()
}

fn ts_to_datetime(secs: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(secs, 0).unwrap_or_else(|| {
        use chrono::TimeZone;
        chrono::Utc.timestamp_opt(0, 0).single().unwrap()
    })
}

/// Build a [`ScanSummary`] from aggregate output + walk metadata.
#[allow(clippy::too_many_arguments)]
pub fn summarize(
    volume: &str,
    out: &AggregateOutput,
    scan_id: u64,
    started: chrono::DateTime<chrono::Utc>,
    duration_ms: u64,
    unreadable_entries: u64,
    volume_free: u64,
    volume_used: u64,
    volume_total: u64,
    kind: crate::models::ScanKind,
    reparse_skipped: u64,
) -> ScanSummary {
    ScanSummary {
        scan_id,
        volume: volume.to_string(),
        kind,
        started_at: started,
        duration_ms,
        total_files: out.total_files,
        total_dirs: out.total_dirs,
        total_size: out.root.size,
        total_allocated: out.root.allocated,
        unreadable_entries,
        unreadable_bytes: unreadable_entries,
        indexed_files: out.large_files.len() as u64,
        hard_links_collapsed: out.hard_links_collapsed,
        reparse_points_skipped: reparse_skipped,
        volume_free,
        volume_used,
        volume_total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(path: &str, size: u64) -> FileRow {
        FileRow {
            path: path.to_string(),
            size,
            allocated: size,
            modified: None,
            created: None,
            category: FileCategory::Document,
            extension: "txt".into(),
            link_id: None,
            sparse: false,
        }
    }

    fn d(path: &str) -> DirRow {
        DirRow { path: path.into(), modified: None, unreadable_child: None }
    }

    #[test]
    fn aggregates_tree_and_rollups() {
        let files = vec![
            row("C:\\a\\b\\big.bin", 1000),
            row("C:\\a\\tiny.txt", 5),
            row("C:\\a\\other.bin", 500),
        ];
        let dirs = vec![d("C:\\"), d("C:\\a"), d("C:\\a\\b")];
        let out = aggregate(
            &AggregateInput {
                volume: "C:\\".into(),
                index_threshold: 100,
                max_indexed_files: 1000,
            },
            &files,
            dirs,
            0,
        );
        assert_eq!(out.root.size, 1505);
        assert_eq!(out.total_files, 3);
        assert_eq!(out.root.file_count, 3);
        let a = out.root.children.iter().find(|c| c.name == "a").unwrap();
        assert_eq!(a.size, 1505);
        let b = a.children.iter().find(|c| c.name == "b").unwrap();
        assert_eq!(b.size, 1000);
        // tiny.txt (5 B) is aggregated into its parent directory
        assert_eq!(a.small_files_size, 5);
        assert_eq!(a.small_files_count, 1);
        assert_eq!(out.large_files.len(), 2);
        assert_eq!(out.large_files[0].size, 1000);
    }

    #[test]
    fn hardlinks_counted_once() {
        let mut f1 = row("C:\\x\\one.dat", 100);
        f1.link_id = Some(7);
        let mut f2 = row("C:\\y\\two.dat", 100);
        f2.link_id = Some(7);
        let dirs = vec![d("C:\\"), d("C:\\x"), d("C:\\y")];
        let out = aggregate(
            &AggregateInput { volume: "C:\\".into(), index_threshold: 10, max_indexed_files: 10 },
            &[f1, f2],
            dirs,
            1,
        );
        assert_eq!(out.total_files, 1);
        assert_eq!(out.root.size, 100);
    }

    #[test]
    fn unreadable_propagates() {
        let dirs = vec![
            d("C:\\"),
            DirRow {
                path: "C:\\locked".into(),
                modified: None,
                unreadable_child: Some(UnreadableEntry {
                    path: "C:\\locked\\sub".into(),
                    reason: "Access is denied".into(),
                }),
            },
        ];
        let out = aggregate(
            &AggregateInput { volume: "C:\\".into(), index_threshold: 10, max_indexed_files: 10 },
            &[],
            dirs,
            0,
        );
        let locked = out.root.children.iter().find(|c| c.name == "locked").unwrap();
        assert_eq!(locked.unreadable.len(), 1);
    }
}
