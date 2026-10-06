//! Layered target validation (spec §56 Layers 1–4).
//!
//! The cleaner engine must call [`validate_targets`] before executing any
//! plan. Validation fails closed: an unknown target is a blocked target.

use crate::models::CoreError;
use crate::safety::protected::{under, zones, Guard};
use crate::util::paths::normalize;

/// What a cleaner rule is allowed to touch.
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// Human-readable rule id for error messages / audit log.
    pub rule_id: String,
    /// Path prefixes this rule is allowed to operate on. A target must be
    /// under at least one prefix, otherwise it is treated as *unknown* and
    /// blocked (fail closed).
    pub allowed_prefixes: Vec<String>,
    /// When true the rule is executed through a Windows-managed mechanism
    /// (DISM, powercfg, …) and does not delete files directly.
    pub windows_managed: bool,
}

/// Validate a batch of cleanup targets. Returns normalized targets.
pub fn validate_targets(ctx: &ValidationContext, targets: &[String]) -> Result<Vec<String>, CoreError> {
    if targets.is_empty() {
        return Ok(Vec::new());
    }
    let mut ok = Vec::with_capacity(targets.len());
    for t in targets {
        let n = normalize(std::path::Path::new(t));
        if n.is_empty() {
            return Err(CoreError::ProtectedPath(format!(
                "rule {}: empty target",
                ctx.rule_id
            )));
        }
        if ctx.windows_managed {
            // Windows-managed rules do not perform raw deletions; targets are
            // informational. Still block absolute zones for defense in depth.
            if let Some(zone) = absolute_zone(&n) {
                return Err(CoreError::ProtectedPath(format!(
                    "rule {}: target {} is inside absolute protected zone {} ({})",
                    ctx.rule_id, n, zone.root, zone.note
                )));
            }
            ok.push(n);
            continue;
        }
        // Layer 1 — target must not be a filesystem root or drive root.
        if is_root(&n) {
            return Err(CoreError::ProtectedPath(format!(
                "rule {}: refusing to operate on filesystem root {}",
                ctx.rule_id, n
            )));
        }
        // Layer 2 — absolute protected zones are always blocked.
        if let Some(zone) = absolute_zone(&n) {
            return Err(CoreError::ProtectedPath(format!(
                "rule {}: target {} is inside absolute protected zone {} ({})",
                ctx.rule_id, n, zone.root, zone.note
            )));
        }
        // Layer 3 — the rule's whitelist decides. Unknown → blocked.
        let allowed = ctx
            .allowed_prefixes
            .iter()
            .any(|p| under(&n, &normalize(std::path::Path::new(p))));
        if !allowed {
            return Err(CoreError::ProtectedPath(format!(
                "rule {}: target {} is not covered by the rule whitelist (unknown target — blocked)",
                ctx.rule_id, n
            )));
        }
        // Guarded zones (Program Files etc.) additionally require that the
        // whitelist prefix itself lives inside the zone and matches exactly —
        // i.e. the rule registered a deliberate sub-path, not a blanket pass.
        if let Some(zone) = guarded_zone(&n) {
            let zone_ok = ctx
                .allowed_prefixes
                .iter()
                .any(|p| {
                    let np = normalize(std::path::Path::new(p));
                    under(&np, &zone.root) && np != zone.root
                });
            if !zone_ok {
                return Err(CoreError::ProtectedPath(format!(
                    "rule {}: guarded zone {} — rule whitelist must declare exact sub-paths inside it",
                    ctx.rule_id, zone.root
                )));
            }
        }
        // Layer 4 — minimum depth (component count) sanity check.
        if component_count(&n) < 2 {
            return Err(CoreError::ProtectedPath(format!(
                "rule {}: target {} is too shallow to be a legitimate cleanup root",
                ctx.rule_id, n
            )));
        }
        ok.push(n);
    }
    Ok(ok)
}

fn is_root(p: &str) -> bool {
    let t = p.trim_end_matches(['\\', '/']);
    t.is_empty()
        || t == "/"
        || (t.len() <= 3 && t.ends_with(':'))      // "C:"
        || (t.len() == 3 && t.as_bytes()[1] == b':') // "C:\"
        || t == "\\\\?\\C:"
}

fn absolute_zone(p: &str) -> Option<&'static crate::safety::protected::ProtectedZone> {
    zones()
        .iter()
        .find(|z| z.guard == Guard::Absolute && under(p, &z.root))
}

fn guarded_zone(p: &str) -> Option<&'static crate::safety::protected::ProtectedZone> {
    zones()
        .iter()
        .find(|z| z.guard == Guard::WhitelistOnly && under(p, &z.root))
}

fn component_count(p: &str) -> usize {
    p.split(['\\', '/']).filter(|s| !s.is_empty()).count()
}

