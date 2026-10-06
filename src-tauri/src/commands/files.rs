//! File operation commands: shell integration and user-initiated deletion.
//! All deletion targets pass validate_user_targets — protected system zones
//! are untouchable even by explicit user request (spec §56).

use crate::state::AppState;
use spaceforge_core::safety::validator::validate_user_targets;
use serde::Serialize;
use tauri::State;

#[tauri::command]
pub fn open_in_explorer_cmd(path: String, select: Option<bool>) -> Result<(), String> {
    spaceforge_core::fsops::shell::open_in_explorer(&path, select.unwrap_or(true))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_path_cmd(path: String) -> Result<(), String> {
    spaceforge_core::fsops::shell::open_path(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_path_cmd(_state: State<AppState>, path: String) -> Result<String, String> {
    // The frontend writes this string to the clipboard; returning it keeps
    // the command pure and testable.
    Ok(path)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteOutcome {
    pub requested: usize,
    pub recycled: usize,
    pub deleted: usize,
    pub skipped: usize,
    pub failures: Vec<String>,
}

#[tauri::command]
pub fn recycle_paths(paths: Vec<String>) -> Result<DeleteOutcome, String> {
    let validated =
        validate_user_targets(&paths).map_err(|e| e.to_string())?;
    match spaceforge_core::fsops::delete::recycle(&validated) {
        Ok(count) => Ok(DeleteOutcome {
            requested: validated.len(),
            recycled: count,
            deleted: 0,
            skipped: validated.len() - count,
            failures: vec![],
        }),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn delete_paths(paths: Vec<String>) -> Result<DeleteOutcome, String> {
    let validated = validate_user_targets(&paths).map_err(|e| e.to_string())?;
    let mut outcome = DeleteOutcome {
        requested: validated.len(),
        recycled: 0,
        deleted: 0,
        skipped: 0,
        failures: vec![],
    };
    for p in &validated {
        match spaceforge_core::fsops::delete::delete_permanent(p) {
            Ok(true) => outcome.deleted += 1,
            Ok(false) => outcome.skipped += 1,
            Err(e) => outcome.failures.push(format!("{p}: {e}")),
        }
    }
    Ok(outcome)
}

#[tauri::command]
pub fn secure_delete_paths(paths: Vec<String>) -> Result<DeleteOutcome, String> {
    let validated = validate_user_targets(&paths).map_err(|e| e.to_string())?;
    let mut outcome = DeleteOutcome {
        requested: validated.len(),
        recycled: 0,
        deleted: 0,
        skipped: 0,
        failures: vec![],
    };
    for p in &validated {
        match spaceforge_core::fsops::delete::secure_delete_file(p) {
            Ok(true) => outcome.deleted += 1,
            Ok(false) => outcome.skipped += 1,
            Err(e) => outcome.failures.push(format!("{p}: {e}")),
        }
    }
    Ok(outcome)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathInfo {
    pub exists: bool,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<i64>,
    pub category: String,
    pub protection: String,
}

#[tauri::command]
pub fn path_info(path: String) -> PathInfo {
    let p = std::path::Path::new(&path);
    let md = std::fs::metadata(p);
    PathInfo {
        exists: p.exists(),
        is_dir: p.is_dir(),
        size: md.as_ref().map(|m| m.len()).unwrap_or(0),
        modified: md
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64),
        category: spaceforge_core::util::classify_path(p).as_str().to_string(),
        protection: format!(
            "{:?}",
            spaceforge_core::safety::classify_protection(&path)
        ),
    }
}
