# Antigravity Stop hook -> MovingText notch webhook.
# Registered in .agents/hooks.json (or ~/.gemini/config/hooks.json) under the Stop event.
# Antigravity pipes camelCase event JSON to stdin and expects JSON back on stdout.

$ErrorActionPreference = 'SilentlyContinue'
[System.Net.ServicePointManager]::Expect100Continue = $false

$raw = [Console]::In.ReadToEnd()
$json = $null
if ($raw) {
    try { $json = $raw | ConvertFrom-Json } catch { $json = $null }
}

$msg = $json.message
if (-not $msg) { $msg = 'Agent finished' }

$payload = @{
    app      = 'Antigravity'
    title    = 'Antigravity'
    body     = $msg
    level    = 'success'
    duration = 4.5
} | ConvertTo-Json -Compress

try {
    Invoke-RestMethod -Uri 'http://127.0.0.1:18923/' -Method Post -Body $payload -ContentType 'application/json' -TimeoutSec 2 | Out-Null
} catch {
    # Notch app not running or webhook disabled - fail silently, never block Antigravity.
}

# Antigravity hooks must return JSON on stdout, even when there's nothing to report.
Write-Output '{}'
exit 0
