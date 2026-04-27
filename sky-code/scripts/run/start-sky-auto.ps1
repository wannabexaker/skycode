# Sky-Code Automated Launcher
# Automatically starts Ollama, LiteLLM, and Sky-Code

param(
    [switch]$Clean = $false
)

$ErrorActionPreference = "SilentlyContinue"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Sky-Code Automated Launcher" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Navigate to script directory
Set-Location $PSScriptRoot

# === STEP 1: Cleanup old processes ===
if ($Clean) {
    Write-Host "[1/5] Cleaning up old processes..." -ForegroundColor Yellow
    Get-Process | Where-Object { 
        $_.ProcessName -like "*litellm*" -or 
        $_.ProcessName -like "*sky*" -or 
        ($_.ProcessName -eq "python" -and $_.MainWindowTitle -like "*litellm*")
    } | ForEach-Object {
        Write-Host "  Stopping: $($_.ProcessName) (PID $($_.Id))" -ForegroundColor Gray
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
    Start-Sleep -Seconds 2
    Write-Host "  [OK] Cleanup complete" -ForegroundColor Green
} else {
    Write-Host "[1/5] Checking for existing processes..." -ForegroundColor Yellow
    $existing = Get-Process | Where-Object { $_.ProcessName -like "*litellm*" }
    if ($existing) {
        Write-Host "  [!] Found existing LiteLLM processes" -ForegroundColor Yellow
        Write-Host "  Run with -Clean flag to stop them first" -ForegroundColor Yellow
    } else {
        Write-Host "  [OK] No conflicts found" -ForegroundColor Green
    }
}

Write-Host ""

# === STEP 2: Start/Verify Ollama ===
Write-Host "[2/5] Starting Ollama..." -ForegroundColor Yellow

$ollamaRunning = $false
try {
    $test = Invoke-RestMethod -Uri "http://localhost:11434/api/tags" -TimeoutSec 2
    $ollamaRunning = $true
    Write-Host "  [OK] Ollama already running with $($test.models.Count) model(s)" -ForegroundColor Green
} catch {
    Write-Host "  Ollama not running, starting it..." -ForegroundColor Gray
    $ollamaPath = "C:\Users\$env:USERNAME\AppData\Local\Programs\Ollama\ollama.exe"
    
    if (-not (Test-Path $ollamaPath)) {
        Write-Host "  [X] ERROR: Ollama not found at $ollamaPath" -ForegroundColor Red
        Read-Host "Press Enter to exit"
        exit 1
    }
    
    Start-Process $ollamaPath -ArgumentList "serve" -WindowStyle Hidden
    Start-Sleep -Seconds 5
    
    try {
        $test = Invoke-RestMethod -Uri "http://localhost:11434/api/tags" -TimeoutSec 3
        $ollamaRunning = $true
        Write-Host "  [OK] Ollama started successfully" -ForegroundColor Green
    } catch {
        Write-Host "  [X] ERROR: Failed to start Ollama" -ForegroundColor Red
        Read-Host "Press Enter to exit"
        exit 1
    }
}

Write-Host ""

# === STEP 3: Start LiteLLM Proxy ===
Write-Host "[3/5] Starting LiteLLM proxy..." -ForegroundColor Yellow

$configFile = "C:\ai-proxy\litellm-config.yaml"
if (-not (Test-Path $configFile)) {
    Write-Host "  [X] ERROR: Config file not found: $configFile" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

# Create temp file for LiteLLM output
$logFile = "$env:TEMP\litellm-sky-$(Get-Date -Format 'yyyyMMdd-HHmmss').log"

Write-Host "  Starting LiteLLM with config file..." -ForegroundColor Gray
Write-Host "  Log file: $logFile" -ForegroundColor DarkGray

# Start LiteLLM in background, redirect output to log
$litellmJob = Start-Job -ScriptBlock {
    param($config, $log)
    # Fix encoding issues on Windows
    $env:PYTHONIOENCODING = "utf-8"
    $env:PYTHONUTF8 = "1"
    litellm --config $config 2>&1 | Tee-Object -FilePath $log
} -ArgumentList $configFile, $logFile

# Wait for startup and detect port
Write-Host "  Waiting for proxy to start (max 20 seconds)..." -ForegroundColor Gray

$proxyPort = $null
$maxWait = 20
$waited = 0

while ($waited -lt $maxWait -and -not $proxyPort) {
    Start-Sleep -Seconds 1
    $waited++
    
    if (Test-Path $logFile) {
        $content = Get-Content $logFile -Tail 30 -ErrorAction SilentlyContinue
        foreach ($line in $content) {
            # Match: "Uvicorn running on http://0.0.0.0:XXXXX" or "http://127.0.0.1:XXXXX"
            if ($line -match 'Uvicorn running on http://.*?:(\d+)') {
                $proxyPort = [int]$matches[1]
                break
            }
        }
    }
    
    # Show progress
    if ($waited % 3 -eq 0) {
        Write-Host "    ..." -ForegroundColor DarkGray
    }
}

if (-not $proxyPort) {
    Write-Host "  [X] ERROR: Could not detect LiteLLM proxy port" -ForegroundColor Red
    Write-Host "  Check log file: $logFile" -ForegroundColor Yellow
    Stop-Job -Job $litellmJob -ErrorAction SilentlyContinue
    Remove-Job -Job $litellmJob -ErrorAction SilentlyContinue
    Read-Host "Press Enter to exit"
    exit 1
}

Write-Host "  [OK] LiteLLM proxy running on port $proxyPort" -ForegroundColor Green
Write-Host ""

# === STEP 4: Verify proxy responds ===
Write-Host "[4/5] Verifying proxy..." -ForegroundColor Yellow

Start-Sleep -Seconds 2

try {
    $headers = @{ "Authorization" = "Bearer local-dev-key" }
    $response = Invoke-RestMethod -Uri "http://localhost:$proxyPort/v1/models" -Headers $headers -TimeoutSec 5
    $modelCount = $response.data.Count
    Write-Host "  [OK] Proxy responds with $modelCount model(s)" -ForegroundColor Green
} catch {
    Write-Host "  [!] WARNING: Proxy not responding yet (may need more time)" -ForegroundColor Yellow
}

Write-Host ""

# === STEP 5: Configure environment and launch Sky-Code ===
Write-Host "[5/5] Launching Sky-Code..." -ForegroundColor Yellow

$env:FILANTHROPIC_BASE_URL = "http://localhost:$proxyPort"
$env:FILANTHROPIC_API_KEY = "local-dev-key"
$env:FILANTHROPIC_MODEL = "claude-apus-4-6"

Write-Host ""
Write-Host "Configuration:" -ForegroundColor Cyan
Write-Host "  Proxy URL: http://localhost:$proxyPort" -ForegroundColor Gray
Write-Host "  Model: claude-apus-4-6 -> llama3.1:8b" -ForegroundColor Gray
Write-Host "  Permission: workspace-write" -ForegroundColor Gray
Write-Host ""

if (-not (Test-Path ".\target\release\sky.exe")) {
    Write-Host "[X] ERROR: sky.exe not found!" -ForegroundColor Red
    Write-Host "    Build first with: cargo build --release" -ForegroundColor Yellow
    Read-Host "Press Enter to exit"
    exit 1
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Starting Sky-Code REPL..." -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "NOTE: LiteLLM log file: $logFile" -ForegroundColor DarkGray
Write-Host ""

# Launch Sky-Code
.\target\release\sky.exe --permission-mode workspace-write

# Cleanup on exit
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Sky-Code closed." -ForegroundColor Yellow
Write-Host ""
Write-Host "LiteLLM proxy is still running (Job ID: $($litellmJob.Id))" -ForegroundColor Yellow
Write-Host "To stop it, run: Stop-Job -Id $($litellmJob.Id); Remove-Job -Id $($litellmJob.Id)" -ForegroundColor Gray
Write-Host ""
Read-Host "Press Enter to exit"
