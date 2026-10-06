//! Throttled progress reporting (spec §54 — batched updates, no per-file IPC).

use crate::models::{ScanPhase, ScanProgress};
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Shared scan counters, updated by all rayon workers.
#[derive(Default)]
pub struct Counters {
    pub files: AtomicU64,
    pub dirs: AtomicU64,
    pub bytes: AtomicU64,
    pub errors: AtomicU64,
    pub reparse_skipped: AtomicU64,
    pub unreadable: Mutex<Vec<(String, String)>>,
    pub current_path: Mutex<String>,
    pub cancelled: AtomicBool,
    pub paused: AtomicBool,
}

impl Counters {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
    pub fn pause(&self, on: bool) {
        self.paused.store(on, Ordering::SeqCst);
    }
    pub fn wait_if_paused(&self) {
        while self.paused.load(Ordering::SeqCst) && !self.is_cancelled() {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    pub fn record_unreadable(&self, path: &str, reason: &str) {
        let mut list = self.unreadable.lock().unwrap_or_else(|e| e.into_inner());
        if list.len() < 500 {
            list.push((path.to_string(), reason.to_string()));
        }
    }
}

pub struct ProgressReporter {
    tx: Sender<ScanProgress>,
    counters: std::sync::Arc<Counters>,
    scan_id: u64,
    volume: String,
    started: Instant,
    last_emit: Instant,
    min_interval: Duration,
}

impl ProgressReporter {
    pub fn new(
        tx: Sender<ScanProgress>,
        counters: std::sync::Arc<Counters>,
        scan_id: u64,
        volume: String,
    ) -> Self {
        Self {
            tx,
            counters,
            scan_id,
            volume,
            started: Instant::now(),
            last_emit: Instant::now() - Duration::from_millis(10_000),
            min_interval: Duration::from_millis(150),
        }
    }

    pub fn emit(&mut self, phase: ScanPhase) {
        let now = Instant::now();
        let force = matches!(
            phase,
            ScanPhase::Done | ScanPhase::Failed | ScanPhase::Cancelled
        );
        if !force && now.duration_since(self.last_emit) < self.min_interval {
            return;
        }
        self.last_emit = now;
        let c = &self.counters;
        let _ = self.tx.send(ScanProgress {
            scan_id: self.scan_id,
            volume: self.volume.clone(),
            phase,
            files_seen: c.files.load(Ordering::Relaxed),
            dirs_seen: c.dirs.load(Ordering::Relaxed),
            bytes_seen: c.bytes.load(Ordering::Relaxed),
            errors: c.errors.load(Ordering::Relaxed),
            current_path: c.current_path.lock().map(|p| p.clone()).unwrap_or_default(),
            started_at: unix_to_datetime(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
            ),
            elapsed_ms: self.started.elapsed().as_millis() as u64,
        });
    }
}

fn unix_to_datetime(secs: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(secs, 0).unwrap_or_else(|| {
        use chrono::TimeZone;
        chrono::Utc.timestamp_opt(0, 0).single().unwrap()
    })
}
