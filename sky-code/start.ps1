# SkyCode Complete Starter
# Runs setup (if needed) then launches chat
# Usage: .\start.ps1

$ErrorActionPreference = "Stop"

Write-Host "`n=== SkyCode Starter ===" -ForegroundColor Cyan
Write-Host "Checking if setup is needed...`n" -ForegroundColor Gray

# Quick check if everything is ready
$ollamaOk = $false
$skybridgeOk = $false

try {
    $wc = New-Object System.Net.WebClient
    $null = $wc.DownloadString('http://localhost:11434/api/tags')
    $ollamaOk = $true
} catch {}

try {
    $wc = New-Object System.Net.WebClient
    $null = $wc.DownloadString('http://localhost:4000/')
    $skybridgeOk = $true
} catch [System.Net.WebException] {
    if ($_.Exception.Response) { $skybridgeOk = $true }
}

# Run setup if needed
if (-not $ollamaOk -or -not $skybridgeOk) {
    Write-Host "Running first-time setup..." -ForegroundColor Yellow
    .\setup-auto.ps1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "`nSetup failed. Please check errors above." -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "Services already running. Starting chat...`n" -ForegroundColor Green
}

# Launch chat
.\chat.ps1
