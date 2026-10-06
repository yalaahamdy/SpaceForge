//! Low-level filesystem operations: volume stats, cluster size, file
//! identity (hard links), and Windows-native mechanisms.

pub mod delete;
pub mod shell;

#[cfg(windows)]
pub mod win;

/// (free, total, used) for the volume containing `path`.
pub fn volume_stats(path: &str) -> Option<(u64, u64, u64)> {
    #[cfg(windows)]
    {
        win::free_space(path).map(|(free, total)| (free, total, total.saturating_sub(free)))
    }
    #[cfg(not(windows))]
    {
        crate::volumes::sysinfo_disk_stats_for(path).map(|(avail, total)| (avail, total, total - avail))
    }
}

/// Bytes per allocation cluster for the volume containing `path`.
pub fn cluster_size(path: &str) -> u64 {
    #[cfg(windows)]
    {
        win::cluster_size(path).unwrap_or(4096)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        4096
    }
}

/// Allocated size approximation: cluster-rounded, logical size for sparse.
pub fn allocated_for(size: u64, cluster: u64, sparse: bool) -> u64 {
    if sparse || cluster == 0 {
        return size;
    }
    size.div_ceil(cluster) * cluster
}

/// (volume_serial, file_index, hard_link_count) when the OS exposes it.
pub fn file_identity(path: &std::path::Path) -> Option<(u64, u64, u64)> {
    #[cfg(windows)]
    {
        win::file_identity(path)
    }
    #[cfg(not(windows))]
    {
        #[cfg(target_os = "linux")]
        use std::os::linux::fs::MetadataExt;
        #[cfg(not(target_os = "linux"))]
        use std::os::unix::fs::MetadataExt;
        let md = std::fs::metadata(path).ok()?;
        Some((md.st_dev(), md.st_ino(), md.st_nlink()))
    }
}

/// True when the metadata marks the file as a reparse point (junction,
/// symlink, mount point, OneDrive placeholder, …).
pub fn is_reparse(attrs: u32) -> bool {
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    cfg!(windows) && (attrs & FILE_ATTRIBUTE_REPARSE_POINT) != 0
}

/// Is the current process elevated (admin on Windows)?
pub fn is_elevated() -> bool {
    #[cfg(windows)]
    {
        win::is_elevated()
    }
    #[cfg(not(windows))]
    {
        // Development hosts are treated as non-elevated for honest reporting.
        false
    }
}

/// Running process names (lowercased) — used for "close app first" warnings.
pub fn running_processes() -> std::collections::HashSet<String> {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    sys.processes()
        .values()
        .filter_map(|p| p.name().to_str())
        .map(|n| n.trim_end_matches(".exe").to_ascii_lowercase())
        .collect()
}

/// Recycle Bin stats across all drives: (item_count, bytes).
pub fn recycle_bin_stats_all() -> (u32, u64) {
    #[cfg(windows)]
    {
        let mut total_count = 0u32;
        let mut total_bytes = 0u64;
        let mut letters: Vec<String> = Vec::new();
        if let Ok(drives) = std::env::var("SystemDrive") {
            letters.push(drives.trim_end_matches('\\').to_string());
        }
        // Query the fixed volumes we know about from sysinfo.
        for d in crate::volumes::list_volumes(false) {
            let m = d.mount_point.trim_end_matches(['\\', '/']).to_string();
            if m.len() >= 2 && m.as_bytes()[1] == b':' && !letters.contains(&m[..2].to_string()) {
                letters.push(m[..2].to_string());
            }
        }
        for l in &letters {
            if let Some((c, b)) = win::recycle_bin_stats(l) {
                total_count += c;
                total_bytes += b;
            }
        }
        (total_count, total_bytes)
    }
    #[cfg(not(windows))]
    {
        // Freedesktop trash approximation for development hosts.
        let home = match crate::util::paths::home_dir() {
            Some(h) => h,
            None => return (0, 0),
        };
        let trash_dir = home.join(".local/share/Trash/files");
        let mut count = 0u32;
        let mut bytes = 0u64;
        if let Ok(rd) = std::fs::read_dir(&trash_dir) {
            for e in rd.flatten() {
                if let Ok(md) = e.metadata() {
                    count += 1;
                    bytes += md.len();
                }
            }
        }
        (count, bytes)
    }
}

/// Empty the Recycle Bin through the supported shell mechanism.
pub fn empty_recycle_bin_all() -> Result<(), crate::models::CoreError> {
    #[cfg(windows)]
    {
        win::empty_recycle_bin(true)
    }
    #[cfg(not(windows))]
    {
        let home = crate::util::paths::home_dir()
            .ok_or(crate::models::CoreError::Other("no home".into()))?;
        for sub in ["files", "info"] {
            let dir = home.join(format!(".local/share/Trash/{sub}"));
            if dir.is_dir() {
                std::fs::remove_dir_all(&dir)
                    .and_then(|_| std::fs::create_dir_all(&dir))
                    .map_err(crate::models::CoreError::Io)?;
            }
        }
        Ok(())
    }
}
