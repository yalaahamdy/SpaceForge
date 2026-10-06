//! SpaceForge core data model.
//!
//! These types form the canonical contract between the Rust engines and the
//! TypeScript frontend. Every IPC command has strongly typed request/response
//! models derived from these definitions (mirrored in `src/api/types.ts`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("path is protected: {0}")]
    ProtectedPath(String),
    #[error("scan failed: {0}")]
    Scan(String),
    #[error("operation cancelled")]
    Cancelled,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid argument: {0}")]
    Invalid(String),
    #[error("windows operation failed: {0}")]
    WindowsOp(String),
    #[error("requires administrator elevation: {0}")]
    ElevationRequired(String),
    #[error("{0}")]
    Other(String),
}

impl serde::Serialize for CoreError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type CoreResult<T> = Result<T, CoreError>;

// ---------------------------------------------------------------------------
// Volumes
// ---------------------------------------------------------------------------

/// Physical media / bus classification, best effort.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Ssd,
    Hdd,
    Network,
    Removable,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DriveKind {
    System,
    Fixed,
    Removable,
    Network,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeInfo {
    /// Drive letter (Windows) or mount point (Unix), e.g. "C:" or "/".
    pub mount_point: String,
    /// Volume label when available.
    pub label: String,
    /// Filesystem name, e.g. NTFS, APFS, ext4.
    pub filesystem: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub used_percent: f64,
    pub kind: DriveKind,
    pub media: MediaType,
    /// True when this volume hosts the running OS.
    pub is_system: bool,
    pub read_only: bool,
    /// True when the scanner can walk this volume (accessible, supported FS).
    pub scan_eligible: bool,
    /// Non-empty when the volume cannot be scanned and explains why.
    pub ineligible_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Categories & risk
// ---------------------------------------------------------------------------

/// High-level content classification used by treemap colors, file-type
/// rollups and the recommendation engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FileCategory {
    Video,
    Audio,
    Image,
    Document,
    Archive,
    DiskImage,
    Executable,
    Library,
    Code,
    Game,
    System,
    Cache,
    Temporary,
    Log,
    Database,
    Developer,
    #[default]
    Unknown,
}

impl FileCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileCategory::Video => "video",
            FileCategory::Audio => "audio",
            FileCategory::Image => "image",
            FileCategory::Document => "document",
            FileCategory::Archive => "archive",
            FileCategory::DiskImage => "disk_image",
            FileCategory::Executable => "executable",
            FileCategory::Library => "library",
            FileCategory::Code => "code",
            FileCategory::Game => "game",
            FileCategory::System => "system",
            FileCategory::Cache => "cache",
            FileCategory::Temporary => "temporary",
            FileCategory::Log => "log",
            FileCategory::Database => "database",
            FileCategory::Developer => "developer",
            FileCategory::Unknown => "unknown",
        }
    }
}

/// Cleanup risk classification (spec §20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Safe,
    Low,
    #[default]
    Review,
    Advanced,
    Dangerous,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Safe => "safe",
            RiskLevel::Low => "low",
            RiskLevel::Review => "review",
            RiskLevel::Advanced => "advanced",
            RiskLevel::Dangerous => "dangerous",
        }
    }
}

/// Whether a cleanup operation can be undone (spec §37).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Reversibility {
    /// Moves to Recycle Bin — normally restorable.
    Recyclable,
    /// Deleted permanently — not restorable via Recycle Bin.
    Permanent,
    /// Performed by a Windows-managed mechanism (DISM, powercfg, …).
    #[default]
    WindowsManaged,
}

/// Protection classification of a path (spec §39).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Protection {
    SystemProtected,
    WindowsManaged,
    ApplicationManaged,
    UserData,
    SafeCache,
    Temporary,
    #[default]
    Unknown,
}

// ---------------------------------------------------------------------------
// Scanning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScanMode {
    /// Conservative resource usage.
    BatterySaver,
    /// Recommended default.
    #[default]
    Balanced,
    /// Higher resource usage.
    Fast,
    /// Maximum CPU and I/O.
    MaxPerformance,
}

