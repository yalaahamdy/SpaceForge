//! Analysis queries over the in-memory scan: drill-down, search, top
//! consumers, file types, large/old files, score and recommendations.

use crate::state::AppState;
use spaceforge_core::models::{
    DirNode, FileCategory, FileEntry, Recommendation, StorageScore,
};
use spaceforge_core::util::classify;
use std::sync::Arc;
use tauri::State;

/// One tree level, IPC-capped (spec §54): children include their own
/// aggregates but never their children's children.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeNode {
    #[serde(flatten)]
    pub node: DirNode,
}

fn flatten_level(node: &DirNode) -> DirNode {
    let mut n = node.clone();
    n.children = node
        .children
        .iter()
        .map(|c| DirNode {
            name: c.name.clone(),
            path: c.path.clone(),
            size: c.size,
            allocated: c.allocated,
            file_count: c.file_count,
            dir_count: c.dir_count,
            modified: c.modified,
            children: vec![],
            files: c
                .files
                .iter()
                .take(50)
                .cloned()
                .collect(),
            small_files_size: c.small_files_size,
            small_files_count: c.small_files_count,
            unreadable_bytes: c.unreadable_bytes,
            unreadable: vec![],
        })
        .collect();
    n.files = node.files.iter().take(200).cloned().collect();
    n.unreadable = node.unreadable.iter().take(50).cloned().collect();
    n
}

fn last_scan(state: &AppState) -> Result<Arc<spaceforge_core::models::ScanResult>, String> {
    state
        .scan_result
        .read()
        .map_err(|_| "internal state error".to_string())?
        .clone()
        .ok_or_else(|| {
            "No scan available yet — scan a volume first".to_string()
        })
}

#[tauri::command]
pub fn latest_scan(state: State<AppState>) -> Result<Option<spaceforge_core::models::ScanSummary>, String> {
    Ok(state
        .scan_result
        .read()
        .map_err(|_| "internal state error".to_string())?
        .as_ref()
        .map(|r| r.summary.clone()))
}

/// Drill into a folder path; empty path = volume root.
#[tauri::command]
pub fn drill_down(state: State<AppState>, path: String) -> Result<TreeNode, String> {
    let scan = last_scan(&state)?;
    let root = &scan.root;
    if path.is_empty() || path == root.path {
        return Ok(TreeNode { node: flatten_level(root) });
    }
    let key = path.to_lowercase();
    if let Some(node) = find_node(root, &key) {
        return Ok(TreeNode { node: flatten_level(node) });
    }
    Err(format!("Path not found in the current scan: {path}"))
}

fn find_node<'a>(node: &'a DirNode, key_lower: &str) -> Option<&'a DirNode> {
    if node.path.to_lowercase() == key_lower {
        return Some(node);
    }
    for c in &node.children {
        if let Some(found) = find_node(c, key_lower) {
            return Some(found);
        }
    }
    None
}

