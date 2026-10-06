<p align="center">
  <img src="public/icon.svg" width="128" height="128" alt="SpaceForge Logo" />
</p>

<h1 align="center">SpaceForge</h1>

<p align="center">
  <strong>Windows Disk Space Intelligence, Analysis, Cleanup, Duplicate Detection & Storage Optimization</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-blue?logo=windows" alt="Platform" />
  <img src="https://img.shields.io/badge/Rust-1.77%2B-orange?logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/Tauri-v2-informational?logo=tauri" alt="Tauri" />
  <img src="https://img.shields.io/badge/React-18-61dafb?logo=react" alt="React" />
  <img src="https://img.shields.io/badge/License-MIT-green" alt="License" />
</p>

SpaceForge answers one question better than any ordinary storage utility:

> *What is consuming my disk space, why is it there, what can safely be reclaimed, what should I keep — and what is the safest, most effective way to reclaim it?*

It combines a multithreaded Rust scanning engine with a Windows 11-class React UI, an explainable cleanup rule engine, byte-verified duplicate detection, snapshot-based growth analysis and a layered safety engine that makes destructive actions predictable and auditable.

SpaceForge is local-first: **no telemetry, no accounts, no cloud, no ads** — every byte of analysis stays on your PC.

---

## Highlights

| Area | What it does |
| --- | --- |
| **Dashboard** | Real volume detection, usage bars, pressure rating, one-click scan, measured reclaimable-space summary |
| **Disk Analyzer** | Sortable/drillable folder tree with sizes, allocated sizes, item counts, unreadable-entry reporting |
| **Storage Map** | Canvas squarified treemap with zoom, breadcrumbs, category colors, filtering, context menus |
| **Large Files / Old Files** | Threshold + age + category filters; "potentially stale" language instead of scare tactics |
| **Duplicates** | Same-size → partial hash → full hash → **byte-by-byte verification**; hard links never count as copies; keep-newest/oldest/shortest-path strategies |
| **Cleanup Center** | Rule-driven engine with risk levels (Safe → Dangerous), profiles (Smart/Standard/Deep/Custom), full preview, transactional execution, actual-vs-estimated reporting |
| **Windows Cleanup** | Temps, shader caches, error reports, crash dumps, thumbnail cache, Recycle Bin (shell API), Delivery Optimization (PowerShell cmdlet), component store (DISM only), hibernation (powercfg only), Windows.old with explicit warnings |
| **App / Browser / Developer cleanup** | Precise per-app cache paths, process-running detection, Chromium/Firefox cache layouts, npm/pnpm/pip/cargo/gradle caches — never whole AppData folders |
| **Storage History & Snapshots** | Timeline built only from real scans; named snapshots with factual before/after diffs |
| **Why Is My Disk Full?** | Factual diagnosis: biggest contributors + measured reclaimable space — zero speculation |
| **Advanced Tools** | DISM component-store analysis, hibernation status, shadow-copy visibility, report export (HTML/CSV/JSON) |
| **Safety Engine** | 7-layer validation, protected-path zones (WinSxS, System32, boot…), fail-closed rule whitelists, audit history |

## Technology

- **Rust** (`spaceforge-core` crate): scanner, aggregation, safety, cleaners, duplicates, snapshots, SQLite index
- **Tauri 2** thin IPC layer (`src-tauri`): typed commands, event streaming, Windows bundling
- **React 18 + TypeScript + Vite**: 17 views, virtualized tables, canvas visualizations, zero heavy UI dependencies
- **SQLite** (bundled): scan sessions, index, snapshots, cleanup history, exclusions, settings

## Requirements

- Windows 10 (1903+) or Windows 11 for the shipped application
- To build: Node.js 18+, Rust 1.77+ (MSVC toolchain), WebView2 (preinstalled on Win 11)

## Build

```powershell
npm install
npm run tauri:dev          # development
npm run tauri:build        # release: installer + executable
```

Release artifacts land in `src-tauri/target/release/bundle/nsis/` (installer) and
`src-tauri/target/release/` (`spaceforge.exe` — the portable executable).

A GitHub Actions workflow (`.github/workflows/release.yml`) builds signed-free
installer + portable binaries on every tag push — no Windows machine needed.
`scripts/build.ps1` mirrors the same steps locally.

## Testing

```bash
cd src-tauri
cargo test -p spaceforge-core   # 33 unit + safety regression tests
```

Frontend: `npm run typecheck` (strict TS) and `npm run build`.

The safety regression suite proves the cleaner engine refuses protected targets
(WinSxS, System32, boot, undeclared zones), blocks unknown targets, and keeps
hard links from being double-counted — on every commit.

## Security model & cleanup philosophy

See [SECURITY.md](SECURITY.md) for the protected-path model and elevation rules,
[CLEANER_RULES.md](CLEANER_RULES.md) for how rules are declared, whitelisted and
validated, and [ARCHITECTURE.md](ARCHITECTURE.md) for engine-level design.

The short version:

1. **Analyze first, delete second** — SCAN → UNDERSTAND → EXPLAIN → PREVIEW → APPROVE → CLEAN → VERIFY.
2. **Never pretend large/old/unknown means junk.** Risk levels are honest; recommendations are explainable rules, not scores for scareware.
3. **Windows-managed data is cleaned only through supported mechanisms** (DISM, powercfg, shell APIs, PowerShell cmdlets). SpaceForge never deletes WinSxS/System32 contents to win a few gigabytes.
4. **Fail closed** — an unknown target is a blocked target.

## Known limitations

- The experimental NTFS MFT fast path requires elevation and is off by default; it falls back to the parallel walker on any inconsistency.
- Incremental rescanning uses full re-walks (USN-journal-based deltas are deliberately not implemented in v1.0 — correctness first; the design leaves room in the index layer).
- Allocated sizes are cluster-rounded estimates for sparse/compressed files; the MFT path reports true allocated sizes.
- Recycle Bin item-level restore is delegated to the Windows shell (SpaceForge empties via the supported shell API but does not reimplement restore).
- Secure delete is best-effort by physics: SSD wear-leveling, snapshots and compression make media-level guarantees impossible, and the UI says so.

## License

MIT.
