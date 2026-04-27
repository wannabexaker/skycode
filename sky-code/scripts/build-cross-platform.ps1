# Multi-Platform Build Script
# Builds Sky-Code for all platforms using cross-compilation
# 100% FREE - uses Docker containers, no macOS runners needed

Write-Host "`n🔨 Sky-Code Multi-Platform Build`n" -ForegroundColor Cyan

# Check Docker
Write-Host "Checking Docker..." -ForegroundColor Yellow
try {
    docker version | Out-Null
    Write-Host "✓ Docker is running" -ForegroundColor Green
} catch {
    Write-Host "✗ Docker is not running. Starting Docker Desktop..." -ForegroundColor Red
    Start-Process "C:\Program Files\Docker\Docker\Docker Desktop.exe"
    Write-Host "Waiting for Docker to start (30 seconds)..." -ForegroundColor Yellow
    Start-Sleep -Seconds 30
}

# Build configuration
$targets = @(
    @{Name="Linux x64 (musl)"; Target="x86_64-unknown-linux-musl"; Binary="sky"; Output="sky-linux-x64"},
    @{Name="Windows x64 (GNU)"; Target="x86_64-pc-windows-gnu"; Binary="sky.exe"; Output="sky-windows-x64.exe"},
    @{Name="macOS Intel"; Target="x86_64-apple-darwin"; Binary="sky"; Output="sky-macos-x64"},
    @{Name="macOS ARM"; Target="aarch64-apple-darwin"; Binary="sky"; Output="sky-macos-arm64"}
)

$outputDir = ".\binaries-multi-platform"
if (!(Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir | Out-Null
}

Write-Host "`nBuilding for $($targets.Count) platforms...`n" -ForegroundColor Cyan

foreach ($target in $targets) {
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
    Write-Host "Building: $($target.Name)" -ForegroundColor Cyan
    Write-Host "Target:   $($target.Target)" -ForegroundColor Gray
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`n" -ForegroundColor Gray
    
    $startTime = Get-Date
    
    try {
        # Build with cross
        cross build --release --target $target.Target -p sky 2>&1 | Out-Null
        
        if ($LASTEXITCODE -eq 0) {
            $elapsed = (Get-Date) - $startTime
            Write-Host "✓ Build succeeded in $([math]::Round($elapsed.TotalSeconds, 1))s" -ForegroundColor Green
            
            # Copy binary to output dir
            $sourcePath = ".\target\$($target.Target)\release\$($target.Binary)"
            $destPath = Join-Path $outputDir $target.Output
            
            if (Test-Path $sourcePath) {
                Copy-Item $sourcePath $destPath -Force
                $size = (Get-Item $destPath).Length / 1MB
                Write-Host "  Binary: $destPath ($([math]::Round($size, 1)) MB)" -ForegroundColor Gray
            } else {
                Write-Host "⚠ Binary not found at $sourcePath" -ForegroundColor Yellow
            }
        } else {
            Write-Host "✗ Build failed (exit code $LASTEXITCODE)" -ForegroundColor Red
        }
    } catch {
        Write-Host "✗ Build error: $_" -ForegroundColor Red
    }
    
    Write-Host ""
}

# SkyBridge builds (same process)
Write-Host "`n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
Write-Host "Building SkyBridge..." -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`n" -ForegroundColor Gray

cd ..\skybridge

foreach ($target in $targets) {
    Write-Host "  - $($target.Name)..." -ForegroundColor Gray -NoNewline
    
    $binaryName = if ($target.Binary -eq "sky.exe") { "skybridge.exe" } else { "skybridge" }
    $outputName = $target.Output -replace "sky", "skybridge"
    
    try {
        cross build --release --target $target.Target 2>&1 | Out-Null
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host " ✓" -ForegroundColor Green
            
            $sourcePath = ".\target\$($target.Target)\release\$binaryName"
            $destPath = Join-Path ..\sky-code\$outputDir $outputName
            
            if (Test-Path $sourcePath) {
                Copy-Item $sourcePath $destPath -Force
            }
        } else {
            Write-Host " ✗" -ForegroundColor Red
        }
    } catch {
        Write-Host " ✗ Error" -ForegroundColor Red
    }
}

cd ..\sky-code

# Summary
Write-Host "`n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
Write-Host "Build Summary" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`n" -ForegroundColor Gray

$binaries = Get-ChildItem $outputDir -File
$totalSize = ($binaries | Measure-Object -Property Length -Sum).Sum / 1MB

Write-Host "Output directory: $outputDir" -ForegroundColor Gray
Write-Host "Binaries created: $($binaries.Count)" -ForegroundColor Green
Write-Host "Total size:       $([math]::Round($totalSize, 1)) MB`n" -ForegroundColor Green

foreach ($binary in $binaries | Sort-Object Name) {
    $size = $binary.Length / 1MB
    Write-Host "  $($binary.Name.PadRight(30)) $([math]::Round($size, 1)) MB" -ForegroundColor Gray
}

Write-Host "`n✅ Multi-platform build complete!`n" -ForegroundColor Green
