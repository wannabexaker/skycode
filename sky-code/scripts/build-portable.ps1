# Build Portable Sky-Code Package
param(
    [string]$Version = "1.0.0",
    [string]$OutputDir = ".\dist"
)

$ErrorActionPreference = "Stop"

Write-Host "[BUILD] Sky-Code Portable Package v$Version" -ForegroundColor Cyan
Write-Host ""

# Paths
$ScriptDir = $PSScriptRoot
$ProjectRoot = Split-Path $ScriptDir -Parent
$SkyBridgePath = Join-Path (Split-Path $ProjectRoot -Parent) "skybridge"
$PackageDir = Join-Path $OutputDir "sky-code-portable-$Version"
$ZipPath = Join-Path $OutputDir "sky-code-portable-$Version.zip"

# Clean previous build
if (Test-Path $OutputDir) {
    Write-Host "[CLEAN] Removing previous build..." -ForegroundColor Yellow
    Remove-Item $OutputDir -Recurse -Force
}

# Create package structure
New-Item -ItemType Directory -Path $PackageDir -Force | Out-Null
New-Item -ItemType Directory -Path "$PackageDir\bin" -Force | Out-Null
New-Item -ItemType Directory -Path "$PackageDir\launchers" -Force | Out-Null

Write-Host "[OK] Package structure created" -ForegroundColor Green
Write-Host ""

# Copy binaries
Write-Host "[COPY] Copying binaries..." -ForegroundColor Cyan

Copy-Item "$ProjectRoot\target\release\sky.exe" "$PackageDir\bin\" -ErrorAction Stop
$SkySize = (Get-Item "$ProjectRoot\target\release\sky.exe").Length / 1MB
Write-Host ("  [OK] sky.exe ({0:F1} MB)" -f $SkySize) -ForegroundColor Gray

Copy-Item "$SkyBridgePath\target\release\skybridge.exe" "$PackageDir\bin\" -ErrorAction Stop
$BridgeSize = (Get-Item "$SkyBridgePath\target\release\skybridge.exe").Length / 1MB
Write-Host ("  [OK] skybridge.exe ({0:F1} MB)" -f $BridgeSize) -ForegroundColor Gray

Write-Host ""

# Copy launchers
Write-Host "[COPY] Copying launchers..." -ForegroundColor Cyan

Copy-Item "$ScriptDir\run\start-with-skybridge.ps1" "$PackageDir\launchers\" -ErrorAction Stop
Copy-Item "$ScriptDir\run\Sky-Code.bat" "$PackageDir\launchers\" -ErrorAction Stop
Copy-Item "$ScriptDir\install\install-desktop-shortcut.ps1" "$PackageDir\launchers\" -ErrorAction Stop

Write-Host "  [OK] Launcher scripts copied" -ForegroundColor Gray
Write-Host ""

# Copy documentation
Write-Host "[COPY] Copying documentation..." -ForegroundColor Cyan

Copy-Item "$ProjectRoot\docs\guides\QUICK-START.md" "$PackageDir\" -ErrorAction Stop
Copy-Item "$ProjectRoot\README.md" "$PackageDir\" -ErrorAction SilentlyContinue
Copy-Item "$ProjectRoot\..\docs\SKY-CODE-ROADMAP.md" "$PackageDir\" -ErrorAction SilentlyContinue

Write-Host "  [OK] Documentation copied" -ForegroundColor Gray
Write-Host ""

# Create portable README
Write-Host "[CREATE] Creating portable README..." -ForegroundColor Cyan

