@echo off
REM ===============================================
REM  SkyCode Health Check Script
REM  Ελέγχει αν όλα τα components είναι OK
REM ===============================================

echo.
echo ═══════════════════════════════════════════
echo   SkyCode Health Check
echo ═══════════════════════════════════════════
echo.

REM 1. Check Ollama
echo [1/5] Checking Ollama...
curl -s http://localhost:11434/api/tags >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ FAIL - Ollama not running on port 11434
    echo.
    echo    Fix: Run "ollama serve" in another terminal
    echo.
    goto :end
) else (
    echo ✅ OK - Ollama running
)

REM 2. Check sky.exe
echo [2/5] Checking sky.exe...
if not exist "target\release\sky.exe" (
    echo ❌ FAIL - sky.exe not found
    echo.
    echo    Fix: Run "cargo build --release"
    echo.
    goto :end
) else (
    echo ✅ OK - sky.exe exists
)

REM 3. Check skybridge.exe
echo [3/5] Checking skybridge.exe...
if not exist "..\skybridge\target\release\skybridge.exe" (
    echo ❌ FAIL - skybridge.exe not found
    echo.
    echo    Fix: cd ..\skybridge ; cargo build --release
    echo.
    goto :end
) else (
    echo ✅ OK - skybridge.exe exists
)

REM 4. Check Node.js (for web UI)
echo [4/5] Checking Node.js...
node --version >nul 2>&1
if %errorlevel% neq 0 (
    echo ⚠️  WARN - Node.js not found (required for web UI only)
    echo.
    echo    Terminal mode WILL work, but skyui.bat won't
    echo.
) else (
    echo ✅ OK - Node.js installed
)

REM 5. Check if Ollama has models
echo [5/5] Checking Ollama models...
for /f "tokens=*" %%i in ('curl -s http://localhost:11434/api/tags ^| findstr "name"') do (
    set MODEL_CHECK=%%i
)
if not defined MODEL_CHECK (
    echo ❌ FAIL - No models found in Ollama
    echo.
    echo    Fix: Run "ollama pull llama3.2:1b"
    echo.
    goto :end
) else (
    echo ✅ OK - Models found
)

echo.
echo ═══════════════════════════════════════════
echo   ✅ All checks passed!
echo ═══════════════════════════════════════════
echo.
echo Ready to use:
echo   • Terminal: .\sky.bat
echo   • Web UI:   .\skyui.bat
echo.

:end
pause
