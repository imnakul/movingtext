# Codex CLI notify hook -> MovingText notch webhook.
# Registered as `notify = [...]` in ~/.codex/config.toml.
# Codex appends the event JSON as the LAST argv argument (stdin/stdout/stderr are closed),
# using kebab-case fields: type, thread-id, turn-id, cwd, client, input-messages, last-assistant-message.

param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Args
)

$ErrorActionPreference = 'SilentlyContinue'
[System.Net.ServicePointManager]::Expect100Continue = $false

if (-not $Args -or $Args.Count -eq 0) {
    exit 0
}

$raw = $Args[$Args.Count - 1]
$json = $null
try { $json = $raw | ConvertFrom-Json } catch { $json = $null }
if (-not $json) { exit 0 }

$msg = $json.'last-assistant-message'
if (-not $msg) { $msg = 'Turn complete' }
if ($msg.Length -gt 220) { $msg = $msg.Substring(0, 220) + '...' }

$payload = @{
    app      = 'Codex'
    title    = 'Codex CLI'
    body     = $msg
    level    = 'success'
    duration = 4.5
} | ConvertTo-Json -Compress

try {
    Invoke-RestMethod -Uri 'http://127.0.0.1:18923/' -Method Post -Body $payload -ContentType 'application/json' -TimeoutSec 2 | Out-Null
} catch {
    # Notch app not running or webhook disabled - fail silently, never block Codex.
}

exit 0
