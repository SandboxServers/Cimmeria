#requires -Version 7
<#
.SYNOPSIS
    Run a UAT spec on the lab with no agent: lab uat <section> [-Rows ...] [-Leases 1-5] [-RunsPerLease 1-20] [-Chaos ...] [-Heal] [-Json].

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
    {"ok":false,"exit":N,"error":...}), for agents. -ProgressToStderr writes
    the progress lines to stderr, under -Json too (lab golden uses it to show
    a long batch live). Every batch writes
    plan.json first, then batch.jsonl (one line per run, as each finishes),
    then batch.json and summary.md, under uat-runs\batch-<time>\ next to the
    runs' own evidence folders. Ctrl+C starts no new run and still writes the summary of the
    runs that finished; a run under way finishes on the daemon and releases
    its lease.

    -Fixtures auto|always|never sets when a row's [row.fixture] is applied
    (lab_uat_run `fixtures`; the daemon's default is auto: a fixture row
    continues a passing chain and starts from its fixture otherwise).

    -Chaos <profile[:level][:knobs]> passes chaos specs to the runner
    unchanged (lab_uat_run `chaos`); profiles land in later lab-chaos packets.
    Give several as -Chaos cpu,flush:pack or -Chaos @('cpu','flush:pack').
    A multi-knob spec contains commas, so quote it: -Chaos 'net:latency=120,loss=1'.
    The run's header (chaos=<spec>, joined with "; ") leads the summary and
    -Json gets a chaos key, only when -Chaos was given.

    -DiffGolden compares every passing run with the section's committed
    golden (<specs>/golden/<section>.json, made by lab golden record): the
    runs are fingerprinted, lab_golden_diff is called for each passing run's
    folder, and each gets a `golden` value: match, the first divergence as
    one line, or error: <why>. A failed run is not compared. Rows' verdicts
    do not change. Without a golden, or with one that cannot be read, it
    stops before anything is driven (exit 2).

    -Heal (needs -Leases 1) recalibrates the poses failed rows missed on:
    after the batch, for each distinct (row, pose) whose row summary carries
    a heal hint (a hover, on-screen or camera miss on a pose), it calls
    lab_uat_calibrate for that pose on the batch's instance (up to its
    15-minute budget each), saves a proposed diff as heal-<row>-<pose>.diff
    in the batch folder and prints it. The summary (batch.json, summary.md,
    -Json) then lists heal: [{row, pose, diff}] (under -Json at most 3, with
    more_heal: <n> for the rest, and diff the file name in the batch folder).
    After a failed call it waits up to 10 minutes for the instance's lease
    to clear before the next pose, and stops healing, naming the poses left,
    when it does not. It never writes the spec file and never commits:
    review the diff, then git apply it (or run lab calibrate <section> <row>
    -Pose <pose> -Apply). Ctrl+C during the batch skips the heal; Ctrl+C
    during the heal still lists the diffs already written.

    Exit codes: 0 every run passed; 1 a run failed; 2 usage; 3 the pre-flight
    or plan failed (nothing was driven); 4 a lane stopped early on its brake;
    5 every run passed but -DiffGolden did not match on one (a divergence, or
    a run it could not compare); 8 a heal proposal was written (SV-12),
    which wins over every other code, 5 included.

