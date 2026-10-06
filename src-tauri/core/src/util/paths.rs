//! Path normalization and manipulation helpers.
//!
//! All engines operate on normalized, absolute paths. Windows paths are
//! compared case-insensitively; separator style is accepted in both forms.

use std::path::{Path, PathBuf};

/// Normalize a path: duplicated separators collapsed, style preserved.
pub fn normalize(path: &Path) -> String {
    let p = path.to_string_lossy();
    let mut out = String::with_capacity(p.len());
    let mut prev_sep = false;
    for (i, ch) in p.chars().enumerate() {
        let is_sep = ch == '\\' || ch == '/';
        if is_sep {
            // Preserve leading separators (POSIX root / UNC prefix).
            if i < 2 {
                out.push(ch);
            } else if !prev_sep {
                out.push(ch);
            }
            prev_sep = true;
        } else {
            out.push(ch);
            prev_sep = false;
        }
    }
    // Trim trailing separators ("C:\a\" → "C:\a"); keep "/" intact.
    while out.len() > 1 && (out.ends_with('\u{5c}') || out.ends_with('/')) {
        out.pop();
    }
    out
}

/// Case-folded comparison key for path equality.
pub fn path_key(path: &str) -> String {
    if cfg!(windows) {
        path.to_lowercase()
    } else {
        path.to_string()
    }
}

/// True when `child` equals or is located under `root`.
pub fn is_under(child: &str, root: &str) -> bool {
    let c = path_key(child);
    let r = path_key(root);
    let r = r.trim_end_matches(['\\', '/']);
    if c == r {
        return true;
    }
    if c.len() <= r.len() || !c.starts_with(&r) {
        return false;
    }
    let rest = &c[r.len()..];
    rest.starts_with('\\') || rest.starts_with('/')
}

/// Parent path string; empty when no parent. Platform-independent:
/// accepts both separators so Windows-style strings work on any host.
pub fn parent_of(path: &str) -> String {
    let t = path.trim_end_matches(['\\', '/']);
    if t.is_empty() {
        return String::new();
    }
    match t.rfind(['\\', '/']) {
        Some(idx) if idx == 0 => t[..1].to_string(),
        Some(idx) => t[..idx].to_string(),
        None => String::new(),
    }
}

/// Last component of a path. Platform-independent.
pub fn name_of(path: &str) -> String {
    let t = path.trim_end_matches(['\\', '/']);
    match t.rfind(['\\', '/']) {
        Some(idx) => t[idx + 1..].to_string(),
        None => t.to_string(),
    }
}

/// Canonical aggregation key: trailing separators trimmed ("C:\" → "C:").
pub fn canon(path: &str) -> String {
    let t = path.trim_end_matches(['\\', '/']);
    if t.is_empty() {
        path.to_string() // "/" or "\\" stays as-is
    } else {
        t.to_string()
    }
}

/// Home directory, best effort.
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_collapses() {
        assert_eq!(normalize(Path::new("C:\\\\a\\\\b\\\\")), "C:\\a\\b");
        assert_eq!(normalize(Path::new("/a//b")), "/a/b");
    }

    #[test]
    fn is_under_checks() {
        assert!(is_under("C:\\Windows\\System32\\x.dll", "C:\\Windows"));
        assert!(is_under("C:\\Windows", "C:\\Windows"));
        assert!(!is_under("C:\\WindowsFoo\\bar", "C:\\Windows"));
        assert!(!is_under("D:\\stuff", "C:\\Windows"));
        assert!(is_under("/home/user/x", "/home/user"));
        assert!(!is_under("/home/username/x", "/home/user"));
    }

    #[test]
    fn parents_and_names() {
        assert_eq!(parent_of("C:\\a\\b\\c.txt"), "C:\\a\\b");
        assert_eq!(name_of("C:\\a\\b\\c.txt"), "c.txt");
    }
}
