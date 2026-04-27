# Sky-Code Installer v1.0.0
# Simple PowerShell Installer - No admin required

param(
    [switch]$Uninstall,
    [string]$InstallPath = "$env:LOCALAPPDATA\SkyCode"
)

$ErrorActionPreference = "Stop"

function Show-Banner {
    Write-Host "`n========================================" -ForegroundColor Cyan
    Write-Host "  Sky-Code Installer v1.0.0" -ForegroundColor Cyan
    Write-Host "  Offline AI Agent - 100% Local" -ForegroundColor Cyan
    Write-Host "========================================`n" -ForegroundColor Cyan
}

if ($Uninstall) {
    Show-Banner
    Write-Host "Uninstalling Sky-Code..." -ForegroundColor Yellow
    
    # Remove files
    if (Test-Path $InstallPath) {
        Remove-Item $InstallPath -Recurse -Force
        Write-Host "[OK] Files removed" -ForegroundColor Green
    }
    
    # Remove shortcuts
    $desktopShortcut = "$env:USERPROFILE\Desktop\Sky-Code.lnk"
    $startMenuDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Sky-Code"
    
    if (Test-Path $desktopShortcut) {
        Remove-Item $desktopShortcut -Force
        Write-Host "[OK] Desktop shortcut removed" -ForegroundColor Green
    }
    
    if (Test-Path $startMenuDir) {
        Remove-Item $startMenuDir -Recurse -Force
        Write-Host "[OK] Start menu shortcuts removed" -ForegroundColor Green
    }
    
    # Remove from PATH
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -like "*$InstallPath*") {
        $newPath = $userPath -replace [regex]::Escape(";$InstallPath\bin"), ""
        $newPath = $newPath -replace [regex]::Escape("$InstallPath\bin;"), ""
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Host "[OK] Removed from PATH" -ForegroundColor Green
    }
    
    Write-Host "`nUninstallation complete!`n" -ForegroundColor Green
    exit 0
}

# Installation
Show-Banner
Write-Host "Installing to: $InstallPath`n" -ForegroundColor Cyan

# Check Ollama
Write-Host "Checking prerequisites..." -ForegroundColor Cyan
$ollamaExists = Get-Command ollama -ErrorAction SilentlyContinue
if ($ollamaExists) {
    Write-Host "[OK] Ollama found" -ForegroundColor Green
} else {
    Write-Host "[!] Ollama not found (download from https://ollama.com)" -ForegroundColor Yellow
}
Write-Host ""

# Create directories
Write-Host "Creating directories..." -ForegroundColor Cyan
if (Test-Path $InstallPath) {
    Write-Host "Installation directory exists, upgrading..." -ForegroundColor Yellow
    Remove-Item "$InstallPath\*" -Recurse -Force -ErrorAction SilentlyContinue
} else {
    New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null
}

New-Item -ItemType Directory -Path "$InstallPath\bin" -Force | Out-Null
New-Item -ItemType Directory -Path "$InstallPath\docs" -Force | Out-Null
Write-Host "[OK] Directories created" -ForegroundColor Green

# Copy files
Write-Host "`nCopying files..." -ForegroundColor Cyan
$sourceDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# Binaries
if (Test-Path "$sourceDir\target\release\sky.exe") {
    Copy-Item "$sourceDir\target\release\sky.exe" "$InstallPath\bin\" -Force
    Write-Host "[OK] sky.exe copied" -ForegroundColor Green
}

if (Test-Path "$sourceDir\..\skybridge\target\release\skybridge.exe") {
    Copy-Item "$sourceDir\..\skybridge\target\release\skybridge.exe" "$InstallPath\bin\" -Force
    Write-Host "[OK] skybridge.exe copied" -ForegroundColor Green
}

# Launcher script
if (Test-Path "$sourceDir\start-with-skybridge.ps1") {
    Copy-Item "$sourceDir\start-with-skybridge.ps1" "$InstallPath\" -Force
    Write-Host "[OK] Launcher script copied" -ForegroundColor Green
}

