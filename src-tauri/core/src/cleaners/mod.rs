//! The modular cleaner rule engine (spec §33).
//!
//! Every cleanup action in SpaceForge is performed by a *rule*: a
//! self-describing unit with detection logic, whitelist, risk level,
//! execution mechanism and validation. Rules never hardcode behavior into
//! the UI, and every execution passes through the layered safety
//! validator — fail closed (spec §56).

pub mod cmds;
pub mod expand;
pub mod generic;
pub mod measure;
pub mod recycle_bin;
pub mod rules_apps;
pub mod rules_browsers;
pub mod rules_dev;
pub mod rules_windows;

use crate::models::*;
use crate::safety::validator::{validate_targets, ValidationContext};
use std::sync::Arc;

/// Execution context handed to rules.
pub struct CleanContext {
    pub elevated: bool,
    pub settings: AppSettings,
}

/// A cleaner rule implementation.
pub trait CleanerRuleDef: Send + Sync {
    /// Static metadata for UI display and audits.
    fn meta(&self) -> &CleanerRule;
    /// Whitelisted path prefixes (the ONLY paths this rule may touch).
    fn allowed_prefixes(&self) -> Vec<String>;
    /// Detect current size/eligibility. Returns None when the rule is not
    /// applicable on this machine (app absent, path missing…).
    fn discover(&self, ctx: &CleanContext) -> Option<CleanupOpportunity>;
    /// Concrete targets for execution (re-resolved at execution time).
    fn targets(&self, ctx: &CleanContext) -> Vec<String>;
    /// Execute the cleanup on already-validated targets.
    fn execute(&self, targets: &[String], ctx: &CleanContext) -> CleanupEntryResult;
    /// How the rule executes (direct / recycle / windows-managed).
    fn mechanism(&self) -> CleanupMechanism;
}

/// Measure directories; used by many rules.
pub struct Discovered {
    pub locations: Vec<String>,
    pub item_count: u64,
    pub current_size: u64,
}

pub struct Registry {
    pub rules: Vec<Arc<dyn CleanerRuleDef>>,
}

impl Registry {
    pub fn all() -> Registry {
        let mut rules: Vec<Arc<dyn CleanerRuleDef>> = Vec::new();
        rules_windows::register(&mut rules);
        rules_apps::register(&mut rules);
        rules_browsers::register(&mut rules);
        rules_dev::register(&mut rules);
        Registry { rules }
    }

    pub fn metas(&self) -> Vec<CleanerRule> {
        self.rules.iter().map(|r| r.meta().clone()).collect()
    }

    /// Discover all applicable opportunities with real measured sizes.
    pub fn discover_all(&self, ctx: &CleanContext) -> Vec<CleanupOpportunity> {
        #[cfg_attr(not(windows), allow(unused_imports))]
        use rayon::prelude::*;
        let running = crate::fsops::running_processes();
        let mut opps: Vec<CleanupOpportunity> = self
            .rules
            .par_iter()
            .filter_map(|rule| {
                let meta = rule.meta();
                // Opt-in rules stay hidden until the user enables them.
                if meta.opt_in && !ctx.settings.enable_opt_in_cleaners {
                    return None;
                }
                let mut opp = rule.discover(ctx)?;
                // Blocked when elevation is required but absent.
                if meta.requires_admin && !ctx.elevated {
                    opp.blocked_reason =
                        Some("Requires administrator privileges".into());
                }
                // Blocked when a conflicting process is running.
                if let Some(p) = meta
                    .conflicts_with_processes
                    .iter()
                    .find(|p| running.contains(&p.to_ascii_lowercase()))
                {
                    opp.blocked_reason = Some(format!(
                        "'{p}' is currently running — close it before cleaning"
                    ));
                }
                if opp.current_size == 0 && opp.rule.windows_managed == false {
                    return None; // nothing to clean
                }
                Some(opp)
            })
            .collect();
        opps.sort_by(|a, b| {
            b.reclaimable_size
                .cmp(&a.reclaimable_size)
                .then_with(|| a.rule.risk.cmp(&b.rule.risk))
        });
        opps
    }

