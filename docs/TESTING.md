# Testing Guide

## Rust core (`src-tauri/core`)

```bash
cd src-tauri
cargo test -p spaceforge-core
```

33 tests across the engines:

| Suite | What it proves |
| --- | --- |
| `safety::validator` | Protected targets (WinSxS/System32/Boot on Windows, /etc on Unix) are blocked; unknown targets are blocked (fail closed); declared rule sub-paths pass; roots/drives are refused; Windows-managed mode still blocks absolute zones |
| `safety::protected` | Zone classification (system, guarded, user data, temp buckets) |
| `scanner::aggregate` | Tree sums include small-file aggregates; per-level attachment is correct; hard-linked content counts once; unreadable entries propagate |
| `duplicate` | True duplicates group (after byte verification); same-size-but-different files do **not** group; hard links are one identity and never redundant; keep strategies pick the expected member; cancellation aborts cleanly |
| `db` | Settings/exclusions/cleanup-history round-trips |
| `cleaners` | Rule metadata contract (ids, reasons, consequences present); DISM output parser; env expansion |
| `util` | Size formatting, category classification, platform-independent path logic |
| `volumes` | At least one real volume is enumerated with consistent numbers |

## Safety regression philosophy (spec §72)

The suite is written so the *validator logic itself* runs on every platform —
Windows-zone cases execute on Windows runners, and the same code paths are
exercised through the Unix zone set on every other host. Any change that
weakens protection breaks the build.

## Frontend

```bash
npm run typecheck   # strict TypeScript, zero errors
npm run build       # production bundle (tsc + vite)
```

Manual smoke checklist (run `npm run tauri:dev` on Windows):

- [ ] Dashboard lists real volumes; scanning completes with live progress
- [ ] Treemap renders, zooms, breadcrumbs work on a real drive
- [ ] Cleanup discovery measures; preview shows plan; execution reports actual reclaim
- [ ] Duplicates run on scan index; hard-linked files are not reported as copies
- [ ] Settings persistence survives restart; theme toggle works
- [ ] Keyboard: Ctrl+K opens the command palette; Tab focus visible everywhere
