# Sky-Code Installer
# Version 1.0.0
# PowerShell-based installer (no admin required)

param(
    [switch]$Uninstall,
    [string]$InstallPath = "$env:LOCALAPPDATA\SkyCode"
)

$ErrorActionPreference = "Stop"

# Colors
function Write-Info { Write-Host $args -ForegroundColor Cyan }
function Write-Success { Write-Host $args -ForegroundColor Green }
function Write-Error { Write-Host $args -ForegroundColor Red }
function Write-Warning { Write-Host $args -ForegroundColor Yellow }

# Banner
Write-Host ""
Write-Info "╔════════════════════════════════════════════╗"
Write-Info "║     Sky-Code Installer v1.0.0              ║"
Write-Info "║     Offline AI Agent - 100% Local          ║"
Write-Info "╚════════════════════════════════════════════╝"
Write-Host ""

if ($Uninstall) {
    Write-Info "🗑️  Uninstalling Sky-Code..."
    
    # Remove installed files
    if (Test-Path $InstallPath) {
        Write-Info "Removing files from $InstallPath..."
        Remove-Item $InstallPath -Recurse -Force
        Write-Success "✅ Files removed"
    }
    
    # Remove shortcuts
    $desktopShortcut = "$env:USERPROFILE\Desktop\Sky-Code.lnk"
    $startMenuDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Sky-Code"
    
    if (Test-Path $desktopShortcut) {
        Remove-Item $desktopShortcut -Force
        Write-Success "✅ Desktop shortcut removed"
    }
    
    if (Test-Path $startMenuDir) {
        Remove-Item $startMenuDir -Recurse -Force
        Write-Success "✅ Start menu shortcuts removed"
    }
    
    # Remove from PATH (if added)
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -like "*$InstallPath*") {
        $newPath = $userPath -replace [regex]::Escape(";$InstallPath\bin"), ""
        $newPath = $newPath -replace [regex]::Escape("$InstallPath\bin;"), ""
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Success "✅ Removed from PATH"
    }
    
    Write-Host ""
    Write-Success "🎉 Sky-Code uninstalled successfully!"
    Write-Info "Τhanks for using Sky-Code! 🌌"
    exit 0
}

# Installation flow
Write-Info "📦 Installing Sky-Code to: $InstallPath"
Write-Host ""

# Check prerequisites
Write-Info "🔍 Checking prerequisites..."

# Check Ollama
$ollamaInstalled = Get-Command ollama -ErrorAction SilentlyContinue
if (-not $ollamaInstalled) {
    Write-Warning "⚠️  Ollama not found in PATH"
    Write-Info "   Download from: https://ollama.com"
    Write-Info "   Continuing installation..."
} else {
    Write-Success "✅ Ollama found"
}

# Check for llama3.1:8b model
if ($ollamaInstalled) {
    $models = & ollama list 2>$null
    if ($models -like "*llama3.1*") {
        Write-Success "✅ llama3.1:8b model found"
    } else {
        Write-Warning "⚠️  llama3.1:8b model not found"
        Write-Info "   Run: ollama pull llama3.1:8b"
    }
}

Write-Host ""

# Create installation directory
Write-Info "📁 Creating installation directory..."
if (Test-Path $InstallPath) {
    Write-Warning "⚠️  Installation directory exists. Upgrading..."
    Remove-Item "$InstallPath\*" -Recurse -Force -ErrorAction SilentlyContinue
} else {
    New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null
}

New-Item -ItemType Directory -Path "$InstallPath\bin" -Force | Out-Null
New-Item -ItemType Directory -Path "$InstallPath\gui" -Force | Out-Null
New-Item -ItemType Directory -Path "$InstallPath\docs" -Force | Out-Null
Write-Success "✅ Directories created"

# Copy files
Write-Info "📋 Copying files..."

$sourceDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# Copy binaries
Copy-Item "$sourceDir\target\release\sky.exe" "$InstallPath\bin\" -Force
Copy-Item "$sourceDir\..\skybridge\target\release\skybridge.exe" "$InstallPath\bin\" -Force
Write-Success "✅ Binaries copied"

# Copy scripts
Copy-Item "$sourceDir\start-with-skybridge.ps1" "$InstallPath\" -Force
Write-Success "✅ Launcher script copied"

# Copy documentation
$docs = @("README.md", "QUICK-START.md", "COMPLETE-FEATURE-REFERENCE.md", "LICENSE", "CHANGELOG.md")
foreach ($doc in $docs) {
    if (Test-Path "$sourceDir\$doc") {
        Copy-Item "$sourceDir\$doc" "$InstallPath\docs\" -Force
    }
}
Write-Success "✅ Documentation copied"

# Copy GUI (if exists)
if (Test-Path "$sourceDir\..\sky-code-gui\src-tauri\target\release\sky-code-gui.exe") {
    Copy-Item "$sourceDir\..\sky-code-gui\src-tauri\target\release\sky-code-gui.exe" "$InstallPath\gui\" -Force
    Write-Success "✅ GUI copied"
    $hasGUI = $true
} else {
    Write-Warning "⚠️  GUI not found (optional)"
    $hasGUI = $false
}

