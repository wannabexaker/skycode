# Install Desktop Shortcut for Sky-Code
# Run this script ONCE to create a desktop shortcut

$DesktopPath = [Environment]::GetFolderPath("Desktop")
$ShortcutPath = Join-Path $DesktopPath "Sky-Code.lnk"

# Get the sky-code root folder (2 levels up from scripts/install/)
$ScriptDir = $PSScriptRoot
$RootDir = Split-Path (Split-Path $ScriptDir -Parent) -Parent

$TargetPath = Join-Path $RootDir "scripts\run\Sky-Code.bat"
$IconPath = Join-Path $RootDir "target\release\sky.exe"

# Verify files exist
if (-not (Test-Path $TargetPath)) {
    Write-Host "Error: Sky-Code.bat not found at $TargetPath" -ForegroundColor Red
    exit 1
}

if (-not (Test-Path $IconPath)) {
    Write-Host "Error: sky.exe not found at $IconPath" -ForegroundColor Red
    exit 1
}

# Create shortcut
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut($ShortcutPath)
$Shortcut.TargetPath = $TargetPath
$Shortcut.WorkingDirectory = $RootDir
$Shortcut.IconLocation = $IconPath
$Shortcut.Description = "Sky-Code - Offline AI Agent"
$Shortcut.Save()

Write-Host ""
Write-Host "Desktop shortcut created successfully!" -ForegroundColor Green
Write-Host "Location: $ShortcutPath" -ForegroundColor Cyan
Write-Host ""
Write-Host "Double-click the Sky-Code icon on your Desktop to launch!" -ForegroundColor Yellow
Write-Host ""
