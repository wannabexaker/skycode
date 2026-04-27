# Sky-Code Offline Launcher (Simple Version)
# Starts LiteLLM and launches Sky-Code

Write-Host "=====================================" -ForegroundColor Cyan
Write-Host "  Sky-Code - Offline Mode" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan
Write-Host ""

Set-Location $PSScriptRoot

# Check Ollama
Write-Host "Checking Ollama..." -ForegroundColor Yellow
try {
    $test = Invoke-RestMethod -Uri "http://localhost:11434/api/tags" -TimeoutSec 2
    Write-Host "  [OK] Ollama running" -ForegroundColor Green
} catch {
    Write-Host "  [X] Ollama NOT running - start it first!" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

# Start LiteLLM proxy with output redirection
Write-Host "Starting LiteLLM proxy..." -ForegroundColor Yellow
$logFile = "$env:TEMP\litellm-output.txt"

# Start LiteLLM in background, redirect output to file
$process = Start-Process powershell -ArgumentList @(
    "-NoExit",
    "-Command",
    "`$env:PYTHONIOENCODING='utf-8'; `$env:PYTHONUTF8='1'; litellm --model ollama/llama3.1:8b --api_base http://localhost:11434 | Tee-Object -FilePath '$logFile' -Append"
) -WindowStyle Minimized -PassThru

# Wait for startup and read port from log
Write-Host "  Waiting for proxy to start..." -ForegroundColor Gray
$proxyPort = $null
$maxWait = 15
$waited = 0

while ($waited -lt $maxWait -and -not $proxyPort) {
    Start-Sleep -Seconds 1
    $waited++
    
    if (Test-Path $logFile) {
        $content = Get-Content $logFile -Tail 20 -ErrorAction SilentlyContinue
        foreach ($line in $content) {
            if ($line -match 'Uvicorn running on http://.*:(\d+)') {
                $proxyPort = [int]$matches[1]
                break
            }
        }
    }
}

if (-not $proxyPort) {
    Write-Host "  [X] Could not detect proxy port" -ForegroundColor Red
    Stop-Process -Id $process.Id -ErrorAction SilentlyContinue
    Read-Host "Press Enter to exit"
    exit 1
}

Write-Host "  [OK] Proxy running on port $proxyPort" -ForegroundColor Green
Write-Host ""

# Configure environment
$env:FILANTHROPIC_BASE_URL = "http://localhost:$proxyPort"
$env:FILANTHROPIC_API_KEY = "ollama"
$env:FILANTHROPIC_MODEL = "llama3.1:8b"

Write-Host "Configuration:" -ForegroundColor Cyan
Write-Host "  URL: http://localhost:$proxyPort" -ForegroundColor Gray
Write-Host "  Model: llama3.1:8b" -ForegroundColor Gray
Write-Host ""

# Launch Sky-Code
Write-Host "Launching Sky-Code..." -ForegroundColor Green
Write-Host ""

.\target\release\sky.exe --permission-mode workspace-write

Write-Host ""
Write-Host "Sky-Code closed." -ForegroundColor Cyan
Write-Host ""
Write-Host "NOTE: LiteLLM proxy is still running in background" -ForegroundColor Yellow
Write-Host "      To stop it, close the minimized PowerShell window" -ForegroundColor Yellow
Read-Host "Press Enter to exit"
