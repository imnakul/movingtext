# Claude Code Notification/Stop hook -> Venu notch webhook.
# Registered in ~/.claude/settings.json under hooks.Notification and hooks.Stop.
# Claude Code pipes the event JSON to this script's stdin.

$ErrorActionPreference = 'SilentlyContinue'
[System.Net.ServicePointManager]::Expect100Continue = $false

$raw = [Console]::In.ReadToEnd()
$json = $null
if ($raw) {
    try { $json = $raw | ConvertFrom-Json } catch { $json = $null }
}

$eventName = $json.hook_event_name
$msg = $json.message
if (-not $msg) {
    if ($eventName -eq 'Stop') {
        $msg = 'Finished responding'
    } else {
        $msg = 'Needs your attention'
    }
}

$level = if ($eventName -eq 'Stop') { 'success' } else { 'action' }

$payload = @{
    app      = 'Claude'
    title    = 'Claude Code'
    body     = $msg
    level    = $level
    duration = 4.5
} | ConvertTo-Json -Compress

try {
    Invoke-RestMethod -Uri 'http://127.0.0.1:18923/' -Method Post -Body $payload -ContentType 'application/json' -TimeoutSec 2 | Out-Null
} catch {
    # Notch app not running or webhook disabled - fail silently, never block Claude Code.
}

exit 0
