//! Command execution helpers for Windows-managed mechanisms.
//!
//! Supported mechanisms only: DISM, powercfg, PowerShell cmdlets, vssadmin.
//! The tool never replaces these with raw filesystem deletion.

/// Run a command, optionally requesting elevation through UAC.
/// Returns trimmed stdout (or stderr on failure).
pub fn run_command(program: &str, args: &[String], elevate: bool) -> Result<String, String> {
    #[cfg(not(windows))]
    {
        let _ = (program, args, elevate);
        return Err("Windows commands are unavailable on this platform".into());
    }
    #[cfg(windows)]
    {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    if elevate {
        // Elevation through UAC is always explicit and user-initiated; the
        // UI explains what will run before this point (spec §42).
        let arg_str = args
            .iter()
            .map(|a| {
                if a.contains(' ') {
                    format!("\"{a}\"")
                } else {
                    a.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let ps = format!(
            "Start-Process -FilePath '{program}' -ArgumentList '{arg_str}' -Verb RunAs -Wait -PassThru | Select-Object -ExpandProperty ExitCode"
        );
        let out = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps])
            .creation_flags(0x0800_0000)
            .output();
        match out {
            Ok(o) if o.status.success() => {
                let code = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if code == "0" {
                    Ok(String::new())
                } else {
                    Err(format!("elevated process exited with code {code}"))
                }
            }
            Ok(o) => Err(String::from_utf8_lossy(&o.stderr).trim().to_string()),
            Err(e) => Err(e.to_string()),
        }
    } else {
        let out = Command::new(program)
            .args(args)
            .creation_flags(0x0800_0000)
            .output();
        match out {
            Ok(o) if o.status.success() => {
                Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
            }
            Ok(o) => Err(String::from_utf8_lossy(&o.stderr).trim().to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
    }
}

/// DISM component-store analysis output (parsed key fields).
pub fn analyze_component_store() -> Result<ComponentStoreInfo, String> {
    // DISM requires an elevated prompt. When the app itself is elevated this
    // succeeds and output is parsed; otherwise the error is surfaced honestly.
    let out = run_command(
        "dism",
        &[
            "/Online".into(),
            "/Cleanup-Image".into(),
            "/AnalyzeComponentStore".into(),
        ],
        false,
    )?;
    Ok(parse_dism_analyze(&out))
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ComponentStoreInfo {
    pub actual_size_bytes: u64,
    pub cleanup_recommended: bool,
    pub rollback_capable: bool,
    pub has_repair_content: bool,
    pub raw: String,
}

fn parse_size_mb(s: &str) -> u64 {
    let num: String = s.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
    num.parse::<f64>()
        .map(|v| (v * 1024.0 * 1024.0) as u64)
        .unwrap_or(0)
}

pub fn parse_dism_analyze(out: &str) -> ComponentStoreInfo {
    let mut info = ComponentStoreInfo { raw: out.to_string(), ..Default::default() };
    for line in out.lines() {
        let l = line.trim();
        let lower = l.to_lowercase();
        if lower.contains("actual size of component store") {
            info.actual_size_bytes = parse_size_mb(l);
        }
        if lower.contains("component store cleanup recommended") {
            info.cleanup_recommended = lower.contains("yes");
        }
        if lower.contains("windows rollback capability") {
            info.rollback_capable = lower.contains("yes") || lower.contains("disabled");
        }
        if lower.contains("has repair content") {
            info.has_repair_content = lower.contains("true");
        }
    }
    info
}

/// Hibernation status: (enabled, hiberfil_size).
pub fn hibernation_status() -> (bool, u64) {
    let mut size = 0u64;
    // The authoritative signal is the hiberfil itself: it exists exactly
    // while hibernation (or Fast Startup) is enabled.
    let win_dir = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let sysdrive = win_dir.chars().take(2).collect::<String>();
    let hiberfil = format!("{sysdrive}\\hiberfil.sys");
    let enabled = match std::fs::metadata(&hiberfil) {
        Ok(md) => {
            size = md.len();
            true
        }
        Err(_) => false,
    };
    (enabled, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dism_output() {
        let sample = "\
Image Version: 10.0.22631.4037
[==========================] 100.0%
Component Store (WinSxS) information:
Windows Explorer view size of Component Store : 12.34 GB
Actual Size of Component Store : 9412 MB
Shared with Windows : 6.13 GB
Component Store Cleanup Recommended : Yes
";
        let info = parse_dism_analyze(sample);
        assert!(info.cleanup_recommended);
        assert!(info.actual_size_bytes > 9_000_000_000);
    }
}
