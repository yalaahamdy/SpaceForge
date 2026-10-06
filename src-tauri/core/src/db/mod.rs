//! SQLite persistence layer (spec §68).
//!
//! Stores: scan sessions (aggregated), retained files, snapshots,
//! cleanup history, exclusions, settings. Never stores every small file —
//! the index threshold bounds database size on huge disks.

use crate::models::*;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Index {
    conn: Mutex<Connection>,
}

fn db_path() -> PathBuf {
    let base = dirs_data();
    std::fs::create_dir_all(&base).expect("cannot create data dir");
    base.join("spaceforge.index.db")
}

fn dirs_data() -> PathBuf {
    if let Some(d) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(d).join("SpaceForge");
    }
    if let Some(d) = std::env::var_os("HOME") {
        return PathBuf::from(d).join(".local/share/SpaceForge");
    }
    PathBuf::from(".")
}

impl Index {
    pub fn open() -> CoreResult<Index> {
        Self::open_at(&db_path())
    }

    pub fn open_at(path: &Path) -> CoreResult<Index> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        let idx = Index { conn: Mutex::new(conn) };
        idx.migrate()?;
        Ok(idx)
    }

    fn migrate(&self) -> CoreResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS scans (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                volume TEXT NOT NULL,
                kind TEXT NOT NULL,
                started_at TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                total_files INTEGER NOT NULL,
                total_dirs INTEGER NOT NULL,
                total_size INTEGER NOT NULL,
                total_allocated INTEGER NOT NULL,
                unreadable_entries INTEGER NOT NULL,
                indexed_files INTEGER NOT NULL,
                volume_free INTEGER NOT NULL,
                volume_used INTEGER NOT NULL,
                volume_total INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_scans_volume ON scans(volume, started_at DESC);

            CREATE TABLE IF NOT EXISTS scan_files (
                scan_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                name TEXT NOT NULL,
                extension TEXT NOT NULL,
                size INTEGER NOT NULL,
                allocated INTEGER NOT NULL,
                modified INTEGER,
                category TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_files_scan ON scan_files(scan_id, size DESC);
            CREATE INDEX IF NOT EXISTS idx_files_name ON scan_files(name);

            CREATE TABLE IF NOT EXISTS scan_dirs (
                scan_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                parent TEXT NOT NULL,
                name TEXT NOT NULL,
                size INTEGER NOT NULL,
                file_count INTEGER NOT NULL,
                dir_count INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_dirs_scan ON scan_dirs(scan_id, size DESC);

            CREATE TABLE IF NOT EXISTS cleanups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                at TEXT NOT NULL,
                profile TEXT NOT NULL,
                categories TEXT NOT NULL,
                item_count INTEGER NOT NULL,
                estimated_reclaim INTEGER NOT NULL,
                actual_reclaim INTEGER NOT NULL,
                failed_items INTEGER NOT NULL,
                skipped_items INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                detail_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                volume TEXT NOT NULL,
                created_at TEXT NOT NULL,
                scan_id INTEGER NOT NULL,
                total_size INTEGER NOT NULL,
                total_files INTEGER NOT NULL,
                note TEXT
            );

            CREATE TABLE IF NOT EXISTS exclusions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pattern TEXT NOT NULL UNIQUE,
                reason TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    // ---- scans ------------------------------------------------------------

    pub fn save_scan(&self, result: &ScanResult) -> CoreResult<i64> {
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = conn.transaction()?;
        let s = &result.summary;
        tx.execute(
            r#"INSERT INTO scans(volume, kind, started_at, duration_ms, total_files, total_dirs,
                                 total_size, total_allocated, unreadable_entries, indexed_files,
                                 volume_free, volume_used, volume_total)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)"#,
            params![
                s.volume,
                format!("{:?}", s.kind),
                s.started_at.to_rfc3339(),
                s.duration_ms as i64,
                s.total_files as i64,
                s.total_dirs as i64,
                s.total_size as i64,
                s.total_allocated as i64,
                s.unreadable_entries as i64,
                s.indexed_files as i64,
                s.volume_free as i64,
                s.volume_used as i64,
                s.volume_total as i64,
            ],
        )?;
        let scan_id = tx.last_insert_rowid();
        for f in &result.large_files {
            tx.execute(
                r#"INSERT INTO scan_files(scan_id, path, name, extension, size, allocated, modified, category)
                   VALUES (?1,?2,?3,?4,?5,?6,?7,?8)"#,
                params![
                    scan_id,
                    f.path,
                    f.name,
                    f.extension,
                    f.size as i64,
                    f.allocated as i64,
                    f.modified.map(|d| d.timestamp()),
                    f.category.as_str(),
                ],
            )?;
        }
        // Persist the top three tree levels for fast dashboard regrowth.
        fn save_dir(tx: &rusqlite::Transaction, scan_id: i64, n: &DirNode, parent: &str, depth: usize) -> CoreResult<()> {
            tx.execute(
                r#"INSERT INTO scan_dirs(scan_id, path, parent, name, size, file_count, dir_count)
                   VALUES (?1,?2,?3,?4,?5,?6,?7)"#,
                params![scan_id, n.path, parent, n.name, n.size as i64, n.file_count as i64, n.dir_count as i64],
            )?;
            if depth < 3 {
                for c in &n.children {
                    save_dir(tx, scan_id, c, &n.path, depth + 1)?;
                }
            }
            Ok(())
        }
        save_dir(&tx, scan_id, &result.root, "", 0)?;
        tx.commit()?;
        Ok(scan_id)
    }

    pub fn latest_scan(&self, volume: &str) -> CoreResult<Option<(i64, ScanSummary)>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let row = conn
            .query_row(
                r#"SELECT id, started_at, duration_ms, total_files, total_dirs, total_size,
                          total_allocated, unreadable_entries, indexed_files, volume_free,
                          volume_used, volume_total, kind
                   FROM scans WHERE volume = ?1 ORDER BY started_at DESC LIMIT 1"#,
                params![volume],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, i64>(5)?,
                        r.get::<_, i64>(6)?,
                        r.get::<_, i64>(7)?,
                        r.get::<_, i64>(8)?,
                        r.get::<_, i64>(9)?,
                        r.get::<_, i64>(10)?,
                        r.get::<_, i64>(11)?,
                        r.get::<_, String>(12)?,
                    ))
                },
            )
            .optional()?;
        Ok(row.map(|(id, started, dur, files, dirs, size, alloc, unread, idx, free, used, total, _kind)| {
            (
                id,
                ScanSummary {
                    scan_id: id as u64,
                    volume: volume.to_string(),
                    kind: ScanKind::Full,
                    started_at: parse_time(&started),
                    duration_ms: dur as u64,
                    total_files: files as u64,
                    total_dirs: dirs as u64,
                    total_size: size as u64,
                    total_allocated: alloc as u64,
                    unreadable_entries: unread as u64,
                    unreadable_bytes: 0,
                    indexed_files: idx as u64,
                    hard_links_collapsed: 0,
                    reparse_points_skipped: 0,
                    volume_free: free as u64,
                    volume_used: used as u64,
                    volume_total: total as u64,
                },
            )
        }))
    }

    pub fn scan_files(&self, scan_id: i64) -> CoreResult<Vec<FileEntry>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT path, name, extension, size, allocated, modified, category FROM scan_files WHERE scan_id = ?1 ORDER BY size DESC",
        )?;
        let rows = stmt.query_map(params![scan_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, Option<i64>>(5)?,
                r.get::<_, String>(6)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (path, name, ext, size, alloc, modified, cat) = row?;
            out.push(FileEntry {
                path,
                name,
                extension: ext,
                size: size as u64,
                allocated: alloc as u64,
                modified: modified.map(ts_to_dt),
                created: None,
                category: category_from(cat.as_str()),
                file_id: None,
                link_count: None,
                sparse: false,
            });
        }
        Ok(out)
    }

    pub fn scan_dirs(&self, scan_id: i64) -> CoreResult<Vec<(String, String, String, i64, i64, i64)>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT path, parent, name, size, file_count, dir_count FROM scan_dirs WHERE scan_id = ?1 ORDER BY size DESC",
        )?;
        let rows = stmt.query_map(params![scan_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn history_points(&self, volume: &str) -> CoreResult<Vec<HistoryPoint>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT started_at, volume_used, volume_free, total_size, id FROM scans WHERE volume=?1 ORDER BY started_at ASC",
        )?;
        let rows = stmt.query_map(params![volume], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (at, used, free, reclaim, id) = row?;
            out.push(HistoryPoint {
                at: parse_time(&at),
                used: used as u64,
                free: free as u64,
                reclaimable: reclaim as u64,
                scan_id: id as u64,
            });
        }
        Ok(out)
    }

    // ---- cleanups -----------------------------------------------------------

    pub fn save_cleanup(&self, report: &CleanupReport) -> CoreResult<i64> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            r#"INSERT INTO cleanups(at, profile, categories, item_count, estimated_reclaim,
                                    actual_reclaim, failed_items, skipped_items, duration_ms, detail_json)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)"#,
            params![
                report.finished_at.to_rfc3339(),
                format!("{:?}", report.profile),
                serde_json::to_string(&report.entries.iter().map(|e| e.rule_id.clone()).collect::<Vec<_>>())?,
                report.entries.iter().map(|e| e.deleted_items).sum::<u64>() as i64,
                report.estimated_reclaim as i64,
                report.actual_reclaim as i64,
                report.entries.iter().map(|e| e.failed_items).sum::<u64>() as i64,
                report.entries.iter().map(|e| e.skipped_items).sum::<u64>() as i64,
                report.duration_ms as i64,
                serde_json::to_string(report)?,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn cleanup_history(&self) -> CoreResult<Vec<CleanupHistoryEntry>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT id, at, profile, categories, item_count, estimated_reclaim, actual_reclaim, failed_items, skipped_items, duration_ms FROM cleanups ORDER BY at DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, i64>(7)?,
                r.get::<_, i64>(8)?,
                r.get::<_, i64>(9)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, at, _profile, cats, items, est, act, failed, skipped, dur) = row?;
            out.push(CleanupHistoryEntry {
                id,
                at: parse_time(&at),
                profile: CleanupProfile::Custom,
                categories: serde_json::from_str(&cats).unwrap_or_default(),
                item_count: items as u64,
                estimated_reclaim: est as u64,
                actual_reclaim: act as u64,
                failed_items: failed as u64,
                skipped_items: skipped as u64,
                duration_ms: dur as u64,
            });
        }
        Ok(out)
    }

    // ---- snapshots ------------------------------------------------------------

    pub fn save_snapshot(&self, name: &str, volume: &str, scan_id: i64, total_size: u64, total_files: u64, note: Option<&str>) -> CoreResult<i64> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO snapshots(name, volume, created_at, scan_id, total_size, total_files, note) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![name, volume, chrono::Utc::now().to_rfc3339(), scan_id, total_size as i64, total_files as i64, note],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn snapshots(&self, volume: &str) -> CoreResult<Vec<SnapshotMeta>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT id, name, volume, created_at, scan_id, total_size, total_files, note FROM snapshots WHERE volume=?1 ORDER BY created_at DESC",
        )?;
        Self::read_snapshots(&mut stmt, params![volume])
    }

    pub fn snapshots_all(&self) -> CoreResult<Vec<SnapshotMeta>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT id, name, volume, created_at, scan_id, total_size, total_files, note FROM snapshots ORDER BY created_at DESC",
        )?;
        Self::read_snapshots(&mut stmt, params![])
    }

    fn read_snapshots<P: rusqlite::Params>(
        stmt: &mut rusqlite::Statement<'_>,
        p: P,
    ) -> CoreResult<Vec<SnapshotMeta>> {
        let rows = stmt.query_map(p, |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, Option<String>>(7)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, name, vol, created, scan_id, size, files, note) = row?;
            out.push(SnapshotMeta {
                id,
                name,
                volume: vol,
                created_at: parse_time(&created),
                scan_id: scan_id as u64,
                total_size: size as u64,
                total_files: files as u64,
                note,
            });
        }
        Ok(out)
    }

    // ---- exclusions -------------------------------------------------------------

    pub fn exclusions(&self) -> CoreResult<Vec<Exclusion>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare("SELECT id, pattern, reason, created_at FROM exclusions ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, pattern, reason, created) = row?;
            out.push(Exclusion { id, pattern, reason, created_at: parse_time(&created) });
        }
        Ok(out)
    }

    pub fn add_exclusion(&self, pattern: &str, reason: &str) -> CoreResult<i64> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR IGNORE INTO exclusions(pattern, reason, created_at) VALUES (?1,?2,?3)",
            params![pattern, reason, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn remove_exclusion(&self, id: i64) -> CoreResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute("DELETE FROM exclusions WHERE id=?1", params![id])?;
        Ok(())
    }

    // ---- settings ---------------------------------------------------------------

    pub fn load_settings(&self) -> AppSettings {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let value: Option<String> = conn
            .query_row("SELECT value FROM settings WHERE key='app'", [], |r| r.get(0))
            .optional()
            .ok()
            .flatten();
        value
            .and_then(|v| serde_json::from_str(&v).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, s: &AppSettings) -> CoreResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO settings(key, value) VALUES ('app', ?1) ON CONFLICT(key) DO UPDATE SET value=?1",
            params![serde_json::to_string(s)?],
        )?;
        Ok(())
    }
}