    /// Build a plan from the given opportunity ids (already UI-filtered).
    pub fn build_plan(
        &self,
        profile: CleanupProfile,
        opps: &[CleanupOpportunity],
        ctx: &CleanContext,
    ) -> CoreResult<CleanupPlan> {
        let mut entries = Vec::new();
        let mut estimated = 0u64;
        for opp in opps {
            // Profile gating — Deep Clean never silently enables advanced.
            if profile != CleanupProfile::Custom {
                let max_risk = match profile {
                    CleanupProfile::SmartClean => RiskLevel::Safe,
                    CleanupProfile::StandardClean => RiskLevel::Low,
                    CleanupProfile::DeepClean => RiskLevel::Review,
                    _ => RiskLevel::Dangerous,
                };
                if opp.rule.risk > max_risk {
                    continue;
                }
            }
            let rule = self
                .rules
                .iter()
                .find(|r| r.meta().id == opp.rule_id)
                .ok_or_else(|| CoreError::NotFound(opp.rule_id.clone()))?;
            let targets = rule.targets(ctx);
            if targets.is_empty() && rule.mechanism() != CleanupMechanism::WindowsManaged {
                continue;
            }
            estimated += opp.reclaimable_size;
            entries.push(CleanupPlanEntry {
                rule_id: opp.rule_id.clone(),
                rule_name: opp.rule.name.clone(),
                targets,
                estimated_reclaim: opp.reclaimable_size,
                item_count: opp.item_count,
                risk: opp.rule.risk,
                reversibility: opp.rule.reversibility,
                mechanism: rule.mechanism(),
            });
        }
        Ok(CleanupPlan {
            id: format!("plan-{}", chrono::Utc::now().timestamp_millis()),
            profile,
            created_at: chrono::Utc::now(),
            entries,
            estimated_reclaim: estimated,
        })
    }

    /// Execute a plan as a validated transaction (spec §36).
    /// `on_progress` receives (completed_entries, total_entries, current_rule).
    pub fn execute_plan(
        &self,
        plan: &CleanupPlan,
        ctx: &CleanContext,
        on_progress: &(dyn Fn(u64, u64, &str) + Send + Sync),
    ) -> CleanupReport {
        let free_before = crate::fsops::volume_stats(&primary_volume())
            .map(|(f, _, _)| f)
            .unwrap_or(0);
        let started = chrono::Utc::now();
        let start = std::time::Instant::now();
        let total = plan.entries.len() as u64;

        // Sequential execution: cleanup touches system state; parallel
        // deletion across rules would make progress and failures harder to
        // attribute. Per-rule item processing is where parallelism lives.
        let mut results = Vec::with_capacity(plan.entries.len());
        for (i, entry) in plan.entries.iter().enumerate() {
            on_progress(i as u64, total, &entry.rule_name);
            let rule = self
                .rules
                .iter()
                .find(|r| r.meta().id == entry.rule_id);
            let Some(rule) = rule else {
                results.push(CleanupEntryResult {
                    rule_id: entry.rule_id.clone(),
                    status: CleanupEntryStatus::Skipped,
                    deleted_items: 0,
                    failed_items: 0,
                    skipped_items: 0,
                    reclaimed_bytes: 0,
                    failures: vec!["rule vanished from registry".into()],
                    skipped_reasons: vec![],
                });
                continue;
            };
            // Layer 3+4 — re-validate immediately before execution.
            let vctx = ValidationContext {
                rule_id: entry.rule_id.clone(),
                allowed_prefixes: rule.allowed_prefixes(),
                windows_managed: matches!(rule.mechanism(), CleanupMechanism::WindowsManaged),
            };
            let validated = match validate_targets(&vctx, &entry.targets) {
                Ok(t) => t,
                Err(e) => {
                    results.push(CleanupEntryResult {
                        rule_id: entry.rule_id.clone(),
                        status: CleanupEntryStatus::Blocked,
                        deleted_items: 0,
                        failed_items: 0,
                        skipped_items: 0,
                        reclaimed_bytes: 0,
                        failures: vec![e.to_string()],
                        skipped_reasons: vec![],
                    });
                    continue;
                }
            };
            // Layer 5 — verify targets still exist (files may vanish).
            let existing: Vec<String> = validated
                .iter()
                .filter(|t| std::path::Path::new(t.as_str()).exists())
                .cloned()
                .collect();
            if existing.is_empty()
                && !matches!(rule.mechanism(), CleanupMechanism::WindowsManaged)
            {
                results.push(CleanupEntryResult {
                    rule_id: entry.rule_id.clone(),
                    status: CleanupEntryStatus::Skipped,
                    deleted_items: 0,
                    failed_items: 0,
                    skipped_items: validated.len() as u64,
                    reclaimed_bytes: 0,
                    failures: vec![],
                    skipped_reasons: vec!["targets no longer exist".into()],
                });
                continue;
            }
            let mut res = rule.execute(&existing, ctx);
            // Layer 6 — post-execution verification: recompute what remains.
            if res.status == CleanupEntryStatus::Success {
                let remaining = existing
                    .iter()
                    .filter(|t| std::path::Path::new(t.as_str()).exists())
                    .count();
                if remaining > 0 && rule.mechanism() == CleanupMechanism::Direct {
                    res.status = CleanupEntryStatus::Partial;
                    res
                        .skipped_reasons
                        .push(format!("{remaining} target(s) could not be fully removed (locked?)"));
                }
            }
            results.push(res);
        }
        on_progress(total, total, "");
        let finished = chrono::Utc::now();
        let free_after = crate::fsops::volume_stats(&primary_volume())
            .map(|(f, _, _)| f)
            .unwrap_or(0);
        CleanupReport {
            plan_id: plan.id.clone(),
            profile: plan.profile,
            started_at: started,
            finished_at: finished,
            duration_ms: start.elapsed().as_millis() as u64,
            estimated_reclaim: plan.estimated_reclaim,
            actual_reclaim: results.iter().map(|r| r.reclaimed_bytes).sum(),
            entries: results,
            free_before,
            free_after,
        }
    }
}

