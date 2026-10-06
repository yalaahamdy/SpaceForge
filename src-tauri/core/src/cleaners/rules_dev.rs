//! Developer cache cleanup rules (spec §32).
//!
//! Never assumed safe by default: package caches are Low risk (re-download),
//! global stores/repositories are Review + opt-in because deleting them can
//! break offline builds.

use super::generic::PathCleanupRule;
use super::{CleanContext, CleanerRuleDef};
use crate::models::*;
use std::sync::Arc;

struct DevRule(PathCleanupRule);

impl CleanerRuleDef for DevRule {
    fn meta(&self) -> &CleanerRule {
        &self.0.rule
    }
    fn allowed_prefixes(&self) -> Vec<String> {
        self.0.allowed_prefixes()
    }
    fn discover(&self, ctx: &CleanContext) -> Option<CleanupOpportunity> {
        self.0.discover(ctx)
    }
    fn targets(&self, ctx: &CleanContext) -> Vec<String> {
        self.0.targets(ctx)
    }
    fn execute(&self, targets: &[String], ctx: &CleanContext) -> CleanupEntryResult {
        self.0.execute(targets, ctx)
    }
    fn mechanism(&self) -> CleanupMechanism {
        CleanupMechanism::Direct
    }
}

fn dev_rule(
    id: &str,
    name: &str,
    desc: &str,
    risk: RiskLevel,
    opt_in: bool,
    reason: &str,
    consequence: &str,
    templates: Vec<String>,
) -> DevRule {
    DevRule(PathCleanupRule {
        rule: CleanerRule {
            id: id.into(),
            name: name.into(),
            description: desc.into(),
            family: CleanupFamily::Developer,
            group: "dev".into(),
            risk,
            reversibility: Reversibility::Permanent,
            requires_admin: false,
            conflicts_with_processes: vec![],
            reason: reason.into(),
            consequence: consequence.into(),
            opt_in,
            windows_managed: false,
            min_os: "Windows 10".into(),
        },
        templates,
        clear_contents: true,
        max_age_days: None,
        pattern: None,
    })
}

pub fn register(out: &mut Vec<Arc<dyn CleanerRuleDef>>) {
    // Low risk: content is a pure download cache.
    out.push(Arc::new(dev_rule(
        "dev.npm-cache",
        "npm cache",
        "npm's package download cache (_cacache).",
        RiskLevel::Low, false,
        "Pure download cache; npm verifies integrity and re-downloads on demand.",
        "Next `npm install` re-downloads missing packages. No project files are touched.",
        vec!["%LOCALAPPDATA%\\npm-cache".into(), "%APPDATA%\\npm-cache".into()],
    )));

    out.push(Arc::new(dev_rule(
        "dev.yarn-cache",
        "Yarn cache",
        "Yarn package manager download cache.",
        RiskLevel::Low, false,
        "Pure download cache.",
        "Packages re-download on demand.",
        vec!["%LOCALAPPDATA%\\Yarn\\Cache".into()],
    )));

    out.push(Arc::new(dev_rule(
        "dev.pnpm-store",
        "pnpm store",
        "pnpm's content-addressable package store.",
        RiskLevel::Low, false,
        "Content-addressable store; packages re-download when missing.",
        "Projects using the store re-download packages on next install.",
        vec!["%LOCALAPPDATA%\\pnpm\\store".into()],
    )));

    out.push(Arc::new(dev_rule(
        "dev.pip-cache",
        "pip cache",
        "Python pip wheel/download cache.",
        RiskLevel::Low, false,
        "Pure download cache.",
        "Next `pip install` re-downloads from PyPI.",
        vec!["%LOCALAPPDATA%\\pip\\cache".into()],
    )));

    out.push(Arc::new(dev_rule(
        "dev.go-build-cache",
        "Go build cache",
        "Go compiler incremental build cache.",
        RiskLevel::Low, false,
        "Build cache; fully reproducible from sources.",
        "The next build recompiles from scratch (slower first build).",
        vec!["%LOCALAPPDATA%\\go-build".into()],
    )));

    out.push(Arc::new(dev_rule(
        "dev.nuget-http-cache",
        "NuGet HTTP cache",
        "NuGet's v3 HTTP download cache (not the global packages folder).",
        RiskLevel::Low, false,
        "Pure download cache.",
        "Packages re-download on restore.",
        vec!["%LOCALAPPDATA%\\NuGet\\v3-cache".into(), "%LOCALAPPDATA%\\NuGet\\plugins-cache".into()],
    )));

    // Review + opt-in: deleting can break offline workflows.
    out.push(Arc::new(dev_rule(
        "dev.cargo-registry",
        "Cargo registry cache",
        "Rust cargo registry sources and downloaded crate archives.",
        RiskLevel::Review, true,
        "Downloaded crates can be re-fetched from crates.io.",
        "Offline builds fail until crates are re-downloaded. The registry index is not touched.",
        vec![
            "%USERPROFILE%\\.cargo\\registry\\cache".into(),
            "%USERPROFILE%\\.cargo\\registry\\src".into(),
        ],
    )));

    out.push(Arc::new(dev_rule(
        "dev.nuget-packages",
        "NuGet global packages folder",
        "The ~/.nuget/packages store used by .NET restores.",
        RiskLevel::Review, true,
        "Packages re-restore from configured feeds.",
        "`dotnet restore` re-downloads everything; offline restore breaks.",
        vec!["%USERPROFILE%\\.nuget\\packages".into()],
    )));

    out.push(Arc::new(dev_rule(
        "dev.gradle-caches",
        "Gradle caches",
        "Gradle build and dependency caches.",
        RiskLevel::Review, true,
        "Dependencies re-download from repositories.",
        "The first build after cleaning re-downloads dependencies and rebuilds caches.",
        vec!["%USERPROFILE%\\.gradle\\caches".into()],
    )));

    out.push(Arc::new(dev_rule(
        "dev.maven-repo",
        "Maven local repository",
        "The ~/.m2/repository dependency store.",
        RiskLevel::Review, true,
        "Artifacts re-download from repositories.",
        "The first build after cleaning re-downloads all artifacts.",
        vec!["%USERPROFILE%\\.m2\\repository".into()],
    )));
}
