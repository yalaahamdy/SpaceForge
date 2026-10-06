//! Windows-native operations exposed to the UI (spec §24, §26, §27, §42).

use spaceforge_core::cleaners::cmds;
use serde::Serialize;
use tauri::State;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElevationStatus {
    pub elevated: bool,
    pub platform: String,
}

#[tauri::command]
pub fn elevation_status() -> ElevationStatus {
    ElevationStatus {
        elevated: spaceforge_core::fsops::is_elevated(),
        platform: std::env::consts::OS.to_string(),
    }
}

#[tauri::command]
pub fn component_store_analyze() -> Result<cmds::ComponentStoreInfo, String> {
    cmds::analyze_component_store()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HibernationInfo {
    pub enabled: bool,
    pub hiberfil_size: u64,
}

#[tauri::command]
pub fn hibernation_status_cmd() -> HibernationInfo {
    let (enabled, size) = cmds::hibernation_status();
    HibernationInfo { enabled, hiberfil_size: size }
}

/// Restore-point storage visibility via the supported vssadmin mechanism.
#[tauri::command]
pub fn restore_storage_info() -> Result<String, String> {
    if !cfg!(windows) {
        return Ok("Shadow copy information is only available on Windows.".into());
    }
    cmds::run_command(
        "vssadmin",
        &["list".to_string(), "shadowstorage".to_string()],
        false,
    )
    .map_err(|e| {
        if e.contains("elevated") || e.to_lowercase().contains("permission") {
            "Requires administrator privileges to read shadow storage".to_string()
        } else {
            e
        }
    })
}

/// Aggregated "advanced maintenance" panel payload.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceInfo {
    pub elevated: bool,
    pub hibernation_enabled: bool,
    pub hiberfil_size: u64,
    pub recycle_bin_items: u32,
    pub recycle_bin_bytes: u64,
}

#[tauri::command]
pub fn advanced_maintenance(_state: State<AppState>) -> MaintenanceInfo {
    let (hib, size) = cmds::hibernation_status();
    let (count, bytes) = spaceforge_core::fsops::recycle_bin_stats_all();
    MaintenanceInfo {
        elevated: spaceforge_core::fsops::is_elevated(),
        hibernation_enabled: hib,
        hiberfil_size: size,
        recycle_bin_items: count,
        recycle_bin_bytes: bytes,
    }
}
