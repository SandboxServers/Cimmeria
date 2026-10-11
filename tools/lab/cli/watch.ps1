#requires -Version 7
<#
.SYNOPSIS
    Watch a running lab uat batch, read-only: lab watch [-Batch <folder>] [-Once] [-Json] [-IntervalSeconds 2].

.DESCRIPTION
    Shows the newest lab uat batch (the batch-* folder under the UAT root with
    the greatest name), or the -Batch folder, redrawn every -IntervalSeconds:

    - the batch line: its name, section, state (live, done, interrupted or
      unknown), runs done of planned, passed, failed and how many failures
      match a known issue; then the last failure and its issue tag.
    - one row per lab instance from the daemon's /status (per plan lane when
      the daemon is down): CLIENT pid, LEASE owner and purpose, RUN (the
      lane's run of runs per lease, "braked" once its brake stopped it), ROW
      (the row under way and its seconds, from /status `uat`), SHOT (the age
      of the run's newest screenshot), and under it the lane's last progress
      line as lab uat printed it.
    - a footer: SGW.exe processes no instance owns (#1342?), client
      relaunches and login retries in labd.log since the batch started.

    Read-only (D-LW3): it takes no lease, asks the daemon only for GET
    /status, and every string it prints goes through Hide-LeaseIds.

    -Once prints one snapshot and exits; -Json prints it as one JSON line and
    implies -Once. With output redirected, the live view prints one -Once
    snapshot instead of looping. The live view uses the terminal's alternate
    screen; Ctrl+C ends it and restores the screen.

    Exit codes: 0 a snapshot was printed, or the live view ended with Ctrl+C;
    1 the daemon is down and there is no batch to show; 2 usage (an
    -IntervalSeconds outside 1 to 60, a -Batch folder with neither plan.json
    nor batch.jsonl, or a -StatusFile that does not exist). Under -Json a
    usage error prints {"ok":false,"exit":2,"error":...}.

    Test seams, not for normal use: -StatusFile <path> reads the /status JSON
    from a file instead of the daemon (a file without the daemon's shape, such
    as `null`, reads as the daemon down), and -LogFile <path> reads that file
    instead of labd.log. Both also read a saved snapshot.

.EXAMPLE
    lab watch
    lab watch -Once
    lab watch -Once -Json
    lab watch -Batch $env:LOCALAPPDATA\cimmeria-lab\uat-runs\batch-20261011-013906 -Once
#>
[CmdletBinding()]
param(
    [string]$Batch,
    [switch]$Once,
    [switch]$Json,
    [int]$IntervalSeconds = 2,
    # Test seam: the /status JSON from this file instead of the daemon.
    [string]$StatusFile,
    # Test seam: this file instead of labd.log.
    [string]$LogFile
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')
. (Join-Path $PSScriptRoot 'uat-lib.ps1')
. (Join-Path $PSScriptRoot 'watch-lib.ps1')

# A usage failure, in lab uat's shape: plain text on stderr, or under -Json one
# object on stdout. Returns the exit code.
function Stop-Watch([int]$Code, [string]$Message) {
    if ($Json) {
        [Console]::Out.WriteLine(([ordered]@{ ok = $false; exit = $Code; error = $Message } | ConvertTo-Json -Compress))
    } else {
        [Console]::Error.WriteLine($Message)
    }
    return $Code
}

# A saved /status, held to the shape Get-LabStatus accepts: anything else (null,
# not JSON, another object) reads as the daemon down, as it would live.
function Read-LabWatchStatusFile([string]$Path) {
    try { $s = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json } catch { return $null }
    if ($s -isnot [pscustomobject]) { return $null }
    $names = $s.PSObject.Properties.Name
    if ($names -notcontains 'daemon' -or $names -notcontains 'instances' -or $null -eq $s.daemon) { return $null }
    return $s
}

# Everything one snapshot reads: /status, the batch folder, the labd.log tail,
# the SGW pids and the time. The only daemon call is Get-LabStatus (GET /status).
function Get-LabWatchInputs {
    $status = if ($StatusFile) { Read-LabWatchStatusFile $StatusFile } else { Get-LabStatus }
    # The newest batch is looked up every tick: a batch started after the
    # watch began replaces the one on screen.
    $dir = if ($Batch) { $Batch } else { Find-LabWatchBatch (Get-UatRoot) }
    $logPath = if ($LogFile) { $LogFile } else { Join-Path (Get-LabHome) 'labd.log' }
    # Assigned, not taken from an if-expression, which would unroll an empty
    # array to $null and a one-line log to a bare string.
    $lines = @()
    if (Test-Path -LiteralPath $logPath -PathType Leaf) { $lines = @(Get-Content -LiteralPath $logPath -Tail 4000) }
    $sgw = @(Get-Process SGW -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })
    return @{ Status = $status; BatchDir = $dir; LogLines = $lines; SgwPids = $sgw; Now = (Get-Date).ToUniversalTime() }
}

function Get-LabWatchFrame {
    $in = Get-LabWatchInputs
    return Get-LabWatchSnapshot -Status $in.Status -BatchDir $in.BatchDir -LogLines $in.LogLines -SgwPids $in.SgwPids -Now $in.Now
}

# The terminal's width less one (a full-width line wraps on some hosts), or
# 120 when there is no console window to measure.
function Get-LabWatchWidth {
    if ([Console]::IsOutputRedirected) { return 120 }
    $w = 0
    try { $w = [Console]::WindowWidth } catch { $w = 0 }
    if ($w -lt 41) { return 120 }
    return $w - 1
}

# Waits one interval. Returns $true when Ctrl+C was pressed: with Ctrl+C read
# as input the view ends through its finally and exits 0, rather than being
# torn down mid-frame by a stopped pipeline.
function Wait-LabWatchTick([bool]$ReadKeys) {
    $until = [datetime]::UtcNow.AddSeconds($IntervalSeconds)
    while ([datetime]::UtcNow -lt $until) {
        if ($ReadKeys -and [Console]::KeyAvailable) {
            $k = [Console]::ReadKey($true)
            if ($k.Key -eq [ConsoleKey]::C -and ($k.Modifiers -band [ConsoleModifiers]::Control)) { return $true }
        }
        Start-Sleep -Milliseconds 100
    }
    return $false
}

# The live view (D-LW2): the alternate screen with the cursor hidden; each tick
# homes the cursor, writes every line followed by erase-to-end-of-line, then
# erases below, so a shorter frame leaves nothing behind. The finally puts the
# cursor and the main screen back on Ctrl+C and on an error alike.
function Show-LabWatchLive {
    $e = [char]27
    $readKeys = $false; $savedCtrlC = $false
    if (-not [Console]::IsInputRedirected) {
        try { $savedCtrlC = [Console]::TreatControlCAsInput; [Console]::TreatControlCAsInput = $true; $readKeys = $true } catch { $readKeys = $false }
    }
    [Console]::Out.Write("$e[?1049h$e[?25l")
    try {
        while ($true) {
            $width = Get-LabWatchWidth
            $lines = [System.Collections.Generic.List[string]]::new()
            $lines.Add("lab watch  $((Get-Date).ToString('HH:mm:ss'))  every ${IntervalSeconds}s  Ctrl+C to quit")
            try {
                foreach ($l in @(Format-LabWatchText (Get-LabWatchFrame) $width)) { $lines.Add($l) }
            } catch {
                # One bad tick (a file mid-write, a daemon restart) is shown, not fatal.
                $lines.Add("error: $(Hide-LeaseIds (("$($_.Exception.Message)" -split "`n")[0]))")
            }
            $frame = [System.Text.StringBuilder]::new("$e[H")
            foreach ($l in $lines) { [void]$frame.Append((Limit-UatText $l $width)).Append("$e[K`r`n") }
            [void]$frame.Append("$e[J")
            [Console]::Out.Write($frame.ToString())
            if (Wait-LabWatchTick $readKeys) { break }
        }
    } finally {
        [Console]::Out.Write("$e[?25h$e[?1049l")
        if ($readKeys) { [Console]::TreatControlCAsInput = $savedCtrlC }
    }
}

function Invoke-LabWatch {
    # Usage checks come first, so they exit 2 whatever the daemon's state.
    if ($IntervalSeconds -lt 1 -or $IntervalSeconds -gt 60) {
        return Stop-Watch 2 "-IntervalSeconds must be 1 to 60 (got $IntervalSeconds)"
    }
    if ($Batch) {
        if (-not (Test-Path -LiteralPath $Batch -PathType Container)) { return Stop-Watch 2 "no batch folder: $Batch" }
        if (-not (Test-Path -LiteralPath (Join-Path $Batch 'plan.json')) -and -not (Test-Path -LiteralPath (Join-Path $Batch 'batch.jsonl'))) {
            return Stop-Watch 2 "not a lab uat batch (no plan.json or batch.jsonl): $Batch"
        }
        $script:Batch = (Resolve-Path -LiteralPath $Batch).ProviderPath
    }
    if ($StatusFile -and -not (Test-Path -LiteralPath $StatusFile -PathType Leaf)) {
        return Stop-Watch 2 "no status file: $StatusFile"
    }

    # A redirected stdout gets one snapshot: escape codes and an endless loop
    # into a file or a pipe help nobody.
    if ($Once -or $Json -or [Console]::IsOutputRedirected) {
        $snap = Get-LabWatchFrame
        if ($Json) {
            [Console]::Out.WriteLine((ConvertTo-LabWatchJson $snap))
        } else {
            foreach ($l in @(Format-LabWatchText $snap (Get-LabWatchWidth))) { [Console]::Out.WriteLine($l) }
        }
        if ($snap.daemon -eq 'down' -and $null -eq $snap.batch) { return 1 }
        return 0
    }

    Show-LabWatchLive
    return 0
}

exit (Invoke-LabWatch)
