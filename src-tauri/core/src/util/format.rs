//! Byte and duration formatting shared with the frontend.

/// Human size using binary units (KiB/MiB/GiB) — matches what Windows
/// Explorer shows for file sizes, which is what users compare against.
pub fn format_bytes_short(bytes: u64) -> String {
    const UNITS: [&str; 7] = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else if value >= 10.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

pub fn format_duration(ms: u64) -> String {
    let secs = ms / 1000;
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(format_bytes_short(0), "0 B");
        assert_eq!(format_bytes_short(512), "512 B");
        assert_eq!(format_bytes_short(1024), "1.00 KB");
        assert_eq!(format_bytes_short(1536), "1.50 KB");
        assert_eq!(format_bytes_short(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes_short(742 * 1024_u64.pow(2)), "742 MB");
        assert_eq!(format_bytes_short(1024_u64.pow(4)), "1.00 TB");
    }
}
