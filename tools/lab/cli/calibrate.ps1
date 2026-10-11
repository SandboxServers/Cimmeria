#requires -Version 7
<#
.SYNOPSIS
    Recalibrate a row's poses on the lab by hover and propose a spec diff: lab calibrate <section> <row> [-Pose <id>] [-Prepare <rows>] [-Apply] [-Json].

.DESCRIPTION
    Calls lab_uat_calibrate on the shared lab daemon: for each pose a
    client_world_click step of the row names (or only -Pose), it sweeps
    stand-off radii, bearings and camera angles around the target, scores
    each by hover, confirms the winner three times, and writes the proposed
    pose line as a diff in a new UAT run folder (with calibrate.jsonl, one
    line per probe). The spec file itself is never written; nothing is
    committed.

    The client must already stand in the world where the row starts.
    -Prepare runs those rows first (one lab_uat_run on the same instance);
    a prepare row that does not PASS stops with exit 3, naming it.

    Before anything is driven: the arguments must be valid, the daemon must
    answer, the instance (-Instance, else the first free one) must be free,
    and no SGW.exe outside the lab may run.

    Output: one line per pose (frost: score 8312, confirmed 3/3, bearing
    322.5 -> 330.0; diff <path>), then the diff's text unless -Quiet. -Json
    prints only one compact JSON object (under about 400 characters, on
    failure too: {"ok":false,"exit":N,"error":...}): ok, section, row, and
    per pose pose, ok, score, confirmed, the changed fields and diff.

    -Apply runs git apply with the diff in this checkout, unless the spec
    file has uncommitted changes (exit 2; the diff stays in the run folder).
    It never stages or commits.

    -BudgetMinutes (1 to 30, default 15) bounds the whole sweep.

    Exit codes: 0 every pose has a proposal (or is unchanged); 1 a pose found
    no candidate; 2 usage, or -Apply refused or failed; 3 the pre-flight,
    the prepare rows or the calibrate call failed.

.EXAMPLE
    lab calibrate first-session FS-P3
    lab calibrate first-session FS-P3 -Pose frost -Prepare FS-01,FS-02,FS-P1,FS-P2
    lab calibrate first-session FS-P3 -Pose frost -BudgetMinutes 8 -Apply
    lab calibrate first-session FS-P3 -Instance p2 -Json
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Section,
    [Parameter(Position = 1)]
    [string]$Row,
    [string]$Pose,
    [string]$Instance,
    [string[]]$Prepare,
    [Nullable[int]]$BudgetMinutes,
    [string]$SpecsDir,
    [switch]$Apply,
    [switch]$Json,
    [switch]$Quiet
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')
. (Join-Path $PSScriptRoot 'uat-lib.ps1')
. (Join-Path $PSScriptRoot 'calibrate-lib.ps1')

# A usage or pre-flight failure: plain text on stderr, or under -Json one
# object on stdout. Returns the exit code.
function Stop-Calibrate([int]$Code, [string]$Message) {
    if ($Json) {
        [Console]::Out.WriteLine(([ordered]@{ ok = $false; exit = $Code; error = (Limit-UatText $Message 300) } | ConvertTo-Json -Compress))
    } else {
        [Console]::Error.WriteLine($Message)
    }
    return $Code
}

