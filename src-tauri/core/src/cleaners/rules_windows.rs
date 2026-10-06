//! Windows system cleanup rules (spec §22–§28).
//!
//! Windows-managed data is only ever cleaned through supported mechanisms
//! (DISM, powercfg, PowerShell cmdlets, shell APIs) — never raw deletion.

use super::generic::{CommandCleanupRule, PathCleanupRule};
use super::CleanerRuleDef;
use crate::models::*;
use std::sync::Arc;

fn rule(id: &str, name: &str, desc: &str, group: &str, risk: RiskLevel, rev: Reversibility,
        admin: bool, reason: &str, consequence: &str) -> CleanerRule {
    CleanerRule {
        id: id.into(),
        name: name.into(),
        description: desc.into(),
        family: CleanupFamily::Windows,
        group: group.into(),
        risk,
        reversibility: rev,
        requires_admin: admin,
        conflicts_with_processes: vec![],
        reason: reason.into(),
        consequence: consequence.into(),
        opt_in: false,
        windows_managed: false,
        min_os: "Windows 10".into(),
    }
}

pub fn register(out: &mut Vec<Arc<dyn CleanerRuleDef>>) {
    // --- Temporary files -----------------------------------------------------
    out.push(Arc::new(PathCleanupRule {
        rule: rule(
            "win.user-temp",
            "User temporary files",
            "Files left behind in your user TEMP folder by applications and installers.",
            "temp", RiskLevel::Safe, Reversibility::Permanent, false,
            "Standard temp location; files here are transient by definition.",
            "Applications rebuild temp files as needed. Open documents in editors keep their own backups.",
        ),
        templates: vec!["%LOCALAPPDATA%\\Temp".into()],
        clear_contents: true,
        max_age_days: None,
        pattern: None,
    }));

    out.push(Arc::new(PathCleanupRule {
        rule: rule(
            "win.system-temp",
            "Windows temporary files",
            "Temporary files in the Windows folder used by services and installers.",
            "temp", RiskLevel::Low, Reversibility::Permanent, true,
            "System temp location; files here are transient by definition.",
            "Some installers running right now may need these files — close installers first.",
        ),
        templates: vec!["%SystemRoot%\\Temp".into()],
        clear_contents: true,
        max_age_days: None,
        pattern: None,
    }));

    // --- Shader caches ---------------------------------------------------------
    out.push(Arc::new(PathCleanupRule {
        rule: rule(
            "win.dx-shader-cache",
            "DirectX shader cache",
            "Compiled shader cache used by DirectX games and apps.",
            "cache", RiskLevel::Safe, Reversibility::Permanent, false,
            "Cache data — rebuilt automatically from game/app shaders.",
            "First game launches may stutter slightly while shaders recompile.",
        ),
        templates: vec!["%LOCALAPPDATA%\\D3DSCache".into()],
        clear_contents: true,
        max_age_days: None,
        pattern: None,
    }));

    out.push(Arc::new(PathCleanupRule {
        rule: rule(
            "win.nvidia-shader-cache",
            "NVIDIA shader caches",
            "NVIDIA GL/DX/Optix compiled shader caches.",
            "cache", RiskLevel::Low, Reversibility::Permanent, false,
            "Cache data — rebuilt automatically by the GPU driver.",
            "Games recompile shaders on next launch; first launches take longer.",
        ),
        templates: vec![
            "%LOCALAPPDATA%\\NVIDIA\\GLCache".into(),
            "%LOCALAPPDATA%\\NVIDIA\\DXCache".into(),
            "%LOCALAPPDATA%\\NVIDIA\\OptixCache".into(),
        ],
        clear_contents: true,
        max_age_days: None,
        pattern: None,
    }));

    out.push(Arc::new(PathCleanupRule {
        rule: rule(
            "win.amd-shader-cache",
            "AMD shader caches",
            "AMD DirectX/Vulkan shader caches.",
            "cache", RiskLevel::Low, Reversibility::Permanent, false,
            "Cache data — rebuilt automatically by the GPU driver.",
            "Games recompile shaders on next launch.",
        ),
        templates: vec![
            "%LOCALAPPDATA%\\AMD\\DxCache".into(),
            "%LOCALAPPDATA%\\AMD\\DxcCache".into(),
            "%LOCALAPPDATA%\\AMD\\GLCache".into(),
        ],
        clear_contents: true,
        max_age_days: None,
        pattern: None,
    }));

    // --- Error reports & dumps --------------------------------------------------
    out.push(Arc::new(PathCleanupRule {
        rule: rule(
            "win.wer-user",
            "Windows error reports (user)",
            "Queued error reports and crash diagnostics for your user account.",
            "diagnostics", RiskLevel::Safe, Reversibility::Permanent, false,
            "Diagnostic data already collected locally; not needed for Windows to run.",
            "Older crash reports are removed; new reports are created as needed.",
        ),
        templates: vec!["%LOCALAPPDATA%\\Microsoft\\Windows\\WER".into()],
        clear_contents: true,
        max_age_days: Some(30),
        pattern: None,
    }));

    out.push(Arc::new(PathCleanupRule {
        rule: rule(
            "win.wer-system",
            "Windows error reports (system)",
            "System-wide error report queue (ProgramData).",
            "diagnostics", RiskLevel::Low, Reversibility::Permanent, true,
            "Diagnostic data already collected locally.",
            "Older crash reports are removed.",
        ),
        templates: vec!["%PROGRAMDATA%\\Microsoft\\Windows\\WER".into()],
        clear_contents: true,
        max_age_days: Some(30),
        pattern: None,
    }));

    out.push(Arc::new(PathCleanupRule {
        rule: rule(
            "win.crash-dumps",
            "Application crash dumps",
            "Local crash dump files written when applications crash.",
            "diagnostics", RiskLevel::Safe, Reversibility::Permanent, false,
            "Dumps are point-in-time debugging artifacts.",
            "Support engineers lose access to old crash dumps for diagnostics.",
        ),
        templates: vec!["%LOCALAPPDATA%\\CrashDumps".into()],
        clear_contents: true,
        max_age_days: None,
        pattern: None,
    }));

    out.push(Arc::new(PathCleanupRule {
        rule: rule(
            "win.memory-dump",
            "Kernel memory dumps",
            "System memory dumps (MEMORY.DMP, Minidump) from blue screens.",
            "diagnostics", RiskLevel::Low, Reversibility::Permanent, true,
            "Point-in-time debugging artifacts from BSODs.",
            "You lose the ability to analyze past blue screens. Windows recreates dumps on future crashes.",
        ),
        templates: vec!["%SystemRoot%\\MEMORY.DMP".into(), "%SystemRoot%\\Minidump".into()],
        clear_contents: true,
        max_age_days: None,
        pattern: None,
    }));

    // --- Shell caches -------------------------------------------------------------
    out.push(Arc::new(PathCleanupRule {
        rule: rule(
            "win.thumbnail-cache",
            "Thumbnail cache",
            "Explorer thumbnail and icon cache databases.",
            "cache", RiskLevel::Safe, Reversibility::Permanent, false,
            "Windows rebuilds thumbnails on demand.",
            "Folders show generic icons briefly while thumbnails regenerate.",
        ),
        templates: vec!["%LOCALAPPDATA%\\Microsoft\\Windows\\Explorer".into()],
        clear_contents: false,
        max_age_days: None,
        pattern: Some("thumbcache_*.db".into()),
    }));

    // --- Recycle Bin ------------------------------------------------------------
    out.push(Arc::new(super::recycle_bin::RecycleBinRule::default()));

    // --- Delivery Optimization (supported mechanism) -----------------------------
    out.push(Arc::new(CommandCleanupRule {
        rule: CleanerRule {
            id: "win.delivery-optimization".into(),
            name: "Delivery Optimization cache".into(),
            description: "Windows Update peer-to-peer download cache, cleared via the supported PowerShell cmdlet.".into(),
            family: CleanupFamily::Windows,
            group: "update".into(),
            risk: RiskLevel::Low,
            reversibility: Reversibility::WindowsManaged,
            requires_admin: true,
            conflicts_with_processes: vec![],
            reason: "Cached update chunks delivered between PCs; no longer needed after updates install.".into(),
            consequence: "Windows re-downloads update content when needed.".into(),
            opt_in: false,
            windows_managed: true,
            min_os: "Windows 10 1803".into(),
        },
        program: "powershell".into(),
        args: vec![
            "-NoProfile".into(),
            "-Command".into(),
            "Delete-DeliveryOptimizationCache -Force".into(),
        ],
        discovery_paths: vec![
            "%SystemRoot%\\ServiceProfiles\\NetworkService\\AppData\\Local\\Microsoft\\Windows\\DeliveryOptimization\\Cache".into(),
        ],
        verify_note: "Verified via the Delivery Optimization cache location.".into(),
    }));

    // --- Component store (DISM only — never WinSxS files) ------------------------
    out.push(Arc::new(CommandCleanupRule {
        rule: CleanerRule {
            id: "win.component-store".into(),
            name: "Component Store cleanup (DISM)".into(),
            description: "Servicing-safe cleanup of the Windows component store using the supported DISM operation.".into(),
            family: CleanupFamily::Windows,
            group: "servicing".into(),
            risk: RiskLevel::Advanced,
            reversibility: Reversibility::WindowsManaged,
            requires_admin: true,
            conflicts_with_processes: vec![],
            reason: "DISM StartComponentCleanup removes superseded components the supported way.".into(),
            consequence: "This can take several minutes. Windows Update must not be interrupted while running. Update rollback for recently installed updates becomes unavailable.".into(),
            opt_in: false,
            windows_managed: true,
            min_os: "Windows 8".into(),
        },
        program: "dism".into(),
        args: vec![
            "/Online".into(),
            "/Cleanup-Image".into(),
            "/StartComponentCleanup".into(),
        ],
        discovery_paths: vec![],
        verify_note: "Run Analyze again in Advanced Tools to see the new size.".into(),
    }));

    // --- Windows.old --------------------------------------------------------------
    out.push(Arc::new(PathCleanupRule {
        rule: CleanerRule {
            id: "win.previous-installation".into(),
            name: "Previous Windows installation (Windows.old)".into(),
            description: "Data from a previous Windows version kept for rollback after an upgrade.".into(),
            family: CleanupFamily::Windows,
            group: "servicing".into(),
            risk: RiskLevel::Advanced,
            reversibility: Reversibility::Permanent,
            requires_admin: true,
            conflicts_with_processes: vec![],
            reason: "Old installation data is only needed to roll back to the previous Windows version.".into(),
            consequence: "WARNING: removing Windows.old makes it impossible to roll back to the previous Windows version or recover its files. This cannot be undone.".into(),
            opt_in: false,
            windows_managed: false,
            min_os: "Windows 10".into(),
        },
        templates: vec!["%SystemDrive%\\Windows.old".into()],
        clear_contents: false,
        max_age_days: None,
        pattern: None,
    }));

    // --- Hibernation file (supported powercfg mechanism) ---------------------------
    out.push(Arc::new(CommandCleanupRule {
        rule: CleanerRule {
            id: "win.hibernation".into(),
            name: "Hibernation file (disable hibernation)".into(),
            description: "Reclaims hiberfil.sys by disabling hibernation through powercfg — the file is never deleted directly.".into(),
            family: CleanupFamily::Windows,
            group: "power".into(),
            risk: RiskLevel::Advanced,
            reversibility: Reversibility::WindowsManaged,
            requires_admin: true,
            conflicts_with_processes: vec![],
            reason: "hiberfil.sys stores the full memory image for hibernation and Fast Startup.".into(),
            consequence: "Hibernation AND Fast Startup are disabled. Re-enable anytime with 'powercfg /h on'. The file is recreated then.".into(),
            opt_in: false,
            windows_managed: true,
            min_os: "Windows 10".into(),
        },
        program: "powercfg".into(),
        args: vec!["/h".into(), "off".into()],
        discovery_paths: vec![],
        verify_note: "hiberfil.sys is removed by Windows when hibernation is disabled.".into(),
    }));

    // --- Restore points (explicit opt-in, big warning) ------------------------------
    out.push(Arc::new(CommandCleanupRule {
        rule: CleanerRule {
            id: "win.restore-points".into(),
            name: "System Restore shadow copies".into(),
            description: "Deletes all restore points via the supported vssadmin mechanism.".into(),
            family: CleanupFamily::Windows,
            group: "recovery".into(),
            risk: RiskLevel::Dangerous,
            reversibility: Reversibility::WindowsManaged,
            requires_admin: true,
            conflicts_with_processes: vec![],
            reason: "Shadow copies consume disk proportional to restore-point history.".into(),
            consequence: "WARNING: you will lose ALL system restore points. You will not be able to roll the system back. Consider reducing the reserved space instead.".into(),
            opt_in: true,
            windows_managed: true,
            min_os: "Windows 10".into(),
        },
        program: "vssadmin".into(),
        args: vec!["Delete".into(), "Shadows".into(), "/All".into(), "/Quiet".into()],
        discovery_paths: vec![],
        verify_note: "Verify with 'vssadmin list shadowstorage' afterwards.".into(),
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_have_valid_metadata() {
        let mut out: Vec<Arc<dyn CleanerRuleDef>> = Vec::new();
        register(&mut out);
        assert!(out.len() >= 14, "expected full windows rule set, got {}", out.len());
        for r in &out {
            assert!(!r.meta().id.is_empty());
            assert!(!r.meta().name.is_empty());
            assert!(!r.meta().reason.is_empty());
            assert!(!r.meta().consequence.is_empty());
        }
    }
}
