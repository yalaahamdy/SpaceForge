# Cleaner Rules

SpaceForge's cleanup behavior is driven by a modular rule engine — no cleanup
logic is hardcoded into the UI. This document explains how rules are
structured, how they are validated, and how to add new ones.

## Rule anatomy

Every rule implements the `CleanerRuleDef` trait (`src-tauri/core/src/cleaners/mod.rs`):

```rust
pub trait CleanerRuleDef: Send + Sync {
    fn meta(&self) -> &CleanerRule;            // static metadata (id, risk, …)
    fn allowed_prefixes(&self) -> Vec<String>; // the ONLY paths it may touch
    fn discover(&self, ctx: &CleanContext)     // measure current size/items
        -> Option<CleanupOpportunity>;
    fn targets(&self, ctx: &CleanContext) -> Vec<String>; // re-resolved at execution
    fn execute(&self, targets: &[String], ctx: &CleanContext) -> CleanupEntryResult;
    fn mechanism(&self) -> CleanupMechanism;   // Direct | Recycle | WindowsManaged
}
```

Metadata (`CleanerRule`) includes the human contract shown in the UI:

- `name`, `description`, `family` (windows / application / browser / developer / privacy)
- `risk`: Safe → Low → Review → Advanced → Dangerous
- `reversibility`: Recyclable / Permanent / Windows-managed
- `reason` — why the data is considered removable
- `consequence` — what happens after cleaning (rebuild, re-download, sign-out…)
- `requires_admin`, `conflicts_with_processes`, `opt_in`, `windows_managed`

A rule without `reason` and `consequence` text fails the metadata test.

## Generic implementations

Most rules are declarations, not code:

### PathCleanupRule
Cleans declared directories (or files) with:
- `%VAR%` environment expansion (explicit, testable — never shell-interpolated)
- glob patterns (Firefox profile dirs, thumbcache files)
- optional age filter (only entries older than N days)
- clear-contents semantics (delete entries, keep the directory — the safe
  pattern for cache roots owned by running applications)

### CommandCleanupRule
Windows-managed operations only: DISM, powercfg, vssadmin, PowerShell cmdlets.
It never deletes files directly; the validator runs in `windows_managed` mode
where targets are informational but absolute zones are still blocked.

## Validation layers (fail closed)

Before any execution the engine:

1. Normalizes every target path.
2. Blocks **absolute zones**: WinSxS, System32, SysWOW64, Boot, Fonts,
   Servicing, registry hive files, filesystem/drive roots.
3. Blocks **unknown targets**: anything not under a prefix the rule declared
   in `allowed_prefixes()`.
4. For **guarded zones** (Windows, Program Files, ProgramData): requires that
   the rule's whitelist prefix lives *inside* the zone and is not the zone
   root itself — blanket passes are impossible.
5. Re-verifies targets exist immediately before execution (targets that
   vanished are reported as skipped, not failed).

The same validation runs twice: when the plan is built and again per entry
during execution. A regression test suite
(`core/src/safety/validator.rs#tests`) proves:

> Protected target → blocked. Unknown target → blocked. Declared sub-path → allowed.

## Discovery & confidence

Discovery performs real measurement (parallel directory walks) and produces a
`CleanupOpportunity` with current size, item count and a confidence value.
Rules are blocked (never silently skipped) when:
- a conflicting process is running (detected via the process table),
- elevation is required but absent,
- the user has not enabled `opt_in` rules in Settings.

## Profiles

| Profile | Risk ceiling | Notes |
| --- | --- | --- |
| Smart Clean | Safe | The beginner one-click experience |
| Standard Clean | Low | + selected low-risk categories |
| Deep Clean | Review | Everything still previewed; Advanced/Dangerous excluded |
| Custom | explicit | User selects each category; Dangerous requires opt-in |

## Execution contract (spec §36)

Execution is a transaction, not a stream of deletions:

- **Before**: re-validate, re-resolve, verify existence.
- **During**: sequential per-rule processing with progress events; per-item
  failures are recorded (locked files are expected, not exceptional).
- **After**: remaining-target check downgrades to `partial`; free space is
  measured before/after; the report (estimated vs actual, failures, skips)
  is persisted and shown.

## Adding a rule

1. Declare it in the appropriate `rules_*.rs` (or a new file + `register`).
2. List precise `allowed_prefixes` — the whitelist *is* the security boundary.
3. Write `reason` and `consequence` as if explaining to a cautious friend.
4. `cargo test -p spaceforge-core` — the metadata test enforces the contract.

Rules for new applications should follow the cache-precision principle:
clean named cache directories, never whole AppData trees; declare the owning
process so the engine can warn; choose the lowest honest risk level.
