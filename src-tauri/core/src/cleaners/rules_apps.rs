//! Application cache cleanup rules (spec §29).
//!
//! Rule-driven: each app has precise, justified cleanup paths — never whole
//! AppData folders. Running applications are detected and block the rule.

use super::generic::PathCleanupRule;
use super::{expand, CleanContext, CleanerRuleDef};
use crate::models::*;
use std::sync::Arc;

struct AppRule(PathCleanupRule);

impl CleanerRuleDef for AppRule {
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

#[allow(clippy::too_many_arguments)]
fn app_rule(
    id: &str,
    name: &str,
    desc: &str,
    group: &str,
    risk: RiskLevel,
    process: &str,
    reason: &str,
    consequence: &str,
    templates: Vec<String>,
) -> AppRule {
    AppRule(PathCleanupRule {
        rule: CleanerRule {
            id: id.into(),
            name: name.into(),
            description: desc.into(),
            family: CleanupFamily::Application,
            group: group.into(),
            risk,
            reversibility: Reversibility::Permanent,
            requires_admin: false,
            conflicts_with_processes: if process.is_empty() {
                vec![]
            } else {
                vec![process.into()]
            },
            reason: reason.into(),
            consequence: consequence.into(),
            opt_in: false,
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
    out.push(Arc::new(app_rule(
        "app.discord-cache",
        "Discord cache",
        "Discord's image, video and code cache.",
        "discord", RiskLevel::Low, "Discord",
        "Cache data; rebuilt automatically while you use Discord.",
        "Recent media thumbnails reload on demand. Login is unaffected.",
        vec![
            "%APPDATA%\\discord\\Cache".into(),
            "%APPDATA%\\discord\\Code Cache".into(),
            "%APPDATA%\\discord\\GPUCache".into(),
        ],
    )));

    out.push(Arc::new(app_rule(
        "app.slack-cache",
        "Slack cache",
        "Slack's web cache and service worker storage.",
        "slack", RiskLevel::Low, "slack",
        "Cache data; rebuilt automatically while you use Slack.",
        "Workspace media reloads on demand. Login is unaffected.",
        vec![
            "%APPDATA%\\Slack\\Cache".into(),
            "%APPDATA%\\Slack\\Code Cache".into(),
            "%APPDATA%\\Slack\\GPUCache".into(),
            "%APPDATA%\\Slack\\Service Worker".into(),
        ],
    )));

    out.push(Arc::new(app_rule(
        "app.teams-cache",
        "Microsoft Teams cache",
        "Classic Teams client cache (not chat history).",
        "teams", RiskLevel::Low, "Teams",
        "Web caches; chat history lives in the cloud.",
        "Teams may take slightly longer to start next time. Chat history is unaffected.",
        vec![
            "%APPDATA%\\Microsoft\\Teams\\Cache".into(),
            "%APPDATA%\\Microsoft\\Teams\\Code Cache".into(),
            "%APPDATA%\\Microsoft\\Teams\\GPUCache".into(),
            "%APPDATA%\\Microsoft\\Teams\\tmp".into(),
        ],
    )));

    out.push(Arc::new(app_rule(
        "app.steam-htmlcache",
        "Steam HTML cache",
        "Steam client's embedded browser cache.",
        "games", RiskLevel::Safe, "steam",
        "Cache data for the Steam store window; rebuilt automatically.",
        "Store pages reload fully on next open.",
        vec!["%LOCALAPPDATA%\\Steam\\htmlcache".into()],
    )));

    out.push(Arc::new(app_rule(
        "app.epic-webcache",
        "Epic Games Launcher web cache",
        "Epic Games Launcher's embedded browser cache.",
        "games", RiskLevel::Low, "EpicGamesLauncher",
        "Cache data for the launcher UI; rebuilt automatically.",
        "The launcher rebuilds its cache on next start.",
        vec!["%LOCALAPPDATA%\\EpicGamesLauncher\\Saved\\webcache".into()],
    )));

    out.push(Arc::new(app_rule(
        "app.spotify-data",
        "Spotify offline cache",
        "Spotify's downloaded song cache used for offline playback.",
        "media", RiskLevel::Review, "Spotify",
        "Cached audio allows offline playback and reduces streaming.",
        "Offline downloads are removed — they re-download when you play them online again.",
        vec!["%LOCALAPPDATA%\\Spotify\\Data".into()],
    )));

    out.push(Arc::new(app_rule(
        "app.vscode-cache",
        "VS Code caches",
        "Visual Studio Code UI caches (not extensions or settings).",
        "devtools", RiskLevel::Low, "Code",
        "Cache data; extensions, settings and workspaces are untouched.",
        "VS Code rebuilds caches on next launch.",
        vec![
            "%APPDATA%\\Code\\Cache".into(),
            "%APPDATA%\\Code\\CachedData".into(),
            "%APPDATA%\\Code\\Code Cache".into(),
            "%APPDATA%\\Code\\GPUCache".into(),
        ],
    )));

    // Multi-location variants (all locations share one rule).
    out.push(Arc::new(app_rule(
        "app.zoom-logs",
        "Zoom logs & temp data",
        "Zoom diagnostic logs and temporary meeting data.",
        "media", RiskLevel::Low, "Zoom",
        "Diagnostic logs are only useful for troubleshooting past meetings.",
        "Old meeting logs are removed; Zoom recreates them as needed.",
        vec!["%APPDATA%\\Zoom\\logs".into()],
    )));
}

/// Stable exposed helpers for tests.
pub fn templates_for_test() -> Vec<String> {
    vec![expand::expand("%APPDATA%")]
}
