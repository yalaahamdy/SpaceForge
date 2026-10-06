//! Report export commands (spec §51). Exports are explicit user actions;
//! nothing is uploaded anywhere (spec §4).

use crate::state::AppState;
use spaceforge_core::reports;
use tauri::State;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub format: String, // "json" | "csv" | "html"
    pub path: String,
    /// When false (default), file paths are omitted from the report body
    /// and replaced by counts (privacy, spec §50).
    pub include_paths: bool,
}

#[tauri::command]
pub fn export_report(state: State<AppState>, request: ExportRequest) -> Result<String, String> {
    // "auto" = write to the user's Downloads folder (explicit local export).
    let mut out_path = request.path.clone();
    if out_path == "auto" {
        let base = std::env::var("USERPROFILE")
            .map(|h| format!("{h}\\Downloads"))
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".into());
        let stamp = chrono::Utc::now().format("%Y-%m-%d_%H%M");
        out_path = format!(
            "{base}\\SpaceForge-report-{stamp}.{}",
            request.format
        );
    }
    let scan = state
        .scan_result
        .read()
        .map_err(|_| "state".to_string())?
        .clone()
        .ok_or("No scan available to export")?;
    let scan = if request.include_paths {
        scan
    } else {
        // Path-privacy mode: strip individual file paths, keep aggregates.
        let mut s2 = spaceforge_core::models::ScanResult {
            summary: scan.summary.clone(),
            root: scan.root.clone(),
            large_files: scan
                .large_files
                .iter()
                .take(100)
                .map(|f| spaceforge_core::models::FileEntry {
                    path: "[path withheld]".into(),
                    name: format!("[{}] file", f.extension),
                    extension: f.extension.clone(),
                    size: f.size,
                    allocated: f.allocated,
                    modified: f.modified,
                    created: f.created,
                    category: f.category,
                    file_id: None,
                    link_count: None,
                    sparse: f.sparse,
                })
                .collect(),
            extensions: scan.extensions.clone(),
            categories: scan.categories.clone(),
        };
        s2.root.files.clear();
        s2.root.unreadable.clear();
        std::sync::Arc::new(s2)
    };
    match request.format.as_str() {
        "json" => reports::export_json(&scan, &out_path).map_err(|e| e.to_string()),
        "csv" => reports::export_csv(&scan, &out_path).map_err(|e| e.to_string()),
        "html" => reports::export_html(&scan, &out_path).map_err(|e| e.to_string()),
        other => Err(format!("unsupported format: {other}")),
    }
    .map(|_| out_path)
}
