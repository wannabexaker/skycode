# Start LiteLLM Proxy Helper
# Launches LiteLLM and returns the port it's running on

param(
    [int]$MaxAttempts = 5,
    [int]$WaitSeconds = 3
)

Write-Host "Starting LiteLLM proxy..." -ForegroundColor Cyan

# Start LiteLLM in background job
$job = Start-Job -ScriptBlock {
    litellm --model ollama/llama3.1:8b --api_base http://localhost:11434 2>&1
}

# Wait for proxy to start and capture the port
$port = $null
$attempt = 0

while ($attempt -lt $MaxAttempts -and -not $port) {
    Start-Sleep -Seconds $WaitSeconds
    $output = Receive-Job -Job $job
    
    # Parse port from output: "Uvicorn running on http://127.0.0.1:XXXXX"
    if ($output -match 'Uvicorn running on http://.*:(\d+)') {
        $port = $matches[1]
        Write-Host "LiteLLM proxy started on port $port" -ForegroundColor Green
        break
    }
    
    $attempt++
}

if (-not $port) {
    Write-Host "ERROR: Could not detect LiteLLM proxy port" -ForegroundColor Red
    Stop-Job -Job $job
    Remove-Job -Job $job
    exit 1
}

# Return port number and job ID
@{
    Port = $port
    JobId = $job.Id
} | ConvertTo-Json
