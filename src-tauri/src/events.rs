//! Event forwarding: scan progress is streamed to the UI in batches.

use crate::state::AppState;
use tauri::{AppHandle, Emitter, Manager};

/// Spawn a thread that drains the progress channel and emits Tauri events.
pub fn forward_scan_progress(
    app: AppHandle,
    rx: crossbeam_channel::Receiver<spaceforge_core::models::ScanProgress>,
) {
    std::thread::spawn(move || {
        while let Ok(progress) = rx.recv() {
            let _ = app.emit("scan-progress", &progress);
            if matches!(
                progress.phase,
                spaceforge_core::models::ScanPhase::Done
                    | spaceforge_core::models::ScanPhase::Failed
                    | spaceforge_core::models::ScanPhase::Cancelled
            ) {
                break;
            }
        }
        let state: tauri::State<AppState> = app.state();
        let _ = state; // state kept alive; scan completion reported via command
    });
}
