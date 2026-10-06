//! Volume discovery (spec §8). Cross-platform via sysinfo, enriched with
//! Windows-native details. Never crashes on inaccessible drives.

use crate::models::{DriveKind, MediaType, VolumeInfo};
use sysinfo::Disks;

/// (available, total) for the sysinfo disk matching `path`, if found.
pub fn sysinfo_disk_stats_for(path: &str) -> Option<(u64, u64)> {
    let disks = Disks::new_with_refreshed_list();
    let key = path.replace('/', std::path::MAIN_SEPARATOR_STR).to_lowercase();
    let key_norm = key.trim_end_matches(std::path::MAIN_SEPARATOR);
    let mut best: Option<(usize, &sysinfo::Disk)> = None;
    for d in disks.list() {
        let mp = d.mount_point().to_string_lossy().to_lowercase();
        let mp_norm = mp.trim_end_matches(std::path::MAIN_SEPARATOR);
        let matched_len = if mp_norm == key_norm {
            mp_norm.len()
        } else if key_norm.starts_with(mp_norm) && mp_norm.len() > 1 {
            mp_norm.len()
        } else if mp_norm.starts_with(key_norm) && key_norm.len() > 1 {
            key_norm.len()
        } else {
            continue;
        };
        if best.map(|(l, _)| matched_len > l).unwrap_or(true) {
            best = Some((matched_len, d));
        }
    }
    best.map(|(_, d)| (d.available_space(), d.total_space()))
}

fn detect_kind(mount: &str) -> DriveKind {
    if cfg!(windows) {
        let t = mount.trim_end_matches(['\\', '/']);
        let letter = if t.len() >= 2 && t.as_bytes()[1] == b':' {
            &t[..2]
        } else {
            mount
        };
        #[cfg(windows)]
        {
            let dt = crate::fsops::win::drive_type(letter);
            return match dt {
                3 => {
                    if is_system_volume(letter) {
                        DriveKind::System
                    } else {
                        DriveKind::Fixed
                    }
                }
                2 => DriveKind::Removable,
                4 => DriveKind::Network,
                _ => DriveKind::Unknown,
            };
        }
        #[cfg(not(windows))]
        {
            let _ = letter;
            DriveKind::Unknown
        }
    } else if mount == "/" {
        DriveKind::System
    } else {
        DriveKind::Fixed
    }
}

#[allow(dead_code)]
fn is_system_volume(letter: &str) -> bool {
    #[cfg(windows)]
    {
        std::env::var("SystemDrive")
            .map(|d| d.trim_end_matches('\\').eq_ignore_ascii_case(letter))
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        let _ = letter;
        false
    }
}

/// Enumerate all volumes with real capacity data.
pub fn list_volumes(settings_scan_network: bool) -> Vec<VolumeInfo> {
    let mut out = Vec::new();
    let disks = Disks::new_with_refreshed_list();
    for d in disks.list() {
        let mount = d.mount_point().to_string_lossy().into_owned();
        let total = d.total_space();
        let free = d.available_space();
        if total == 0 {
            continue; // pseudo-filesystems (proc, sys, …)
        }
        let fs_raw = d.file_system().to_string_lossy().to_lowercase();
        let is_network = mount.starts_with("\\\\")
            || mount.starts_with("//")
            || fs_raw.contains("smb")
            || fs_raw.contains("nfs")
            || fs_raw.contains("cifs");
        let removable = d.is_removable();
        let kind = detect_kind(&mount);
        let (label, fs_name) = {
            #[cfg(windows)]
            {
                crate::fsops::win::volume_label_fs(&mount)
                    .map(|(l, f)| {
                        let f2 = if f.is_empty() {
                            d.file_system().to_string_lossy().into_owned()
                        } else {
                            f
                        };
                        (l, f2)
                    })
                    .unwrap_or_default()
            }
            #[cfg(not(windows))]
            {
                (
                    d.name().to_string_lossy().into_owned(),
                    d.file_system().to_string_lossy().into_owned(),
                )
            }
        };
        let media = if is_network {
            MediaType::Network
        } else if removable {
            MediaType::Removable
        } else {
            #[cfg(windows)]
            {
                crate::fsops::win::detect_media_type(&mount)
            }
            #[cfg(not(windows))]
            {
                MediaType::Unknown
            }
        };
        let used = total.saturating_sub(free);
        let accessible = std::path::Path::new(&mount).exists();
        let scan_eligible = accessible && total > 0 && (settings_scan_network || !is_network);
        out.push(VolumeInfo {
            mount_point: mount,
            label,
            filesystem: fs_name,
            total_bytes: total,
            used_bytes: used,
            free_bytes: free,
            used_percent: if total > 0 { used as f64 / total as f64 * 100.0 } else { 0.0 },
            kind,
            media,
            is_system: kind == DriveKind::System,
            read_only: false, // probing would cause side effects; detected on scan failure
            scan_eligible,
            ineligible_reason: if scan_eligible {
                None
            } else if is_network && !settings_scan_network {
                Some("Network drives are disabled in settings".into())
            } else {
                Some("Volume is not accessible".into())
            },
        });
    }
    out.sort_by(|a, b| {
        b.is_system
            .cmp(&a.is_system)
            .then_with(|| a.mount_point.cmp(&b.mount_point))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_at_least_one_volume() {
        let vols = list_volumes(false);
        assert!(!vols.is_empty(), "expected at least one volume");
        for v in &vols {
            assert!(v.total_bytes > 0);
            assert_eq!(v.used_bytes, v.total_bytes - v.free_bytes);
        }
    }
}
