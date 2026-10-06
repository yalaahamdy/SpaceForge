//! Environment-variable path expansion for rule declarations.
//!
//! Rules declare paths with `%VAR%` segments; expansion is explicit and
//! testable (never shell-interpolated).

use std::path::PathBuf;

/// Expand `%VAR%` (Windows style) and `${VAR}` in a path template.
/// Unknown variables expand to empty strings — callers must treat
/// resulting paths as non-existent and return no locations.
pub fn expand(template: &str) -> String {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if let Some(end) = template[i + 1..].find('%') {
                let var = &template[i + 1..i + 1 + end];
                out.push_str(&std::env::var(var).unwrap_or_default());
                i += end + 2;
                continue;
            }
        }
        if bytes[i] == b'$' && bytes.get(i + 1) == Some(&b'{') {
            if let Some(end) = template[i + 2..].find('}') {
                let var = &template[i + 2..i + 2 + end];
                out.push_str(&std::env::var(var).unwrap_or_default());
                i += end + 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    // Non-ASCII safety: rebuild from the original via char iteration when
    // multi-byte content is present.
    if !template.is_ascii() {
        return expand_chars(template);
    }
    out
}

fn expand_chars(template: &str) -> String {
    let mut out = String::with_capacity(template.len());
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '%') {
                let var: String = chars[i + 1..i + 1 + end].iter().collect();
                out.push_str(&std::env::var(&var).unwrap_or_default());
                i += end + 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Home directory with Windows naming (USERPROFILE).
pub fn home() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// LocalAppData equivalent across platforms.
pub fn local_app_data() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local/share"))
}

/// Roaming AppData equivalent.
pub fn roaming_app_data() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".config"))
}

/// ProgramData equivalent.
pub fn program_data() -> PathBuf {
    std::env::var_os("PROGRAMDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/lib"))
}

/// The Windows directory (C:\Windows), or "/" on other systems.
pub fn windows_dir() -> PathBuf {
    std::env::var_os("SystemRoot")
        .or_else(|| std::env::var_os("WINDIR"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_known_vars() {
        std::env::set_var("SF_TEST_VAR", "hello");
        assert_eq!(expand("%SF_TEST_VAR%/sub"), "hello/sub");
    }

    #[test]
    fn unknown_var_expands_empty() {
        std::env::remove_var("SF_MISSING_VAR_9");
        let r = expand("%SF_MISSING_VAR_9%/x");
        assert_eq!(r, "/x");
    }

    #[test]
    fn plain_path_untouched() {
        assert_eq!(expand("C:\\plain\\path"), "C:\\plain\\path");
    }
}
