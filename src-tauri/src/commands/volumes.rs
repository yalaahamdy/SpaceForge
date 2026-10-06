//! Volume discovery command.

use crate::state::AppState;
use spaceforge_core::models::VolumeInfo;
use tauri::State;

#[tauri::command]
pub fn list_volumes_cmd(state: State<AppState>) -> Vec<VolumeInfo> {
    let scan_network = state
        .settings
        .read()
        .map(|s| s.scan_network_drives)
        .unwrap_or(false);
    spaceforge_core::volumes::list_volumes(scan_network)
}
