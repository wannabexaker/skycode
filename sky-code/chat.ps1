# Quick Chat Launcher for SkyCode
# Usage: .\chat.ps1
# Just type and talk with the AI!

$ErrorActionPreference = "Stop"

# Set environment variables
$env:HOME = $env:USERPROFILE
$env:FILANTHROPIC_BASE_URL = 'http://localhost:4000'
$env:FILANTHROPIC_TIMEOUT_SECS = '180'
$env:ANTHROPIC_MODEL = 'llama3.2:1b'

Write-Host "`n=== SkyCode Interactive Chat ===" -ForegroundColor Cyan
Write-Host "Model: llama3.2:1b (fast, offline)" -ForegroundColor Gray
Write-Host "Type your messages directly (no command needed)" -ForegroundColor Gray
Write-Host "Slash commands: /help /status /model /clear" -ForegroundColor Gray
Write-Host "Exit: Ctrl+D or Ctrl+C`n" -ForegroundColor Gray

# Change to sky-code directory and start REPL
cd C:\Projects\MyTests\ClaudeCode\sky-code
.\target\release\sky.exe
