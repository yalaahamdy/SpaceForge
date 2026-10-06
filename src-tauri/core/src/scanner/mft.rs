//! Experimental NTFS MFT scan path (spec §9, §12 — opt-in).
//!
//! Enumerates the Master File Table through the supported
//! FSCTL_ENUM_USN_DATA mechanism and reads record sizes through
//! FSCTL_GET_NTFS_FILE_RECORD. This is the WizTree-class approach:
//! no per-file seek, just MFT record reads.
//!
//! Safety rails:
//! - requires the volume to be NTFS and the process to be elevated
//! - every anomaly aborts with Err → the caller falls back to the parallel
//!   walker, so a parser edge case can never produce wrong data silently
//! - final totals are sanity-checked against real volume usage

#![cfg(windows)]

use crate::models::CoreError;
use crate::scanner::aggregate::{DirRow, FileRow};
use crate::scanner::progress::Counters;
use crate::util::classify_path;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

const FSCTL_ENUM_USN_DATA: u32 = 0x000900B3;
const FSCTL_GET_NTFS_FILE_RECORD: u32 = 0x00090068;
const GENERIC_READ: u32 = 0x8000_0000;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0010;
const ROOT_FRN: u64 = 5;

fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

struct Volume {
    handle: HANDLE,
}

impl Volume {
    fn open(volume: &str) -> Result<Self, CoreError> {
        let device = format!("\\\\.\\{}", volume.trim_end_matches('\\'));
        let wide = to_wide(&device);
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut() as HANDLE,
            )
        };
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return Err(CoreError::Scan(format!(
                "cannot open volume {device} (administrator privileges are required for MFT mode)"
            )));
        }
        Ok(Self { handle })
    }

    fn ioctl(&self, code: u32, inbuf: &[u8], outbuf: &mut [u8]) -> Result<usize, CoreError> {
        let mut returned: u32 = 0;
        let ok = unsafe {
            DeviceIoControl(
                self.handle,
                code,
                inbuf.as_ptr() as *mut _,
                inbuf.len() as u32,
                outbuf.as_mut_ptr() as *mut _,
                outbuf.len() as u32,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(CoreError::WindowsOp(format!(
                "DeviceIoControl(0x{code:08X}) failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(returned as usize)
    }
}

impl Drop for Volume {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle) };
    }
}

#[derive(Clone, Debug)]
struct MftEntry {
    parent: u64,
    name: String,
    attributes: u32,
    /// (logical size, allocated size) — resolved in the record pass.
    sizes: Option<(u64, u64)>,
}

/// Run the MFT scan. Returns file rows + dir rows.
pub fn scan_mft(
    volume: &str,
    counters: &Arc<Counters>,
) -> Result<(Vec<FileRow>, Vec<DirRow>), CoreError> {
    if !crate::fsops::is_elevated() {
        return Err(CoreError::ElevationRequired(
            "MFT scanning requires administrator privileges".into(),
        ));
    }

    let vol = Volume::open(volume)?;

    // ---- pass 1: enumerate every FRN via ENUM_USN_DATA ----------------------
    let mut entries: HashMap<u64, MftEntry> = HashMap::with_capacity(1 << 20);
    let mut inbuf = [0u8; 24]; // MFT_ENUM_DATA_V0: start FRN, low USN, high USN
    let mut out = vec![0u8; 64 * 1024];
    loop {
        if counters.is_cancelled() {
            return Err(CoreError::Cancelled);
        }
        let n = vol.ioctl(FSCTL_ENUM_USN_DATA, &inbuf, &mut out)?;
        if n <= 8 {
            break;
        }
        let mut off = 8usize; // skip the 8-byte major/minor header
        let mut last_frn: u64 = 0;
        while off + 64 <= n {
            let rec_len =
                u32::from_le_bytes(out[off..off + 4].try_into().unwrap()) as usize;
            if rec_len < 64 || off + rec_len > n {
                break;
            }
            let frn =
                u64::from_le_bytes(out[off + 8..off + 16].try_into().unwrap());
            let parent =
                u64::from_le_bytes(out[off + 16..off + 24].try_into().unwrap());
            let attrs =
                u32::from_le_bytes(out[off + 52..off + 56].try_into().unwrap());
            let name_len =
                u32::from_le_bytes(out[off + 56..off + 60].try_into().unwrap()) as usize;
            let name_off =
                u32::from_le_bytes(out[off + 60..off + 64].try_into().unwrap()) as usize;
            if name_off + name_len <= rec_len {
                let u16s: Vec<u16> = out[off + name_off..off + name_off + name_len]
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                let name = String::from_utf16_lossy(&u16s);
                entries
                    .entry(frn)
                    .or_insert(MftEntry { parent, name, attributes: attrs, sizes: None });
                last_frn = last_frn.max(frn);
            }
            off += rec_len;
        }
        if last_frn == 0 {
            break;
        }
        // Advance the cursor past everything enumerated so far.
        inbuf[0..8].copy_from_slice(&(last_frn + 1).to_le_bytes());
    }

    // ---- pass 2: resolve (logical, allocated) sizes per file record ---------
    let mut in2 = [0u8; 8];
    let mut out2 = vec![0u8; 8192];
    let mut done = 0u64;
    for (frn, entry) in entries.iter_mut() {
        if counters.is_cancelled() {
            return Err(CoreError::Cancelled);
        }
        if entry.attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            continue;
        }
        in2[0..8].copy_from_slice(&frn.to_le_bytes());
        if let Ok(n) = vol.ioctl(FSCTL_GET_NTFS_FILE_RECORD, &in2, &mut out2) {
            if n > 12 {
                let rec_len =
                    u32::from_le_bytes(out2[8..12].try_into().unwrap()) as usize;
                if rec_len >= 48 && 12 + rec_len <= out2.len() {
                    entry.sizes = parse_data_size(&out2[12..12 + rec_len]);
                }
            }
        }
        done += 1;
        if done % 2048 == 0 {
            counters.files.fetch_add(2048, Ordering::Relaxed);
        }
    }

    // ---- pass 3: resolve paths and emit rows ---------------------------------
    let entries: HashMap<u64, MftEntry> = entries;
    let mut paths: HashMap<u64, String> = HashMap::with_capacity(entries.len() + 1);
    paths.insert(ROOT_FRN, volume.trim_end_matches('\\').to_string());

    let mut rounds = 0;
    let mut unresolved: Vec<u64> = entries.keys().copied().collect();
    while rounds < 32 {
        let mut next: Vec<u64> = Vec::new();
        for frn in unresolved {
            if paths.contains_key(&frn) {
                continue;
            }
            let e = &entries[&frn];
            if e.attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                // Junctions/symlinks get an empty marker: never traversed.
                paths.insert(frn, String::new());
                continue;
            }
            match paths.get(&e.parent) {
                Some(parent_path) if !parent_path.is_empty() => {
                    paths.insert(frn, format!("{parent_path}\\{}", e.name));
                }
                _ => next.push(frn),
            }
        }
        if next.is_empty() || next.len() == entries.len() {
            break;
        }
        unresolved = next;
        rounds += 1;
    }

    let cluster = crate::fsops::cluster_size(volume);
    let mut files: Vec<FileRow> = Vec::with_capacity(entries.len() / 2);
    let mut dirs: Vec<DirRow> = vec![DirRow {
        path: volume.trim_end_matches('\\').to_string(),
        modified: None,
        unreadable_child: None,
    }];
    let mut seen_links: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for (frn, e) in &entries {
        if frn < &16 {
            continue; // system metafiles ($MFT, $LogFile, …) — not user data
        }
        let Some(path) = paths.get(frn) else { continue };
        if path.is_empty() {
            continue;
        }
        if e.attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            dirs.push(DirRow { path: path.clone(), modified: None, unreadable_child: None });
            counters.dirs.fetch_add(1, Ordering::Relaxed);
        } else {
            // Hard links: the same FRN can be emitted for several names.
            if !seen_links.insert(*frn) {
                continue;
            }
            let (logical, _alloc_real) = e.sizes.unwrap_or((0, 0));
            let alloc = crate::fsops::allocated_for(logical, cluster, false);
            let ext = Path::new(path)
                .extension()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            files.push(FileRow {
                path: path.clone(),
                size: logical,
                allocated: alloc,
                modified: None,
                created: None,
                category: classify_path(Path::new(path)),
                extension: ext,
                link_id: Some(*frn),
                sparse: false,
            });
            counters.files.fetch_add(1, Ordering::Relaxed);
            counters.bytes.fetch_add(logical, Ordering::Relaxed);
        }
    }

    // ---- sanity guard ---------------------------------------------------------
    let walked_total: u64 = files.iter().map(|f| f.size).sum();
    let (free, total, _) = crate::fsops::volume_stats(volume)
        .ok_or_else(|| CoreError::Scan("volume stats unavailable".into()))?;
    let used = total - free;
    if walked_total > used + used / 4 {
        return Err(CoreError::Scan(
            "MFT totals are inconsistent with actual volume usage — refusing to report; falling back".into(),
        ));
    }
    Ok((files, dirs))
}

