# Sky-Code Launcher with SkyBridge
# Uses SkyBridge translator instead of LiteLLM for reliable streaming

Write-Host "======================================" -ForegroundColor Cyan
Write-Host "  Sky-Code - Offline Mode" -ForegroundColor Cyan
Write-Host "  (with SkyBridge Translator)" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

# Get paths (script is in scripts/run/, need to go to sky-code root)
$ScriptDir = $PSScriptRoot
$RootDir = Split-Path (Split-Path $ScriptDir -Parent) -Parent
$SkybridgeRoot = Split-Path $RootDir -Parent

Set-Location $RootDir

# Step 1: Check Ollama
Write-Host "[1/3] Checking Ollama..." -ForegroundColor Yellow
try {
    $test = Invoke-RestMethod -Uri "http://localhost:11434/api/tags" -TimeoutSec 2
    Write-Host "  [OK] Ollama running with $($test.models.Count) model(s)" -ForegroundColor Green
} catch {
    Write-Host "  [X] Ollama NOT running!" -ForegroundColor Red
    Write-Host "      Starting Ollama..." -ForegroundColor Yellow
    $ollamaPath = "C:\Users\$env:USERNAME\AppData\Local\Programs\Ollama\ollama.exe"
    Start-Process $ollamaPath -ArgumentList "serve" -WindowStyle Hidden
    Start-Sleep -Seconds 5
    Write-Host "  [OK] Ollama started" -ForegroundColor Green
}
Write-Host ""

# Step 2: Start SkyBridge
Write-Host "[2/3] Starting SkyBridge translator..." -ForegroundColor Yellow

# Check if SkyBridge binary exists
$skybridgePath = Join-Path $SkybridgeRoot "skybridge\target\release\skybridge.exe"
if (-not (Test-Path $skybridgePath)) {
    Write-Host "  [X] SkyBridge not found at: $skybridgePath" -ForegroundColor Red
    Write-Host "      Build it first:" -ForegroundColor Yellow
    Write-Host "      cd ..\skybridge" -ForegroundColor Gray
    Write-Host "      cargo build --release" -ForegroundColor Gray
    Read-Host "Press Enter to exit"
    exit 1
}

# Kill old SkyBridge instances
Get-Process | Where-Object { $_.ProcessName -eq "skybridge" } | Stop-Process -Force -ErrorAction SilentlyContinue

# Start SkyBridge in background window
$skybridgeDir = Join-Path $SkybridgeRoot "skybridge"
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$skybridgeDir'; .\target\release\skybridge.exe" -WindowStyle Minimized

Write-Host "  Waiting for SkyBridge to start..." -ForegroundColor Gray
Start-Sleep -Seconds 3

# Verify SkyBridge is responding
try {
    Invoke-RestMethod -Uri "http://localhost:4000/health" -TimeoutSec 2 | Out-Null
    Write-Host "  [OK] SkyBridge running on port 4000" -ForegroundColor Green
} catch {
    Write-Host "  [!] SkyBridge may need more time to start" -ForegroundColor Yellow
}
Write-Host ""

# Step 3: Configure and launch Sky-Code
Write-Host "[3/3] Launching Sky-Code..." -ForegroundColor Yellow

$env:FILANTHROPIC_BASE_URL = "http://localhost:4000"
$env:FILANTHROPIC_API_KEY = "skybridge"
$env:FILANTHROPIC_MODEL = "claude-apus-4-6"

Write-Host ""
Write-Host "Configuration:" -ForegroundColor Cyan
Write-Host "  SkyBridge: http://localhost:4000" -ForegroundColor Gray
Write-Host "  Ollama: http://localhost:11434" -ForegroundColor Gray
Write-Host "  Model: claude-apus-4-6 -> llama3.1:8b" -ForegroundColor Gray
Write-Host "  Permission: workspace-write" -ForegroundColor Gray
Write-Host ""

$skyPath = Join-Path $RootDir "target\release\sky.exe"
if (-not (Test-Path $skyPath)) {
    Write-Host "[X] sky.exe not found at: $skyPath" -ForegroundColor Red
    Write-Host "    Build first: cargo build --release" -ForegroundColor Yellow
    Read-Host "Press Enter to exit"
    exit 1
}

Write-Host "Starting sky.exe..." -ForegroundColor Cyan
& $skyPath --permission-mode workspace-write

Write-Host "======================================" -ForegroundColor Cyan
Write-Host "  Starting Sky-Code..." -ForegroundColor Green
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "NOTE: SkyBridge and Ollama are running in background" -ForegroundColor DarkGray
Write-Host ""

.\target\release\sky.exe --permission-mode workspace-write

Write-Host ""
Write-Host "Sky-Code closed." -ForegroundColor Yellow
Write-Host "SkyBridge is still running (close the minimized window to stop it)" -ForegroundColor Gray
Read-Host "Press Enter to exit"
