/**
 * SpaceForge canonical TypeScript contracts.
 *
 * These types mirror the Rust models in
 * `src-tauri/core/src/models/mod.rs` (serde is configured with
 * `rename_all = "camelCase"` on both sides). Keep them in sync.
 */

export type ScanMode = "battery_saver" | "balanced" | "fast" | "max_performance";

export type FileCategory =
  | "video" | "audio" | "image" | "document" | "archive" | "disk_image"
  | "executable" | "library" | "code" | "game" | "system" | "cache"
  | "temporary" | "log" | "database" | "developer" | "unknown";

export type RiskLevel = "safe" | "low" | "review" | "advanced" | "dangerous";

export type Reversibility = "recyclable" | "permanent" | "windows_managed";

export type CleanupFamily =
  | "windows" | "application" | "browser" | "developer" | "privacy";

export type CleanupProfile = "smart_clean" | "standard_clean" | "deep_clean" | "custom";

export type KeepStrategy =
  | "keep_newest" | "keep_oldest" | "keep_shortest_path"
  | "keep_preferred_folder" | "keep_first";

export interface VolumeInfo {
  mountPoint: string;
  label: string;
  filesystem: string;
  totalBytes: number;
  usedBytes: number;
  freeBytes: number;
  usedPercent: number;
  kind: "system" | "fixed" | "removable" | "network" | "unknown";
  media: "ssd" | "hdd" | "network" | "removable" | "unknown";
  isSystem: boolean;
  readOnly: boolean;
  scanEligible: boolean;
  ineligibleReason: string | null;
}

export type ScanPhase =
  | "preparing" | "walking" | "aggregating" | "finalizing"
  | "done" | "failed" | "cancelled";

export interface ScanProgress {
  scanId: number;
  volume: string;
  phase: ScanPhase;
  filesSeen: number;
  dirsSeen: number;
  bytesSeen: number;
  errors: number;
  currentPath: string;
  startedAt: string;
  elapsedMs: number;
}

export interface ScanSummary {
  scanId: number;
  volume: string;
  kind: "first" | "full" | "reverify";
  startedAt: string;
  durationMs: number;
  totalFiles: number;
  totalDirs: number;
  totalSize: number;
  totalAllocated: number;
  unreadableEntries: number;
  unreadableBytes: number;
  indexedFiles: number;
  hardLinksCollapsed: number;
  reparsePointsSkipped: number;
  volumeFree: number;
  volumeUsed: number;
  volumeTotal: number;
}

export interface FileEntry {
  path: string;
  name: string;
  extension: string;
  size: number;
  allocated: number;
  modified: string | null;
  created: string | null;
  category: FileCategory;
  fileId: number | null;
  linkCount: number | null;
  sparse: boolean;
}

export interface DirNode {
  name: string;
  path: string;
  size: number;
  allocated: number;
  fileCount: number;
  dirCount: number;
  modified: string | null;
  children: DirNode[];
  files: FileEntry[];
  smallFilesSize: number;
  smallFilesCount: number;
  unreadableBytes: number;
  unreadable: { path: string; reason: string }[];
}

export interface CleanerRule {
  id: string;
  name: string;
  description: string;
  family: CleanupFamily;
  group: string;
  risk: RiskLevel;
  reversibility: Reversibility;
  requiresAdmin: boolean;
  conflictsWithProcesses: string[];
  reason: string;
  consequence: string;
  optIn: boolean;
  windowsManaged: boolean;
  minOs: string;
}

export interface CleanupOpportunity {
  ruleId: string;
  rule: CleanerRule;
  locations: string[];
  itemCount: number;
  currentSize: number;
  reclaimableSize: number;
  confidence: number;
  discoveredAt: string;
  blockedReason: string | null;
}

export interface CleanupPlanEntry {
  ruleId: string;
  ruleName: string;
  targets: string[];
  estimatedReclaim: number;
  itemCount: number;
  risk: RiskLevel;
  reversibility: Reversibility;
  mechanism: "direct" | "recycle" | "windows_managed";
}

export interface CleanupPlan {
  id: string;
  profile: CleanupProfile;
  createdAt: string;
  entries: CleanupPlanEntry[];
  estimatedReclaim: number;
}

