//! Tauri IPC commands. Each has strongly typed request/response models and
//! documented cancellation semantics (spec §70).

pub mod analyze;
pub mod cleanup;
pub mod duplicates;
pub mod files;
pub mod history;
pub mod reports;
pub mod scan;
pub mod settings;
pub mod snapshots;
pub mod volumes;
pub mod windows_ops;
