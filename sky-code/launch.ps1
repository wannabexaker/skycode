# SkyCode Launcher - Double-click to start!
# This runs setup-auto.ps1 and then opens SkyCode

Write-Host ""
Write-Host "========================================================================" -ForegroundColor Cyan
Write-Host " SkyCode Launcher" -ForegroundColor Green
Write-Host "========================================================================" -ForegroundColor Cyan
Write-Host ""

# Change to script directory
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

# Run auto-setup
Write-Host "Running auto-setup..." -ForegroundColor Yellow
.\setup-auto.ps1

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "========================================================================" -ForegroundColor Cyan
    Write-Host " SkyCode is ready! What would you like to do?" -ForegroundColor Green
    Write-Host "========================================================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "1. Run a quick test prompt" -ForegroundColor White
    Write-Host "2. Start interactive mode (REPL)" -ForegroundColor White
    Write-Host "3. View doctor report" -ForegroundColor White
    Write-Host "4. View configuration" -ForegroundColor White
    Write-Host "5. Exit" -ForegroundColor White
    Write-Host ""
    
    $choice = Read-Host "Enter choice (1-5)"
    
    switch ($choice) {
        "1" {
            Write-Host ""
            Write-Host "Running test: 'What is 5 + 3?'" -ForegroundColor Cyan
            .\target\release\sky.exe prompt "What is 5 + 3? Answer only the number."
        }
        "2" {
            Write-Host ""
            Write-Host "Starting interactive mode..." -ForegroundColor Cyan
            Write-Host "Type your prompts and press Enter. Type 'exit' to quit." -ForegroundColor Gray
            .\target\release\sky.exe repl
        }
        "3" {
            Write-Host ""
            .\target\release\sky.exe doctor
        }
        "4" {
            Write-Host ""
            .\target\release\sky.exe config list
        }
        "5" {
            Write-Host "Goodbye!" -ForegroundColor Cyan
        }
        default {
            Write-Host "Invalid choice. Exiting." -ForegroundColor Yellow
        }
    }
} else {
    Write-Host ""
    Write-Host "Setup failed. Please check the errors above." -ForegroundColor Red
    Write-Host "Press any key to exit..."
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
}

Write-Host ""
