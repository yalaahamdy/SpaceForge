//! Cleanup commands: rules, discovery, plan building, execution, history.

use crate::state::AppState;
use spaceforge_core::cleaners::Registry;
use spaceforge_core::models::*;
use tauri::{AppHandle, Emitter, State};

static REGISTRY: std::sync::OnceLock<Registry> = std::sync::OnceLock::new();

fn registry() -> &'static Registry {
    REGISTRY.get_or_init(Registry::all)
}

#[tauri::command]
pub fn list_cleaner_rules() -> Vec<CleanerRule> {
    registry().metas()
}

#[tauri::command]
pub fn discover_cleanups(state: State<AppState>) -> Result<Vec<CleanupOpportunity>, String> {
    let settings = state.settings.read().map(|s| s.clone()).unwrap_or_default();
    let ctx = spaceforge_core::cleaners::CleanContext {
        elevated: spaceforge_core::fsops::is_elevated(),
        settings,
    };
    let opps = registry().discover_all(&ctx);
    let total: u64 = opps
        .iter()
        .filter(|o| o.rule.risk <= RiskLevel::Low && o.blocked_reason.is_none())
        .map(|o| o.reclaimable_size)
        .sum();
    *state
        .last_reclaimable_bytes
        .write()
        .map_err(|_| "state".to_string())? = total;
    Ok(opps)
}

#[tauri::command]
pub fn build_cleanup_plan(
    state: State<AppState>,
    profile: CleanupProfile,
    opportunity_ids: Vec<String>,
) -> Result<CleanupPlan, String> {
    let settings = state.settings.read().map(|s| s.clone()).unwrap_or_default();
    let ctx = spaceforge_core::cleaners::CleanContext {
        elevated: spaceforge_core::fsops::is_elevated(),
        settings,
    };
    // Re-discover so the plan always reflects current reality (spec §36).
    let all = registry().discover_all(&ctx);
    let selected: Vec<CleanupOpportunity> = all
        .into_iter()
        .filter(|o| opportunity_ids.iter().any(|id| *id == o.rule_id))
        .collect();
    registry()
        .build_plan(profile, &selected, &ctx)
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteRequest {
    pub plan: CleanupPlan,
}

#[tauri::command]
pub fn execute_cleanup(
    app: AppHandle,
    state: State<AppState>,
    request: ExecuteRequest,
) -> Result<CleanupReport, String> {
    let settings = state.settings.read().map(|s| s.clone()).unwrap_or_default();
    let ctx = spaceforge_core::cleaners::CleanContext {
        elevated: spaceforge_core::fsops::is_elevated(),
        settings,
    };
    let plan_id = request.plan.id.clone();
    let progress = |done: u64, total: u64, rule: &str| {
        let _ = app.emit(
            "cleanup-progress",
            serde_json::json!({ "done": done, "total": total, "rule": rule, "planId": plan_id }),
        );
    };
    let report = registry().execute_plan(&request.plan, &ctx, &progress);
    let _ = state.index.save_cleanup(&report);
    Ok(report)
}

#[tauri::command]
pub fn cleanup_history(state: State<AppState>) -> Result<Vec<CleanupHistoryEntry>, String> {
    state.index.cleanup_history().map_err(|e| e.to_string())
}
