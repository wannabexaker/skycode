# Sky-Code Simple Launcher
# Uses simple LiteLLM setup without config file

$ErrorActionPreference = "SilentlyContinue"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Sky-Code Simple Launcher" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Set-Location $PSScriptRoot

# Step 1: Clean old processes
Write-Host "[1/4] Cleaning up..." -ForegroundColor Yellow
Get-Process | Where-Object { $_.ProcessName -like "*litellm*" } | ForEach-Object {
    Write-Host "  Stopping: $($_.ProcessName) (PID $($_.Id))" -ForegroundColor Gray
    Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Seconds 2
Write-Host "  [OK] Done" -ForegroundColor Green
Write-Host ""

# Step 2: Check/Start Ollama
Write-Host "[2/4] Checking Ollama..." -ForegroundColor Yellow
try {
    $test = Invoke-RestMethod -Uri "http://localhost:11434/api/tags" -TimeoutSec 2
    Write-Host "  [OK] Ollama running with $($test.models.Count) model(s)" -ForegroundColor Green
} catch {
    Write-Host "  Starting Ollama..." -ForegroundColor Gray
    $ollamaPath = "C:\Users\$env:USERNAME\AppData\Local\Programs\Ollama\ollama.exe"
    Start-Process $ollamaPath -ArgumentList "serve" -WindowStyle Hidden
    Start-Sleep -Seconds 5
    Write-Host "  [OK] Ollama started" -ForegroundColor Green
}
Write-Host ""

# Step 3: Start LiteLLM (simple mode)
Write-Host "[3/4] Starting LiteLLM..." -ForegroundColor Yellow
Write-Host "  This will open a new PowerShell window" -ForegroundColor Gray
Write-Host "  LEAVE IT OPEN while using Sky-Code!" -ForegroundColor Yellow
Write-Host ""

$litellmCommand = @"
`$env:PYTHONIOENCODING='utf-8'; `$env:PYTHONUTF8='1'; litellm --model ollama/llama3.1:8b --api_base http://localhost:11434
"@

Start-Process powershell -ArgumentList "-NoExit", "-Command", $litellmCommand -WindowStyle Normal

Write-Host "  Waiting for LiteLLM to start..." -ForegroundColor Gray
Start-Sleep -Seconds 10

# Detect port by checking common ports
$proxyPort = $null
foreach ($testPort in 4000..4200) {
    try {
        Invoke-WebRequest -Uri "http://localhost:$testPort/health" -TimeoutSec 0.5 -UseBasicParsing -ErrorAction Stop | Out-Null
        $proxyPort = $testPort
        break
    } catch {}
}

if (-not $proxyPort) {
    Write-Host "  [!] Could not auto-detect port" -ForegroundColor Yellow
    Write-Host "  Check the LiteLLM window for the port number" -ForegroundColor Yellow
    Write-Host "  Look for: 'Uvicorn running on http://0.0.0.0:XXXXX'" -ForegroundColor Gray
    Write-Host ""
    $proxyPort = Read-Host "  Enter the port number"
}

Write-Host "  [OK] Using port $proxyPort" -ForegroundColor Green
Write-Host ""

# Step 4: Launch Sky-Code  
Write-Host "[4/4] Launching Sky-Code..." -ForegroundColor Yellow

$env:FILANTHROPIC_BASE_URL = "http://localhost:$proxyPort"
$env:FILANTHROPIC_API_KEY = "ollama"
$env:FILANTHROPIC_MODEL = "llama3.1:8b"

Write-Host ""
Write-Host "Configuration:" -ForegroundColor Cyan
Write-Host "  Proxy URL: http://localhost:$proxyPort" -ForegroundColor Gray
Write-Host "  Model: llama3.1:8b (direct)" -ForegroundColor Gray
Write-Host "  Permission: workspace-write" -ForegroundColor Gray
Write-Host ""

if (-not (Test-Path ".\target\release\sky.exe")) {
    Write-Host "[X] sky.exe not found!" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Starting Sky-Code..." -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

.\target\release\sky.exe --permission-mode workspace-write

Write-Host ""
Write-Host "Sky-Code closed." -ForegroundColor Yellow
Write-Host "Remember to close the LiteLLM PowerShell window!" -ForegroundColor Yellow
Read-Host "Press Enter to exit"