# Documentation
$docs = @("README.md", "QUICK-START.md", "COMPLETE-FEATURE-REFERENCE.md", "LICENSE", "CHANGELOG.md")
foreach ($doc in $docs) {
    if (Test-Path "$sourceDir\$doc") {
        Copy-Item "$sourceDir\$doc" "$InstallPath\docs\" -Force
    }
}
Write-Host "[OK] Documentation copied" -ForegroundColor Green

# Create launcher batch file
$launcherBat = @"
@echo off
cd /d "$InstallPath"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$InstallPath\start-with-skybridge.ps1"
"@
$launcherBat | Out-File "$InstallPath\Sky-Code.bat" -Encoding ASCII -Force
Write-Host "[OK] Launcher created" -ForegroundColor Green

# Add to PATH
Write-Host "`nAdding to PATH..." -ForegroundColor Cyan
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallPath\bin*") {
    if ($userPath -match ';\s*$') {
        $newPath = $userPath + "$InstallPath\bin"
    } else {
        $newPath = $userPath + ";$InstallPath\bin"
    }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "[OK] Added to PATH (restart terminal)" -ForegroundColor Green
} else {
    Write-Host "[OK] Already in PATH" -ForegroundColor Green
}

# Create Start Menu shortcuts
Write-Host "`nCreating Start Menu shortcuts..." -ForegroundColor Cyan
$startMenuDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Sky-Code"
New-Item -ItemType Directory -Path $startMenuDir -Force | Out-Null

$WshShell = New-Object -ComObject WScript.Shell

# CLI shortcut
$shortcut = $WshShell.CreateShortcut("$startMenuDir\Sky-Code.lnk")
$shortcut.TargetPath = "$InstallPath\Sky-Code.bat"
$shortcut.WorkingDirectory = "$InstallPath"
$shortcut.Description = "Sky-Code Offline AI Agent"
$shortcut.Save()

# Documentation shortcut
$docsShortcut = $WshShell.CreateShortcut("$startMenuDir\Documentation.lnk")
$docsShortcut.TargetPath = "$InstallPath\docs"
$docsShortcut.Save()

Write-Host "[OK] Start Menu shortcuts created" -ForegroundColor Green

# Desktop shortcut
Write-Host "`nCreate Desktop shortcut? (Y/n): " -ForegroundColor Cyan -NoNewline
$response = Read-Host
if ($response -ne "n" -and $response -ne "N") {
    $desktopShortcut = $WshShell.CreateShortcut("$env:USERPROFILE\Desktop\Sky-Code.lnk")
    $desktopShortcut.TargetPath = "$InstallPath\Sky-Code.bat"
    $desktopShortcut.WorkingDirectory = "$InstallPath"
    $desktopShortcut.Description = "Sky-Code Offline AI Agent"
    $desktopShortcut.Save()
    Write-Host "[OK] Desktop shortcut created" -ForegroundColor Green
}

# Create uninstaller
Copy-Item $MyInvocation.MyCommand.Path "$InstallPath\install-sky-code.ps1" -Force

# Installation complete
Write-Host "`n========================================" -ForegroundColor Green
Write-Host "  Installation Complete!" -ForegroundColor Green
Write-Host "========================================`n" -ForegroundColor Green

Write-Host "Installed to: $InstallPath" -ForegroundColor Cyan
Write-Host "Documentation: $InstallPath\docs`n" -ForegroundColor Cyan

Write-Host "Quick Start:" -ForegroundColor Cyan
Write-Host "  1. Start Menu > Sky-Code > Sky-Code"
Write-Host "  2. Or run: sky.exe (after restarting terminal)`n"

Write-Host "Documentation:" -ForegroundColor Cyan
Write-Host "  - Quick Start: $InstallPath\docs\QUICK-START.md"
Write-Host "  - Full Reference: $InstallPath\docs\COMPLETE-FEATURE-REFERENCE.md`n"

if (-not $ollamaExists) {
    Write-Host "Prerequisites:" -ForegroundColor Yellow
    Write-Host "  1. Install Ollama: https://ollama.com"
    Write-Host "  2. Download model: ollama pull llama3.1:8b`n"
}

Write-Host "Ready to use!" -ForegroundColor Green
Write-Host "Uninstall with: cd `"$InstallPath`"; .\install-sky-code.ps1 -Uninstall`n"
