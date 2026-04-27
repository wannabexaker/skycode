#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ═══════════════════════════════════════════
#  SkyCode — Installer Build Script
#  Output: src-tauri/target/release/bundle/msi/SkyCode_*.msi
# ═══════════════════════════════════════════

$Root     = Split-Path $PSScriptRoot -Parent   # ClaudeCode\
$GuiDir   = $PSScriptRoot                       # ClaudeCode\sky-code-gui\
$BinDir   = Join-Path $GuiDir "src-tauri\binaries"

function Step([string]$msg) {
    Write-Host ""
    Write-Host "── $msg" -ForegroundColor Cyan
}

function Fail([string]$msg) {
    Write-Host "FAIL: $msg" -ForegroundColor Red
    exit 1
}

# ── 1. Get Rust target triple ────────────────────────────────────────────────
Step "Detecting Rust target triple"
$triple = (rustc -vV 2>&1 | Select-String "^host:").ToString().Trim() -replace "^host:\s+", ""
if (-not $triple) { Fail "Cannot detect Rust target triple. Is rustc installed?" }
Write-Host "  Triple: $triple"

# ── 2. Build sky.exe ─────────────────────────────────────────────────────────
Step "Building sky (CLI)"
$skyDir = Join-Path $Root "sky-code"
Push-Location $skyDir
    cargo build --release
    if ($LASTEXITCODE -ne 0) { Fail "cargo build failed for sky-code" }
Pop-Location

$skySrc = Join-Path $skyDir "target\release\sky.exe"
if (-not (Test-Path $skySrc)) { Fail "sky.exe not found after build: $skySrc" }

# ── 3. Build skybridge.exe ───────────────────────────────────────────────────
Step "Building skybridge (proxy)"
$bridgeDir = Join-Path $Root "skybridge"
Push-Location $bridgeDir
    cargo build --release
    if ($LASTEXITCODE -ne 0) { Fail "cargo build failed for skybridge" }
Pop-Location

$bridgeSrc = Join-Path $bridgeDir "target\release\skybridge.exe"
if (-not (Test-Path $bridgeSrc)) { Fail "skybridge.exe not found after build: $bridgeSrc" }

# ── 4. Stage binaries for Tauri ─────────────────────────────────────────────
Step "Staging binaries → src-tauri/binaries/"
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

$skyDest    = Join-Path $BinDir "sky-$triple.exe"
$bridgeDest = Join-Path $BinDir "skybridge-$triple.exe"

Copy-Item $skySrc    -Destination $skyDest    -Force
Copy-Item $bridgeSrc -Destination $bridgeDest -Force
Write-Host "  sky.exe      → $skyDest"
Write-Host "  skybridge.exe → $bridgeDest"
# ── 5. Install npm deps (if needed) ─────────────────────────────────────────
Step "Checking npm dependencies"
Push-Location $GuiDir
    if (-not (Test-Path "node_modules")) {
        Write-Host "  Running npm install..."
        npm install
        if ($LASTEXITCODE -ne 0) { Fail "npm install failed" }
    } else {
        Write-Host "  node_modules already present, skipping."
    }

# ── 6. Build installer ───────────────────────────────────────────────────────
Step "Building Tauri installer (.msi)"
    npm run tauri build
    if ($LASTEXITCODE -ne 0) { Fail "tauri build failed" }
Pop-Location

# ── 7. Report output ────────────────────────────────────────────────────────
Step "Done"
$msiDir = Join-Path $GuiDir "src-tauri\target\release\bundle\msi"
$msiFile = Get-ChildItem -Path $msiDir -Filter "*.msi" -ErrorAction SilentlyContinue | Select-Object -First 1

if ($msiFile) {
    $size = [math]::Round($msiFile.Length / 1MB, 1)
    Write-Host ""
    Write-Host "═══════════════════════════════════════" -ForegroundColor Green
    Write-Host "  Installer ready!" -ForegroundColor Green
    Write-Host "  $($msiFile.FullName)" -ForegroundColor White
    Write-Host "  Size: ${size} MB" -ForegroundColor White
    Write-Host "═══════════════════════════════════════" -ForegroundColor Green
} else {
    Write-Host "Build completed but .msi not found in expected location: $msiDir"
}
