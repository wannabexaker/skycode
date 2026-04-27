@echo off
setlocal

REM ─── SkyCode Web UI Launcher ──────────────────────────────────────────────────
REM  Usage: skyui.bat
REM  Opens the full chat interface in your browser.
REM ─────────────────────────────────────────────────────────────────────────────

set "SKY_DIR=%~dp0"
set "BRIDGE_EXE_COMPILED=%SKY_DIR%..\skybridge\target\release\skybridge.exe"
set "BRIDGE_EXE_PREBUILT=%SKY_DIR%..\sky-code-npm\binaries\win-x64\skybridge.exe"
set "NPM_SERVER=%SKY_DIR%..\sky-code-npm"
set "UI_PORT=4321"

REM ── 1. Set environment ────────────────────────────────────────────────────────
set "HOME=%USERPROFILE%"
set "FILANTHROPIC_BASE_URL=http://localhost:4000"
set "FILANTHROPIC_API_KEY=ollama"
set "FILANTHROPIC_TIMEOUT_SECS=180"

REM ── 2. Resolve skybridge.exe ──────────────────────────────────────────────────
if exist "%BRIDGE_EXE_COMPILED%" (
    set "BRIDGE_EXE=%BRIDGE_EXE_COMPILED%"
) else if exist "%BRIDGE_EXE_PREBUILT%" (
    set "BRIDGE_EXE=%BRIDGE_EXE_PREBUILT%"
) else (
    echo [!] skybridge.exe not found. Build with: cd skybridge ^&^& cargo build --release
    pause & exit /b 1
)

REM ── 3. Auto-detect model ──────────────────────────────────────────────────────
for /f "delims=" %%M in ('powershell -NoProfile -Command "$m=try{(Invoke-WebRequest http://localhost:11434/api/tags -UseBasicParsing -TimeoutSec 3|ConvertFrom-Json).models|Where-Object{$_.name -ne ''}|Select-Object -First 1 -ExpandProperty name}catch{'llama3.2:1b'};if($m){$m}else{'llama3.2:1b'}"') do set "ANTHROPIC_MODEL=%%M"
echo [OK] Using model: %ANTHROPIC_MODEL%

REM ── 4. Kill wrong skybridge, start correct one ───────────────────────────────
powershell -NoProfile -Command ^
  "$correct='%BRIDGE_EXE%'.ToLower(); $proc=Get-Process skybridge -EA SilentlyContinue; if($proc){$running=($proc|Select-Object -First 1).Path.ToLower(); if($running -ne $correct){Write-Host '[!] Wrong skybridge running, restarting...'; $proc|Stop-Process -Force; Start-Sleep 1; Start-Process '%BRIDGE_EXE%' -WindowStyle Hidden; Start-Sleep 2}}else{Start-Process '%BRIDGE_EXE%' -WindowStyle Hidden; Start-Sleep 2}"

REM ── 5. Start web server and open browser ──────────────────────────────────────
echo.
echo  Starting SkyCode Web UI on http://localhost:%UI_PORT%
echo  Press Ctrl+C to stop.
echo.

if exist "%NPM_SERVER%\bin\skycode.js" (
    node "%NPM_SERVER%\bin\skycode.js" serve --port %UI_PORT%
) else (
    echo [!] Web server not found at %NPM_SERVER%
    echo     Ensure sky-code-npm folder is present.
    pause & exit /b 1
)

endlocal
