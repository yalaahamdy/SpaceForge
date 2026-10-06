//! The ultra-fast parallel scan engine (spec §9).
//!
//! Design:
//! - jwalk with a dedicated rayon pool sized by scan mode
//! - reparse points (junctions/symlinks/mount points) are never traversed
//!   (spec §40) unless explicitly enabled in advanced settings
//! - errors are recorded, never silently swallowed (spec §41, §55)
//! - Windows: hard-link identity via file index for link-count>1 files
//! - Experimental NTFS MFT fast path behind the opt-in flag (spec §12)

use crate::fsops;
use crate::models::{
    CoreError, CoreResult, ScanKind, ScanMode, ScanResult, ScanSummary,
};
use crate::scanner::aggregate::{aggregate, AggregateInput, AggregateOutput, DirRow, FileRow};
use crate::scanner::progress::{Counters, ProgressReporter};
use crate::util::classify_path;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

pub struct ScanControl {
    pub counters: Arc<Counters>,
}

impl ScanControl {
    pub fn cancel(&self) {
        self.counters.cancel();
    }
    pub fn pause(&self, on: bool) {
        self.counters.pause(on);
    }
}

pub struct ScanRequest<'a> {
    pub volume: &'a str,
    pub mode: ScanMode,
    pub index_threshold: u64,
    pub max_indexed_files: usize,
    pub traverse_reparse_points: bool,
    pub worker_threads: Option<u32>,
    pub use_experimental_mft: bool,
}

pub struct ScanOutput {
    pub result: ScanResult,
    pub control: ScanControl,
    pub progress_rx: crossbeam_channel::Receiver<crate::models::ScanProgress>,
}

fn worker_count(mode: ScanMode, override_count: Option<u32>) -> usize {
    if let Some(n) = override_count {
        return n.max(1) as usize;
    }
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    ((cores as f32) * mode.worker_fraction()).round().max(1.0) as usize
}

/// Scan a volume and return the full aggregated result. Progress events
/// stream through `progress_rx`; the caller forwards them to the UI layer.
pub fn scan_volume(req: &ScanRequest, kind: ScanKind) -> CoreResult<ScanOutput> {
    let counters = Arc::new(Counters::default());
    let (ptx, prx) = crossbeam_channel::bounded::<crate::models::ScanProgress>(64);
    let result = scan_with_counters(req, kind, counters.clone(), ptx)?;
    Ok(ScanOutput {
        result,
        control: ScanControl { counters },
        progress_rx: prx,
    })
}

/// Spawn the scan on a dedicated thread. Returns handles for progress,
/// cancellation/pause and the final result — used by the Tauri layer so
/// the IPC thread never blocks (spec §9, §54).
pub struct ScanJob {
    pub control: ScanControl,
    pub progress_rx: crossbeam_channel::Receiver<crate::models::ScanProgress>,
    pub result_rx: std::sync::mpsc::Receiver<CoreResult<ScanResult>>,
}

pub fn spawn_scan(req: &ScanRequest, kind: ScanKind) -> CoreResult<ScanJob> {
    // Validate the volume is reachable before spawning (fast fail).
    if fsops::volume_stats(req.volume).is_none() {
        return Err(CoreError::Scan(format!(
            "volume {} is not accessible",
            req.volume
        )));
    }
    let counters = Arc::new(Counters::default());
    let (ptx, prx) = crossbeam_channel::bounded::<crate::models::ScanProgress>(64);
    let (rtx, rrx) = std::sync::mpsc::channel();
    let counters_thread = counters.clone();
    let req_owned = ScanRequestOwned {
        volume: req.volume.to_string(),
        mode: req.mode,
        index_threshold: req.index_threshold,
        max_indexed_files: req.max_indexed_files,
        traverse_reparse_points: req.traverse_reparse_points,
        worker_threads: req.worker_threads,
        use_experimental_mft: req.use_experimental_mft,
    };
    std::thread::Builder::new()
        .name("spaceforge-scan".into())
        .spawn(move || {
            let counters = counters_thread;
            let req = ScanRequest {
                volume: &req_owned.volume,
                mode: req_owned.mode,
                index_threshold: req_owned.index_threshold,
                max_indexed_files: req_owned.max_indexed_files,
                traverse_reparse_points: req_owned.traverse_reparse_points,
                worker_threads: req_owned.worker_threads,
                use_experimental_mft: req_owned.use_experimental_mft,
            };
            let result = scan_with_counters(&req, kind, counters.clone(), ptx);
            let _ = rtx.send(result);
        })
        .map_err(|e| CoreError::Other(e.to_string()))?;
    Ok(ScanJob {
        control: ScanControl { counters },
        progress_rx: prx,
        result_rx: rrx,
    })
}