# Create launcher script in install dir
Write-Info "🚀 Creating launcher..."
$launcherContent = @"
@echo off
echo Starting Sky-Code...
cd /d "$InstallPath"
start powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$InstallPath\start-with-skybridge.ps1"
"@
$launcherContent | Out-File "$InstallPath\Sky-Code.bat" -Encoding ASCII -Force
Write-Success "✅ Launcher created"

# Add to PATH
Write-Info "🛤️  Adding to PATH..."
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallPath\bin*") {
    $newPath = "$userPath;$InstallPath\bin"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Success "✅ Added to PATH"
    Write-Warning "⚠️  Restart terminal for PATH changes to take effect"
} else {
    Write-Info "   Already in PATH"
}

# Create Start Menu entry
Write-Info "📌 Creating Start Menu shortcuts..."
$startMenuDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Sky-Code"
New-Item -ItemType Directory -Path $startMenuDir -Force | Out-Null

$WshShell = New-Object -ComObject WScript.Shell

# CLI Shortcut
$shortcut = $WshShell.CreateShortcut("$startMenuDir\Sky-Code CLI.lnk")
$shortcut.TargetPath = "$InstallPath\Sky-Code.bat"
$shortcut.WorkingDirectory = "$InstallPath"
$shortcut.Description = "Sky-Code Offline AI Agent (CLI)"
$shortcut.Save()

# GUI Shortcut (if exists)
if ($hasGUI) {
    $guiShortcut = $WshShell.CreateShortcut("$startMenuDir\Sky-Code GUI.lnk")
    $guiShortcut.TargetPath = "$InstallPath\gui\sky-code-gui.exe"
    $guiShortcut.WorkingDirectory = "$InstallPath"
    $guiShortcut.Description = "Sky-Code Offline AI Agent (GUI)"
    $guiShortcut.Save()
}

# Documentation shortcut
$docsShortcut = $WshShell.CreateShortcut("$startMenuDir\Documentation.lnk")
$docsShortcut.TargetPath = "$InstallPath\docs"
$docsShortcut.Description = "Sky-Code Documentation"
$docsShortcut.Save()

# Uninstaller shortcut
$uninstallShortcut = $WshShell.CreateShortcut("$startMenuDir\Uninstall Sky-Code.lnk")
$uninstallShortcut.TargetPath = "powershell.exe"
$uninstallShortcut.Arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$InstallPath\uninstall.ps1`""
$uninstallShortcut.Description = "Uninstall Sky-Code"
$uninstallShortcut.Save()

Write-Success "✅ Start Menu shortcuts created"

# Create desktop shortcut (ask user)
Write-Host ""
$createDesktop = Read-Host "Create Desktop shortcut? (Y/n)"
if ($createDesktop -ne "n" -and $createDesktop -ne "N") {
    $desktopShortcut = $WshShell.CreateShortcut("$env:USERPROFILE\Desktop\Sky-Code.lnk")
    $desktopShortcut.TargetPath = "$InstallPath\Sky-Code.bat"
    $desktopShortcut.WorkingDirectory = "$InstallPath"
    $desktopShortcut.Description = "Sky-Code Offline AI Agent"
    $desktopShortcut.Save()
    Write-Success "✅ Desktop shortcut created"
}

# Create uninstaller
Write-Info "🗑️  Creating uninstaller..."
$uninstallerScript = @"
# Sky-Code Uninstaller
Write-Host 'Uninstalling Sky-Code...' -ForegroundColor Cyan
`$scriptPath = Split-Path -Parent `$MyInvocation.MyCommand.Path
`$installerPath = Join-Path `$scriptPath 'install-sky-code.ps1'

if (Test-Path `$installerPath) {
    & `$installerPath -Uninstall -InstallPath '$InstallPath'
} else {
    Write-Error 'Installer script not found!'
}
"@
$uninstallerScript | Out-File "$InstallPath\uninstall.ps1" -Encoding UTF8 -Force

# Copy this installer to install dir for future uninstalls
Copy-Item $MyInvocation.MyCommand.Path "$InstallPath\install-sky-code.ps1" -Force
Write-Success "✅ Uninstaller created"

# Installation complete
Write-Host ""
Write-Host "╔════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║  🎉 Installation Complete!                 ║" -ForegroundColor Green
Write-Host "╚════════════════════════════════════════════╝" -ForegroundColor Green
Write-Host ""

Write-Info "📍 Installed to: $InstallPath"
Write-Info "📚 Docs: $InstallPath\docs"
Write-Host ""

Write-Info "🚀 Quick Start:"
Write-Host "   1. Start Menu → Sky-Code → Sky-Code CLI"
if ($hasGUI) {
    Write-Host "   2. Or: Start Menu → Sky-Code → Sky-Code GUI"
}
Write-Host "   3. Or run: sky.exe (after restarting terminal)"
Write-Host ""

Write-Info "📖 Documentation:"
Write-Host "   - Quick Start: $InstallPath\docs\QUICK-START.md"
Write-Host "   - Tool Reference: $InstallPath\docs\COMPLETE-FEATURE-REFERENCE.md"
Write-Host ""

Write-Info "🔧 Prerequisites (if not installed):"
Write-Host "   1. Download Ollama: https://ollama.com"
Write-Host "   2. Install model: ollama pull llama3.1:8b"
Write-Host ""

Write-Success "Ready to use! 🌌"
