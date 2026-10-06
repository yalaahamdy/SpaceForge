//! Generic rule implementations reused by all rule categories.

use super::expand::expand;
use super::measure;
use super::{CleanContext, CleanerRuleDef};
use crate::models::*;

/// A rule that cleans the contents of one or more declared directories.
pub struct PathCleanupRule {
    pub rule: CleanerRule,
    /// Path templates; may contain %VARS%. Each expands to one target dir.
    pub templates: Vec<String>,
    /// Remove directory entries but keep the directory itself.
    pub clear_contents: bool,
    /// Only remove files older than this (days). None = all.
    pub max_age_days: Option<u64>,
    /// Only remove entries whose file name matches this glob (top level).
    pub pattern: Option<String>,
}

impl PathCleanupRule {
    fn expand_all(&self) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        for t in &self.templates {
            let expanded = expand(t);
            if expanded.contains('*') || expanded.contains('?') {
                if let Ok(paths) = glob::glob(&expanded.replace('\\', "/")) {
                    for p in paths.flatten() {
                        out.push(p);
                    }
                }
            } else if !expanded.is_empty() {
                out.push(std::path::PathBuf::from(expanded));
            }
        }
        out
    }

    fn matching_entries(&self, dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        let Ok(rd) = std::fs::read_dir(dir) else {
            return out;
        };
        let glob = self.pattern.as_deref().map(|p| {
            globset::GlobBuilder::new(p)
                .case_insensitive(true)
                .build()
                .map(|g| globset::GlobSetBuilder::new().add(g).build().expect("globset"))
                .ok()
        });
        let cutoff = self.max_age_days.map(|days| {
            std::time::SystemTime::now()
                - std::time::Duration::from_secs(days * 24 * 3600)
        });
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(Some(gs)) = glob.as_ref().map(|g: &Option<globset::GlobSet>| g.as_ref()) {
                if !gs.is_match(&name) {
                    continue;
                }
            }
            if let (Some(cutoff), Ok(md)) = (cutoff, entry.metadata()) {
                let mtime = md.modified().unwrap_or(std::time::SystemTime::now());
                if mtime > cutoff {
                    continue;
                }
            }
            out.push(entry.path());
        }
        out
    }
}

impl CleanerRuleDef for PathCleanupRule {
    fn meta(&self) -> &CleanerRule {
        &self.rule
    }

