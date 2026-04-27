@echo off
REM Sky-Code Desktop Launcher
REM Double-click this file to start Sky-Code

echo.
echo ====================================
echo    Sky-Code Launcher v1.0
echo ====================================
echo.

REM Change to Sky-Code root directory (2 levels up from scripts/run/)
cd /d "%~dp0\..\..\"

REM Check if binaries exist
if not exist "target\release\sky.exe" (
    echo [ERROR] sky.exe not found!
    echo Please build the project first:
    echo    cargo build --release -p sky
    pause
    exit /b 1
)

if not exist "..\skybridge\target\release\skybridge.exe" (
    echo [ERROR] skybridge.exe not found!
    echo Please build SkyBridge first:
    echo    cd ..\skybridge
    echo    cargo build --release
    pause
    exit /b 1
)

REM Launch with PowerShell script
echo Starting Sky-Code with SkyBridge...
echo.
powershell.exe -ExecutionPolicy Bypass -File "scripts\run\start-with-skybridge.ps1"

pause