/// The volume whose free space we report for cleanup impact.
fn primary_volume() -> String {
    if cfg!(windows) {
        std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into())
    } else {
        "/".into()
    }
}

/// Recommendation engine (spec §34): deterministic, explainable ranking.
pub fn recommend(
    opps: &[CleanupOpportunity],
    duplicate_waste: u64,
    large_old_files_bytes: u64,
) -> Vec<Recommendation> {
    let mut recs = Vec::new();
    for o in opps.iter().take(6) {
        let priority = match o.rule.risk {
            RiskLevel::Safe => "high",
            RiskLevel::Low => "medium",
            _ => "review",
        };
        recs.push(Recommendation {
            title: format!("{} — {}", o.rule.name, crate::util::format_bytes_short(o.reclaimable_size)),
            detail: format!(
                "{} Confidence: {}%",
                o.rule.reason, o.confidence
            ),
            priority: priority.into(),
            size_bytes: o.reclaimable_size,
            target: match o.rule.family {
                CleanupFamily::Windows => "windows-cleanup".into(),
                CleanupFamily::Browser => "browser-cleanup".into(),
                CleanupFamily::Developer => "developer-cleanup".into(),
                CleanupFamily::Application => "app-cleanup".into(),
                CleanupFamily::Privacy => "cleanup-center".into(),
            },
            confidence: o.confidence,
        });
    }
    if duplicate_waste > 1024 * 1024 * 1024 {
        recs.push(Recommendation {
            title: format!(
                "Duplicate content — {} wasted",
                crate::util::format_bytes_short(duplicate_waste)
            ),
            detail: "Files with identical content exist in multiple locations. Review duplicate groups and keep one copy.".into(),
            priority: "review".into(),
            size_bytes: duplicate_waste,
            target: "duplicates".into(),
            confidence: 95,
        });
    }
    if large_old_files_bytes > 5 * 1024 * 1024 * 1024 {
        recs.push(Recommendation {
            title: format!(
                "Large files untouched for 6+ months — {}",
                crate::util::format_bytes_short(large_old_files_bytes)
            ),
            detail: "Potentially stale large files. Large does not mean disposable — review before removing anything.".into(),
            priority: "review".into(),
            size_bytes: large_old_files_bytes,
            target: "old-files".into(),
            confidence: 60,
        });
    }
    recs
}