/// Validate user-initiated file operations (Explorer-style recycle/delete).
/// The user is explicitly targeting specific items, but absolute protected
/// zones and filesystem roots remain untouchable, always (spec §56 Layer 2).
pub fn validate_user_targets(targets: &[String]) -> Result<Vec<String>, CoreError> {
    let mut ok = Vec::with_capacity(targets.len());
    for t in targets {
        let n = normalize(std::path::Path::new(t));
        if n.is_empty() || is_root(&n) {
            return Err(CoreError::ProtectedPath(format!(
                "refusing to operate on filesystem root: {n}"
            )));
        }
        for z in zones() {
            if z.guard == Guard::Absolute {
                let root_key = crate::util::paths::path_key(&z.root);
                let child_key = crate::util::paths::path_key(&n);
                let root_key = root_key.trim_end_matches(['\\', '/']);
                let inside = child_key == root_key
                    || (child_key.len() > root_key.len()
                        && child_key.starts_with(root_key)
                        && matches!(child_key.as_bytes()[root_key.len()], b'\\' | b'/'));
                if inside {
                    return Err(CoreError::ProtectedPath(format!(
                        "protected system location ({})",
                        z.note
                    )));
                }
            }
        }
        ok.push(n);
    }
    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(prefixes: &[&str]) -> ValidationContext {
        ValidationContext {
            rule_id: "test.rule".into(),
            allowed_prefixes: prefixes.iter().map(|s| s.to_string()).collect(),
            windows_managed: false,
        }
    }

    // ---- Safety regression suite (spec §72) -------------------------------
    // Windows-zone regression tests run on Windows; the same validator code
    // paths are exercised cross-platform through the unix zone set below.

    #[cfg(windows)]
    #[test]
    fn blocks_winsxs() {
        let c = ctx(&["C:\\Windows\\Temp"]);
        assert!(validate_targets(&c, &["C:\\Windows\\WinSxS\\x".to_string()]).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn blocks_system32() {
        let c = ctx(&["C:\\Windows\\System32"]);
        assert!(validate_targets(&c, &["C:\\Windows\\System32\\config\\SOFTWARE".to_string()]).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn blocks_boot() {
        let c = ctx(&["C:\\Windows\\Boot"]);
        assert!(validate_targets(&c, &["C:\\Windows\\Boot\\EFI".to_string()]).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn blocks_program_files_without_declared_subpath() {
        let c = ctx(&["C:\\Windows\\Temp"]);
        assert!(validate_targets(&c, &["C:\\Program Files\\SomeApp\\cache".to_string()]).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn guarded_zone_allows_exact_declared_subpath() {
        let c = ctx(&["C:\\ProgramData\\Microsoft\\Windows\\WER"]);
        let out = validate_targets(
            &c,
            &["C:\\ProgramData\\Microsoft\\Windows\\WER\\ReportQueue".to_string()],
        );
        assert!(out.is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn blocks_absolute_zone_etc() {
        // /etc is an absolute zone in the unix zone set.
        let c = ctx(&["/var/tmp-cache"]);
        assert!(validate_targets(&c, &["/etc/passwd".to_string()]).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn guarded_zone_var_rules() {
        // Whitelist inside /var that is not /var itself → allowed.
        let c = ctx(&["/var/tmp-cache"]);
        assert!(validate_targets(&c, &["/var/tmp-cache/sub".to_string()]).is_ok());
        // Whitelist not inside /var → guarded zone rejects.
        let c2 = ctx(&["/home/user/cache"]);
        assert!(validate_targets(&c2, &["/var/data".to_string()]).is_err());
    }

    #[test]
    fn blocks_unknown_target() {
        // Target exists but is not covered by the rule whitelist → blocked.
        let c = ctx(&["/home/bob/tmp"]);
        assert!(validate_targets(&c, &["/home/other/folder".to_string()]).is_err());
    }

    #[test]
    fn blocks_root_and_drive() {
        let c = ctx(&["C:\\", "/"]);
        assert!(validate_targets(&c, &["C:\\".to_string()]).is_err());
        assert!(validate_targets(&c, &["/".to_string()]).is_err());
    }

    #[test]
    fn allows_declared_subpath() {
        let c = ctx(&["/home/bob/tmp"]);
        let out = validate_targets(&c, &["/home/bob/tmp/x".to_string()])
            .expect("declared sub-path must pass");
        assert_eq!(out.len(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn windows_managed_still_blocks_absolute_zones() {
        let mut c = ctx(&[]);
        c.windows_managed = true;
        assert!(validate_targets(&c, &["C:\\Windows\\WinSxS".to_string()]).is_err());
    }

    #[test]
    fn empty_targets_ok() {
        let c = ctx(&["C:\\x"]);
        assert_eq!(validate_targets(&c, &[]).unwrap().len(), 0);
    }
}
