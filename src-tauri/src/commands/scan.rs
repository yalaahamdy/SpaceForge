//! Scan lifecycle commands: start / pause / resume / cancel / status.
//!
//! `start_scan` spawns the engine on a background thread and returns
//! immediately. Progress streams as `scan-progress` events; completion is
//! announced with a `scan-complete` event carrying the summary, while the
//! full tree stays in memory and is queried via the analyze commands
//! (spec §54 — no giant IPC payloads).

use crate::state::{ActiveScan, AppState};
use spaceforge_core::scanner::{spawn_scan, ScanRequest};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartScanResponse {
    pub accepted: bool,
    pub reason: Option<String>,
}

#[tauri::command]
pub fn start_scan(app: AppHandle, state: State<AppState>, volume: String) -> StartScanResponse {
    if state.has_active_scan() {
        return StartScanResponse {
            accepted: false,
            reason: Some("A scan is already running".into()),
        };
    }
    let settings = state.settings.read().map(|s| s.clone()).unwrap_or_default();
    let req = ScanRequest {
        volume: &volume,
        mode: settings.scan_mode,
        index_threshold: settings.index_threshold_bytes,
        max_indexed_files: 200_000,
        traverse_reparse_points: settings.traverse_reparse_points,
        worker_threads: settings.worker_threads,
        use_experimental_mft: settings.enable_experimental_mft,
    };
    let job = match spawn_scan(&req, spaceforge_core::models::ScanKind::Full) {
        Ok(j) => j,
        Err(e) => return StartScanResponse { accepted: false, reason: Some(e.to_string()) },
    };

    state.set_scan(ActiveScan {
        control: job.control,
        volume: volume.clone(),
    });
    crate::events::forward_scan_progress(app.clone(), job.progress_rx);

    // Result collector thread: stores the tree in state, persists to the
    // index, notifies the UI with the summary only.
    let app2 = app.clone();
    let result_rx = job.result_rx;
    std::thread::spawn(move || {
        if let Ok(result) = result_rx.recv() {
            let state: tauri::State<AppState> = app2.state();
            match result {
                Ok(res) => {
                    let summary = res.summary.clone();
                    let arc = std::sync::Arc::new(res);
                    let _ = state.index.save_scan(&arc);
                    *state.scan_result.write().unwrap_or_else(|e| e.into_inner()) = Some(arc);
                    state.take_scan();
                    let _ = app2.emit("scan-complete", &summary);
                }
                Err(e) => {
                    state.take_scan();
                    let _ = app2.emit("scan-failed", e.to_string());
                }
            }
        }
    });

    StartScanResponse { accepted: true, reason: None }
}

#[tauri::command]
pub fn pause_scan(state: State<AppState>, paused: bool) {
    if let Some(h) = state.active_control() {
        h.pause(paused);
    }
}

#[tauri::command]
pub fn resume_scan(state: State<AppState>) {
    pause_scan(state, false);
}

#[tauri::command]
pub fn cancel_scan(state: State<AppState>) {
    state.cancel_scan();
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    pub active: bool,
    pub has_result: bool,
}

#[tauri::command]
pub fn scan_status(state: State<AppState>) -> ScanStatus {
    ScanStatus {
        active: state.has_active_scan(),
        has_result: state
            .scan_result
            .read()
            .map(|r| r.is_some())
            .unwrap_or(false),
    }
}