fn parse_time(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

fn ts_to_dt(secs: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(secs, 0).unwrap_or_else(|| {
        use chrono::TimeZone;
        chrono::Utc.timestamp_opt(0, 0).single().unwrap()
    })
}

fn category_from(s: &str) -> FileCategory {
    match s {
        "video" => FileCategory::Video,
        "audio" => FileCategory::Audio,
        "image" => FileCategory::Image,
        "document" => FileCategory::Document,
        "archive" => FileCategory::Archive,
        "disk_image" => FileCategory::DiskImage,
        "executable" => FileCategory::Executable,
        "library" => FileCategory::Library,
        "code" => FileCategory::Code,
        "game" => FileCategory::Game,
        "system" => FileCategory::System,
        "cache" => FileCategory::Cache,
        "temporary" => FileCategory::Temporary,
        "log" => FileCategory::Log,
        "database" => FileCategory::Database,
        "developer" => FileCategory::Developer,
        _ => FileCategory::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_index() -> Index {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        // Leak the tempdir for the lifetime of the test process (small).
        let path = path.to_path_buf();
        std::mem::forget(dir);
        Index::open_at(&path).unwrap()
    }

    #[test]
    fn settings_roundtrip() {
        let idx = temp_index();
        let mut s = AppSettings::default();
        s.theme = "light".into();
        s.large_file_threshold = 12345;
        idx.save_settings(&s).unwrap();
        let loaded = idx.load_settings();
        assert_eq!(loaded.theme, "light");
        assert_eq!(loaded.large_file_threshold, 12345);
    }

    #[test]
    fn exclusions_roundtrip() {
        let idx = temp_index();
        idx.add_exclusion("D:\\Games\\**", "keep games").unwrap();
        let list = idx.exclusions().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].pattern, "D:\\Games\\**");
        idx.remove_exclusion(list[0].id).unwrap();
        assert!(idx.exclusions().unwrap().is_empty());
    }

    #[test]
    fn cleanup_history_roundtrip() {
        let idx = temp_index();
        let report = CleanupReport {
            plan_id: "p1".into(),
            profile: CleanupProfile::SmartClean,
            started_at: chrono::Utc::now(),
            finished_at: chrono::Utc::now(),
            duration_ms: 42,
            estimated_reclaim: 1000,
            actual_reclaim: 900,
            entries: vec![],
            free_before: 100,
            free_after: 1000,
        };
        idx.save_cleanup(&report).unwrap();
        let hist = idx.cleanup_history().unwrap();
        assert_eq!(hist.len(), 1);
        assert_eq!(hist[0].actual_reclaim, 900);
    }
}
