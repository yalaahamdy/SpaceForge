//! Browser cleanup rules (spec §30, §31).
//!
//! Chromium browsers share a cache layout, so one factory generates rules
//! per browser. Only cache-class data is ever touched by default — saved
//! passwords, cookies, bookmarks and passkeys are NEVER included. Privacy
//! additions (history, cookies) are separate opt-in rules with explicit
//! consequences.

use super::generic::PathCleanupRule;
use super::{CleanContext, CleanerRuleDef};
use crate::models::*;
use std::sync::Arc;

struct BrowserRule(PathCleanupRule);

impl CleanerRuleDef for BrowserRule {
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

struct ChromiumBrowser {
    id: &'static str,
    name: &'static str,
    /// User Data root template.
    root: &'static str,
    process: &'static str,
    /// Alternative "Opera-style" layout where the root IS the profile.
    flat_profile: bool,
}

const CHROMIUM_BROWSERS: &[ChromiumBrowser] = &[
    ChromiumBrowser {
        id: "browser.chrome",
        name: "Google Chrome",
        root: "%LOCALAPPDATA%\\Google\\Chrome\\User Data",
        process: "chrome",
        flat_profile: false,
    },
    ChromiumBrowser {
        id: "browser.edge",
        name: "Microsoft Edge",
        root: "%LOCALAPPDATA%\\Microsoft\\Edge\\User Data",
        process: "msedge",
        flat_profile: false,
    },
    ChromiumBrowser {
        id: "browser.brave",
        name: "Brave",
        root: "%LOCALAPPDATA%\\BraveSoftware\\Brave-Browser\\User Data",
        process: "brave",
        flat_profile: false,
    },
    ChromiumBrowser {
        id: "browser.vivaldi",
        name: "Vivaldi",
        root: "%LOCALAPPDATA%\\Vivaldi\\User Data",
        process: "vivaldi",
        flat_profile: false,
    },
    ChromiumBrowser {
        id: "browser.chromium",
        name: "Chromium",
        root: "%LOCALAPPDATA%\\Chromium\\User Data",
        process: "chromium",
        flat_profile: false,
    },
    ChromiumBrowser {
        id: "browser.opera",
        name: "Opera",
        root: "%APPDATA%\\Opera Software",
        process: "opera",
        flat_profile: true,
    },
];

/// Chromium cache subdirectories (relative to each profile folder).
const CHROMIUM_CACHE_SUBDIRS: &[&str] = &[
    "Cache",
    "Code Cache",
    "GPUCache",
    "Media Cache",
    "Service Worker\\CacheStorage",
    "Service Worker\\ScriptCache",
    "ShaderCache",
    "GrShaderCache",
];

impl ChromiumBrowser {
    fn cache_dirs(&self) -> Vec<String> {
        let root = super::expand::expand(self.root);
        let mut out = Vec::new();
        if self.flat_profile {
            for sub in CHROMIUM_CACHE_SUBDIRS {
                out.push(format!("{root}\\{}\\{sub}", "Opera Stable"));
            }
            // Multiple Opera profiles.
            out.push(format!("{root}\\Opera Stable\\Cache"));
            return out;
        }
        // Enumerate profile dirs (Default, Profile 1, Profile 2, Guest…).
        let Ok(rd) = std::fs::read_dir(&root) else {
            return out;
        };
        for entry in rd.flatten() {
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if !is_dir {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_profile = name == "Default"
                || name.starts_with("Profile ")
                || name == "Guest Profile"
                || name == "System Profile";
            if !is_profile {
                continue;
            }
            for sub in CHROMIUM_CACHE_SUBDIRS {
                out.push(format!("{}\\{}\\{sub}", entry.path().to_string_lossy(), name));
            }
        }
        out
    }
}

#[allow(clippy::too_many_arguments)]
fn cache_rule(id: &str, name: &str, templates: Vec<String>, process: &str) -> BrowserRule {
    BrowserRule(PathCleanupRule {
        rule: CleanerRule {
            id: id.into(),
            name: format!("{name} cache"),
            description: format!(
                "{name}'s browser cache: images, scripts and service-worker data. Never includes passwords, cookies, bookmarks or passkeys."
            ),
            family: CleanupFamily::Browser,
            group: "browser-cache".into(),
            risk: RiskLevel::Low,
            reversibility: Reversibility::Permanent,
            requires_admin: false,
            conflicts_with_processes: vec![process.into()],
            reason: "Cache data is rebuilt automatically; personal data is not touched."
                .into(),
            consequence: "Web sites may load slightly slower the first time while caches rebuild. Logins are unaffected.".into(),
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

/// Privacy rules (opt-in, spec §31) — every one states its consequence.
#[allow(clippy::too_many_arguments)]
fn privacy_rule(
    id: &str,
    name: &str,
    templates: Vec<String>,
    process: &str,
    consequence: &str,
    clear_contents: bool,
) -> BrowserRule {
    BrowserRule(PathCleanupRule {
        rule: CleanerRule {
            id: id.into(),
            name: name.into(),
            description: format!("Privacy-oriented cleanup: {name}. Opt-in only."),
            family: CleanupFamily::Privacy,
            group: "privacy".into(),
            risk: RiskLevel::Review,
            reversibility: Reversibility::Permanent,
            requires_admin: false,
            conflicts_with_processes: vec![process.into()],
            reason: "You asked to remove browsing traces from this machine.".into(),
            consequence: consequence.into(),
            opt_in: true,
            windows_managed: false,
            min_os: "Windows 10".into(),
        },
        templates,
        clear_contents,
        max_age_days: None,
        pattern: None,
    })
}

pub fn register(out: &mut Vec<Arc<dyn CleanerRuleDef>>) {
    for b in CHROMIUM_BROWSERS {
        out.push(Arc::new(cache_rule(b.id, b.name, b.cache_dirs(), b.process)));
    }

    // Firefox (different layout; cache lives under LocalAppData).
    out.push(Arc::new(BrowserRule(PathCleanupRule {
        rule: CleanerRule {
            id: "browser.firefox".into(),
            name: "Mozilla Firefox cache".into(),
            description: "Firefox startup/cache2 storage. Never includes passwords, cookies or bookmarks.".into(),
            family: CleanupFamily::Browser,
            group: "browser-cache".into(),
            risk: RiskLevel::Low,
            reversibility: Reversibility::Permanent,
            requires_admin: false,
            conflicts_with_processes: vec!["firefox".into()],
            reason: "Cache data is rebuilt automatically; personal data is not touched.".into(),
            consequence: "Web sites may load slightly slower the first time.".into(),
            opt_in: false,
            windows_managed: false,
            min_os: "Windows 10".into(),
        },
        templates: vec![
            "%LOCALAPPDATA%\\Mozilla\\Firefox\\Profiles\\*.default*\\cache2".into(),
            "%LOCALAPPDATA%\\Mozilla\\Firefox\\Profiles\\*\\cache2".into(),
        ],
        clear_contents: true,
        max_age_days: None,
        pattern: None,
    })));

    // NOTE: Firefox profile globbing is handled by direct expansion of the
    // wildcard in cache_dirs-like logic below at discover time.
    out.push(Arc::new(privacy_rule(
        "privacy.chrome-history",
        "Browsing history (Chromium browsers)",
        CHROMIUM_BROWSERS
            .iter()
            .flat_map(|b| {
                let root = super::expand::expand(b.root);
                if b.flat_profile {
                    vec![format!("{root}\\Opera Stable\\History")]
                } else {
                    vec![format!("{root}\\Default\\History")]
                }
            })
            .collect(),
        "chrome",
        "Your entire browsing history for the default profiles is deleted permanently.",
        true, // delete the History file (browser closed required)
    )));

    out.push(Arc::new(privacy_rule(
        "privacy.chrome-cookies",
        "Cookies (Chromium browsers)",
        CHROMIUM_BROWSERS
            .iter()
            .flat_map(|b| {
                let root = super::expand::expand(b.root);
                if b.flat_profile {
                    vec![format!("{root}\\Opera Stable\\Cookies")]
                } else {
                    vec![format!("{root}\\Default\\Cookies")]
                }
            })
            .collect(),
        "chrome",
        "You will be signed out of most web sites. This can also invalidate some app sign-ins tied to browser cookies.",
        true,
    )));
}
