//! Explorer / shell integration (spec §66). All best-effort, never crash.

/// Open Explorer with the item selected (or open the folder itself when
/// `select` is false).
pub fn open_in_explorer(path: &str, select: bool) -> Result<(), crate::models::CoreError> {
    #[cfg(windows)]
    {
        use std::process::Command;
        let arg = if select {
            format!("/select,{path}")
        } else {
            path.to_string()
        };
        Command::new("explorer.exe")
            .arg(arg)
            .spawn()
            .map_err(|e| crate::models::CoreError::WindowsOp(e.to_string()))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = select;
        open_path(path)
    }
}

/// Open a path with its default application.
pub fn open_path(path: &str) -> Result<(), crate::models::CoreError> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        Command::new("cmd")
            .args(["/C", "start", "", path])
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| crate::models::CoreError::WindowsOp(e.to_string()))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        use std::process::Command;
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| crate::models::CoreError::Other(e.to_string()))?;
        Ok(())
    }
}

/// Show file properties dialog (Windows shell).
pub fn file_properties(path: &str) -> Result<(), crate::models::CoreError> {
    #[cfg(windows)]
    {
        // The shell properties verb via PowerShell is the supported path.
        use std::process::Command;
        let script = format!(
            "(New-Object -ComObject Shell.Application).NameSpace('{}')",
            std::path::Path::new(path)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default()
                .replace('\\', "\\\\")
        );
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
            .spawn();
        // Fallback: select in Explorer (properties are one key away).
        open_in_explorer(path, true)
    }
    #[cfg(not(windows))]
    {
        open_in_explorer(path, false)
    }
}
