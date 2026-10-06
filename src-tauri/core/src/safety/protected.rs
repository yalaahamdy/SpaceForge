//! The protected-path model (spec §39, §56 Layer 2).
//!
//! Zones marked [`Guard::Absolute`] are never touched by SpaceForge —
//! no rule, no user click, no "deep clean" can target them.
//! Zones marked [`Guard::WhitelistOnly`] reject everything except
//! rule-declared exact sub-paths (e.g. the Windows Error Reporting queue
//! under ProgramData), and only through the layered validator.

use crate::models::Protection;
use crate::util::paths::path_key;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guard {
    /// Nothing under this zone may ever be modified or deleted.
    Absolute,
    /// Only exact sub-paths registered by an approved cleaner rule may be
    /// touched; everything else under the zone is blocked.
    WhitelistOnly,
}

#[derive(Debug, Clone)]
pub struct ProtectedZone {
    /// Normalized zone root, e.g. `C:\Windows\WinSxS`.
    pub root: String,
    pub guard: Guard,
    pub protection: Protection,
    pub note: &'static str,
}

fn win(root: &str, guard: Guard, protection: Protection, note: &'static str) -> ProtectedZone {
    ProtectedZone {
        root: root.to_string(),
        guard,
        protection,
        note,
    }
}

/// Windows drive root — resolved once; defaults to `C:` if detection fails.
pub fn system_drive() -> String {
    static DRIVE: OnceLock<String> = OnceLock::new();
    DRIVE
        .get_or_init(|| {
            if cfg!(windows) {
                if let Some(windir) = std::env::var_os("SystemRoot") {
                    let w = windir.to_string_lossy().into_owned();
                    if w.len() >= 2 {
                        return w[..2].to_string();
                    }
                }
                "C:".to_string()
            } else {
                "/".to_string()
            }
        })
        .clone()
}

/// Compute the protected zone list for the current machine.
pub fn zones() -> &'static Vec<ProtectedZone> {
    zones_inner()
}

fn zones_inner() -> &'static Vec<ProtectedZone> {
    static ZONES: OnceLock<Vec<ProtectedZone>> = OnceLock::new();
    ZONES.get_or_init(|| {
        let d = system_drive();
        let w = format!("{d}\\Windows");
        if cfg!(windows) {
            vec![
                win(&format!("{w}\\WinSxS"), Guard::Absolute, Protection::SystemProtected,
                    "Windows component store — only DISM may service it"),
                win(&format!("{w}\\System32"), Guard::Absolute, Protection::SystemProtected,
                    "Core operating system directory"),
                win(&format!("{w}\\SysWOW64"), Guard::Absolute, Protection::SystemProtected,
                    "32-bit compatibility system directory"),
                win(&format!("{w}\\Sysnative"), Guard::Absolute, Protection::SystemProtected,
                    "File-system redirection alias"),
                win(&format!("{w}\\Boot"), Guard::Absolute, Protection::SystemProtected,
                    "Boot configuration data"),
                win(&format!("{w}\\Fonts"), Guard::Absolute, Protection::SystemProtected,
                    "System fonts"),
                win(&format!("{w}\\Servicing"), Guard::Absolute, Protection::SystemProtected,
                    "Servicing stack"),
                win(&format!("{w}\\CSC"), Guard::Absolute, Protection::SystemProtected,
                    "Offline files cache"),
                win(&format!("{w}\\config"), Guard::Absolute, Protection::SystemProtected,
                    "Registry hives (SOFTWARE, SYSTEM, SAM, SECURITY)"),
                win(&format!("{w}\\registration"), Guard::Absolute, Protection::SystemProtected,
                    "COM registration database"),
                win(&w, Guard::WhitelistOnly, Protection::WindowsManaged,
                    "Windows directory — only rule-declared sub-paths may be touched"),
                win(&format!("{d}\\Program Files"), Guard::WhitelistOnly, Protection::ApplicationManaged,
                    "Installed applications"),
                win(&format!("{d}\\Program Files (x86)"), Guard::WhitelistOnly, Protection::ApplicationManaged,
                    "32-bit installed applications"),
                win(&format!("{d}\\ProgramData"), Guard::WhitelistOnly, Protection::ApplicationManaged,
                    "Shared application data"),
            ]
        } else {
            // Unix-like hosts (only relevant for development/test runs).
            vec![
                win("/etc", Guard::Absolute, Protection::SystemProtected, "System configuration"),
                win("/usr", Guard::Absolute, Protection::SystemProtected, "System binaries"),
                win("/bin", Guard::Absolute, Protection::SystemProtected, "System binaries"),
                win("/boot", Guard::Absolute, Protection::SystemProtected, "Boot files"),
                win("/var", Guard::WhitelistOnly, Protection::ApplicationManaged, "Variable data"),
            ]
        }
    })
}

/// Classify any path into a protection category (spec §39 categories).
pub fn classify_protection(path: &str) -> Protection {
    let key = path_key(path);
    for z in zones() {
        if under(path, &z.root) {
            return match z.protection {
                Protection::SystemProtected => Protection::SystemProtected,
                Protection::WindowsManaged => Protection::WindowsManaged,
                Protection::ApplicationManaged => Protection::ApplicationManaged,
                _ => z.protection,
            };
        }
    }
    // Known safe-ish buckets by path shape.
    let lower = key.to_lowercase();
    if lower.contains("\\temp\\") || lower.ends_with("\\temp") || lower.contains("/tmp/") {
        return Protection::Temporary;
    }
    if lower.contains("cache") {
        return Protection::SafeCache;
    }
    if lower.contains("\\users\\") || lower.contains("/home/") {
        return Protection::UserData;
    }
    Protection::Unknown
}

pub fn under(child: &str, root: &str) -> bool {
    crate::util::paths::is_under(child, root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn winsxs_is_absolute() {
        assert_eq!(
            classify_protection("C:\\Windows\\WinSxS\\amd64_something"),
            Protection::SystemProtected
        );
    }

    #[cfg(windows)]
    #[test]
    fn program_files_guarded() {
        assert_eq!(
            classify_protection("C:\\Program Files\\SomeApp\\app.exe"),
            Protection::ApplicationManaged
        );
    }

    #[cfg(unix)]
    #[test]
    fn root_zones_classify() {
        assert_eq!(classify_protection("/etc/passwd"), Protection::SystemProtected);
        assert_eq!(classify_protection("/usr/bin/ls"), Protection::SystemProtected);
    }

    #[test]
    fn user_data() {
        assert_eq!(
            classify_protection("C:\\Users\\bob\\Documents\\report.docx"),
            Protection::UserData
        );
    }

    #[test]
    fn temp_bucket() {
        assert_eq!(
            classify_protection("C:\\Users\\bob\\AppData\\Local\\Temp\\x.tmp"),
            Protection::Temporary
        );
    }
}
