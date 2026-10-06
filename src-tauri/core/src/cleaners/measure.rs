//! Directory measurement used by rule discovery.
//!
//! Uses jwalk with a small parallel pool; error-tolerant, never crashes.

use std::path::Path;

/// (file_count, total_bytes) of a directory tree. Missing → (0, 0).
pub fn measure_dir(path: &Path) -> (u64, u64) {
    if !path.is_dir() {
        return (0, 0);
    }
    let mut files = 0u64;
    let mut bytes = 0u64;
    let walk = jwalk::WalkDir::new(path)
        .parallelism(jwalk::Parallelism::RayonDefaultPool { busy_timeout: std::time::Duration::from_secs(1) })
        .skip_hidden(false)
        .process_read_dir(|_d, _p, _s, children| {
            children.retain(|e| e.is_ok());
        });
    for entry in walk {
        let Ok(entry) = entry else { continue };
        if entry.file_type().is_file() {
            files += 1;
            if let Ok(md) = entry.metadata() {
                bytes += md.len();
            }
        }
    }
    (files, bytes)
}

/// Directories from a list that actually exist.
pub fn existing_dirs(paths: &[std::path::PathBuf]) -> Vec<String> {
    paths
        .iter()
        .filter(|p| p.is_dir())
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

/// Measure several directories in parallel; returns per-dir (files, bytes).
pub fn measure_many(paths: &[std::path::PathBuf]) -> Vec<(u64, u64)> {
    use rayon::prelude::*;
    paths.par_iter().map(|p| measure_dir(p)).collect()
}
