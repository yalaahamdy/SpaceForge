/**
 * Typed IPC bridge. Every call maps to a Tauri command with strongly typed
 * request/response models (spec §70).
 */

import type {
  AppSettings, CleanupHistoryEntry, CleanupOpportunity, CleanupPlan,
  CleanupReport, ComponentStoreInfo, DeleteOutcome, DirNode, DuplicateGroup,
  Exclusion, FileEntry, HistoryPoint, KeepStrategy, MaintenanceInfo,
  PathInfo, Recommendation, ScanSummary, SnapshotDiff, SnapshotMeta,
  StorageScore, TopConsumers, VolumeInfo,
} from "./types";

type Cmd = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke(cmd, args) as Promise<T>;
}

export const ipc = {
  // ---- volumes -------------------------------------------------------------
  listVolumes: () => invoke<VolumeInfo[]>("list_volumes_cmd"),

  // ---- scanning --------------------------------------------------------------
  startScan: (volume: string) =>
    invoke<{ accepted: boolean; reason: string | null }>("start_scan", { volume }),
  pauseScan: (paused: boolean) => invoke<void>("pause_scan", { paused }),
  resumeScan: () => invoke<void>("resume_scan"),
  cancelScan: () => invoke<void>("cancel_scan"),
  scanStatus: () =>
    invoke<{ active: boolean; hasResult: boolean }>("scan_status"),

  // ---- analysis ---------------------------------------------------------------
  latestScan: () => invoke<ScanSummary | null>("latest_scan"),
  drillDown: (path: string) => invoke<DirNode>("drill_down", { path }),
  searchFiles: (query: string, minSize?: number, limit?: number) =>
    invoke<FileEntry[]>("search_files", { query, minSize, limit }),
  topConsumers: () => invoke<TopConsumers>("top_consumers"),
  fileTypes: () => invoke<TopConsumers>("file_types"),
  largeFiles: (opts: {
    minSize?: number; olderThanDays?: number; category?: string;
    limit?: number; offset?: number;
  }) => invoke<FileEntry[]>("large_files", opts),
  oldFiles: (olderThanDays: number, limit?: number) =>
    invoke<FileEntry[]>("old_files", { olderThanDays, limit }),
  storageScore: () => invoke<StorageScore>("storage_score"),
  recommendations: (opportunities?: CleanupOpportunity[]) =>
    invoke<Recommendation[]>("recommendations", { opportunities }),

  // ---- duplicates -----------------------------------------------------------
  findDuplicates: (req: {
    useScan: boolean;
    extraFolders: string[];
    minSize?: number;
    keepStrategy?: KeepStrategy;
    preferredFolder?: string;
  }) => invoke<DuplicateGroup[]>("find_duplicate_groups", { request: req }),
  cancelDuplicates: () => invoke<void>("cancel_duplicates"),

  // ---- cleanup ----------------------------------------------------------------
  listCleanerRules: () => invoke<import("./types").CleanerRule[]>("list_cleaner_rules"),
  discoverCleanups: () => invoke<CleanupOpportunity[]>("discover_cleanups"),
  buildCleanupPlan: (profile: string, opportunityIds: string[]) =>
    invoke<CleanupPlan>("build_cleanup_plan", { profile, opportunityIds }),
  executeCleanup: (plan: CleanupPlan) =>
    invoke<CleanupReport>("execute_cleanup", { request: { plan } }),
  cleanupHistory: () => invoke<CleanupHistoryEntry[]>("cleanup_history"),

  // ---- windows ops ------------------------------------------------------------
  elevationStatus: () =>
    invoke<{ elevated: boolean; platform: string }>("elevation_status"),
  componentStoreAnalyze: () => invoke<ComponentStoreInfo>("component_store_analyze"),
  hibernationStatus: () =>
    invoke<{ enabled: boolean; hiberfilSize: number }>("hibernation_status_cmd"),
  restoreStorageInfo: () => invoke<string>("restore_storage_info"),
  advancedMaintenance: () => invoke<MaintenanceInfo>("advanced_maintenance"),

  // ---- snapshots / history ------------------------------------------------------
  saveSnapshot: (name: string, note?: string) =>
    invoke<SnapshotMeta>("save_snapshot_cmd", { name, note }),
  listSnapshots: (volume: string) => invoke<SnapshotMeta[]>("list_snapshots", { volume }),
  compareSnapshots: (olderId: number, newerId: number) =>
    invoke<SnapshotDiff>("compare_snapshots", {
      olderSnapshotId: olderId, newerSnapshotId: newerId,
    }),
  storageHistory: (volume: string) => invoke<HistoryPoint[]>("storage_history", { volume }),

  // ---- reports / files -----------------------------------------------------------
  exportReport: (format: string, path: string, includePaths: boolean) =>
    invoke<string>("export_report", { request: { format, path, includePaths } }),
  openInExplorer: (path: string, select = true) =>
    invoke<void>("open_in_explorer_cmd", { path, select }),
  openPath: (path: string) => invoke<void>("open_path_cmd", { path }),
  copyPath: (path: string) => invoke<string>("copy_path_cmd", { path }),
  recyclePaths: (paths: string[]) => invoke<DeleteOutcome>("recycle_paths", { paths }),
  deletePaths: (paths: string[]) => invoke<DeleteOutcome>("delete_paths", { paths }),
  secureDeletePaths: (paths: string[]) =>
    invoke<DeleteOutcome>("secure_delete_paths", { paths }),
  pathInfo: (path: string) => invoke<PathInfo>("path_info", { path }),

  // ---- settings --------------------------------------------------------------------
  getSettings: () => invoke<AppSettings>("get_settings"),
  saveSettings: (settings: AppSettings) =>
    invoke<void>("save_settings_cmd", { settings }),
  listExclusions: () => invoke<Exclusion[]>("list_exclusions"),
  addExclusion: (pattern: string, reason: string) =>
    invoke<Exclusion>("add_exclusion_cmd", { pattern, reason }),
  removeExclusion: (id: number) => invoke<void>("remove_exclusion_cmd", { id }),
};

/** Listen to backend events (scan progress, completion, cleanup progress). */
export async function listen<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<() => void> {
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<T>(event, (e) => handler(e.payload));
  return unlisten;
}
