#requires -Version 7
<#
.SYNOPSIS
    Run a UAT spec on the lab with no agent: lab uat <section> [-Rows ...] [-Leases 1-5] [-RunsPerLease 1-20] [-Json].

.DESCRIPTION
    Calls lab_uat_run on the shared lab daemon. -Leases lanes run at once, one
    per hosted lab instance in the daemon's order (default, p2, ...); each
    lane runs the rows -RunsPerLease times, one after another. Every run takes
    and releases its own lease, so nothing stays held. Lanes start
    -StaggerSeconds apart (run ids are time based, and simultaneous client
    boots are where logins go wrong).

    Before anything is driven: the arguments must be valid, the daemon must
    answer, the chosen instances must be free, no SGW.exe outside the lab may
    run, and a plan-only pass of the rows must be ready (every requested row
    found). A lane stops after -MaxConsecutiveFailures failed runs in a row
    (0 never stops); the other lanes go on. A run that times out is followed
    by a wait for its instance's lease to clear, so the next run does not
    collide with it; a lease that never clears ends the lane.

    Output: one progress line per run, then a summary (pass rate per row and
    per lane, failures grouped by their first failing row, known issues
    tagged). -Json prints only one compact JSON object (on failure too:
    {"ok":false,"exit":N,"error":...}), for agents. Every batch writes
    batch.jsonl (one line per run, as each finishes), then batch.json and
    summary.md, under uat-runs\batch-<time>\ next to the runs' own evidence
    folders. Ctrl+C starts no new run and still writes the summary of the
    runs that finished; a run under way finishes on the daemon and releases
    its lease.

    Exit codes: 0 every run passed; 1 a run failed; 2 usage; 3 the pre-flight
    or plan failed (nothing was driven); 4 a lane stopped early on its brake.

.EXAMPLE
    lab uat first-session -Rows FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5
    lab uat first-session -Rows FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5 -Leases 2 -RunsPerLease 5
    lab uat first-session -Instance p2 -PlanOnly
    lab uat first-session -Leases 3 -RunsPerLease 10 -Json
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Section,
    [string[]]$Rows,
    [int]$Leases = 1,
    [int]$RunsPerLease = 1,
    [string]$Instance,
    [switch]$PlanOnly,
    [string]$SpecsDir,
    [string]$ServerVersion,
    [int]$StaggerSeconds = 20,
    [int]$RunTimeoutMinutes = 15,
    [int]$MaxConsecutiveFailures = 3,
    [switch]$Json,
    [switch]$Quiet
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')
. (Join-Path $PSScriptRoot 'uat-lib.ps1')

# The daemon's MCP URL, from labd.pid's bind (as Get-LabStatus reads it).
function Get-LabMcpUrl {
    $bind = '127.0.0.1:8779'
    $info = Get-DaemonInfo
    if ($info -and $info.PSObject.Properties.Name -contains 'bind' -and $info.bind) { $bind = [string]$info.bind }
    return "http://$bind/mcp"
}

# The repo's spec folder when this runs from a checkout, else $null (the
# daemon then uses its own default).
function Get-DefaultSpecsDir {
    $dir = Join-Path $PSScriptRoot '..\..\..\docs\guides\uat-specs'
    if (Test-Path -LiteralPath $dir) { return (Resolve-Path -LiteralPath $dir).Path }
    return $null
}

# Where the daemon writes run evidence (CIMMERIA_LAB_UAT_DIR, else the lab
# home's uat-runs), so a batch folder sits next to its runs.
function Get-UatRoot {
    $map = Get-LabdEnv
    foreach ($raw in @($map['CIMMERIA_LAB_UAT_DIR'], $env:CIMMERIA_LAB_UAT_DIR)) {
        if ("$raw".Trim()) { return "$raw".Trim() }
    }
    return Join-Path (Get-LabHome) 'uat-runs'
}

# A usage or pre-flight failure: plain text on stderr, or under -Json one
# object on stdout. Returns the exit code.
function Stop-Uat([int]$Code, [string]$Message) {
    if ($Json) {
        [Console]::Out.WriteLine(([ordered]@{ ok = $false; exit = $Code; error = $Message } | ConvertTo-Json -Compress))
    } else {
        [Console]::Error.WriteLine($Message)
    }
    return $Code
}

