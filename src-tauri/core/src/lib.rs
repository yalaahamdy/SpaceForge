//! spaceforge-core — the engines behind SpaceForge.
//!
//! Crate layout (spec §69):
//! - `models`    — canonical typed contracts shared with the frontend
//! - `safety`    — protected paths + layered fail-closed validation
//! - `scanner`   — parallel scan engine, aggregation, progress
//! - `duplicate` — multi-stage duplicate detection
//! - `cleaners`  — modular cleaner rule engine + rule sets
//! - `db`        — SQLite index (scans, snapshots, history, settings)
//! - `snapshot`  — factual snapshot comparison
//! - `volumes`   — volume discovery
//! - `reports`   — JSON/CSV/HTML export
//! - `fsops`     — deletion semantics, shell integration, native helpers
//! - `util`      — formatting, categories, path helpers

pub mod cleaners;
pub mod db;
pub mod duplicate;
pub mod fsops;
pub mod models;
pub mod reports;
pub mod safety;
pub mod scanner;
pub mod snapshot;
pub mod util;
pub mod volumes;
