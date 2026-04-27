# Sky-Code Launcher Script
# Runs Sky-Code in offline mode with Ollama + LiteLLM proxy

Write-Host "======================================" -ForegroundColor Cyan
Write-Host "  Sky-Code - Offline Mode" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

# Navigate to sky-code directory
Set-Location $PSScriptRoot

# === OFFLINE ENVIRONMENT CONFIGURATION ===
# Redirect Anthropic API calls to local LiteLLM proxy
$env:FILANTHROPIC_BASE_URL = "http://localhost:4000"
$env:FILANTHROPIC_API_KEY = "local-dev-key"  # Dummy key for proxy
$env:FILANTHROPIC_MODEL = "claude-apus-4-6"   # Maps to llama3.1:8b via proxy

Write-Host "Environment configured for offline mode:" -ForegroundColor Green
Write-Host "  Base URL: $env:FILANTHROPIC_BASE_URL" -ForegroundColor Gray
Write-Host "  Model: $env:FILANTHROPIC_MODEL -> llama3.1:8b (via Ollama)" -ForegroundColor Gray
Write-Host ""

# Check if LiteLLM proxy is running
$proxyRunning = $false
try {
    $headers = @{ "Authorization" = "Bearer local-dev-key" }
    $response = Invoke-WebRequest -Uri "http://localhost:4000/v1/models" -Headers $headers -TimeoutSec 2 -ErrorAction SilentlyContinue
    $proxyRunning = $true
} catch {
    $proxyRunning = $false
}

if (-not $proxyRunning) {
    Write-Host "[!] WARNING: LiteLLM proxy not running on port 4000" -ForegroundColor Yellow
    Write-Host "Start it first with:" -ForegroundColor Yellow
    Write-Host "  litellm --config C:\ai-proxy\litellm-config.yaml --port 4000" -ForegroundColor Cyan
    Write-Host ""
    $continue = Read-Host "Continue anyway? (y/N)"
    if ($continue -ne "y") {
        exit 0
    }
}

# Check if Ollama is running
$ollamaRunning = $false
try {
    $response = Invoke-WebRequest -Uri "http://localhost:11434/api/tags" -TimeoutSec 2 -ErrorAction SilentlyContinue
    $ollamaRunning = $true
} catch {
    $ollamaRunning = $false
}

if (-not $ollamaRunning) {
    Write-Host "[!] WARNING: Ollama not running on port 11434" -ForegroundColor Yellow
    Write-Host "    Start it first or ensure Ollama service is running" -ForegroundColor Yellow
    Write-Host ""
}

# Check if binary exists
if (-not (Test-Path ".\target\release\sky.exe")) {
    Write-Host "[X] ERROR: sky.exe not found!" -ForegroundColor Red
    Write-Host "    Build first with: cargo build --release" -ForegroundColor Yellow
    Read-Host "Press Enter to exit"
    exit 1
}

# Run Sky-Code with workspace-write permissions (requires approval for all actions)
Write-Host "[OK] Launching Sky-Code with workspace-write permissions..." -ForegroundColor Green
Write-Host "     (Tool execution will require your approval)" -ForegroundColor Gray
Write-Host ""

.\target\release\sky.exe --permission-mode workspace-write

Write-Host ""
Write-Host "Sky-Code closed." -ForegroundColor Cyan
Read-Host "Press Enter to exit"
