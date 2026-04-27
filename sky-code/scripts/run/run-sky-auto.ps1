# Sky-Code Offline Launcher
# Auto-starts LiteLLM proxy and launches Sky-Code

Write-Host "======================================" -ForegroundColor Cyan
Write-Host "  Sky-Code - Offline Mode" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

Set-Location $PSScriptRoot

# Step 1: Check Ollama
Write-Host "[1/3] Checking Ollama..." -ForegroundColor Yellow
try {
    $ollamaTest = Invoke-RestMethod -Uri "http://localhost:11434/api/tags" -TimeoutSec 2
    Write-Host "  [OK] Oll ama running with $($ollamaTest.models.Count) model(s)" -ForegroundColor Green
} catch {
    Write-Host "  [X] ERROR: Ollama not running!" -ForegroundColor Red
    Write-Host "      Start Ollama first" -ForegroundColor Yellow
    Read-Host "Press Enter to exit"
    exit 1
}

# Step 2: Start/Find LiteLLM Proxy
Write-Host "[2/3] Starting LiteLLM proxy..." -ForegroundColor Yellow

# Check if already running
$proxyPort = $null
foreach ($testPort in @(4000, 8000, 8080, 11435)) {
    try {
        Invoke-WebRequest -Uri "http://localhost:$testPort/health" -TimeoutSec 1 -ErrorAction Stop | Out-Null
        $proxyPort = $testPort
        Write-Host "  [OK] Found existing proxy on port $proxyPort" -ForegroundColor Green
        break
    } catch {}
}

# If not found, start it
if (-not $proxyPort) {
    Write-Host "  Starting new proxy (this may take 10-15 seconds)..." -ForegroundColor Gray
    
    # Launch LiteLLM in background PowerShell window
    $litellmCommand = "litellm --model ollama/llama3.1:8b --api_base http://localhost:11434"
    $process = Start-Process powershell -ArgumentList "-NoExit", "-Command", $litellmCommand -WindowStyle Minimized -PassThru
    
    # Wait for it to start and detect port
    $maxWait = 20  # seconds
    $waited = 0
    $found = $false
    
    while ($waited -lt $maxWait -and -not $found) {
        Start-Sleep -Seconds 1
        $waited++
        
        # Check various ports LiteLLM might use
        foreach ($testPort in 1024..65535) {
            try {
                $response = Invoke-WebRequest -Uri "http://localhost:$testPort/health" -TimeoutSec 0.5 -ErrorAction Stop
                if ($response.StatusCode -eq 200 -or $response.StatusCode -eq 401) {
                    $proxyPort = $testPort
                    $found = $true
                    Write-Host "  [OK] Proxy started on port $proxyPort" -ForegroundColor Green
                    break
                }
            } catch {}
        }
    }
    
    if (-not $found) {
        Write-Host "  [X] ERROR: Could not start proxy" -ForegroundColor Red
        Stop-Process -Id $process.Id -ErrorAction SilentlyContinue
        Read-Host "Press Enter to exit"
        exit 1
    }
}

# Step 3: Configure Environment
Write-Host "[3/3] Launching Sky-Code..." -ForegroundColor Yellow

$env:FILANTHROPIC_BASE_URL = "http://localhost:$proxyPort"
$env:FILANTHROPIC_API_KEY = "ollama"
$env:FILANTHROPIC_MODEL = "llama3.1:8b"

Write-Host ""
Write-Host "Configuration:" -ForegroundColor Cyan
Write-Host "  Proxy URL: http://localhost:$proxyPort" -ForegroundColor Gray
Write-Host "  Model: llama3.1:8b (via Ollama)" -ForegroundColor Gray
Write-Host "  Permission: workspace-write" -ForegroundColor Gray
Write-Host ""

# Check binary exists
if (-not (Test-Path ".\target\release\sky.exe")) {
    Write-Host "[X] sky.exe not found! Build first with: cargo build --release" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

# Launch Sky-Code
Write-Host "[OK] Starting Sky-Code REPL..." -ForegroundColor Green
Write-Host ""

.\target\release\sky.exe --permission-mode workspace-write

Write-Host ""
Write-Host "Sky-Code closed." -ForegroundColor Cyan
Read-Host "Press Enter to exit"