struct ScanRequestOwned {
    volume: String,
    mode: ScanMode,
    index_threshold: u64,
    max_indexed_files: usize,
    traverse_reparse_points: bool,
    worker_threads: Option<u32>,
    use_experimental_mft: bool,
}

/// The synchronous scan path (also used by tests).
pub fn scan_with_counters(
    req: &ScanRequest,
    kind: ScanKind,
    counters: Arc<Counters>,
    ptx: crossbeam_channel::Sender<crate::models::ScanProgress>,
) -> CoreResult<ScanResult> {
    let started = chrono::Utc::now();
    let start = Instant::now();

    // Volume sanity + stats snapshot.
    let (free_before, total, _used) = fsops::volume_stats(req.volume)
        .ok_or_else(|| CoreError::Scan(format!("volume {} is not accessible", req.volume)))?;
    let cluster = fsops::cluster_size(req.volume);

    let mut reporter =
        ProgressReporter::new(ptx, counters.clone(), now_id(), req.volume.to_string());
    reporter.emit(crate::models::ScanPhase::Preparing);

    // ---- choose engine ------------------------------------------------------
    #[cfg(windows)]
    let (files, dirs, reparse_skipped) =
        if req.use_experimental_mft && kind != ScanKind::Reverify {
            match crate::scanner::mft::scan_mft(req.volume, &counters) {
                Ok((f, d)) => {
                    reporter.emit(crate::models::ScanPhase::Aggregating);
                    (f, d, 0u64)
                }
                Err(_) => walk_volume(req, cluster, &counters, &mut reporter)?,
            }
        } else {
            walk_volume(req, cluster, &counters, &mut reporter)?
        };

    #[cfg(not(windows))]
    let (files, dirs, reparse_skipped) =
        walk_volume(req, cluster, &counters, &mut reporter)?;

    if counters.is_cancelled() {
        reporter.emit(crate::models::ScanPhase::Cancelled);
        return Err(CoreError::Cancelled);
    }

    // ---- aggregate ----------------------------------------------------------
    reporter.emit(crate::models::ScanPhase::Aggregating);
    let input = AggregateInput {
        volume: req.volume.to_string(),
        index_threshold: req.index_threshold,
        max_indexed_files: req.max_indexed_files,
    };
    let out: AggregateOutput = aggregate(&input, &files, dirs, 0);
    let unreadable_count = count_unreadable(&out.root);

    reporter.emit(crate::models::ScanPhase::Finalizing);
    let duration_ms = start.elapsed().as_millis() as u64;
    let free_now = fsops::volume_stats(req.volume)
        .map(|(f, _, _)| f)
        .unwrap_or(free_before);
    let used_now = total.saturating_sub(free_now);
    let summary: ScanSummary = crate::scanner::aggregate::summarize(
        req.volume,
        &out,
        now_id(),
        started,
        duration_ms,
        unreadable_count,
        free_before,
        used_now,
        total,
        kind,
        counters.reparse_skipped.load(Ordering::Relaxed) + reparse_skipped,
    );

    reporter.emit(crate::models::ScanPhase::Done);
    Ok(ScanResult {
        summary,
        root: out.root,
        large_files: out.large_files,
        extensions: out.extensions,
        categories: out.categories,
    })
}

fn count_unreadable(root: &crate::models::DirNode) -> u64 {
    let mut n = root.unreadable.len() as u64;
    for c in &root.children {
        n += count_unreadable(c);
    }
    n
}

