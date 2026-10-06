//! Shared application state: the index database, settings, the active scan
//! handle and the duplicate-search cancellation flag.

use spaceforge_core::db::Index;
use spaceforge_core::models::{AppSettings, ScanResult};
use spaceforge_core::scanner::ScanControl;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

pub struct AppState {
    pub index: Index,
    pub settings: RwLock<AppSettings>,
    scan: Mutex<Option<ActiveScan>>,
    pub scan_result: RwLock<Option<Arc<ScanResult>>>,
    pub duplicates_cancel: Arc<AtomicBool>,
    /// Total wasted bytes from the last duplicate run (for recommendations).
    pub duplicate_waste: RwLock<u64>,
    /// Total reclaimable bytes measured by the latest cleanup discovery.
    pub last_reclaimable_bytes: RwLock<u64>,
}

pub struct ActiveScan {
    pub control: ScanControl,
    pub volume: String,
}

impl AppState {
    pub fn new(index: Index, settings: AppSettings) -> Self {
        Self {
            index,
            settings: RwLock::new(settings),
            scan: Mutex::new(None),
            scan_result: RwLock::new(None),
            duplicates_cancel: Arc::new(AtomicBool::new(false)),
            duplicate_waste: RwLock::new(0),
            last_reclaimable_bytes: RwLock::new(0),
        }
    }

    pub fn last_reclaimable(&self) -> u64 {
        self.last_reclaimable_bytes
            .read()
            .map(|g| *g)
            .unwrap_or(0)
    }

    pub fn set_scan(&self, active: ActiveScan) {
        let mut s = self.scan.lock().unwrap_or_else(|e| e.into_inner());
        *s = Some(active);
    }

    pub fn take_scan(&self) -> Option<ActiveScan> {
        let mut s = self.scan.lock().unwrap_or_else(|e| e.into_inner());
        s.take()
    }

    pub fn has_active_scan(&self) -> bool {
        self.scan.lock().map(|s| s.is_some()).unwrap_or(false)
    }

    pub fn active_control(&self) -> Option<ScanControl> {
        self.scan
            .lock()
            .ok()
            .and_then(|s| s.as_ref().map(|a| ScanControl { counters: a.control.counters.clone() }))
    }

    pub fn cancel_scan(&self) {
        if let Some(c) = self.active_control() {
            c.cancel();
        }
        self.duplicates_cancel.store(true, Ordering::SeqCst);
    }
}