    fn allowed_prefixes(&self) -> Vec<String> {
        self.expand_all()
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    fn discover(&self, _ctx: &CleanContext) -> Option<CleanupOpportunity> {
        let dirs = self.expand_all();
        let existing: Vec<String> = dirs
            .iter()
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        if existing.is_empty() {
            return None;
        }
        let mut item_count = 0u64;
        let mut current_size = 0u64;
        for dir in &existing {
            let p = std::path::Path::new(dir);
            if p.is_file() {
                if let Ok(md) = std::fs::metadata(p) {
                    item_count += 1;
                    current_size += md.len();
                }
                continue;
            }
            match (&self.pattern, self.max_age_days) {
                (None, None) => {
                    let (c, b) = measure::measure_dir(std::path::Path::new(dir));
                    item_count += c;
                    current_size += b;
                }
                _ => {
                    for entry in self.matching_entries(std::path::Path::new(dir)) {
                        if entry.is_file() {
                            if let Ok(md) = std::fs::metadata(&entry) {
                                item_count += 1;
                                current_size += md.len();
                            }
                        } else {
                            let (c, b) = measure::measure_dir(&entry);
                            item_count += c;
                            current_size += b;
                        }
                    }
                }
            }
        }
        Some(CleanupOpportunity {
            rule_id: self.rule.id.clone(),
            rule: self.rule.clone(),
            locations: existing,
            item_count,
            current_size,
            reclaimable_size: current_size,
            confidence: 90,
            discovered_at: chrono::Utc::now(),
            blocked_reason: None,
        })
    }

    fn targets(&self, _ctx: &CleanContext) -> Vec<String> {
        self.expand_all()
            .into_iter()
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    fn execute(&self, targets: &[String], _ctx: &CleanContext) -> CleanupEntryResult {
        let mut res = CleanupEntryResult {
            rule_id: self.rule.id.clone(),
            status: CleanupEntryStatus::Success,
            deleted_items: 0,
            failed_items: 0,
            skipped_items: 0,
            reclaimed_bytes: 0,
            failures: vec![],
            skipped_reasons: vec![],
        };
        for t in targets {
            let p = std::path::Path::new(t);
            if p.is_file() {
                // Direct file target (privacy rules delete History/Cookies DBs).
                let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
                match std::fs::remove_file(p) {
                    Ok(_) => {
                        res.deleted_items += 1;
                        res.reclaimed_bytes += size;
                    }
                    Err(e) => {
                        res.failed_items += 1;
                        res.failures.push(format!("{t}: {e}"));
                    }
                }
                continue;
            }
            if self.clear_contents {
                let before = measure::measure_dir(std::path::Path::new(t)).1;
                match crate::fsops::delete::clear_dir_contents(t) {
                    Ok((deleted, failed)) => {
                        res.deleted_items += deleted;
                        res.failed_items += failed;
                        let after = measure::measure_dir(std::path::Path::new(t)).1;
                        res.reclaimed_bytes += before.saturating_sub(after);
                    }
                    Err(e) => {
                        res.failed_items += 1;
                        res.failures.push(format!("{t}: {e}"));
                    }
                }
            } else {
                for entry in self.matching_entries(std::path::Path::new(t)) {
                    let size = std::fs::metadata(&entry).map(|m| m.len()).unwrap_or(0);
                    let is_dir = entry.is_dir();
                    let r = if is_dir {
                        std::fs::remove_dir_all(&entry)
                    } else {
                        std::fs::remove_file(&entry)
                    };
                    match r {
                        Ok(_) => {
                            res.deleted_items += 1;
                            res.reclaimed_bytes += size;
                        }
                        Err(e) => {
                            res.failed_items += 1;
                            res.failures.push(format!("{}: {e}", entry.display()));
                        }
                    }
                }
            }
        }
        if res.failed_items > 0 && res.deleted_items == 0 {
            res.status = CleanupEntryStatus::Failed;
        } else if res.failed_items > 0 {
            res.status = CleanupEntryStatus::Partial;
        }
        res
    }

    fn mechanism(&self) -> CleanupMechanism {
        CleanupMechanism::Direct
    }
}

/// A rule executed through a supported Windows mechanism (PowerShell/DISM/
/// powercfg/vssadmin). It never deletes files directly.
pub struct CommandCleanupRule {
    pub rule: CleanerRule,
    /// The command to run: (program, args). Executed elevated when required.
    pub program: String,
    pub args: Vec<String>,
    /// Paths measured for discovery (may be None when unmeasurable).
    pub discovery_paths: Vec<String>,
    /// Expected post-condition description for verification.
    pub verify_note: String,
}

impl CommandCleanupRule {
    fn run(&self, ctx: &CleanContext) -> Result<String, String> {
        if !cfg!(windows) {
            return Err("Windows-managed operations require Windows".into());
        }
        if self.rule.requires_admin && !ctx.elevated {
            return Err("requires elevation".into());
        }
        super::cmds::run_command(&self.program, &self.args, self.rule.requires_admin)
    }
}

impl CleanerRuleDef for CommandCleanupRule {
    fn meta(&self) -> &CleanerRule {
        &self.rule
    }

    fn allowed_prefixes(&self) -> Vec<String> {
        // Windows-managed rules do not delete files; empty whitelist and
        // windows_managed=true keeps the validator in managed mode.
        vec![]
    }

    fn discover(&self, ctx: &CleanContext) -> Option<CleanupOpportunity> {
        if self.discovery_paths.is_empty() {
            // No measurable target — expose the rule with 0 bytes so the UI
            // can still offer the (Windows-managed) operation.
            return Some(CleanupOpportunity {
                rule_id: self.rule.id.clone(),
                rule: self.rule.clone(),
                locations: vec![],
                item_count: 0,
                current_size: 0,
                reclaimable_size: 0,
                confidence: 85,
                discovered_at: chrono::Utc::now(),
                blocked_reason: None,
            });
        }
        let paths: Vec<std::path::PathBuf> = self
            .discovery_paths
            .iter()
            .map(|t| std::path::PathBuf::from(expand(t)))
            .collect();
        let existing = measure::existing_dirs(&paths);
        let measures = measure::measure_many(&paths);
        let item_count: u64 = measures.iter().map(|(c, _)| c).sum();
        let current_size: u64 = measures.iter().map(|(_, b)| b).sum();
        if existing.is_empty() && current_size == 0 {
            return None;
        }
        let _ = ctx;
        Some(CleanupOpportunity {
            rule_id: self.rule.id.clone(),
            rule: self.rule.clone(),
            locations: existing,
            item_count,
            current_size,
            reclaimable_size: current_size,
            confidence: 85,
            discovered_at: chrono::Utc::now(),
            blocked_reason: None,
        })
    }

    fn targets(&self, _ctx: &CleanContext) -> Vec<String> {
        vec![] // informational only; execution goes through the mechanism
    }

    fn execute(&self, _targets: &[String], ctx: &CleanContext) -> CleanupEntryResult {
        let mut res = CleanupEntryResult {
            rule_id: self.rule.id.clone(),
            status: CleanupEntryStatus::Failed,
            deleted_items: 0,
            failed_items: 0,
            skipped_items: 0,
            reclaimed_bytes: 0,
            failures: vec![],
            skipped_reasons: vec![],
        };
        match self.run(ctx) {
            Ok(out) => {
                res.status = CleanupEntryStatus::Success;
                res.skipped_reasons.push(format!(
                    "Windows-managed operation completed. {}",
                    self.verify_note
                ));
                if !out.trim().is_empty() {
                    res.skipped_reasons.push(format!("Output: {}", out.trim()));
                }
            }
            Err(e) => {
                res.failures.push(e);
            }
        }
        res
    }

    fn mechanism(&self) -> CleanupMechanism {
        CleanupMechanism::WindowsManaged
    }
}