#[tauri::command]
pub fn search_files(
    state: State<AppState>,
    query: String,
    min_size: Option<u64>,
    limit: Option<usize>,
) -> Result<Vec<FileEntry>, String> {
    let scan = last_scan(&state)?;
    let q = query.to_lowercase();
    let min = min_size.unwrap_or(0);
    let limit = limit.unwrap_or(200);
    let mut out: Vec<FileEntry> = scan
        .large_files
        .iter()
        .filter(|f| {
            f.size >= min
                && (f.name.to_lowercase().contains(&q)
                    || f.path.to_lowercase().contains(&q)
                    || f.extension.contains(&q))
        })
        .take(limit)
        .cloned()
        .collect();
    out.sort_by(|a, b| b.size.cmp(&a.size));
    Ok(out)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopConsumers {
    pub top_folders: Vec<(String, u64, u64)>, // path, size, file_count
    pub top_extensions: Vec<(String, u64, u64)>,
    pub top_categories: Vec<(String, u64, u64)>,
}

#[tauri::command]
pub fn top_consumers(state: State<AppState>) -> Result<TopConsumers, String> {
    let scan = last_scan(&state)?;
    let mut folders: Vec<(String, u64, u64)> = Vec::new();
    collect_folders(&scan.root, 0, &mut folders);
    folders.sort_by(|a, b| b.1.cmp(&a.1));
    folders.truncate(50);
    Ok(TopConsumers {
        top_folders: folders,
        top_extensions: scan.extensions.iter().take(30).cloned().collect(),
        top_categories: scan
            .categories
            .iter()
            .map(|(c, s, n)| (c.as_str().to_string(), *s, *n))
            .collect(),
    })
}

fn collect_folders(node: &DirNode, _depth: usize, out: &mut Vec<(String, u64, u64)>) {
    if !node.path.is_empty() {
        out.push((node.path.clone(), node.size, node.file_count));
    }
    for c in &node.children {
        collect_folders(c, _depth + 1, out);
    }
}

#[tauri::command]
pub fn file_types(state: State<AppState>) -> Result<TopConsumers, String> {
    top_consumers(state)
}

#[tauri::command]
pub fn large_files(
    state: State<AppState>,
    min_size: Option<u64>,
    older_than_days: Option<u64>,
    category: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<FileEntry>, String> {
    let scan = last_scan(&state)?;
    let min = min_size.unwrap_or(500 * 1024 * 1024);
    let now = chrono::Utc::now().timestamp();
    let cat = category.as_deref();
    let mut out: Vec<FileEntry> = scan
        .large_files
        .iter()
        .filter(|f| {
            if f.size < min {
                return false;
            }
            if let Some(days) = older_than_days {
                match f.modified {
                    Some(m) => {
                        if now - m.timestamp() < (days * 86400) as i64 {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            if let Some(c) = cat {
                if f.category.as_str() != c {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();
    out.sort_by(|a, b| b.size.cmp(&a.size));
    let off = offset.unwrap_or(0);
    let lim = limit.unwrap_or(200);
    Ok(out.into_iter().skip(off).take(lim).collect())
}

#[tauri::command]
pub fn old_files(
    state: State<AppState>,
    older_than_days: u64,
    limit: Option<usize>,
) -> Result<Vec<FileEntry>, String> {
    let scan = last_scan(&state)?;
    let now = chrono::Utc::now().timestamp();
    let cutoff = now - (older_than_days * 86400) as i64;
    let mut out: Vec<FileEntry> = scan
        .large_files
        .iter()
        .filter(|f| f.modified.map(|m| m.timestamp() < cutoff).unwrap_or(false))
        .cloned()
        .collect();
    out.sort_by(|a, b| {
        a.modified
            .unwrap_or_default()
            .cmp(&b.modified.unwrap_or_default())
    });
    Ok(out.into_iter().take(limit.unwrap_or(300)).collect())
}

#[tauri::command]
pub fn storage_score(state: State<AppState>) -> Result<StorageScore, String> {
    let scan = last_scan(&state)?;
    let s = &scan.summary;
    let free_percent = if s.volume_total > 0 {
        s.volume_free as f64 / s.volume_total as f64 * 100.0
    } else {
        0.0
    };
    // Evidence-based pressure rating from free-space share (spec §58).
    let pressure = if free_percent < 5.0 {
        "Critical"
    } else if free_percent < 10.0 {
        "High"
    } else if free_percent < 20.0 {
        "Moderate"
    } else {
        "Good"
    };
    let mut explanations = vec![format!(
        "{:.1}% of the volume is free. Windows performance can degrade below 10% free on the system drive.",
        free_percent
    )];
    // Reclaimable = safe + low-risk opportunities, measured at discovery time.
    let reclaimable = state.last_reclaimable();
    if reclaimable > 0 {
        explanations.push(format!(
            "{} of measured cleanup opportunities were found in the latest discovery pass.",
            spaceforge_core::util::format_bytes_short(reclaimable)
        ));
    }
    let dup = *state.duplicate_waste.read().map_err(|_| "state".to_string())?;
    if dup > 0 {
        explanations.push(format!(
            "{} of duplicate content was detected in the latest duplicate pass.",
            spaceforge_core::util::format_bytes_short(dup)
        ));
    }
    Ok(StorageScore {
        pressure: pressure.into(),
        free_percent,
        reclaimable_bytes: reclaimable,
        duplicate_waste: dup,
        explanations,
    })
}

#[tauri::command]
pub fn recommendations(
    state: State<AppState>,
    opportunities: Option<Vec<spaceforge_core::models::CleanupOpportunity>>,
) -> Result<Vec<Recommendation>, String> {
    // The UI passes the discovered opportunities it already shows; when not
    // provided, recommendations are computed from stored state only.
    let dup = *state.duplicate_waste.read().map_err(|_| "state".to_string())?;
    let opps = opportunities.unwrap_or_default();
    Ok(spaceforge_core::cleaners::recommend(&opps, dup, 0))
}

#[allow(dead_code)]
fn unused(_f: FileEntry, _e: &str) -> FileCategory {
    classify("")
}