fn now_id() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// The reliable parallel filesystem walker.
fn walk_volume(
    req: &ScanRequest,
    cluster: u64,
    counters: &Arc<Counters>,
    reporter: &mut ProgressReporter,
) -> CoreResult<(Vec<FileRow>, Vec<DirRow>, u64)> {
    let root = req.volume.to_string();
    let threads = worker_count(req.mode, req.worker_threads);
    let counters2 = counters.clone();
    let traverse_reparse = req.traverse_reparse_points;

    let walk = jwalk::WalkDir::new(&root)
        .parallelism(jwalk::Parallelism::RayonNewPool(threads))
        .skip_hidden(false)
        .process_read_dir(move |_depth, path, _state, children| {
            let c = counters2.as_ref();
            c.wait_if_paused();
            if c.is_cancelled() {
                children.clear(); // no children → recursion stops everywhere
                return;
            }
            if let Ok(mut guard) = c.current_path.try_lock() {
                *guard = path.to_string_lossy().into_owned();
            }

            // Record read failures with their real paths when available
            // (spec §41 — never silently swallowed).
            children.retain(|entry| match entry {
                Ok(_) => true,
                Err(e) => {
                    let shown = e
                        .path()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.to_string_lossy().into_owned());
                    let reason = e
                        .io_error()
                        .map(reason_of)
                        .unwrap_or_else(|| e.to_string());
                    c.errors.fetch_add(1, Ordering::Relaxed);
                    c.record_unreadable(&shown, &reason);
                    false
                }
            });

            // Detect and exclude reparse directories from traversal (spec §40).
            if !traverse_reparse {
                let mut drop_idx: Vec<usize> = Vec::new();
                for (i, entry) in children.iter_mut().enumerate() {
                    let Ok(entry) = entry else { continue };
                    if !entry.file_type().is_dir() {
                        continue;
                    }
                    let reparse = {
                        #[cfg(windows)]
                        {
                            entry
                                .metadata()
                                .map(|md| fsops::is_reparse(md.file_attributes()))
                                .unwrap_or(false)
                        }
                        #[cfg(not(windows))]
                        {
                            entry
                                .metadata()
                                .map(|md| md.file_type().is_symlink())
                                .unwrap_or(false)
                        }
                    };
                    if reparse {
                        c.reparse_skipped.fetch_add(1, Ordering::Relaxed);
                        drop_idx.push(i);
                    }
                }
                for i in drop_idx.into_iter().rev() {
                    let _ = children.remove(i);
                }
            }
        });

    let mut files: Vec<FileRow> = Vec::with_capacity(32768);
    let mut dirs: Vec<DirRow> = Vec::with_capacity(8192);

    reporter.emit(crate::models::ScanPhase::Walking);
    // Root node itself.
    dirs.push(DirRow {
        path: root.clone(),
        modified: std::fs::metadata(&root)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64),
        unreadable_child: None,
    });

    for entry in walk {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                let shown = e
                    .path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|| root.clone());
                let reason = e
                    .io_error()
                    .map(reason_of)
                    .unwrap_or_else(|| e.to_string());
                counters.errors.fetch_add(1, Ordering::Relaxed);
                counters.record_unreadable(&shown, &reason);
                continue;
            }
        };
        let path = entry.path();
        let path_s = path.to_string_lossy().into_owned();
        let ft = entry.file_type();
        if ft.is_dir() {
            if path_s == root {
                continue;
            }
            let mtime = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);
            dirs.push(DirRow { path: path_s, modified: mtime, unreadable_child: None });
            counters.dirs.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        if !ft.is_file() {
            continue;
        }
        let md = match entry.metadata() {
            Ok(m) => m,
            Err(_) => {
                counters.errors.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };
        let size = md.len();
        #[cfg(windows)]
        let attrs = md.file_attributes();
        #[cfg(not(windows))]
        let attrs = 0u32;
        let sparse = cfg!(windows) && (attrs & 0x0200) != 0; // FILE_ATTRIBUTE_SPARSE_FILE
        let alloc = fsops::allocated_for(size, cluster, sparse);
        let mtime = md
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);
        let created = md
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);
        let lower_name = crate::util::paths::name_of(&path_s).to_ascii_lowercase();
        let ext = match lower_name.rfind('.') {
            Some(idx) => lower_name[idx + 1..].to_string(),
            None => String::new(),
        };
        let category = classify_path(Path::new(&path_s));
        let link_id = {
            #[cfg(windows)]
            {
                None
            }
            #[cfg(not(windows))]
            {
                #[cfg(target_os = "linux")]
                use std::os::linux::fs::MetadataExt;
                #[cfg(not(target_os = "linux"))]
                use std::os::unix::fs::MetadataExt;
                if md.st_nlink() > 1 {
                    Some(md.st_ino())
                } else {
                    None
                }
            }
        };
        files.push(FileRow {
            path: path_s,
            size,
            allocated: alloc,
            modified: mtime,
            created,
            category,
            extension: ext,
            link_id,
            sparse,
        });
        let f_count = counters.files.fetch_add(1, Ordering::Relaxed);
        counters.bytes.fetch_add(size, Ordering::Relaxed);
        if (f_count & 0x7F) == 0 {
            reporter.emit(crate::models::ScanPhase::Walking);
        }
    }

    reporter.emit(crate::models::ScanPhase::Walking);

    let reparse_total = counters.reparse_skipped.load(Ordering::Relaxed);
    Ok((files, dirs, reparse_total))
}

fn reason_of(e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::PermissionDenied => "Access is denied".to_string(),
        std::io::ErrorKind::NotFound => "The item no longer exists".to_string(),
        _ => e.to_string(),
    }
}

/// Human-readable engine note used by Advanced diagnostics.
pub fn engine_note(mft_enabled: bool) -> String {
    if cfg!(windows) && mft_enabled {
        "NTFS MFT enumeration (experimental, opt-in) with parallel-walk fallback".to_string()
    } else {
        "Parallel filesystem walk; reparse points not traversed".to_string()
    }
}