.EXAMPLE
    lab uat first-session -Rows FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5
    lab uat first-session -Rows FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5 -Leases 2 -RunsPerLease 5
    lab uat first-session -Instance p2 -PlanOnly
    lab uat first-session -Rows FS-P4 -RunsPerLease 20
    lab uat first-session -Rows FS-P5,FS-P3,FS-P4 -Fixtures always
    lab uat first-session -Rows FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5 -DiffGolden
    lab uat first-session -Leases 3 -RunsPerLease 10 -Json
    lab uat first-session -Rows FS-01,FS-02,FS-P1,FS-P2,FS-P3 -Heal
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
    [string]$Fixtures,
    # One or more chaos specs (profile[:level][:knobs]); a multi-knob spec must be quoted.
    [string[]]$Chaos,
    [switch]$DiffGolden,
    [switch]$Heal,
    # Used by lab golden: runs capture what a golden fingerprint needs.
    [switch]$Fingerprint,
    # Used by lab golden: progress lines go to stderr, even under -Json.
    [switch]$ProgressToStderr,
    [switch]$Json,
    [switch]$Quiet
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')
. (Join-Path $PSScriptRoot 'uat-lib.ps1')
. (Join-Path $PSScriptRoot 'golden-lib.ps1')
. (Join-Path $PSScriptRoot 'calibrate-lib.ps1')

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
        return Stop-Uat 2 'usage: lab uat <section> [-Rows FS-01,...] [-Leases 1-5] [-RunsPerLease 1-20] [-Instance p2] [-Fixtures auto|always|never] [-DiffGolden] [-Heal] [-PlanOnly] [-Json]'
    }
    # Argument checks come before the daemon is asked, so they exit 2 whatever its state.
    $shape = Test-UatArguments $Leases $RunsPerLease $Instance $StaggerSeconds $RunTimeoutMinutes $MaxConsecutiveFailures $Fixtures $Heal.IsPresent
    if ($shape) { return Stop-Uat 2 $shape }
    $specs = if ($SpecsDir) { $SpecsDir } else { Get-DefaultSpecsDir }
    # A diff needs a golden: without one, stop before anything is driven.
    if ($DiffGolden) {
        if (-not $specs) { return Stop-Uat 2 '-DiffGolden needs -SpecsDir (no spec folder found next to this checkout)' }
        $goldenPath = Get-GoldenPath $specs $Section
        if (-not (Test-Path -LiteralPath $goldenPath)) {
            return Stop-Uat 2 "no golden for $Section; record one with lab golden record"
        }
        $unreadable = Test-UatGoldenFile $goldenPath
        if ($unreadable) { return Stop-Uat 2 "golden for $Section cannot be read: $unreadable" }
    }
    $requested = @(ConvertTo-UatRowIds $Rows)
    $progress = Get-UatProgressMode $Json $Quiet $ProgressToStderr
    $say = { param($m) Write-UatProgress $progress $m }

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
    $baseArgs = @{ sections = @($Section) }
    if ($requested.Count) { $baseArgs.rows = $requested }
    if ($specs) { $baseArgs.specs_dir = $specs }
    if ($ServerVersion) { $baseArgs.server_version = $ServerVersion }
    if ($Fixtures) { $baseArgs.fixtures = $Fixtures }
    if ($Chaos) { $baseArgs.chaos = @($Chaos) }
    $chaosHeader = Format-UatChaosHeader $Chaos
    if ($Fingerprint -or $DiffGolden) { $baseArgs.fingerprint = $true }
    # lab_golden_diff's arguments less run_dir, or $null without -DiffGolden.
    $diffArgs = if ($DiffGolden) { @{ section = $Section; specs_dir = $specs } } else { $null }

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

    $started = Get-Date
    $batchDir = Join-Path (Get-UatRoot) "batch-$(Get-Date -Format 'yyyyMMdd-HHmmss')"
    $jsonl = Join-Path $batchDir 'batch.jsonl'
    try {
        New-Item -ItemType Directory -Force -Path $batchDir | Out-Null
        # plan.json before any lane starts, so a watcher never sees runs without a plan.
        Write-UatPlan (Join-Path $batchDir 'plan.json') ([ordered]@{
            schema = 1; section = $Section; rows = $rowIds
            lanes = @($plan.Lanes | ForEach-Object { [ordered]@{ lane = $_.Lane; instance = $_.Instance } })
            runs_per_lease = $RunsPerLease; started = $started.ToUniversalTime().ToString('o'); pid = $PID
        })
    } catch {
        return Stop-Uat 3 "cannot write the batch folder ${batchDir}: $(($_.Exception.Message -split "`n")[0])"
    }
    $lib = Join-Path $PSScriptRoot 'uat-lib.ps1'
    $common = Join-Path $PSScriptRoot 'common.ps1'

    try {
        $plan.Lanes | ForEach-Object -ThrottleLimit $plan.Lanes.Count -Parallel {
            . $using:common
            . $using:lib
            $lane = $_
            $progressL = $using:progress
            $n = $using:RunsPerLease
            $outcomes = @()
            try {
                if ($lane.Lane -gt 1) { Start-Sleep -Seconds (($lane.Lane - 1) * $using:StaggerSeconds) }
                for ($i = 1; $i -le $n; $i++) {
                    $t0 = Get-Date
                    $rec = [ordered]@{ Lane = $lane.Lane; Instance = $lane.Instance; Index = $i; Ok = $false; Rows = @()
                                       Verdict = $null; RunDir = $null; Error = $null; Seconds = 0; Braked = $false
                                       Started = $t0.ToUniversalTime().ToString('o'); Ended = $null }
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
                    # -DiffGolden: compare a passing run's folder with the
                    # golden. The run's verdict stays as it is (README D-GD7).
                    $urlL = $using:url; $tokenL = $using:token
                    $golden = Get-UatRunGolden $rec $using:diffArgs {
                        param($ga)
                        $gs = New-McpSession $urlL $tokenL
                        try { Invoke-McpTool $gs 'lab_golden_diff' $ga 120 } finally { Close-McpSession $gs }
                    }
                    if ($null -ne $golden) { $rec.Golden = $golden }
                    $outcomes += $rec.Ok
                    # A run cut short here may still hold its lease on the
                    # daemon: wait for it, or the next run collides with it.
                    $stuck = $false
                    if ($timedOut) { $stuck = -not (Wait-UatLeaseFree $lane.Instance 600) }
                    $brake = Test-UatStopLane $i $n $outcomes $using:MaxConsecutiveFailures
                    $rec.Braked = $brake -or $stuck
                    $rec.Ended = (Get-Date).ToUniversalTime().ToString('o')
                    Add-UatRecord $using:jsonl $rec
                    Write-UatProgress $progressL (Format-UatProgressLine $rec $n)
                    if ($stuck) { Write-UatProgress $progressL "[$($lane.Instance)] its lease did not clear after the timeout; lane stopped" }
                    elseif ($brake) { Write-UatProgress $progressL "[$($lane.Instance)] stopped after $($using:MaxConsecutiveFailures) failed runs in a row" }
                    if ($rec.Braked) { break }
                }
            } catch {
                # A lane that dies outside a run still leaves a record, so the batch reports it.
                $err = Hide-LeaseIds (($_.Exception.Message -split "`n")[0])
                $at = (Get-Date).ToUniversalTime().ToString('o')
                Add-UatRecord $using:jsonl ([ordered]@{ Lane = $lane.Lane; Instance = $lane.Instance; Index = 0; Ok = $false
                    Rows = @(); Verdict = @{ Ok = $false; Row = $null; Result = $null; Reason = $err; Issue = $null }
                    RunDir = $null; Error = "lane: $err"; Seconds = 0; Braked = $true; Started = $at; Ended = $at })
            }
        }
    } finally {
        # Also on Ctrl+C: summarise whatever finished.
        $allRuns = @(Read-UatRecords $jsonl)
        $brakedNames = @($allRuns | Where-Object { $_.Braked } | ForEach-Object { $_.Instance } | Select-Object -Unique)
        $summary = Merge-UatResults $allRuns $plan.Lanes $brakedNames
        if ($chaosHeader) { $summary['chaos'] = $chaosHeader }
        $title = "lab uat $Section"
        $batch = @{ Dir = $batchDir; Title = $title; Section = $Section; RowIds = $rowIds
                    LaneCount = $plan.Lanes.Count; RunsPerLease = $RunsPerLease; Started = $started }
        $md = Write-UatBatchSummary $summary $batch
    }

    # -Heal runs after the finally, not in it: a finally cannot be stopped,
    # and a heal is up to 15 minutes of lab time per pose, so Ctrl+C during
    # the batch skips it. The summary is already on disk; Invoke-UatHealPass
    # rewrites it with heal (in its own finally, so Ctrl+C mid-heal keeps the
    # diffs already written) only when a proposal was written. After a failed
    # call it waits for the instance's lease, as a lane does after a timeout.
    if ($Heal) {
        $healMd = Invoke-UatHealPass $allRuns $summary $batch $specs $plan.Lanes[0].Instance {
            param($ca)
            Invoke-LabCalibrate $url $token $ca (Get-CalibrateTimeoutSec $null)
        } $say { param($i) Wait-UatLeaseFree $i 600 } ($progress -eq 'host')
        if ($healMd) { $md = $healMd }
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
