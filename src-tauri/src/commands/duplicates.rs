//! Duplicate detection commands (spec §19).

use crate::state::AppState;
use spaceforge_core::duplicate::{
    find_duplicates, DuplicateCandidate, DuplicateOptions, ProgressFn,
};
use spaceforge_core::models::{DuplicateGroup, KeepStrategy};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{Manager, State, Window};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateRequest {
    /// Volume paths already scanned (candidates come from the scan index).
    pub use_scan: bool,
    /// Extra folders to walk for candidates (user-selected scope).
    pub extra_folders: Vec<String>,
    pub min_size: Option<u64>,
    pub keep_strategy: Option<KeepStrategy>,
    pub preferred_folder: Option<String>,
}

#[tauri::command]
pub async fn find_duplicate_groups(
    window: Window,
    state: State<'_, AppState>,
    request: DuplicateRequest,
) -> Result<Vec<DuplicateGroup>, String> {
    // Reset cancellation for this run.
    state
        .duplicates_cancel
        .store(false, Ordering::SeqCst);

    let mut candidates: Vec<DuplicateCandidate> = Vec::new();

    if request.use_scan {
        let scan = state
            .scan_result
            .read()
            .map_err(|_| "state".to_string())?
            .clone()
            .ok_or("No scan available — scan a volume first")?;
        let min = request.min_size.unwrap_or(1024 * 1024);
        for f in &scan.large_files {
            if f.size >= min {
                candidates.push(DuplicateCandidate {
                    path: f.path.clone(),
                    size: f.size,
                    modified: f.modified.map(|m| m.timestamp()),
                    file_id: f.file_id,
                });
            }
        }
    }

    for folder in &request.extra_folders {
        collect_candidates(folder, request.min_size.unwrap_or(1024 * 1024), &mut candidates);
    }

    // Enrich candidates with file identity where missing (hard-link awareness).
    for c in candidates.iter_mut() {
        if c.file_id.is_none() {
            if let Some((_, idx, links)) = spaceforge_core::fsops::file_identity(std::path::Path::new(&c.path)) {
                if links > 1 {
                    c.file_id = Some(idx);
                }
            }
        }
    }

    let app = window.app_handle().clone();
    let progress: ProgressFn = Arc::new(move |stage: &str, done: u64, total: u64| {
        use tauri::Emitter;
        let _ = app.emit(
            "duplicates-progress",
            serde_json::json!({ "stage": stage, "done": done, "total": total }),
        );
    });

    let settings = state.settings.read().map(|s| s.clone()).unwrap_or_default();
    let opts = DuplicateOptions {
        min_size: request.min_size.unwrap_or(1024 * 1024),
        keep_strategy: request.keep_strategy.unwrap_or(KeepStrategy::KeepNewest),
        preferred_folder: request
            .preferred_folder
            .or(settings.preferred_duplicates_folder),
        on_progress: progress,
        cancelled: state.duplicates_cancel.clone(),
    };

    tauri::async_runtime::spawn_blocking(move || {
        find_duplicates(candidates, &opts)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
    .map(|groups| {
        let waste: u64 = groups.iter().map(|g| g.wasted).sum();
        if let Ok(mut w) = state.duplicate_waste.write() {
            *w = waste;
        }
        groups
    })
}

fn collect_candidates(root: &str, min_size: u64, out: &mut Vec<DuplicateCandidate>) {
    let walk = jwalk::WalkDir::new(root)
        .parallelism(jwalk::Parallelism::RayonDefaultPool { busy_timeout: std::time::Duration::from_secs(1) })
        .process_read_dir(|_d, _p, _s, children| {
            children.retain(|e| e.is_ok());
        });
    for entry in walk {
        let Ok(entry) = entry else { continue };
        let ft = entry.file_type();
        if !ft.is_file() {
            continue;
        }
        let Ok(md) = entry.metadata() else { continue };
        if md.len() < min_size {
            continue;
        }
        out.push(DuplicateCandidate {
            path: entry.path().to_string_lossy().into_owned(),
            size: md.len(),
            modified: md
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64),
            file_id: None,
        });
    }
}

#[tauri::command]
pub fn cancel_duplicates(state: State<AppState>) {
    state.duplicates_cancel.store(true, Ordering::SeqCst);
}
