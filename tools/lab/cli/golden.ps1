#requires -Version 7
<#
.SYNOPSIS
    Record or show a golden fingerprint: lab golden record <section> -Rows ... [-Runs 5] -ServerVersion <sha> [-SpecsDir <dir>] [-Rebless] [-Json]

.DESCRIPTION
    record runs the rows -Runs times in fingerprint mode through `lab uat`
    (serial on one instance by default; -Leases up to the run count splits
    them over that many lanes), then records the section's golden from
    those runs with lab_golden_record on the shared lab daemon. A golden is
    recorded only from five or more fresh runs that all pass every row and
    agree exactly (README D-GD6), so -Runs is 5 to 20, and the lane brake is
    one failed run. -ServerVersion names the server build the runs drove;
    the tool refuses runs without one. The golden goes to
    <specs>/golden/<section>.json (-SpecsDir, default the checkout's
    docs/guides/uat-specs). An existing golden is replaced only with
    -Rebless, from fresh runs and with every row it held.

    While recording, don't call lab_timeline on the lab character: it reads
    and drains the same packet tap, and a drained tap cannot be recorded.

    show prints a committed golden: section, rows, total ordered events,
    server build, recorded time and evidence run ids. It needs no daemon.

    -Json prints only one compact JSON object: {"ok":true,"exit":0,...} or
    {"ok":false,"exit":N,"error":...}.

    Exit codes: 0 recorded (or shown); 1 a run or row did not pass; 2 usage,
    or a golden exists without -Rebless (show: no golden); 3 the lab uat
    pre-flight or plan failed (nothing driven); 6 the runs disagree; 7 the
    runs cannot be recorded as given.

.EXAMPLE
    lab golden record first-session -Rows FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5 -ServerVersion 1a2b3c4
    lab golden record first-session -Runs 10 -Leases 2 -ServerVersion 1a2b3c4 -Rebless -Json
    lab golden show first-session
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Action,
    [Parameter(Position = 1)]
    [string]$Section,
    [string[]]$Rows,
    [int]$Runs = 5,
    [int]$Leases = 1,
    [string]$ServerVersion,
    [string]$SpecsDir,
    [switch]$Rebless,
    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')
. (Join-Path $PSScriptRoot 'uat-lib.ps1')
. (Join-Path $PSScriptRoot 'golden-lib.ps1')

# A failure: plain text on stderr, or under -Json one object on stdout.
# Returns the exit code.
function Stop-Golden([int]$Code, [string]$Message) {
    $Message = Hide-LeaseIds $Message
    if ($Json) {
        [Console]::Out.WriteLine(([ordered]@{ ok = $false; exit = $Code; error = $Message } | ConvertTo-Json -Compress))
    } else {
        [Console]::Error.WriteLine($Message)
    }
    return $Code
}

function Show-Golden([string]$Specs) {
    if (-not $Specs) { return Stop-Golden 2 'no spec folder here; pass -SpecsDir' }
    $path = Get-GoldenPath $Specs $Section
    if (-not (Test-Path -LiteralPath $path)) { return Stop-Golden 2 "no golden for $Section ($path); record one with lab golden record" }
    try { $golden = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json -Depth 50 }
    catch { return Stop-Golden 1 "cannot read ${path}: $(($_.Exception.Message -split "`n")[0])" }
    $summary = Get-GoldenSummary $golden
    if ($Json) {
        $o = [ordered]@{ ok = $true; exit = 0 }
        foreach ($k in $summary.Keys) { $o[$k] = $summary[$k] }
        [Console]::Out.WriteLine(($o | ConvertTo-Json -Depth 4 -Compress))
    } else {
        Write-Host (Format-GoldenSummary $summary)
    }
    return 0
}

function Invoke-LabGolden {
    $bad = Test-GoldenArguments $Action $Section $Runs $Leases $ServerVersion
    if ($bad) { return Stop-Golden 2 $bad }
    $specs = if ($SpecsDir) { $SpecsDir } else { Get-DefaultSpecsDir }
    if ($Action -eq 'show') { return Show-Golden $specs }

    # Away from a checkout the daemon picks its own spec folder, and its
    # `exists` refusal says the same thing after the runs.
    if ($specs -and -not $Rebless -and (Test-Path -LiteralPath (Get-GoldenPath $specs $Section))) {
        return Stop-Golden 2 'a golden exists; re-blessing needs -Rebless and five fresh runs'
    }
    $requested = @(ConvertTo-UatRowIds $Rows)

    # The batch runs as a child process so its -Json line can be captured: a
    # bare & call would lose what uat.ps1 writes to [Console]::Out.
    $childArgs = @($Section)
    if ($requested.Count) { $childArgs += '-Rows', ($requested -join ',') }
    $childArgs += '-Leases', $Leases, '-RunsPerLease', [int]($Runs / $Leases), '-ServerVersion', $ServerVersion
    if ($specs) { $childArgs += '-SpecsDir', $specs }
    $childArgs += '-Fingerprint', '-MaxConsecutiveFailures', 1, '-Json'
    if (-not $Json) {
        Write-Host "lab golden record $Section`: $Runs fingerprint run(s) on $Leases lane(s), then lab_golden_record (a run takes a few minutes)"
    }
    $pwshExe = (Get-Process -Id $PID).Path
    $lines = @(& $pwshExe -NoProfile -File (Join-Path $PSScriptRoot 'uat.ps1') @childArgs)
    $batch = ConvertFrom-GoldenBatchOutput $lines $LASTEXITCODE
    if ($batch.Exit -ne 0) { return Stop-Golden $batch.Exit $batch.Error }

    # Run folders come from batch.jsonl: -Json names only the batch (F16).
    $dirs = Get-GoldenRunDirs (Read-UatRecords (Join-Path $batch.Batch 'batch.jsonl'))
    if ($null -eq $dirs) { return Stop-Golden 7 "the batch passed but a run has no run folder; $($batch.Batch)" }
    $toolArgs = @{ section = $Section; run_dirs = @($dirs); rebless = [bool]$Rebless }
    if ($requested.Count) { $toolArgs.rows = $requested }
    if ($specs) { $toolArgs.specs_dir = $specs }
    $token = Get-LabToken
    if (-not $token) { return Stop-Golden 7 "the lab daemon is not answering (lab status); the runs are in $($batch.Batch)" }
    try {
        $session = New-McpSession (Get-LabMcpUrl) $token
        try { $reply = Invoke-McpTool $session 'lab_golden_record' $toolArgs 300 }
        finally { Close-McpSession $session }
    } catch {
        return Stop-Golden 7 "lab_golden_record failed: $(($_.Exception.Message -split "`n")[0]); the runs are in $($batch.Batch)"
    }
    $code = ConvertTo-GoldenExit $reply
    if ($Json) {
        [Console]::Out.WriteLine((Hide-LeaseIds (ConvertTo-GoldenCompactJson $reply $code)))
    } elseif ($code -eq 0) {
        Write-Host (Format-GoldenRecordLine $Section $reply)
    } else {
        [Console]::Error.WriteLine((Hide-LeaseIds "$(Get-UatField $reply 'code'): $(Get-UatField $reply 'error'); the runs are in $($batch.Batch)"))
    }
    return $code
}

if ($MyInvocation.InvocationName -ne '.') {
    exit (Invoke-LabGolden)
}
