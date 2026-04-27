# Sky-Code Stack Test
# Verifies all components working end-to-end

Write-Host "🧪 Sky-Code Stack Test" -ForegroundColor Cyan
Write-Host "=" * 60 -ForegroundColor Gray
Write-Host ""

$allGreen = $true

# Test 1: Ollama
Write-Host "[1/4] Testing Ollama..." -ForegroundColor Yellow
try {
    $ollamaVersion = ollama --version 2>&1 | Select-Object -First 1
    if ($ollamaVersion -match "ollama version") {
        Write-Host "  ✅ Ollama: $ollamaVersion" -ForegroundColor Green
    } else {
        Write-Host "  ❌ Ollama not found" -ForegroundColor Red
        $allGreen = $false
    }
} catch {
    Write-Host "  ❌ Ollama error: $($_.Exception.Message)" -ForegroundColor Red
    $allGreen = $false
}

# Test 2: Ollama Model
Write-Host "[2/4] Testing Ollama model..." -ForegroundColor Yellow
try {
    $models = ollama list 2>&1 | Select-String "llama3.1:8b"
    if ($models) {
        Write-Host "  ✅ Model llama3.1:8b found" -ForegroundColor Green
    } else {
        Write-Host "  ❌ Model llama3.1:8b not pulled" -ForegroundColor Red
        Write-Host "     Run: ollama pull llama3.1:8b" -ForegroundColor Yellow
        $allGreen = $false
    }
} catch {
    Write-Host "  ❌ Model check error: $($_.Exception.Message)" -ForegroundColor Red
    $allGreen = $false
}

# Test 3: LiteLLM Proxy
Write-Host "[3/4] Testing LiteLLM proxy..." -ForegroundColor Yellow
try {
    $headers = @{ "Authorization" = "Bearer local-dev-key" }
    $response = Invoke-RestMethod -Uri "http://localhost:4000/v1/models" -Headers $headers -TimeoutSec 5 -ErrorAction Stop
    $modelCount = $response.data.Count
    Write-Host "  ✅ LiteLLM proxy responding ($modelCount models available)" -ForegroundColor Green
} catch {
    Write-Host "  ⚠️  LiteLLM proxy not running on port 4000" -ForegroundColor Red
    Write-Host "     Start with: litellm --config C:\ai-proxy\litellm-config.yaml --host 127.0.0.1 --port 4000" -ForegroundColor Yellow
    $allGreen = $false
}

# Test 4: Sky-Code Binary
Write-Host "[4/4] Testing Sky-Code binary..." -ForegroundColor Yellow
try {
    $skyPath = "C:\Projects\MyTests\ClaudeCode\sky-code\target\release\sky.exe"
    if (Test-Path $skyPath) {
        $size = [math]::Round((Get-Item $skyPath).Length / 1MB, 1)
        Write-Host "  ✅ Sky-Code binary found (${size}MB)" -ForegroundColor Green
    } else {
        Write-Host "  ❌ Sky-Code binary not found" -ForegroundColor Red
        Write-Host "     Build with: cargo build --release" -ForegroundColor Yellow
        $allGreen = $false
    }
} catch {
    Write-Host "  ❌ Binary check error: $($_.Exception.Message)" -ForegroundColor Red
    $allGreen = $false
}

Write-Host ""
Write-Host "=" * 60 -ForegroundColor Gray

if ($allGreen) {
    Write-Host "✅ ALL TESTS PASSED!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Your offline stack is ready! 🚀" -ForegroundColor Cyan
    Write-Host "Run: .\scripts\run\run-sky.ps1" -ForegroundColor White
} else {
    Write-Host "❌ SOME TESTS FAILED" -ForegroundColor Red
    Write-Host ""
    Write-Host "Fix the issues above, then re-run this test." -ForegroundColor Yellow
}

Write-Host ""
Read-Host "Press Enter to exit"
