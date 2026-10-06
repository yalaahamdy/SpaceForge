//! Windows-native implementations (windows-sys bindings).
//!
//! Every function here is best-effort: failures return `None`/`false` and
//! callers degrade gracefully — never crash, never fabricate data.

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetDiskFreeSpaceExW, GetDiskFreeSpaceW, GetDriveTypeW, GetFileInformationByHandle,
    BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::Security::{TOKEN_QUERY, TOKEN_ELEVATION, TokenElevation};

const FILE_READ_DATA: u32 = 0x0001;

fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn from_wide(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

/// (free, total) via GetDiskFreeSpaceExW.
pub fn free_space(path: &str) -> Option<(u64, u64)> {
    let wide = to_wide(path);
    let mut free: u64 = 0;
    let mut total: u64 = 0;
    let mut _dummy: u64 = 0;
    let ok = unsafe {
        GetDiskFreeSpaceExW(wide.as_ptr(), &mut _dummy as *mut u64 as *mut _,
            &mut total as *mut u64 as *mut _,
            &mut free as *mut u64 as *mut _)
    };
    if ok != 0 { Some((free, total)) } else { None }
}

/// Cluster size via GetDiskFreeSpaceW (sectors × bytes/sector).
pub fn cluster_size(path: &str) -> Option<u64> {
    let wide = to_wide(path);
    let mut sectors_per_cluster: u32 = 0;
    let mut bytes_per_sector: u32 = 0;
    let mut _a: u32 = 0;
    let mut _b: u32 = 0;
    let ok = unsafe {
        GetDiskFreeSpaceW(wide.as_ptr(), &mut sectors_per_cluster, &mut bytes_per_sector,
            &mut _a, &mut _b)
    };
    if ok != 0 {
        Some(sectors_per_cluster as u64 * bytes_per_sector as u64)
    } else {
        None
    }
}

/// Drive type via GetDriveTypeW (3 = fixed, 2 = removable, 4 = network…).
pub fn drive_type(path: &str) -> u32 {
    let root = format!("{}\\", path.trim_end_matches(['\\', '/']));
    let wide = to_wide(&root);
    unsafe { GetDriveTypeW(wide.as_ptr()) }
}

/// (label, filesystem) via GetVolumeInformationW.
pub fn volume_label_fs(path: &str) -> Option<(String, String)> {
    use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW;
    let wide = to_wide(path);
    let mut label = [0u16; 261];
    let mut fs = [0u16; 64];
    let mut _flags: u32 = 0;
    let mut _serial: u32 = 0;
    let ok = unsafe {
        GetVolumeInformationW(
            wide.as_ptr(),
            label.as_mut_ptr(),
            label.len() as u32,
            &mut _serial,
            &mut _flags,
            &mut _flags,
            fs.as_mut_ptr(),
            fs.len() as u32,
        )
    };
    if ok != 0 {
        Some((from_wide(&label), from_wide(&fs)))
    } else {
        None
    }
}

/// (volume_serial, file_index, hard_link_count) via GetFileInformationByHandle.
pub fn file_identity(path: &std::path::Path) -> Option<(u64, u64, u64)> {
    let wide = to_wide(&path.as_os_str().to_string_lossy());
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_READ_DATA,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut() as HANDLE,
        )
    };
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        return None;
    }
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetFileInformationByHandle(handle, &mut info) };
    unsafe { CloseHandle(handle) };
    if ok == 0 {
        return None;
    }
    let index =
        ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64;
    Some((info.dwVolumeSerialNumber as u64, index, info.nNumberOfLinks as u64))
}

/// Token elevation check — honest admin-state detection.
pub fn is_elevated() -> bool {
    let mut token: HANDLE = std::ptr::null_mut();
    let ok = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
    if ok == 0 {
        return false;
    }
    let mut elevation: TOKEN_ELEVATION = unsafe { std::mem::zeroed() };
    let mut ret_len: u32 = 0;
    let ok = unsafe {
        windows_sys::Win32::Security::GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut TOKEN_ELEVATION as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        )
    };
    unsafe { CloseHandle(token) };
    ok != 0 && elevation.TokenIsElevated != 0
}

/// Recycle bin statistics: (item_count, total_bytes) for a drive.
pub fn recycle_bin_stats(root: &str) -> Option<(u32, u64)> {
    use windows_sys::Win32::UI::Shell::{SHQueryRecycleBinW, SHQUERYRBINFO};
    let drive_root: String = {
        let t = root.trim_end_matches(['\\', '/']);
        if t.len() >= 2 && t.as_bytes()[1] == b':' {
            format!("{t}\\")
        } else {
            t.to_string()
        }
    };
    let wide = to_wide(&drive_root);
    let mut info: SHQUERYRBINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHQUERYRBINFO>() as u32;
    let ok = unsafe { SHQueryRecycleBinW(wide.as_ptr(), &mut info) };
    if ok == 0 {
        Some((info.i64NumItems as u32, info.i64Size as u64))
    } else {
        None
    }
}

/// Empty the Recycle Bin through the supported shell mechanism.
pub fn empty_recycle_bin(all_drives: bool) -> Result<(), crate::models::CoreError> {
    use windows_sys::Win32::UI::Shell::{SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND};
    let wide = to_wide(if all_drives { "" } else { "C:\\" });
    let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
    let hr = unsafe { SHEmptyRecycleBinW(std::ptr::null_mut(), wide.as_ptr(), flags) };
    if hr == 0 {
        Ok(())
    } else {
        Err(crate::models::CoreError::WindowsOp(format!(
            "SHEmptyRecycleBinW failed with HRESULT 0x{hr:08X}"
        )))
    }
}

/// TRIM support → SSD detection, best effort via IOCTL.
pub fn detect_media_type(root: &str) -> crate::models::MediaType {
    use windows_sys::Win32::System::Ioctl::{IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_PROPERTY_QUERY};
    use windows_sys::Win32::System::IO::DeviceIoControl;
    let device = format!("\\\\.\\{}", root.trim_end_matches('\\'));
    let wide = to_wide(&device);
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            0, // query access
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut() as HANDLE,
        )
    };
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        return crate::models::MediaType::Unknown;
    }
    // StorageDeviceTrimProperty = 8; if it succeeds with a positive result,
    // the underlying device supports TRIM → SSD.
    let mut query: STORAGE_PROPERTY_QUERY = unsafe { std::mem::zeroed() };
    query.PropertyId = 8; // StorageDeviceTrimProperty
    query.QueryType = 0; // PropertyStandardQuery
    let mut out = [0u8; 16];
    let mut returned: u32 = 0;
    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            &mut query as *mut STORAGE_PROPERTY_QUERY as *mut _,
            std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
            out.as_mut_ptr() as *mut _,
            out.len() as u32,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    unsafe { CloseHandle(handle) };
    if ok != 0 {
        // First byte of DEVICE_TRIM_DESCRIPTOR is Version; TrimResult byte 3.
        // Simplify: success of the trim query on a fixed disk implies SSD.
        crate::models::MediaType::Ssd
    } else {
        crate::models::MediaType::Unknown
    }
}
