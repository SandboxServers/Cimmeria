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

    Before anything is driven: the daemon must answer, the chosen instances
    must be free, no SGW.exe outside the lab may run, and a plan-only pass of
    the rows must be ready. A lane stops after -MaxConsecutiveFailures failed
    runs in a row (0 never stops); the other lanes go on. Ctrl+C starts no new
    run; a run already started finishes on the daemon and releases its lease.

    Output: one progress line per run, then a summary (pass rate per row and
    per lane, failures grouped by their first failing row, known issues
    tagged). -Json prints only one compact JSON object, for agents. Every
    batch writes batch.json and summary.md under the lab home's
    uat-runs\batch-<time>\, next to each run's own evidence folder.

    Exit codes: 0 every run passed; 1 a run failed; 2 usage; 3 the pre-flight
    or plan failed (nothing was driven); 4 a lane stopped on its brake.

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

function Write-Err([string]$m) { [Console]::Error.WriteLine($m) }

function Invoke-LabUat {
    if (-not $Section) { Write-Err 'usage: lab uat <section> [-Rows FS-01,...] [-Leases 1-5] [-RunsPerLease 1-20] [-Instance p2] [-PlanOnly] [-Json]'; return 2 }
    $quiet = $Quiet -or $Json
    $say = { param($m) if (-not $quiet) { Write-Host $m } }

    # Pre-flight: daemon, instances, leases, foreign clients.
    $token = Get-LabToken
    $status = Get-LabStatus
    if (-not $token -or -not $status) { Write-Err 'the lab daemon is not answering (lab status; lab start)'; return 3 }
    $hosted = @($status.instances | ForEach-Object { [string]$_.instance })
    $leased = @($status.instances | Where-Object { [bool](Get-UatField (Get-UatField $_ 'lease') 'held') } |
        ForEach-Object { [string]$_.instance })
    $max = 0
    $envMax = (Get-LabdEnv)['CIMMERIA_LAB_MAX_CLIENTS']
    if ("$envMax" -match '^\d+$') { $max = [int]$envMax }
    $plan = Get-UatPlan $Leases $RunsPerLease $hosted $leased $max $Instance
    if ($plan.Error) {
        Write-Err $plan.Error
        return $(if ($plan.Error -like '-*' -or $plan.Error -like 'unknown*') { 2 } else { 3 })
    }
    $labPids = @($status.instances | ForEach-Object { Get-UatField $_ 'client_pid' } | Where-Object { "$_" -match '^\d+$' } | ForEach-Object { [int]$_ })
    $foreign = @(Get-Process SGW -ErrorAction SilentlyContinue | Where-Object { $labPids -notcontains $_.Id })
    if ($foreign) {
        Write-Err "SGW.exe runs outside the lab (pid $($foreign.Id -join ', ')); close it first, or the lab cannot start its clients"
        return 3
    }

    $url = Get-LabMcpUrl
    $specs = if ($SpecsDir) { $SpecsDir } else { Get-DefaultSpecsDir }
    $baseArgs = @{ sections = @($Section) }
    if ($Rows) { $baseArgs.rows = @($Rows | ForEach-Object { $_ -split ',' } | Where-Object { $_ }) }
    if ($specs) { $baseArgs.specs_dir = $specs }
    if ($ServerVersion) { $baseArgs.server_version = $ServerVersion }

    # Plan once: a row that cannot run stops the batch before anything is driven.
    $session = New-McpSession $url $token
    $planArgs = $baseArgs.Clone(); $planArgs.plan_only = $true; $planArgs.instance = $plan.Lanes[0].Instance
    try { $planned = Invoke-McpTool $session 'lab_uat_run' $planArgs 120 } catch { Write-Err $_.Exception.Message; return 3 }
    $notReady = @($planned.rows | Where-Object { $_.result -ne 'SKIPPED' })
    if (-not @($planned.rows).Count) { Write-Err "no rows matched in section $Section"; return 2 }
    if ($notReady) {
        $notReady | ForEach-Object { Write-Err "$($_.row) $($_.result): $(@($_.reasons)[0])" }
        return 3
    }
    $rowIds = @($planned.rows | ForEach-Object { $_.row })
    if ($PlanOnly) {
        if ($Json) { [Console]::Out.WriteLine(([ordered]@{ ok = $true; plan_only = $true; rows = $rowIds; lanes = @($plan.Lanes.Instance) } | ConvertTo-Json -Compress)) }
        else { & $say "plan: $($rowIds.Count) row(s) ready on $(@($plan.Lanes.Instance) -join ', '): $($rowIds -join ', ')" }
        return 0
    }

    $total = $plan.Lanes.Count * $RunsPerLease
    & $say "lab uat $Section`: $($plan.Lanes.Count) lease(s) x $RunsPerLease run(s) = $total run(s) on $(@($plan.Lanes.Instance) -join ', '); rows $($rowIds -join ', ')"

    $batchId = Get-Date -Format 'yyyyMMdd-HHmmss'
    $batchDir = Join-Path (Get-LabHome) "uat-runs\batch-$batchId"
    New-Item -ItemType Directory -Force -Path $batchDir | Out-Null
    $lib = Join-Path $PSScriptRoot 'uat-lib.ps1'
    $started = Get-Date

    $laneResults = $plan.Lanes | ForEach-Object -ThrottleLimit $plan.Lanes.Count -Parallel {
        . $using:lib
        $lane = $_
        $quietL = $using:quiet
        if ($lane.Lane -gt 1) { Start-Sleep -Seconds (($lane.Lane - 1) * $using:StaggerSeconds) }
        $runs = @(); $outcomes = @(); $braked = $false
        for ($i = 1; $i -le $using:RunsPerLease; $i++) {
            $t0 = Get-Date
            $rec = @{ Lane = $lane.Lane; Instance = $lane.Instance; Index = $i; Ok = $false; Rows = @()
                      Verdict = $null; RunDir = $null; Error = $null; Seconds = 0 }
            try {
                $s = New-McpSession $using:url $using:token
                $a = ($using:baseArgs).Clone(); $a.instance = $lane.Instance
                $out = Invoke-McpTool $s 'lab_uat_run' $a ($using:RunTimeoutMinutes * 60)
                $rec.Rows = @($out.rows); $rec.RunDir = [string]$out.run_dir
                $rec.Verdict = Get-UatRunVerdict $out.rows
                $rec.Ok = $rec.Verdict.Ok
            } catch {
                $rec.Error = ($_.Exception.Message -split "`n")[0]
                $rec.Verdict = @{ Ok = $false; Row = $null; Result = $null; Reason = $rec.Error; Issue = $null }
            }
            $rec.Seconds = [int]((Get-Date) - $t0).TotalSeconds
            $runs += $rec; $outcomes += $rec.Ok
            if (-not $quietL) {
                $what = if ($rec.Ok) { 'PASS' } elseif ($rec.Error) { "ERROR $($rec.Error)" } else {
                    "FAIL $($rec.Verdict.Row): $($rec.Verdict.Reason)$(if ($rec.Verdict.Issue) { " [known $($rec.Verdict.Issue)]" })" }
                Write-Host ("[{0} {1}/{2}] {3} ({4}s)" -f $lane.Instance, $i, $using:RunsPerLease, $what, $rec.Seconds)
            }
            if (Test-UatBrake $outcomes $using:MaxConsecutiveFailures) {
                $braked = $true
                if (-not $quietL) { Write-Host "[$($lane.Instance)] stopped after $($using:MaxConsecutiveFailures) failed runs in a row" }
                break
            }
        }
        @{ Instance = $lane.Instance; Runs = $runs; Braked = $braked }
    }

    $allRuns = @($laneResults | ForEach-Object { $_.Runs })
    $brakedNames = @($laneResults | Where-Object { $_.Braked } | ForEach-Object { $_.Instance })
    $summary = Merge-UatResults $allRuns $plan.Lanes $brakedNames
    $title = "lab uat $Section"
    $md = Format-UatSummary $summary $title $batchDir
    Set-Content -LiteralPath (Join-Path $batchDir 'summary.md') -Value $md -Encoding utf8
    [ordered]@{
        section = $Section; rows = $rowIds; leases = $plan.Lanes.Count; runs_per_lease = $RunsPerLease
        started = $started.ToString('o'); minutes = [math]::Round(((Get-Date) - $started).TotalMinutes, 1)
        summary = $summary
        runs = @($allRuns | ForEach-Object { [ordered]@{ instance = $_.Instance; index = $_.Index; ok = $_.Ok
            first_failure = $(if ($_.Ok) { $null } else { "$($_.Verdict.Row) $($_.Verdict.Reason)" })
            issue = $_.Verdict.Issue; seconds = $_.Seconds; run_dir = $_.RunDir; error = $_.Error } })
    } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $batchDir 'batch.json') -Encoding utf8

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