function Invoke-LabCalibrateCommand {
    # Argument checks come before the daemon is asked, so they exit 2 whatever its state.
    $shape = Test-CalibrateArguments $Section $Row $BudgetMinutes
    if ($shape) { return Stop-Calibrate 2 $shape }
    $specs = if ($SpecsDir) { $SpecsDir } else { Get-DefaultSpecsDir }
    $repo = Get-CalibrateRepoRoot
    # A spec with local edits is refused before any lab time is spent; the
    # file the diff names is checked again before git apply.
    if ($Apply) {
        $guess = "docs/guides/uat-specs/$Section.toml"
        if ((Test-Path -LiteralPath (Join-Path $repo $guess)) -and (Test-CalibrateSpecDirty $repo $guess)) {
            return Stop-Calibrate 2 "$guess has uncommitted changes; -Apply would clobber them (commit or revert first, or drop -Apply)"
        }
    }
    $prepareRows = @(ConvertTo-UatRowIds $Prepare)

    # Pre-flight: daemon, instance, lease, foreign clients.
    $token = Get-LabToken
    $status = Get-LabStatus
    if (-not $token -or -not $status) { return Stop-Calibrate 3 'the lab daemon is not answering (lab status; lab start)' }
    $hosted = @($status.instances | ForEach-Object { [string](Get-UatField $_ 'instance') })
    $leased = @($status.instances | Where-Object { [bool](Get-UatField (Get-UatField $_ 'lease') 'held') } |
        ForEach-Object { [string](Get-UatField $_ 'instance') })
    $plan = Get-UatPlan 1 1 $hosted $leased 0 $Instance
    if ($plan.Error) { return Stop-Calibrate $(if ($plan.Error -like 'unknown instance*') { 2 } else { 3 }) $plan.Error }
    $lane = $plan.Lanes[0].Instance
    $labPids = @($status.instances | ForEach-Object { Get-UatField $_ 'client_pid' } |
        Where-Object { "$_" -match '^\d+$' } | ForEach-Object { [int]$_ })
    $foreign = @(Get-Process SGW -ErrorAction SilentlyContinue | Where-Object { $labPids -notcontains $_.Id })
    if ($foreign) {
        return Stop-Calibrate 3 "SGW.exe runs outside the lab (pid $($foreign.Id -join ', ')); close it first, or the lab cannot start its clients"
    }
    $url = Get-LabMcpUrl
    $say = { param($m) if (-not $Json) { Write-Host $m } }

    # -Prepare: walk the client to where the row starts.
    if ($prepareRows.Count) {
        & $say "prepare: $($prepareRows -join ', ') on $lane"
        $pa = @{ sections = @($Section); rows = $prepareRows; instance = $lane }
        if ($specs) { $pa.specs_dir = $specs }
        try {
            $s = New-McpSession $url $token
            try { $prep = Invoke-McpTool $s 'lab_uat_run' $pa (15 * 60) } finally { Close-McpSession $s }
        } catch {
            return Stop-Calibrate 3 "prepare failed: $(Hide-LeaseIds (($_.Exception.Message -split "`n")[0]))"
        }
        $v = Get-UatRunVerdict @(Get-UatField $prep 'rows') $prepareRows
        if (-not $v.Ok) { return Stop-Calibrate 3 "prepare row $($v.Row) $($v.Result): $($v.Reason)" }
    }

    $ca = @{ section = $Section; row = $Row; instance = $lane }
    if ($Pose) { $ca.pose = $Pose }
    if ($specs) { $ca.specs_dir = $specs }
    if ($null -ne $BudgetMinutes) { $ca.budget_minutes = [double]$BudgetMinutes }
    & $say "calibrate $Section $Row$(if ($Pose) { " pose $Pose" }) on $lane (budget $(if ($null -ne $BudgetMinutes) { $BudgetMinutes } else { 15 }) min)"
    try {
        $reply = Invoke-LabCalibrate $url $token $ca (Get-CalibrateTimeoutSec $BudgetMinutes)
    } catch {
        return Stop-Calibrate 3 (Format-CalibrateCallError $_.Exception.Message)
    }
    $outcomes = @(Get-UatField $reply 'outcomes' | Where-Object { $null -ne $_ })
    $code = Get-CalibrateExitCode $outcomes
    $diffPath = $outcomes | ForEach-Object { [string](Get-UatField $_ 'diff_path') } | Where-Object { $_ } | Select-Object -First 1

    $applied = $false
    if ($Apply -and $diffPath) {
        $r = Invoke-CalibrateApply $repo $diffPath
        if (-not $r.Ok) {
            if (-not $Json) { foreach ($o in $outcomes) { & $say (Format-CalibrateLine $o) } }
            return Stop-Calibrate 2 "-Apply: $($r.Error)"
        }
        $applied = $true
    }

    if ($Json) {
        [Console]::Out.WriteLine((ConvertTo-CalibrateCompactJson $Section $Row $outcomes $applied))
        return $code
    }
    if (-not $outcomes.Count) { Write-Host "lab_uat_calibrate returned no outcomes; run folder $(Get-UatField $reply 'run_dir')" }
    foreach ($o in $outcomes) { Write-Host (Format-CalibrateLine $o) }
    if ($diffPath -and -not $Quiet -and (Test-Path -LiteralPath $diffPath)) {
        Write-Host ''
        Write-Host (Get-Content -LiteralPath $diffPath -Raw)
    }
    if ($applied) { Write-Host "applied to the checkout ($repo); review it with git diff, nothing is staged or committed" }
    elseif ($diffPath) { Write-Host "to apply: git -C `"$repo`" apply `"$diffPath`" (or rerun with -Apply)" }
    return $code
}

if ($MyInvocation.InvocationName -ne '.') {
    exit (Invoke-LabCalibrateCommand)
}
