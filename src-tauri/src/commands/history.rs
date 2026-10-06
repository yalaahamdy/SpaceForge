//! Storage history (spec §45): timeline points recorded from real scans.

use crate::state::AppState;
use spaceforge_core::models::HistoryPoint;
use tauri::State;

#[tauri::command]
pub fn storage_history(state: State<AppState>, volume: String) -> Result<Vec<HistoryPoint>, String> {
    state
        .index
        .history_points(&volume)
        .map_err(|e| e.to_string())
}
