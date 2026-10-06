//! Tauri application layer: IPC commands and event forwarding.
//!
//! This crate is a thin shell. All filesystem, safety, cleanup and
//! intelligence logic lives in `spaceforge-core` and is unit-tested there.

pub mod commands;
pub mod events;
pub mod state;

use spaceforge_core::models::AppSettings;
use tauri::Manager;

pub fn run() {
    let index = spaceforge_core::db::Index::open()
        .expect("SpaceForge failed to open its local index database");
    let settings: AppSettings = index.load_settings();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::new(index, settings))
        .setup(|app| {
            if let Some(icon) = app.default_window_icon() {
                for (_, window) in app.webview_windows() {
                    let _ = window.set_icon(icon.clone());
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::volumes::list_volumes_cmd,
            commands::scan::start_scan,
            commands::scan::pause_scan,
            commands::scan::resume_scan,
            commands::scan::cancel_scan,
            commands::scan::scan_status,
            commands::analyze::latest_scan,
            commands::analyze::drill_down,
            commands::analyze::search_files,
            commands::analyze::top_consumers,
            commands::analyze::file_types,
            commands::analyze::large_files,
            commands::analyze::old_files,
            commands::analyze::storage_score,
            commands::analyze::recommendations,
            commands::duplicates::find_duplicate_groups,
            commands::duplicates::cancel_duplicates,
            commands::cleanup::list_cleaner_rules,
            commands::cleanup::discover_cleanups,
            commands::cleanup::build_cleanup_plan,
            commands::cleanup::execute_cleanup,
            commands::cleanup::cleanup_history,
            commands::windows_ops::component_store_analyze,
            commands::windows_ops::hibernation_status_cmd,
            commands::windows_ops::elevation_status,
            commands::windows_ops::advanced_maintenance,
            commands::snapshots::save_snapshot_cmd,
            commands::snapshots::list_snapshots,
            commands::snapshots::compare_snapshots,
            commands::history::storage_history,
            commands::reports::export_report,
            commands::files::open_in_explorer_cmd,
            commands::files::open_path_cmd,
            commands::files::copy_path_cmd,
            commands::files::recycle_paths,
            commands::files::delete_paths,
            commands::files::secure_delete_paths,
            commands::files::path_info,
            commands::settings::get_settings,
            commands::settings::save_settings_cmd,
            commands::settings::list_exclusions,
            commands::settings::add_exclusion_cmd,
            commands::settings::remove_exclusion_cmd,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Cancel any running scan so the process can exit promptly.
                let state: tauri::State<state::AppState> = window.app_handle().state();
                state.cancel_scan();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running SpaceForge");
}
