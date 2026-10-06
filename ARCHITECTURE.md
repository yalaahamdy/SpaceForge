# SpaceForge Architecture

## Crate & process layout

```
SpaceForge/
├── src/                      React frontend (presentation only)
│   ├── api/                  typed IPC contracts (types.ts mirrors Rust models)
│   ├── components/           shell + reusable primitives (VirtualTable, Donut…)
│   ├── views/                the 17 application areas
│   ├── lib/                  formatting, treemap (squarify), category colors
│   └── state/                global app state (scan lifecycle, settings, toasts)
└── src-tauri/
    ├── src/                  Tauri 2 application layer
    │   ├── lib.rs            builder, command registration, window events
    │   ├── state.rs          AppState: index db, settings, active scan handle
    │   ├── events.rs         scan-progress forwarding (batched, throttled)
    │   └── commands/         one module per IPC area (scan, analyze, cleanup…)
    └── core/                 spaceforge-core — ALL engines live here
        └── src/
            ├── models/       canonical serde types (camelCase wire format)
            ├── safety/       protected zones + layered fail-closed validation
            ├── scanner/      jwalk+rayon parallel walker, aggregation, MFT path
            ├── duplicate/    size → partial hash → full hash → byte compare
            ├── cleaners/     rule engine + windows/app/browser/dev rule sets
            ├── db/           SQLite index (rusqlite, WAL)
            ├── snapshot/     factual snapshot comparison
            ├── volumes/      sysinfo + Windows-native volume discovery
            ├── fsops/        deletion semantics, shell integration, native APIs
            └── util/         formatting, classification, path helpers
```

The frontend never performs filesystem recursion or hashing. It owns
presentation and interaction; every expensive operation is a Rust command.

## Scanning engine

`spawn_scan` validates the volume, then runs the walk on a dedicated thread:

- **Parallelism**: jwalk with a dedicated rayon pool sized by scan mode
  (Battery Saver 25 % … Max Performance 100 % of logical cores).
- **Reparse safety**: directories with `FILE_ATTRIBUTE_REPARSE_POINT`
  (junctions, symlinks, mount points) are detected during directory reads and
  excluded from traversal — no loops, no cross-volume surprises (spec §40).
  Traversal can be force-enabled in advanced settings, off by default.
- **Errors**: access-denied/failed reads are counted, kept (path + reason,
  capped at 500 entries) and surfaced in the UI. Nothing is silently skipped.
- **Cancellation/pause**: atomic gates shared with all rayon workers; a
  cancelled walk empties directory batches, so recursion stops everywhere.
- **Hard links**: files reporting link count > 1 get a true file-index lookup
  (BY_HANDLE_FILE_INFORMATION on Windows; inode on Unix). Aggregation counts
  each identity once per scan.
- **Aggregation**: raw rows → `DirNode` tree with bottom-up sums, per-category
  and per-extension rollups, and a bounded top-N file index (default 200 000
  entries ≥ the 10 MB threshold). Small files are aggregated per directory
  (`smallFilesSize`), which keeps IPC payloads and memory bounded on
  million-file volumes.

### Experimental MFT path (opt-in)

`scanner/mft.rs` enumerates the Master File Table via `FSCTL_ENUM_USN_DATA`
and reads per-record sizes via `FSCTL_GET_NTFS_FILE_RECORD`, parsing the
primary `$DATA` attribute (resident and non-resident) for logical + allocated
sizes. It requires elevation, refuses non-NTFS volumes, hard-fails on any
anomaly, and sanity-checks totals against actual volume usage — on any failure
the engine silently falls back to the parallel walker.

## Safety engine (7 layers, fail closed)

1. **Path normalization** — every target is normalized before comparison.
2. **Protected zones** — `WinSxS`, `System32`, `SysWOW64`, `Boot`, `Fonts`,
   `Servicing`, registry hive locations and drive roots are *absolute* zones:
   untouchable even for user-initiated operations. `Windows`, `Program Files`,
   `ProgramData` are *guarded* zones: only exact rule-declared sub-paths pass.
3. **Rule whitelist** — a rule may only touch paths under prefixes it
   declared at registration. An uncovered target is *unknown → blocked*.
4. **Pre-execution verification** — targets are re-resolved and re-validated
   immediately before execution; vanished targets are skipped honestly.
5. **Runtime monitoring** — per-item failures are recorded, never masked.
6. **Post-execution verification** — remaining targets downgrade the entry to
   `partial`; free space is measured before/after.
7. **Audit log** — every cleanup is persisted to the local index with
   estimated vs actual reclaim, failures and skipped items.

Regression tests (`core/src/safety/validator.rs`) prove protected and unknown
targets are blocked on every platform and CI run.

## Cleaner rule engine

Rules implement `CleanerRuleDef`: metadata, whitelist, discovery (real
measurement), target resolution and execution. Two generic implementations
cover most rules — `PathCleanupRule` (dirs/files, glob patterns, age filters,
clear-contents semantics) and `CommandCleanupRule` (Windows-managed
mechanisms: DISM, powercfg, vssadmin, PowerShell cmdlets). The Recycle Bin is
its own rule using `SHQueryRecycleBinW` / `SHEmptyRecycleBinW`.

Profiles map to risk ceilings: Smart Clean → Safe, Standard → Low,
Deep → Review, Custom → explicit selection. Advanced/Dangerous categories are
never enabled silently.

## Index & persistence

SQLite (WAL) stores: scan sessions, the retained file index, three levels of
the directory tree per scan, snapshots (references to scan ids), cleanup
history (with full JSON detail), exclusions and settings. Small files are not
persisted individually — the index threshold bounds database growth on huge
disks.

## IPC design (spec §54, §70)

- Scan progress streams as throttled events (≥150 ms apart) — no per-file
  messages, no million-element arrays.
- The full scan tree stays in Rust memory; the frontend queries slices:
  `drill_down` (one level, IPC-capped children), `large_files` (paged),
  `search_files` (indexed), `top_consumers`, `old_files`.
- Completion announces only the summary; heavy payloads never cross the wire.

## Frontend architecture

State lives in one context (`AppStateProvider`): volumes, scan lifecycle,
settings, toasts, opportunities. Views are pure functions of backend queries.
The treemap is a hand-rolled canvas renderer using the squarified algorithm
(`src/lib/treemap.ts`) with hover inspection, zoom stack and category legend.
`VirtualTable` renders windowed rows, so 100k-row tables stay smooth.
