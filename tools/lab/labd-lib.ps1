<#
.SYNOPSIS
    Pure helpers for tools/lab/daemon.ps1, kept apart so
    tools/lab/test-labd-lib.ps1 can test them without touching the task,
    the processes or %LOCALAPPDATA%. Dot-source it; it defines functions only.
    Runs under Windows PowerShell 5.1 (the scheduled task) and pwsh 7.
#>

# UTF-8 without a BOM, for files a person edits and the daemon reads back.
# Windows PowerShell 5.1's -Encoding utf8 writes a BOM and its ascii
# replaces every non-ASCII character with '?', so go through .NET.
$script:Utf8NoBom = New-Object System.Text.UTF8Encoding $false

function Write-LabdEnvFile([string]$Path, [string[]]$Lines) {
    [System.IO.File]::WriteAllLines($Path, $Lines, $script:Utf8NoBom)
}

# labd.env as an ordered KEY -> VALUE map: comments and blank lines skipped,
# the value is everything after the first '=' (it may itself contain '=').
function Read-LabdEnvFile([string]$Path) {
    $map = [ordered]@{}
    if (-not (Test-Path -LiteralPath $Path)) { return $map }
    foreach ($line in [System.IO.File]::ReadAllLines($Path, $script:Utf8NoBom)) {
        $l = $line.Trim()
        # A file someone saved with a BOM still parses.
        $l = $l.TrimStart([char]0xFEFF)
        if (-not $l -or $l.StartsWith('#')) { continue }
        $i = $l.IndexOf('=')
        if ($i -lt 1) { continue }
        $map[$l.Substring(0, $i).Trim()] = $l.Substring($i + 1)
    }
    return $map
}

# The KEY=VALUE lines for the stdio cimmeria-lab entry's env block in a
# .mcp.json text, or $null when it has none.
function Get-McpEnvLines([string]$McpJsonText) {
    $json = $McpJsonText | ConvertFrom-Json
    $entry = $json.mcpServers.'cimmeria-lab'
    if (-not $entry -or -not $entry.env) { return $null }
    $out = @()
    foreach ($p in $entry.env.PSObject.Properties) { $out += '{0}={1}' -f $p.Name, $p.Value }
    return , $out
}

# Whether $Process is the daemon that wrote $Info (labd.pid). A pid alone
# is not enough: a daemon that died without cleaning up leaves labd.pid
# behind, and Windows reuses pids, possibly for another cimmeria-lab (a
# stdio supervisor of some session). The daemon runs only from its own
# copy ($DaemonExe), and it writes labd.pid moments after it starts, so the
# process must run that file and have started shortly before started_at.
function Test-DaemonProcess($Info, $Process, [string]$DaemonExe) {
    if (-not $Info -or -not $Process) { return $false }
    if (-not $Process.Path -or -not $DaemonExe) { return $false }
    $a = [System.IO.Path]::GetFullPath($Process.Path)
    $b = [System.IO.Path]::GetFullPath($DaemonExe)
    if (-not [string]::Equals($a, $b, [System.StringComparison]::OrdinalIgnoreCase)) { return $false }
    if (-not $Info.started_at -or -not $Process.StartTime) { return $false }
    try {
        $written = [DateTimeOffset]::Parse([string]$Info.started_at).UtcDateTime
    } catch { return $false }
    $started = ([datetime]$Process.StartTime).ToUniversalTime()
    $lead = ($written - $started).TotalSeconds
    # Started before it wrote the file (clock jitter allowed), and not long
    # before: binding and opening the log take seconds, not minutes.
    return ($lead -ge -2 -and $lead -le 120)
}

# The address to probe: the running daemon's recorded bind, else $Default.
function Get-DaemonBind($Info, [string]$Default) {
    if ($Info -and $Info.bind) { return [string]$Info.bind }
    return $Default
}
