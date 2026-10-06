//! Deletion semantics: Recyclable vs Permanent vs Secure (spec §37, §38).
//!
//! - User files → Recycle Bin via the OS shell mechanism (`trash` crate,
//!   which uses IFileOperation on Windows) so normal Windows restore works.
//! - Cache/temp cleanup → permanent delete of *rule-whitelisted* paths only,
//!   after layered validation.
//! - Secure delete → best-effort overwrite; honest limits are documented
//!   (SSD wear-leveling, snapshots, compression make guarantees impossible).

use crate::models::CoreError;
use std::path::Path;

/// Move paths to the Recycle Bin (restorable through normal Windows UI).
pub fn recycle(paths: &[String]) -> Result<usize, CoreError> {
    let mut count = 0usize;
    for p in paths {
        match trash::delete(Path::new(p)) {
            Ok(_) => count += 1,
            Err(e) => {
                // Missing files are skipped, not failures.
                if !Path::new(p).exists() {
                    continue;
                }
                return Err(CoreError::Other(format!("recycle {p}: {e}")));
            }
        }
    }
    Ok(count)
}

/// Permanently delete one path (file or directory tree). Missing paths are
/// reported as skipped, not failed. Read-only attributes are cleared.
pub fn delete_permanent(path: &str) -> Result<bool, CoreError> {
    let p = Path::new(path);
    if !p.exists() {
        return Ok(false); // skipped
    }
    if p.is_dir() {
        clear_readonly_recursive(p)?;
        std::fs::remove_dir_all(p).map_err(CoreError::Io)?;
        Ok(true)
    } else {
        clear_readonly(p);
        std::fs::remove_file(p).map_err(CoreError::Io)?;
        Ok(true)
    }
}

/// Delete the *contents* of a directory while keeping the directory itself.
/// This is the safest pattern for cache roots owned by running applications.
pub fn clear_dir_contents(path: &str) -> Result<(u64, u64), CoreError> {
    let p = Path::new(path);
    if !p.is_dir() {
        return Ok((0, 0));
    }
    let mut deleted = 0u64;
    let mut failed = 0u64;
    let mut entries: Vec<std::fs::DirEntry> = Vec::new();
    for e in std::fs::read_dir(p)? {
        match e {
            Ok(e) => entries.push(e),
            Err(_) => failed += 1,
        }
    }
    for e in entries {
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let res = if is_dir {
            let _ = clear_readonly_recursive(&e.path());
            std::fs::remove_dir_all(e.path())
        } else {
            clear_readonly(&e.path());
            std::fs::remove_file(e.path())
        };
        match res {
            Ok(_) => deleted += 1,
            Err(_) => failed += 1,
        }
    }
    Ok((deleted, failed))
}

fn clear_readonly(p: &Path) {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if let Ok(md) = std::fs::metadata(p) {
            const READONLY: u32 = 0x1;
            if md.file_attributes() & READONLY != 0 {
                let mut perms = md.permissions();
                perms.set_readonly(false);
                let _ = std::fs::set_permissions(p, perms);
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = p;
    }
}

fn clear_readonly_recursive(root: &Path) -> std::io::Result<()> {
    clear_readonly(root);
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            clear_readonly_recursive(&entry.path())?;
        } else {
            clear_readonly(&entry.path());
        }
    }
    Ok(())
}

/// Best-effort secure delete: overwrite content with zeros then a random
/// pass, then remove. On SSDs, wear-leveling means this cannot guarantee
/// media-level erasure — the caller must present this honestly (spec §38).
pub fn secure_delete_file(path: &str) -> Result<bool, CoreError> {
    let p = Path::new(path);
    if !p.is_file() {
        return Ok(false);
    }
    let md = std::fs::metadata(p)?;
    let len = md.len();
    if len == 0 {
        std::fs::remove_file(p)?;
        return Ok(true);
    }
    {
        let mut f = std::fs::OpenOptions::new().write(true).open(p)?;
        use std::io::{Seek, SeekFrom, Write};
        let chunk = 1024 * 1024;
        let zeros = vec![0u8; chunk];
        let ones = vec![0xFFu8; chunk];
        let mut rnd = vec![0u8; chunk];
        getrandom_fill(&mut rnd);
        for pattern in [&zeros, &ones, &rnd] {
            f.seek(SeekFrom::Start(0))?;
            let mut left = len;
            while left > 0 {
                let n = left.min(chunk as u64) as usize;
                f.write_all(&pattern[..n])?;
                left -= n as u64;
            }
            f.flush()?;
        }
        f.sync_all()?;
    }
    std::fs::remove_file(p)?;
    Ok(true)
}

fn getrandom_fill(buf: &mut [u8]) {
    // Small custom PRNG seeded from time/address entropy — sufficient for
    // overwrite patterns; this is not a crypto primitive.
    let mut state = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E3779B97F4A7C15)
        | 1;
    for b in buf.iter_mut() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *b = (state >> 24) as u8;
    }
}