impl ScanMode {
    /// Worker thread fraction (of logical cores) for each mode.
    pub fn worker_fraction(&self) -> f32 {
        match self {
            ScanMode::BatterySaver => 0.25,
            ScanMode::Balanced => 0.5,
            ScanMode::Fast => 0.75,
            ScanMode::MaxPerformance => 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub scan_id: u64,
    pub volume: String,
    pub phase: ScanPhase,
    pub files_seen: u64,
    pub dirs_seen: u64,
    pub bytes_seen: u64,
    /// Entries the scanner could not read; never silently skipped.
    pub errors: u64,
    pub current_path: String,
    pub started_at: DateTime<Utc>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanPhase {
    Preparing,
    Walking,
    Aggregating,
    Finalizing,
    Done,
    Failed,
    Cancelled,
}

/// One file recorded by a scan. Only files at or above the session index
/// threshold are retained individually; smaller files are aggregated into
/// their parent directory (spec §54 — no million-element IPC streams).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub extension: String,
    /// Logical size (bytes readable through the filesystem).
    pub size: u64,
    /// Space actually allocated on disk (cluster rounded, sparse aware).
    pub allocated: u64,
    pub modified: Option<DateTime<Utc>>,
    pub created: Option<DateTime<Utc>>,
    pub category: FileCategory,
    /// Windows file index when resolvable — groups hard links together.
    pub file_id: Option<u64>,
    /// Hard-link count when the filesystem exposes it.
    pub link_count: Option<u64>,
    /// True when the file is an NTFS sparse file.
    pub sparse: bool,
}

/// Aggregated directory node. `size` and `file_count` always include the
/// full recursive subtree. Hard-linked content is counted once per subtree.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DirNode {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub allocated: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub modified: Option<DateTime<Utc>>,
    /// Direct children, sorted by size descending (lazily populated).
    pub children: Vec<DirNode>,
    /// Files at or above the index threshold in *this* directory only.
    pub files: Vec<FileEntry>,
    /// Aggregate size of files below the index threshold in this directory.
    pub small_files_size: u64,
    pub small_files_count: u64,
    /// Bytes the scanner could not read (access denied, vanished, …).
    pub unreadable_bytes: u64,
    /// Paths under this directory that could not be read.
    pub unreadable: Vec<UnreadableEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreadableEntry {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanKind {
    First,
    Full,
    Reverify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub scan_id: u64,
    pub volume: String,
    pub kind: ScanKind,
    pub started_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub total_files: u64,
    pub total_dirs: u64,
    pub total_size: u64,
    pub total_allocated: u64,
    pub unreadable_entries: u64,
    pub unreadable_bytes: u64,
    /// Files kept individually in the index (size >= index threshold).
    pub indexed_files: u64,
    /// Hard-link instances collapsed (counted once).
    pub hard_links_collapsed: u64,
    /// Reparse points (junctions/symlinks) deliberately not traversed.
    pub reparse_points_skipped: u64,
    /// Snapshot of volume free/used at scan time.
    pub volume_free: u64,
    pub volume_used: u64,
    pub volume_total: u64,
}

/// Complete result of one scan session, held in memory and persisted
/// (aggregated) to the local index database.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub summary: ScanSummary,
    pub root: DirNode,
    /// Files at or above the index threshold, largest first.
    pub large_files: Vec<FileEntry>,
    /// Extension rollup: extension → (total size, file count).
    pub extensions: Vec<(String, u64, u64)>,
    /// Category rollup: category → (total size, file count).
    pub categories: Vec<(FileCategory, u64, u64)>,
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CleanupFamily {
    #[default]
    Windows,
    Application,
    Browser,
    Developer,
    Privacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CleanupMechanism {
    /// SpaceForge deletes files itself after layered validation.
    #[default]
    Direct,
    /// Items are moved to the Recycle Bin (restorable).
    Recycle,
    /// Executed through a supported Windows mechanism (DISM, powercfg, …).
    WindowsManaged,
}

/// Static, structured metadata for one cleaner rule (spec §33).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanerRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub family: CleanupFamily,
    /// Machine-readable sub-group, e.g. "chrome", "npm", "temp".
    pub group: String,
    pub risk: RiskLevel,
    pub reversibility: Reversibility,
    pub requires_admin: bool,
    /// Process names that must be closed before this rule can run.
    pub conflicts_with_processes: Vec<String>,
    /// Human explanation of why this data is considered removable.
    pub reason: String,
    /// Human explanation of what happens after cleaning (rebuild, re-download…).
    pub consequence: String,
    /// True when the rule is only offered after the user opts in.
    pub opt_in: bool,
    /// True when the rule uses a supported Windows maintenance mechanism.
    pub windows_managed: bool,
    /// Supported since this Windows version; informational.
    pub min_os: String,
}

/// A detected, currently-available cleanup opportunity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupOpportunity {
    pub rule_id: String,
    pub rule: CleanerRule,
    /// Concrete paths the rule would touch.
    pub locations: Vec<String>,
    pub item_count: u64,
    /// Bytes currently occupied.
    pub current_size: u64,
    /// Bytes expected to be reclaimed (may differ from current_size when the
    /// mechanism is Windows-managed and only reports a best estimate).
    pub reclaimable_size: u64,
    /// Confidence 0–100 derived from rule evidence.
    pub confidence: u8,
    pub discovered_at: DateTime<Utc>,
    /// Blocked reason when the rule cannot currently run
    /// (process running, needs elevation, opt-in disabled…).
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupProfile {
    SmartClean,
    StandardClean,
    DeepClean,
    Custom,
}

/// A planned, user-approved cleanup transaction (spec §36).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPlan {
    pub id: String,
    pub profile: CleanupProfile,
    pub created_at: DateTime<Utc>,
    pub entries: Vec<CleanupPlanEntry>,
    pub estimated_reclaim: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPlanEntry {
    pub rule_id: String,
    pub rule_name: String,
    pub targets: Vec<String>,
    pub estimated_reclaim: u64,
    pub item_count: u64,
    pub risk: RiskLevel,
    pub reversibility: Reversibility,
    pub mechanism: CleanupMechanism,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupEntryResult {
    pub rule_id: String,
    pub status: CleanupEntryStatus,
    pub deleted_items: u64,
    pub failed_items: u64,
    pub skipped_items: u64,
    pub reclaimed_bytes: u64,
    pub failures: Vec<String>,
    pub skipped_reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupEntryStatus {
    Success,
    Partial,
    Failed,
    Skipped,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupReport {
    pub plan_id: String,
    pub profile: CleanupProfile,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub estimated_reclaim: u64,
    pub actual_reclaim: u64,
    pub entries: Vec<CleanupEntryResult>,
    /// Free space before / after, measured from the volume.
    pub free_before: u64,
    pub free_after: u64,
}

// ---------------------------------------------------------------------------
// Duplicates
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    /// Stable group id (hash of content identity).
    pub id: String,
    pub size_each: u64,
    pub copies: u32,
    /// Wasted space = size_each × (copies − 1).
    pub wasted: u64,
    /// Recommended keeper path given the selection strategy.
    pub keep: String,
    /// Full paths; index 0 is the recommended keeper.
    pub paths: Vec<DuplicateMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateMember {
    pub path: String,
    pub modified: Option<DateTime<Utc>>,
    /// Paths that are hard links to the same inode/file id share this id.
    pub file_id: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum KeepStrategy {
    KeepNewest,
    KeepOldest,
    KeepShortestPath,
    KeepPreferredFolder,
    #[default]
    KeepFirst,
}

// ---------------------------------------------------------------------------
// Snapshots & history
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotMeta {
    pub id: i64,
    pub name: String,
    pub volume: String,
    pub created_at: DateTime<Utc>,
    pub scan_id: u64,
    pub total_size: u64,
    pub total_files: u64,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDiff {
    pub grew: Vec<DirDelta>,
    pub shrank: Vec<DirDelta>,
    pub new_large_files: Vec<FileEntry>,
    pub removed_files: Vec<FileEntry>,
    pub category_deltas: Vec<(FileCategory, i64)>,
    pub total_delta: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirDelta {
    pub path: String,
    pub name: String,
    pub before: u64,
    pub after: u64,
    pub delta: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPoint {
    pub at: DateTime<Utc>,
    pub used: u64,
    pub free: u64,
    pub reclaimable: u64,
    pub scan_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupHistoryEntry {
    pub id: i64,
    pub at: DateTime<Utc>,
    pub profile: CleanupProfile,
    pub categories: Vec<String>,
    pub item_count: u64,
    pub estimated_reclaim: u64,
    pub actual_reclaim: u64,
    pub failed_items: u64,
    pub skipped_items: u64,
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Exclusions & settings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exclusion {
    pub id: i64,
    /// Glob pattern matched against full paths, e.g. `D:\Games\**`.
    pub pattern: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub theme: String,              // "dark" | "light" | "system"
    pub scan_mode: ScanMode,
    pub confirm_before_clean: bool,
    pub prefer_recycle_bin: bool,
    /// Files below this size are aggregated, not indexed individually.
    pub index_threshold_bytes: u64,
    /// Large-file finder default threshold.
    pub large_file_threshold: u64,
    /// Reparse points (junctions/symlinks) are never traversed when true.
    pub traverse_reparse_points: bool,
    pub scan_network_drives: bool,
    pub enable_experimental_mft: bool,
    pub enable_opt_in_cleaners: bool,
    pub background_autoscan: bool,
    pub worker_threads: Option<u32>, // None = auto per scan mode
    pub telemetry: bool,             // always false; exposed for transparency
    pub debug_logging: bool,
    pub preferred_duplicates_folder: Option<String>,
    pub excluded_patterns: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            scan_mode: ScanMode::Balanced,
            confirm_before_clean: true,
            prefer_recycle_bin: true,
            index_threshold_bytes: 10 * 1024 * 1024, // 10 MB
            large_file_threshold: 500 * 1024 * 1024, // 500 MB
            traverse_reparse_points: false,
            scan_network_drives: false,
            enable_experimental_mft: false,
            enable_opt_in_cleaners: false,
            background_autoscan: false,
            worker_threads: None,
            telemetry: false,
            debug_logging: false,
            preferred_duplicates_folder: None,
            excluded_patterns: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Insights
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub title: String,
    pub detail: String,
    /// One of: high | medium | review
    pub priority: String,
    pub size_bytes: u64,
    /// Where the recommendation points, e.g. view id.
    pub target: String,
    pub confidence: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageScore {
    /// Human rating: Good | Moderate | High | Critical
    pub pressure: String,
    pub free_percent: f64,
    pub reclaimable_bytes: u64,
    pub duplicate_waste: u64,
    pub explanations: Vec<String>,
}
