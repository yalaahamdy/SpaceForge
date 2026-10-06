# SpaceForge local build script (Windows).
# Produces the NSIS installer and the portable executable.
# Requires: Node 18+, Rust MSVC toolchain, WebView2 runtime.

$ErrorActionPreference = "Stop"

Write-Host "== SpaceForge build ==" -ForegroundColor Cyan

Push-Location $PSScriptRoot\..

Write-Host "-- installing frontend dependencies" -ForegroundColor Yellow
npm ci

Write-Host "-- typechecking frontend" -ForegroundColor Yellow
npm run typecheck

Write-Host "-- building frontend" -ForegroundColor Yellow
npm run build

Write-Host "-- running core tests (including safety regression suite)" -ForegroundColor Yellow
cargo test -p spaceforge-core --manifest-path src-tauri\Cargo.toml

Write-Host "-- building release bundle" -ForegroundColor Yellow
npm run tauri:build

New-Item -ItemType Directory -Force -Path release | Out-Null
Copy-Item src-tauri\target\release\spaceforge.exe release\SpaceForge-Portable.exe -Force
Get-ChildItem src-tauri\target\release\bundle\nsis -Filter *.exe |
  Copy-Item -Destination release -Force

Write-Host ""
Write-Host "== Done. Artifacts in release/ ==" -ForegroundColor Green
Get-ChildItem release

Pop-Location
