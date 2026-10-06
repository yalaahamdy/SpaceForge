//! Recycle Bin intelligence and rule (spec §28, §37).
//!
//! Uses the supported shell mechanisms: SHQueryRecycleBinW for stats and
//! SHEmptyRecycleBinW for emptying. Item-level restore stays in the Windows
//! shell — SpaceForge never invents its own restore format.

use super::{CleanContext, CleanerRuleDef};
use crate::models::*;
use std::sync::Arc;

#[derive(Default)]
pub struct RecycleBinRule;

impl CleanerRuleDef for RecycleBinRule {
    fn meta(&self) -> &CleanerRule {
        static META: std::sync::OnceLock<CleanerRule> = std::sync::OnceLock::new();
        META.get_or_init(|| CleanerRule {
            id: "win.recycle-bin".into(),
            name: "Recycle Bin".into(),
            description: "Deleted files still held in the Recycle Bin across all drives.".into(),
            family: CleanupFamily::Windows,
            group: "recycle".into(),
            risk: RiskLevel::Safe,
            reversibility: Reversibility::Permanent,
            requires_admin: false,
            conflicts_with_processes: vec![],
            reason: "Contents were already deleted by you; emptying releases the space.".into(),
            consequence: "Items in the Recycle Bin can no longer be restored afterwards.".into(),
            opt_in: false,
            windows_managed: true,
            min_os: "Windows 10".into(),
        })
    }

    fn allowed_prefixes(&self) -> Vec<String> {
        vec![] // windows-managed (shell API), not path-based
    }

    fn discover(&self, _ctx: &CleanContext) -> Option<CleanupOpportunity> {
        let (count, bytes) = crate::fsops::recycle_bin_stats_all();
        if count == 0 && bytes == 0 {
            return None;
        }
        Some(CleanupOpportunity {
            rule_id: "win.recycle-bin".into(),
            rule: self.meta().clone(),
            locations: vec!["Recycle Bin".into()],
            item_count: count as u64,
            current_size: bytes,
            reclaimable_size: bytes,
            confidence: 100,
            discovered_at: chrono::Utc::now(),
            blocked_reason: None,
        })
    }

    fn targets(&self, _ctx: &CleanContext) -> Vec<String> {
        vec!["Recycle Bin".into()]
    }

    fn execute(&self, _targets: &[String], _ctx: &CleanContext) -> CleanupEntryResult {
        let mut res = CleanupEntryResult {
            rule_id: "win.recycle-bin".into(),
            status: CleanupEntryStatus::Success,
            deleted_items: 0,
            failed_items: 0,
            skipped_items: 0,
            reclaimed_bytes: 0,
            failures: vec![],
            skipped_reasons: vec![],
        };
        let (count, bytes) = crate::fsops::recycle_bin_stats_all();
        match crate::fsops::empty_recycle_bin_all() {
            Ok(()) => {
                res.deleted_items = count as u64;
                res.reclaimed_bytes = bytes;
            }
            Err(e) => {
                res.status = CleanupEntryStatus::Failed;
                res.failures.push(e.to_string());
            }
        }
        res
    }

    fn mechanism(&self) -> CleanupMechanism {
        CleanupMechanism::WindowsManaged
    }
}

pub fn register(out: &mut Vec<Arc<dyn CleanerRuleDef>>) {
    out.push(Arc::new(RecycleBinRule));
}
