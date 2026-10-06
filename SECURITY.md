# SpaceForge Security Model

## Local-first, private by architecture

- No telemetry, no analytics, no advertising, no accounts, no cloud endpoints.
- The application makes **no network requests** — the CSP allows only local
  resources (`default-src 'self'`), and no HTTP client exists in either the
  frontend or the backend.
- All analysis data (scan index, snapshots, cleanup history) stays in a local
  SQLite database under the user's profile (`%LOCALAPPDATA%\SpaceForge`).
- Report export (HTML/CSV/JSON) is an explicit user action; by default the
  export **withholds individual file paths**.

## Permissions & elevation

- SpaceForge is fully useful **without administrator rights**: volume listing,
  scanning of user-readable data, duplicates, user-file cleanup and exports
  all work from a standard token.
- Elevation is never silent. When a rule requires admin (system temp, WER
  system queue, Delivery Optimization, DISM, powercfg, Windows.old), the UI
  states exactly what will run and why before any UAC prompt is raised.
- The current elevation state is honestly displayed in the status bar
  (`standard user` / `administrator`), detected via `TokenElevation`.

## Protected path model

The safety engine classifies every target into protection categories:

| Category | Meaning |
| --- | --- |
| `SYSTEM_PROTECTED` (absolute) | WinSxS, System32, SysWOW64, Boot, Fonts, Servicing, registry hives, drive roots — **never touched, by rules or by explicit user request** |
| `WINDOWS_MANAGED` (guarded) | `C:\Windows` internals — only rule-declared exact sub-paths, and preferably only via supported Windows mechanisms |
| `APPLICATION_MANAGED` (guarded) | Program Files / ProgramData — only rule-declared sub-paths (e.g. the WER queue) |
| `USER_DATA` | The user's own files — deletions require explicit confirmation, prefer Recycle Bin semantics |
| `SAFE_CACHE` / `TEMPORARY` | Rule-declared cache/temp locations — the only things permanent cleanup touches |
| `UNKNOWN` | **Always blocked** — fail closed |

### Enforcement points

1. Rule whitelists (`ValidationContext.allowed_prefixes`) — checked at plan
   build *and* immediately before execution.
2. `validate_user_targets` for Explorer-style operations (recycle/delete/
   secure-delete of arbitrary user selections) — absolute zones and roots
   remain untouchable even then.
3. The regression test suite asserts WinSxS / System32 / Boot / undeclared
   targets are refused, on every CI run.

## Windows-managed mechanisms

Where Windows exposes a supported operation, SpaceForge uses it instead of
raw deletion: DISM (`StartComponentCleanup`, `AnalyzeComponentStore`),
`powercfg /h` (hibernation — the file is never deleted manually),
`Delete-DeliveryOptimizationCache`, `vssadmin` (shadow storage — opt-in,
explicit warning), `SHQueryRecycleBinW`/`SHEmptyRecycleBinW` (Recycle Bin).

## Deletion semantics

- **Recyclable** — user file operations use the shell mechanism
  (IFileOperation via the `trash` crate); items remain restorable.
- **Permanent** — cache/temp cleanup deletes rule-whitelisted contents only;
  the preview states this before execution.
- **Windows-managed** — performed by Windows itself; SpaceForge reports the
  exit state honestly and never fabricates reclaimed bytes.
- **Secure delete** — best-effort overwrite (zero/FF/random passes) with an
  explicit disclaimer: SSD wear-leveling, snapshots, compression and sparse
  storage make forensic guarantees impossible, and the UI says so.

## Auditability

Every cleanup is persisted locally: profile, categories, item counts,
estimated vs actual reclaim, failures, skips and duration (`Cleanup History`
view). Failed deletions are reported as failures — never as success.

## Process hygiene

Rules declare conflicting processes (browsers, Discord, Steam…). Discovery
detects running processes and blocks the rule with a human explanation
("close it before cleaning") instead of terminating anything silently.