$ReadmeContent = "# Sky-Code Portable Edition v$Version`n`n"
$ReadmeContent += "100% Offline AI Agent - Works anywhere, no installation needed!`n`n"
$ReadmeContent += "## Quick Start`n`n"
$ReadmeContent += "1. Double-click: launchers\Sky-Code.bat`n"
$ReadmeContent += "2. Wait for SkyBridge to start`n"
$ReadmeContent += "3. Start chatting!`n`n"
$ReadmeContent += "## What's Inside`n`n"
$ReadmeContent += "- bin\sky.exe - Main AI agent`n"
$ReadmeContent += "- bin\skybridge.exe - Anthropic to Ollama translator`n"
$ReadmeContent += "- launchers\ - Startup scripts`n"
$ReadmeContent += "- QUICK-START.md - Full setup guide`n`n"
$ReadmeContent += "## Requirements`n`n"
$ReadmeContent += "- Windows 10/11`n"
$ReadmeContent += "- Ollama installed (for local LLM)`n"
$ReadmeContent += "- Model: ollama pull llama3.1:8b`n`n"
$ReadmeContent += "## Troubleshooting`n`n"
$ReadmeContent += "Connection refused on port 4000:`n"
$ReadmeContent += "  - SkyBridge not running. Start: .\bin\skybridge.exe`n`n"
$ReadmeContent += "Ollama connection failed:`n"
$ReadmeContent += "  - Run: ollama serve`n"
$ReadmeContent += "  - Check: ollama list (should see llama3.1:8b)`n`n"
$ReadmeContent += "## Privacy`n`n"
$ReadmeContent += "100% Offline & Private:`n"
$ReadmeContent += "  - No cloud API calls`n"
$ReadmeContent += "  - No telemetry`n"
$ReadmeContent += "  - No internet required`n"
$ReadmeContent += "  - All data stays on your machine`n`n"
$ReadmeContent += "Build: $Version - $(Get-Date -Format 'yyyy-MM-dd HH:mm')`n"

Set-Content -Path "$PackageDir\README-PORTABLE.txt" -Value $ReadmeContent -Encoding UTF8

Write-Host "  [OK] Portable README created" -ForegroundColor Gray
Write-Host ""

# Create version info
$VersionInfo = @{
    version = $Version
    build_date = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    components = @{
        sky_code = @{
            size_mb = [math]::Round((Get-Item "$ProjectRoot\target\release\sky.exe").Length / 1MB, 2)
            path = "bin\sky.exe"
        }
        skybridge = @{
            size_mb = [math]::Round((Get-Item "$SkyBridgePath\target\release\skybridge.exe").Length / 1MB, 2)
            path = "bin\skybridge.exe"
        }
    }
} | ConvertTo-Json -Depth 5

Set-Content -Path "$PackageDir\version.json" -Value $VersionInfo -Encoding UTF8

Write-Host "  [OK] Version manifest created" -ForegroundColor Gray
Write-Host ""

# Calculate total size
$TotalSize = (Get-ChildItem $PackageDir -Recurse | Measure-Object -Property Length -Sum).Sum / 1MB

Write-Host "[STATS] Package Statistics:" -ForegroundColor Cyan
Write-Host ("  Total files: {0}" -f (Get-ChildItem $PackageDir -Recurse -File).Count) -ForegroundColor Gray
Write-Host ("  Total size: {0:F1} MB" -f $TotalSize) -ForegroundColor Gray
Write-Host ""

# Create zip
Write-Host "[ZIP] Creating zip archive..." -ForegroundColor Cyan

if (Test-Path $ZipPath) {
    Remove-Item $ZipPath -Force
}

Compress-Archive -Path "$PackageDir\*" -DestinationPath $ZipPath -CompressionLevel Optimal

$ZipSize = (Get-Item $ZipPath).Length / 1MB

Write-Host ("  [OK] Compressed to {0:F1} MB" -f $ZipSize) -ForegroundColor Gray
Write-Host ""

# Summary
Write-Host "========================================" -ForegroundColor Green
Write-Host "  Sky-Code Portable Package Built!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Package: $ZipPath" -ForegroundColor Cyan
Write-Host "Folder:  $PackageDir" -ForegroundColor Cyan
Write-Host ("Size:    {0:F1} MB (compressed)" -f $ZipSize) -ForegroundColor Cyan
Write-Host ""
Write-Host "Ready for distribution!" -ForegroundColor Yellow
Write-Host ""
Write-Host "Next steps:" -ForegroundColor White
Write-Host "  1. Test: Extract zip and run Sky-Code.bat" -ForegroundColor Gray
Write-Host "  2. Share: Upload to GitHub releases" -ForegroundColor Gray
Write-Host "  3. USB: Copy folder to USB stick" -ForegroundColor Gray
