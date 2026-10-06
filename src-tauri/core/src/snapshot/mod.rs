//! Snapshot comparison (spec §44, §46): factual growth analysis from two
//! scan indexes. Never invents history — comparison requires both snapshots.

use crate::db::Index;
use crate::models::{CoreResult, DirDelta, FileEntry, FileCategory, SnapshotDiff};

/// Compare two scans by id. Only stored (index-threshold) files and the
/// persisted directory levels are compared — this is the factual subset.
pub fn compare(index: &Index, older_scan: i64, newer_scan: i64) -> CoreResult<SnapshotDiff> {
    let older_files = index.scan_files(older_scan)?;
    let newer_files = index.scan_files(newer_scan)?;

    let older_map: std::collections::HashMap<&str, &FileEntry> =
        older_files.iter().map(|f| (f.path.as_str(), f)).collect();
    let newer_map: std::collections::HashMap<&str, &FileEntry> =
        newer_files.iter().map(|f| (f.path.as_str(), f)).collect();

    let mut new_large = Vec::new();
    let mut removed = Vec::new();
    let mut total_delta: i64 = 0;

    for (path, f) in &newer_map {
        match older_map.get(*path) {
            None => {
                new_large.push((*f).clone());
                total_delta += f.size as i64;
            }
            Some(old) => {
                let d = f.size as i64 - old.size as i64;
                if d != 0 {
                    total_delta += d;
                }
            }
        }
    }
    for (path, f) in &older_map {
        if !newer_map.contains_key(*path) {
            removed.push((*f).clone());
            total_delta -= f.size as i64;
        }
    }
    new_large.sort_by(|a, b| b.size.cmp(&a.size));
    removed.sort_by(|a, b| b.size.cmp(&a.size));
    new_large.truncate(500);
    removed.truncate(500);

    // Directory-level deltas from persisted scan_dirs rows (top 3 levels).
    let grew = dir_deltas(index, older_scan, newer_scan, true)?;
    let shrank = dir_deltas(index, older_scan, newer_scan, false)?;

    // Category deltas from retained files.
    let mut cat_delta: std::collections::HashMap<FileCategory, i64> = Default::default();
    for (path, f) in &newer_map {
        match older_map.get(*path) {
            None => *cat_delta.entry(f.category).or_default() += f.size as i64,
            Some(old) => {
                let d = f.size as i64 - old.size as i64;
                if d != 0 {
                    *cat_delta.entry(f.category).or_default() += d;
                }
            }
        }
    }
    for (path, f) in &older_map {
        if !newer_map.contains_key(*path) {
            *cat_delta.entry(f.category).or_default() -= f.size as i64;
        }
    }
    let mut category_deltas: Vec<(FileCategory, i64)> = cat_delta.into_iter().collect();
    category_deltas.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(SnapshotDiff {
        grew,
        shrank,
        new_large_files: new_large,
        removed_files: removed,
        category_deltas,
        total_delta,
    })
}

fn dir_deltas(
    index: &Index,
    older: i64,
    newer: i64,
    growing: bool,
) -> CoreResult<Vec<DirDelta>> {
    let old_dirs = index.scan_dirs(older)?;
    let new_dirs = index.scan_dirs(newer)?;
    // Tuple layout: (path, parent, name, size, file_count, dir_count)
    let old_map: std::collections::HashMap<&str, i64> = old_dirs
        .iter()
        .map(|d| (d.0.as_str(), d.3))
        .collect();
    let mut deltas: Vec<DirDelta> = Vec::new();
    for d in &new_dirs {
        if let Some(before) = old_map.get(d.0.as_str()) {
            let delta = d.3 - *before;
            if (growing && delta > 0) || (!growing && delta < 0) {
                deltas.push(DirDelta {
                    path: d.0.clone(),
                    name: d.2.clone(),
                    before: *before as u64,
                    after: d.3 as u64,
                    delta,
                });
            }
        }
    }
    deltas.sort_by(|a, b| b.delta.abs().cmp(&a.delta.abs()));
    deltas.truncate(50);
    Ok(deltas)
}
