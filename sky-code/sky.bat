@echo off
setlocal

REM ─── SkyCode Terminal Launcher ───────────────────────────────────────────────
REM  Usage: sky.bat
REM  Double-click OR run from any terminal.
REM ─────────────────────────────────────────────────────────────────────────────

set "SKY_DIR=%~dp0"
set "SKY_EXE_COMPILED=%SKY_DIR%target\release\sky.exe"
set "SKY_EXE_PREBUILT=%SKY_DIR%..\sky-code-npm\binaries\win-x64\sky.exe"
set "BRIDGE_EXE_COMPILED=%SKY_DIR%..\skybridge\target\release\skybridge.exe"
set "BRIDGE_EXE_PREBUILT=%SKY_DIR%..\sky-code-npm\binaries\win-x64\skybridge.exe"

REM ── 1. Resolve sky.exe (compiled > prebuilt > build from source) ──────────────
if exist "%SKY_EXE_COMPILED%" (
    set "SKY_EXE=%SKY_EXE_COMPILED%"
    echo [OK] Using compiled sky.exe
) else if exist "%SKY_EXE_PREBUILT%" (
    set "SKY_EXE=%SKY_EXE_PREBUILT%"
    echo [OK] Using pre-built sky.exe
) else (
    echo [!] sky.exe not found. Building from source...
    cd /d "%SKY_DIR%"
    cargo build --release -p sky
    if errorlevel 1 ( echo [!] Build failed. & pause & exit /b 1 )
    set "SKY_EXE=%SKY_EXE_COMPILED%"
)

REM ── 2. Resolve skybridge.exe ──────────────────────────────────────────────────
if exist "%BRIDGE_EXE_COMPILED%" (
    set "BRIDGE_EXE=%BRIDGE_EXE_COMPILED%"
) else if exist "%BRIDGE_EXE_PREBUILT%" (
    set "BRIDGE_EXE=%BRIDGE_EXE_PREBUILT%"
) else (
    echo [!] skybridge.exe not found. Build with: cd skybridge ^&^& cargo build --release
    pause & exit /b 1
)

REM ── 3. Set environment ────────────────────────────────────────────────────────
set "HOME=%USERPROFILE%"
set "FILANTHROPIC_BASE_URL=http://localhost:4000"
set "FILANTHROPIC_API_KEY=ollama"
set "FILANTHROPIC_TIMEOUT_SECS=180"

REM ── 4. Model selection: argument > env var > auto-detect ─────────────────────
if not "%~1"=="" (
    set "ANTHROPIC_MODEL=%~1"
    echo [OK] Using model (from arg): %~1
) else if not "%ANTHROPIC_MODEL%"=="" (
    echo [OK] Using model (from env): %ANTHROPIC_MODEL%
) else (
    for /f "delims=" %%M in ('powershell -NoProfile -Command "$m=try{(Invoke-WebRequest http://localhost:11434/api/tags -UseBasicParsing -TimeoutSec 3|ConvertFrom-Json).models|Where-Object{$_.name -ne ''}|Select-Object -First 1 -ExpandProperty name}catch{'llama3.2:1b'};if($m){$m}else{'llama3.2:1b'}"') do set "ANTHROPIC_MODEL=%%M"
    echo [OK] Using model (auto): %ANTHROPIC_MODEL%
)

REM ── 5. Kill wrong skybridge, start correct one ───────────────────────────────
powershell -NoProfile -Command ^
  "$correct='%BRIDGE_EXE%'.ToLower(); $proc=Get-Process skybridge -EA SilentlyContinue; if($proc){$running=($proc|Select-Object -First 1).Path.ToLower(); if($running -ne $correct){Write-Host '[!] Wrong skybridge running, restarting...'; $proc|Stop-Process -Force; Start-Sleep 1; Start-Process '%BRIDGE_EXE%' -WindowStyle Hidden; Start-Sleep 2}}else{Start-Process '%BRIDGE_EXE%' -WindowStyle Hidden; Start-Sleep 2}"

REM ── 6. Launch sky ─────────────────────────────────────────────────────────────
"%SKY_EXE%"
endlocal