export interface CleanupEntryResult {
  ruleId: string;
  status: "success" | "partial" | "failed" | "skipped" | "blocked";
  deletedItems: number;
  failedItems: number;
  skippedItems: number;
  reclaimedBytes: number;
  failures: string[];
  skippedReasons: string[];
}

export interface CleanupReport {
  planId: string;
  profile: CleanupProfile;
  startedAt: string;
  finishedAt: string;
  durationMs: number;
  estimatedReclaim: number;
  actualReclaim: number;
  entries: CleanupEntryResult[];
  freeBefore: number;
  freeAfter: number;
}

export interface DuplicateMember {
  path: string;
  modified: string | null;
  fileId: number | null;
}

export interface DuplicateGroup {
  id: string;
  sizeEach: number;
  copies: number;
  wasted: number;
  keep: string;
  paths: DuplicateMember[];
}

export interface SnapshotMeta {
  id: number;
  name: string;
  volume: string;
  createdAt: string;
  scanId: number;
  totalSize: number;
  totalFiles: number;
  note: string | null;
}

export interface DirDelta {
  path: string;
  name: string;
  before: number;
  after: number;
  delta: number;
}

export interface SnapshotDiff {
  grew: DirDelta[];
  shrank: DirDelta[];
  newLargeFiles: FileEntry[];
  removedFiles: FileEntry[];
  categoryDeltas: [FileCategory, number][];
  totalDelta: number;
}

export interface HistoryPoint {
  at: string;
  used: number;
  free: number;
  reclaimable: number;
  scanId: number;
}

export interface CleanupHistoryEntry {
  id: number;
  at: string;
  profile: CleanupProfile;
  categories: string[];
  itemCount: number;
  estimatedReclaim: number;
  actualReclaim: number;
  failedItems: number;
  skippedItems: number;
  durationMs: number;
}

export interface Exclusion {
  id: number;
  pattern: string;
  reason: string;
  createdAt: string;
}

export interface AppSettings {
  theme: string;
  scanMode: ScanMode;
  confirmBeforeClean: boolean;
  preferRecycleBin: boolean;
  indexThresholdBytes: number;
  largeFileThreshold: number;
  traverseReparsePoints: boolean;
  scanNetworkDrives: boolean;
  enableExperimentalMft: boolean;
  enableOptInCleaners: boolean;
  backgroundAutoscan: boolean;
  workerThreads: number | null;
  telemetry: boolean;
  debugLogging: boolean;
  preferredDuplicatesFolder: string | null;
  excludedPatterns: string[];
}

export interface Recommendation {
  title: string;
  detail: string;
  priority: "high" | "medium" | "review";
  sizeBytes: number;
  target: string;
  confidence: number;
}

export interface StorageScore {
  pressure: string;
  freePercent: number;
  reclaimableBytes: number;
  duplicateWaste: number;
  explanations: string[];
}

export interface TopConsumers {
  topFolders: [string, number, number][];
  topExtensions: [string, number, number][];
  topCategories: [string, number, number][];
}

export interface ComponentStoreInfo {
  actualSizeBytes: number;
  cleanupRecommended: boolean;
  rollbackCapable: boolean;
  hasRepairContent: boolean;
  raw: string;
}

export interface MaintenanceInfo {
  elevated: boolean;
  hibernationEnabled: boolean;
  hiberfilSize: number;
  recycleBinItems: number;
  recycleBinBytes: number;
}

export interface PathInfo {
  exists: boolean;
  isDir: boolean;
  size: number;
  modified: number | null;
  category: string;
  protection: string;
}

export interface DeleteOutcome {
  requested: number;
  recycled: number;
  deleted: number;
  skipped: number;
  failures: string[];
}

export const DEFAULT_SETTINGS: AppSettings = {
  theme: "dark",
  scanMode: "balanced",
  confirmBeforeClean: true,
  preferRecycleBin: true,
  indexThresholdBytes: 10 * 1024 * 1024,
  largeFileThreshold: 500 * 1024 * 1024,
  traverseReparsePoints: false,
  scanNetworkDrives: false,
  enableExperimentalMft: false,
  enableOptInCleaners: false,
  backgroundAutoscan: false,
  workerThreads: null,
  telemetry: false,
  debugLogging: false,
  preferredDuplicatesFolder: null,
  excludedPatterns: [],
};
