//! Snapshot commands (spec §44): save named snapshots of the current scan,
//! list them and compare two snapshots factually.

use crate::state::AppState;
use spaceforge_core::models::{SnapshotDiff, SnapshotMeta};
use tauri::State;

#[tauri::command]
pub fn save_snapshot_cmd(
    state: State<AppState>,
    name: String,
    note: Option<String>,
) -> Result<SnapshotMeta, String> {
    let scan = state
        .scan_result
        .read()
        .map_err(|_| "state".to_string())?
        .clone()
        .ok_or("No scan available — scan a volume before saving a snapshot")?;
    let summary = &scan.summary;
    let id = state
        .index
        .save_snapshot(
            &name,
            &summary.volume,
            summary.scan_id as i64,
            summary.total_size,
            summary.total_files,
            note.as_deref(),
        )
        .map_err(|e| e.to_string())?;
    Ok(SnapshotMeta {
        id,
        name,
        volume: summary.volume.clone(),
        created_at: chrono::Utc::now(),
        scan_id: summary.scan_id,
        total_size: summary.total_size,
        total_files: summary.total_files,
        note,
    })
}

#[tauri::command]
pub fn list_snapshots(state: State<AppState>, volume: String) -> Result<Vec<SnapshotMeta>, String> {
    state.index.snapshots(&volume).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn compare_snapshots(
    state: State<AppState>,
    older_snapshot_id: i64,
    newer_snapshot_id: i64,
) -> Result<SnapshotDiff, String> {
    // Snapshot rows store the scan ids they were created from.
    let snaps = state.index.snapshots_all().map_err(|e| e.to_string())?;
    let find = |id: i64| snaps.iter().find(|s| s.id == id).map(|s| s.scan_id as i64);
    let older = find(older_snapshot_id).ok_or("older snapshot not found")?;
    let newer = find(newer_snapshot_id).ok_or("newer snapshot not found")?;
    spaceforge_core::snapshot::compare(&state.index, older, newer).map_err(|e| e.to_string())
}
