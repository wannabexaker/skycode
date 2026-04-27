# SkyCode Auto-Setup Script
# One-click setup and health check

param(
    [switch]$SkipBrowser,
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "========================================================================" -ForegroundColor Cyan
Write-Host " SkyCode Auto-Setup - One-Click Configuration" -ForegroundColor Green
Write-Host "========================================================================" -ForegroundColor Cyan
Write-Host ""

# Step 1: Auto-set environment variables
Write-Host "[1/5] Setting up environment..." -ForegroundColor Yellow

$env:HOME = $env:USERPROFILE
$env:FILANTHROPIC_BASE_URL = 'http://localhost:4000'
$env:FILANTHROPIC_TIMEOUT_SECS = '180'
$env:SKYCODE_LOG = 'warn'  # Quiet by default

Write-Host "  [OK] Environment configured" -ForegroundColor Green

# Step 2: Check if Ollama is installed
Write-Host ""
Write-Host "[2/5] Checking Ollama..." -ForegroundColor Yellow

$ollamaPath = Get-Command ollama -ErrorAction SilentlyContinue
if (-not $ollamaPath) {
    # Check default installation location
    $ollamaExe = "$env:LOCALAPPDATA\Programs\Ollama\ollama.exe"
    if (Test-Path $ollamaExe) {
        Write-Host "  [OK] Ollama found at: $ollamaExe" -ForegroundColor Green
    } else {
        Write-Host "  [!] Ollama not found. Please install from: https://ollama.ai" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "  [OK] Ollama installed: $($ollamaPath.Source)" -ForegroundColor Green
}

# Helper: test Ollama API (avoids proxy/TLS issues with Invoke-WebRequest)
function Test-OllamaApi {
    try {
        $wc = New-Object System.Net.WebClient
        $null = $wc.DownloadString('http://localhost:11434/api/tags')
        return $true
    } catch {
        return $false
    }
}

# Check if Ollama API is already responding
$ollamaApiOk = $false
if (Test-OllamaApi) {
    $ollamaApiOk = $true
    Write-Host "  [OK] Ollama is running and API is ready" -ForegroundColor Green
}

if (-not $ollamaApiOk) {
    # Check for processes - if running, just wait. Don't kill them!
    $ollamaProcs = Get-Process *ollama* -ErrorAction SilentlyContinue
    $setupRunning = $ollamaProcs | Where-Object { $_.ProcessName -like "*Setup*" }
    $serverRunning = $ollamaProcs | Where-Object { $_.ProcessName -notlike "*Setup*" }

    if ($setupRunning) {
        Write-Host "  [~] Ollama is updating itself - waiting for update to finish..." -ForegroundColor Yellow
    } elseif ($serverRunning) {
        Write-Host "  [~] Ollama process is starting up - waiting for API..." -ForegroundColor Yellow
    } else {
        # No Ollama processes at all - need to start it
        Write-Host "  [!] Ollama not running. Starting now..." -ForegroundColor Yellow
        $ollamaAppPath = "$env:LOCALAPPDATA\Programs\Ollama\ollama app.exe"
        if (-not (Test-Path $ollamaAppPath)) {
            Write-Host "  [!] Ollama not found. Download from https://ollama.com/download" -ForegroundColor Red
            exit 1
        }
        Start-Process -FilePath $ollamaAppPath -WindowStyle Hidden
        Write-Host "  [~] Ollama started (may auto-update on first run)..." -ForegroundColor Gray
    }

    # Wait for API - up to 5 minutes
    $maxWait = 300
    $waited = 0
    while ($waited -lt $maxWait) {
        if (Test-OllamaApi) {
            Write-Host "`r  [OK] Ollama API ready after $waited seconds!                    " -ForegroundColor Green
            $ollamaApiOk = $true
            break
        }
        $procs = Get-Process *ollama* -ErrorAction SilentlyContinue
        $isUpdating = [bool]($procs | Where-Object { $_.ProcessName -like "*Setup*" })
        $status = if ($isUpdating) { "updating" } else { "starting" }
        if ($waited % 5 -eq 0) {
            Write-Host "`r  [~] Ollama $status... $waited/$maxWait s" -NoNewline -ForegroundColor Gray
        }
        Start-Sleep -Seconds 1
        $waited++
    }

    if (-not $ollamaApiOk) {
        Write-Host "`r  [!] Ollama did not respond after 5 minutes.                    " -ForegroundColor Red
        Write-Host ""
        Write-Host "  Troubleshooting:" -ForegroundColor Cyan
        Write-Host "    1. Open Start Menu, search 'Ollama', click to launch it" -ForegroundColor White
        Write-Host "    2. Wait for the Ollama icon in the system tray (bottom-right)" -ForegroundColor White
        Write-Host "    3. If Windows Firewall asks, click 'Allow'" -ForegroundColor White
        Write-Host "    4. Run .\setup-auto.ps1 again" -ForegroundColor White
        exit 1
    }
}

# Step 3: Check for models
Write-Host ""
Write-Host "[3/5] Checking Ollama models..." -ForegroundColor Yellow

try {
    $wc = New-Object System.Net.WebClient
    $modelsJson = $wc.DownloadString('http://localhost:11434/api/tags')
    $models = ($modelsJson | ConvertFrom-Json).models
    
    if ($models.Count -eq 0) {
        Write-Host "  [!] No models installed. Downloading llama3.1:8b..." -ForegroundColor Yellow
        Write-Host "  [~] This may take a few minutes (4.9 GB download)..." -ForegroundColor Gray
        
        # Pull model
        $pullProcess = Start-Process -FilePath "$env:LOCALAPPDATA\Programs\Ollama\ollama.exe" `
            -ArgumentList "pull", "llama3.1:8b" `
            -NoNewWindow `
            -PassThru `
            -Wait
        
        if ($pullProcess.ExitCode -eq 0) {
            Write-Host "  [OK] Model downloaded successfully" -ForegroundColor Green
        } else {
            Write-Host "  [!] Model download failed" -ForegroundColor Red
            exit 1
        }
    } else {
        Write-Host "  [OK] Found $($models.Count) model(s):" -ForegroundColor Green
        foreach ($model in $models | Select-Object -First 3) {
            $sizeMB = [math]::Round($model.size / 1MB, 1)
            Write-Host "      - $($model.name) ($sizeMB MB)" -ForegroundColor Gray
        }
        # Auto-select best available model: prefer larger ones
        $preferred = @('dolphin-mistral:7b','llama3.1:8b','llama3.2:3b','qwen2.5:14b','qwen2.5:7b','mistral:7b','llama3.2:1b')
        $selectedModel = $null
        foreach ($pref in $preferred) {
            if ($models | Where-Object { $_.name -eq $pref }) {
                $selectedModel = $pref
                break
            }
        }
        if (-not $selectedModel) { $selectedModel = $models[0].name }
        $env:ANTHROPIC_MODEL = $selectedModel
        $env:ANTHROPIC_SMALL_FAST_MODEL = $models[0].name
        Write-Host "  [OK] Using model: $selectedModel" -ForegroundColor Cyan
    }
} catch {
    Write-Host "  [!] Could not check models: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# Step 4: Check and start SkyBridge
Write-Host ""
Write-Host "[4/5] Checking SkyBridge..." -ForegroundColor Yellow

$skybridgePath = "C:\Projects\MyTests\ClaudeCode\skybridge\target\release\skybridge.exe"

if (-not (Test-Path $skybridgePath)) {
    Write-Host "  [!] SkyBridge binary not found at: $skybridgePath" -ForegroundColor Red
    Write-Host "      Please build SkyBridge first" -ForegroundColor Yellow
    exit 1
}

# Helper: test SkyBridge (any response including 404 = running)
function Test-SkyBridgeApi {
    try {
        $wc = New-Object System.Net.WebClient
        $null = $wc.DownloadString('http://localhost:4000/')
        return $true
    } catch [System.Net.WebException] {
        # 404 means server is running but endpoint not found - that's fine
        if ($_.Exception.Response) { return $true }
        return $false
    } catch { return $false }
}

# Check if SkyBridge is already running
$localSkyBridgePath = (Resolve-Path $skybridgePath).Path
$runningSkybridge = Get-Process -Name 'skybridge' -ErrorAction SilentlyContinue
if ($runningSkybridge) {
    $runningPath = $runningSkybridge | Select-Object -First 1 | ForEach-Object { $_.Path }
    if ($runningPath -ne $localSkyBridgePath) {
        Write-Host "  [!] Wrong SkyBridge running ($runningPath). Restarting with local build..." -ForegroundColor Yellow
        $runningSkybridge | Stop-Process -Force
        Start-Sleep -Seconds 1
    }
}
if (Test-SkyBridgeApi) {
    Write-Host "  [OK] SkyBridge is already running" -ForegroundColor Green
} else {
    Write-Host "  [!] SkyBridge not running. Starting in background..." -ForegroundColor Yellow
    $bridgeProcess = Start-Process -FilePath $skybridgePath -WindowStyle Hidden -PassThru
    Write-Host "  [OK] SkyBridge started (PID: $($bridgeProcess.Id))" -ForegroundColor Green
    
    Write-Host "  [~] Waiting for SkyBridge to initialize..." -ForegroundColor Gray
    $maxWait = 15
    $waited = 0
    while ($waited -lt $maxWait) {
        if (Test-SkyBridgeApi) {
            Write-Host "  [OK] SkyBridge ready!" -ForegroundColor Green
            break
        }
        Start-Sleep -Seconds 1
        $waited++
    }
    if ($waited -ge $maxWait) {
        Write-Host "  [!] SkyBridge took too long to start" -ForegroundColor Yellow
        Write-Host "      Continuing anyway - may work!" -ForegroundColor Gray
    }
}

# Step 5: Run health check
Write-Host ""
Write-Host "[5/5] Running health check..." -ForegroundColor Yellow

$skyPath = "C:\Projects\MyTests\ClaudeCode\sky-code\target\release\sky.exe"

if (-not (Test-Path $skyPath)) {
    Write-Host "  [!] SkyCode binary not found at: $skyPath" -ForegroundColor Red
    Write-Host "      Please build SkyCode first" -ForegroundColor Yellow
    exit 1
}

# Run doctor check
cd C:\Projects\MyTests\ClaudeCode\sky-code
$doctorOutput = & $skyPath doctor 2>&1 | Out-String

if ($doctorOutput -match '0 errors') {
    Write-Host "  [OK] All health checks passed!" -ForegroundColor Green
} else {
    Write-Host "  [!] Some health checks failed:" -ForegroundColor Yellow
    Write-Host $doctorOutput
}

# Success summary
Write-Host ""
Write-Host "========================================================================" -ForegroundColor Cyan
Write-Host " Setup Complete! SkyCode is ready to use." -ForegroundColor Green
Write-Host "========================================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Quick Start Commands:" -ForegroundColor White
Write-Host "  .\target\release\sky.exe doctor           # Health check" -ForegroundColor Gray
Write-Host "  .\target\release\sky.exe config list      # View config" -ForegroundColor Gray
Write-Host "  .\target\release\sky.exe prompt 'hello'   # Try a prompt" -ForegroundColor Gray
Write-Host ""

# Offer to add to PowerShell profile
Write-Host "Would you like to add environment variables to your PowerShell profile?" -ForegroundColor Yellow
Write-Host "This will make them permanent across sessions. [Y/N]" -ForegroundColor Yellow
$response = Read-Host

if ($response -eq 'Y' -or $response -eq 'y') {
    $profileContent = @"

# SkyCode Auto-Configuration (added by setup-auto.ps1)
`$env:HOME = `$env:USERPROFILE
`$env:FILANTHROPIC_BASE_URL = 'http://localhost:4000'
`$env:FILANTHROPIC_TIMEOUT_SECS = '180'
"@

    # Create profile if it doesn't exist
    if (-not (Test-Path $PROFILE)) {
        New-Item -Path $PROFILE -ItemType File -Force | Out-Null
    }

    # Check if already added
    $currentProfile = Get-Content $PROFILE -Raw -ErrorAction SilentlyContinue
    if ($currentProfile -notmatch 'SkyCode Auto-Configuration') {
        Add-Content -Path $PROFILE -Value $profileContent
        Write-Host "[OK] Environment variables added to: $PROFILE" -ForegroundColor Green
        Write-Host "    They will be available in new PowerShell sessions" -ForegroundColor Gray
    } else {
        Write-Host "[OK] Environment variables already in profile" -ForegroundColor Green
    }
} else {
    Write-Host "[~] Skipped profile update. Environment variables are only for this session." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Happy coding!" -ForegroundColor Cyan
Write-Host ""
