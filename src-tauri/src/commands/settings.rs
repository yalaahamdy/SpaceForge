//! Settings & exclusion commands (spec §52, §49).

use crate::state::AppState;
use spaceforge_core::models::{AppSettings, Exclusion};
use tauri::State;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> AppSettings {
    state.settings.read().map(|s| s.clone()).unwrap_or_default()
}

#[tauri::command]
pub fn save_settings_cmd(state: State<AppState>, settings: AppSettings) -> Result<(), String> {
    // Privacy guarantees are structural (spec §4): telemetry stays off no
    // matter what a client sends.
    let mut s = settings;
    s.telemetry = false;
    *state.settings.write().map_err(|_| "state".to_string())? = s.clone();
    state.index.save_settings(&s).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_exclusions(state: State<AppState>) -> Result<Vec<Exclusion>, String> {
    state.index.exclusions().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_exclusion_cmd(
    state: State<AppState>,
    pattern: String,
    reason: String,
) -> Result<Exclusion, String> {
    let id = state
        .index
        .add_exclusion(&pattern, &reason)
        .map_err(|e| e.to_string())?;
    Ok(Exclusion {
        id,
        pattern,
        reason,
        created_at: chrono::Utc::now(),
    })
}

#[tauri::command]
pub fn remove_exclusion_cmd(state: State<AppState>, id: i64) -> Result<(), String> {
    state.index.remove_exclusion(id).map_err(|e| e.to_string())
}