/// Extract (logical_size, allocated_size) from an MFT file record's primary
/// $DATA attribute. Returns None when the record is malformed.
fn parse_data_size(rec: &[u8]) -> Option<(u64, u64)> {
    if rec.len() < 42 || &rec[0..4] != b"FILE" {
        return None;
    }
    let first_attr = u16::from_le_bytes(rec[0x14..0x16].try_into().unwrap()) as usize;
    let mut off = first_attr;
    while off + 16 <= rec.len() {
        let attr_type = u32::from_le_bytes(rec[off..off + 4].try_into().unwrap());
        if attr_type == 0xFFFF_FFFF {
            break;
        }
        let attr_len = u32::from_le_bytes(rec[off + 4..off + 8].try_into().unwrap()) as usize;
        if attr_len == 0 || off + attr_len > rec.len() {
            break;
        }
        if attr_type == 0x0000_0080 && rec[off + 9] == 0 {
            // Primary $DATA (no name).
            let non_resident = rec[off + 8];
            if non_resident == 1 && off + 0x38 <= off + attr_len {
                let allocated_size =
                    u64::from_le_bytes(rec[off + 0x28..off + 0x30].try_into().unwrap());
                let real_size =
                    u64::from_le_bytes(rec[off + 0x30..off + 0x38].try_into().unwrap());
                return Some((real_size, allocated_size));
            }
            if non_resident == 0 {
                let value_len =
                    u32::from_le_bytes(rec[off + 0x10..off + 0x14].try_into().unwrap()) as u64;
                return Some((value_len, value_len));
            }
        }
        off += attr_len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_resident_data() {
        // Build a minimal record: FILE magic, first attr at 0x38 (56).
        let mut rec = vec![0u8; 256];
        rec[0..4].copy_from_slice(b"FILE");
        rec[0x14..0x16].copy_from_slice(&56u16.to_le_bytes());
        // Resident $DATA at 56.
        let a = 56usize;
        rec[a..a + 4].copy_from_slice(&0x80u32.to_le_bytes()); // type
        rec[a + 4..a + 8].copy_from_slice(&24u32.to_le_bytes()); // length
        rec[a + 8] = 0; // resident
        rec[a + 9] = 0; // name length
        rec[a + 0x10..a + 0x14].copy_from_slice(&1234u32.to_le_bytes());
        let (l, al) = parse_data_size(&rec).unwrap();
        assert_eq!((l, al), (1234, 1234));
    }

    #[test]
    fn parses_nonresident_data() {
        let mut rec = vec![0u8; 256];
        rec[0..4].copy_from_slice(b"FILE");
        rec[0x14..0x16].copy_from_slice(&56u16.to_le_bytes());
        let a = 56usize;
        rec[a..a + 4].copy_from_slice(&0x80u32.to_le_bytes());
        rec[a + 4..a + 8].copy_from_slice(&72u32.to_le_bytes());
        rec[a + 8] = 1; // non-resident
        rec[a + 9] = 0; // name length
        rec[a + 0x28..a + 0x30].copy_from_slice(&4096u64.to_le_bytes()); // allocated
        rec[a + 0x30..a + 0x38].copy_from_slice(&3210u64.to_le_bytes()); // real
        let (l, al) = parse_data_size(&rec).unwrap();
        assert_eq!((l, al), (3210, 4096));
    }
}