function Invoke-LabUat {
    if (-not $Section) {
        return Stop-Uat 2 'usage: lab uat <section> [-Rows FS-01,...] [-Leases 1-5] [-RunsPerLease 1-20] [-Instance p2] [-PlanOnly] [-Json]'
    }
    # Argument checks come before the daemon is asked, so they exit 2 whatever its state.
    $shape = Test-UatArguments $Leases $RunsPerLease $Instance $StaggerSeconds $RunTimeoutMinutes $MaxConsecutiveFailures
    if ($shape) { return Stop-Uat 2 $shape }
    $requested = @(ConvertTo-UatRowIds $Rows)
    $quiet = $Quiet -or $Json
    $say = { param($m) if (-not $quiet) { Write-Host $m } }

    # Pre-flight: daemon, instances, leases, foreign clients.
    $token = Get-LabToken
    $status = Get-LabStatus
    if (-not $token -or -not $status) { return Stop-Uat 3 'the lab daemon is not answering (lab status; lab start)' }
    $hosted = @($status.instances | ForEach-Object { [string](Get-UatField $_ 'instance') })
    $leased = @($status.instances | Where-Object { [bool](Get-UatField (Get-UatField $_ 'lease') 'held') } |
        ForEach-Object { [string](Get-UatField $_ 'instance') })
    $max = 0
    $envMax = (Get-LabdEnv)['CIMMERIA_LAB_MAX_CLIENTS']
    if ("$envMax" -match '^\d+$') { $max = [int]$envMax }
    $plan = Get-UatPlan $Leases $RunsPerLease $hosted $leased $max $Instance
    if ($plan.Error) { return Stop-Uat $(if ($plan.Error -like 'unknown instance*') { 2 } else { 3 }) $plan.Error }
    if (-not $PlanOnly) {
        $labPids = @($status.instances | ForEach-Object { Get-UatField $_ 'client_pid' } |
            Where-Object { "$_" -match '^\d+$' } | ForEach-Object { [int]$_ })
        $foreign = @(Get-Process SGW -ErrorAction SilentlyContinue | Where-Object { $labPids -notcontains $_.Id })
        if ($foreign) {
            return Stop-Uat 3 "SGW.exe runs outside the lab (pid $($foreign.Id -join ', ')); close it first, or the lab cannot start its clients"
        }
    }

    $url = Get-LabMcpUrl
    $specs = if ($SpecsDir) { $SpecsDir } else { Get-DefaultSpecsDir }
    $baseArgs = @{ sections = @($Section) }
    if ($requested.Count) { $baseArgs.rows = $requested }
    if ($specs) { $baseArgs.specs_dir = $specs }
    if ($ServerVersion) { $baseArgs.server_version = $ServerVersion }

    # Plan once: a row that cannot run, or a requested row the spec lacks,
    # stops the batch before anything is driven.
    try {
        $session = New-McpSession $url $token
        $planArgs = $baseArgs.Clone(); $planArgs.plan_only = $true; $planArgs.instance = $plan.Lanes[0].Instance
        $planned = Invoke-McpTool $session 'lab_uat_run' $planArgs 120
        Close-McpSession $session
    } catch {
        return Stop-Uat 3 "the plan call failed: $(Hide-LeaseIds (($_.Exception.Message -split "`n")[0]))"
    }
    $rowIds = @(@(Get-UatField $planned 'rows') | ForEach-Object { [string](Get-UatField $_ 'row') })
    if (-not $rowIds.Count) { return Stop-Uat 2 "no rows matched in section $Section" }
    $missing = @($requested | Where-Object { $rowIds -notcontains $_ })
    if ($missing) { return Stop-Uat 2 "section $Section has no row(s) $($missing -join ', ') (row ids are exact and case sensitive)" }
    $notReady = @(@(Get-UatField $planned 'rows') | Where-Object { (Get-UatField $_ 'result') -ne 'SKIPPED' })
    if ($notReady) {
        $why = ($notReady | ForEach-Object { "$($_.row) $($_.result): $(@($_.reasons)[0])" }) -join '; '
        return Stop-Uat 3 "not ready: $why"
    }
    if ($PlanOnly) {
        if ($Json) { [Console]::Out.WriteLine(([ordered]@{ ok = $true; plan_only = $true; rows = $rowIds; lanes = @($plan.Lanes.Instance) } | ConvertTo-Json -Compress)) }
        else { & $say "plan: $($rowIds.Count) row(s) ready on $(@($plan.Lanes.Instance) -join ', '): $($rowIds -join ', ')" }
        return 0
    }

    $total = $plan.Lanes.Count * $RunsPerLease
    & $say "lab uat $Section`: $($plan.Lanes.Count) lease(s) x $RunsPerLease run(s) = $total run(s) on $(@($plan.Lanes.Instance) -join ', '); rows $($rowIds -join ', ')"

    $batchDir = Join-Path (Get-UatRoot) "batch-$(Get-Date -Format 'yyyyMMdd-HHmmss')"
    New-Item -ItemType Directory -Force -Path $batchDir | Out-Null
    $jsonl = Join-Path $batchDir 'batch.jsonl'
    $lib = Join-Path $PSScriptRoot 'uat-lib.ps1'
    $common = Join-Path $PSScriptRoot 'common.ps1'
    $started = Get-Date

    try {
        $plan.Lanes | ForEach-Object -ThrottleLimit $plan.Lanes.Count -Parallel {
            . $using:common
            . $using:lib
            $lane = $_
            $quietL = $using:quiet
            $n = $using:RunsPerLease
            $outcomes = @()
            try {
                if ($lane.Lane -gt 1) { Start-Sleep -Seconds (($lane.Lane - 1) * $using:StaggerSeconds) }
                for ($i = 1; $i -le $n; $i++) {
                    $t0 = Get-Date
                    $rec = [ordered]@{ Lane = $lane.Lane; Instance = $lane.Instance; Index = $i; Ok = $false; Rows = @()
                                       Verdict = $null; RunDir = $null; Error = $null; Seconds = 0; Braked = $false }
                    $timedOut = $false
                    try {
                        $s = New-McpSession $using:url $using:token
                        $a = ($using:baseArgs).Clone(); $a.instance = $lane.Instance
                        try { $out = Invoke-McpTool $s 'lab_uat_run' $a ($using:RunTimeoutMinutes * 60) }
                        finally { Close-McpSession $s }
                        $rec.Rows = @(Get-UatField $out 'rows'); $rec.RunDir = [string](Get-UatField $out 'run_dir')
                        $rec.Verdict = Get-UatRunVerdict $rec.Rows $using:rowIds
                        $rec.Ok = $rec.Verdict.Ok
                    } catch {
                        $rec.Error = Hide-LeaseIds (($_.Exception.Message -split "`n")[0])
                        $timedOut = $rec.Error -match 'Timeout|timed out|canceled'
                        $rec.Verdict = @{ Ok = $false; Row = $null; Result = $null; Reason = $rec.Error; Issue = $null }
                    }
                    $rec.Seconds = [int]((Get-Date) - $t0).TotalSeconds
                    $outcomes += $rec.Ok
                    # A run cut short here may still hold its lease on the
                    # daemon: wait for it, or the next run collides with it.
                    $stuck = $false
                    if ($timedOut) { $stuck = -not (Wait-UatLeaseFree $lane.Instance 600) }
                    $brake = Test-UatStopLane $i $n $outcomes $using:MaxConsecutiveFailures
                    $rec.Braked = $brake -or $stuck
                    Add-UatRecord $using:jsonl $rec
                    if (-not $quietL) {
                        $what = if ($rec.Ok) { 'PASS' } elseif ($rec.Error) { "ERROR $($rec.Error)" } else {
                            "FAIL $($rec.Verdict.Row): $($rec.Verdict.Reason)$(if ($rec.Verdict.Issue) { " [known $($rec.Verdict.Issue)]" })" }
                        Write-Host ("[{0} {1}/{2}] {3} ({4}s)" -f $lane.Instance, $i, $n, $what, $rec.Seconds)
                        if ($stuck) { Write-Host "[$($lane.Instance)] its lease did not clear after the timeout; lane stopped" }
                        elseif ($brake) { Write-Host "[$($lane.Instance)] stopped after $($using:MaxConsecutiveFailures) failed runs in a row" }
                    }
                    if ($rec.Braked) { break }
                }
            } catch {
                # A lane that dies outside a run still leaves a record, so the batch reports it.
                $err = Hide-LeaseIds (($_.Exception.Message -split "`n")[0])
                Add-UatRecord $using:jsonl ([ordered]@{ Lane = $lane.Lane; Instance = $lane.Instance; Index = 0; Ok = $false
                    Rows = @(); Verdict = @{ Ok = $false; Row = $null; Result = $null; Reason = $err; Issue = $null }
                    RunDir = $null; Error = "lane: $err"; Seconds = 0; Braked = $true })
            }
        }
    } finally {
        # Also on Ctrl+C: summarise whatever finished.
        $allRuns = @(Read-UatRecords $jsonl)
        $brakedNames = @($allRuns | Where-Object { $_.Braked } | ForEach-Object { $_.Instance } | Select-Object -Unique)
        $summary = Merge-UatResults $allRuns $plan.Lanes $brakedNames
        $title = "lab uat $Section"
        $md = Format-UatSummary $summary $title $batchDir
        Set-Content -LiteralPath (Join-Path $batchDir 'summary.md') -Value $md -Encoding utf8
        [ordered]@{
            section = $Section; rows = $rowIds; leases = $plan.Lanes.Count; runs_per_lease = $RunsPerLease
            started = $started.ToString('o'); minutes = [math]::Round(((Get-Date) - $started).TotalMinutes, 1)
            summary = $summary
        } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $batchDir 'batch.json') -Encoding utf8
    }

    # Straight to stdout: a bare expression here would join the function's
    # return value and turn the exit code into an array.
    if ($Json) { [Console]::Out.WriteLine((ConvertTo-UatCompactJson $summary $batchDir)) }
    elseif (-not $Quiet) { Write-Host ''; Write-Host $md }
    else { Write-Host "$title`: $($summary.passed)/$($summary.runs) passed; $batchDir" }
    return (Get-UatExitCode $summary)
}

if ($MyInvocation.InvocationName -ne '.') {
    exit (Invoke-LabUat)
}
