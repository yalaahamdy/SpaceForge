# Tests

- **Rust core** (scanner, safety, duplicates, cleaners, db, snapshot):
  `cd src-tauri && cargo test -p spaceforge-core` — see docs/TESTING.md
  for the full suite inventory, including the spec §72 safety regression
  tests that prove protected paths can never be cleaned.
- **Frontend**: `npm run typecheck` (strict) and `npm run build`.
- **CI**: `.github/workflows/release.yml` runs the full matrix on
  windows-latest for every tag and produces the installer + portable exe.
