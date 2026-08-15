# Claude Code statusLine hook -> Venu notch webhook.
# Registered in ~/.claude/settings.json under statusLine.command.
# Claude Code pipes the full statusLine JSON schema to this script's stdin after
# every turn, and prints whatever this script writes to stdout as the terminal
# status line.

$ErrorActionPreference = 'SilentlyContinue'
[System.Net.ServicePointManager]::Expect100Continue = $false

$raw = [Console]::In.ReadToEnd()
$json = $null
if ($raw) {
    try { $json = $raw | ConvertFrom-Json } catch { $json = $null }
}

$contextPct = $null
if ($json.context_window -and $null -ne $json.context_window.used_percentage) {
    $contextPct = [math]::Round([double]$json.context_window.used_percentage, 1)
}

$cost = $null
if ($json.cost -and $null -ne $json.cost.total_cost_usd) {
    $cost = [math]::Round([double]$json.cost.total_cost_usd, 2)
}

$rate5h = $null
$rate5hResets = $null
if ($json.rate_limits -and $json.rate_limits.five_hour) {
    if ($null -ne $json.rate_limits.five_hour.used_percentage) {
        $rate5h = [math]::Round([double]$json.rate_limits.five_hour.used_percentage, 1)
    }
    $rate5hResets = $json.rate_limits.five_hour.resets_at
}

$rate7d = $null
$rate7dResets = $null
if ($json.rate_limits -and $json.rate_limits.seven_day) {
    if ($null -ne $json.rate_limits.seven_day.used_percentage) {
        $rate7d = [math]::Round([double]$json.rate_limits.seven_day.used_percentage, 1)
    }
    $rate7dResets = $json.rate_limits.seven_day.resets_at
}

$payload = @{
    context_used_pct   = $contextPct
    cost_usd           = $cost
    rate_5h_pct        = $rate5h
    rate_5h_resets_at  = $rate5hResets
    rate_7d_pct        = $rate7d
    rate_7d_resets_at  = $rate7dResets
} | ConvertTo-Json -Compress

try {
    Invoke-RestMethod -Uri 'http://127.0.0.1:18923/usage' -Method Post -Body $payload -ContentType 'application/json' -TimeoutSec 2 | Out-Null
} catch {
    # Notch app not running or webhook disabled - fail silently, never block Claude Code.
}

# Print a compact status line back to Claude Code's own terminal status bar.
$parts = @()
if ($null -ne $contextPct) { $parts += "ctx $contextPct%" }
if ($null -ne $cost) { $parts += "`$$cost" }
if ($null -ne $rate5h) { $parts += "5h $rate5h%" }
Write-Output ($parts -join '  |  ')

exit 0
