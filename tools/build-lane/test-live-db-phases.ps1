<#
.SYNOPSIS
    Tests live-db-test.ps1's two phases (a heavy --build-only call, then a db slot for the
    reload and the run) against a scratch LANE_ROOT, with stub tier and reload scripts. No
    cargo and no Postgres; never touches the real lane root.

        pwsh -NoProfile -File tools/build-lane/test-live-db-phases.ps1
#>
$ErrorActionPreference = 'Stop'

$script:failed = 0
function Check($cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

$liveDb = Join-Path $PSScriptRoot 'live-db-test.ps1'
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("live-db-phases-test-" + [guid]::NewGuid().ToString('N'))
$metrics = Join-Path $tmp 'metrics'
$jobsFile = Join-Path $metrics 'jobs.jsonl'
$calls = Join-Path $tmp 'calls.txt'
New-Item -ItemType Directory (Join-Path $tmp 'lane') | Out-Null

# The tier stub records its argv in $env:STUB_CALLS and exits with $env:STUB_BUILD_EXIT when
# it is the --build-only call. The reload stub records 'reload' and prints a DATABASE_URL.
$tier = Join-Path $tmp 'tier-stub.ps1'
$reload = Join-Path $tmp 'reload-stub.ps1'
@'
Add-Content -LiteralPath $env:STUB_CALLS -Value ("tier " + ($args -join ' '))
if ($args -contains '--build-only' -and $env:STUB_BUILD_EXIT) { exit ([int]$env:STUB_BUILD_EXIT) }
exit 0
'@ | Set-Content -LiteralPath $tier
@'
Add-Content -LiteralPath $env:STUB_CALLS -Value 'reload'
Write-Output 'DATABASE_URL=postgres://stub'
'@ | Set-Content -LiteralPath $reload

# Every lane call in this test runs with these; a $null value removes the variable.
$baseEnv = @{
    LANE_ROOT = $tmp; LANE_MIN_FREE_GB = '0'; LANE_VERBOSE = '1'; LANE_PRUNE = '0'
    RUSTC_WRAPPER = 'none'; LANE_METRICS = '1'; LANE_METRICS_DIR = $metrics
    LANE_SLOTS = $null; LANE_LIGHT_SLOTS = $null; LANE_DB_SLOTS = $null; CARGO_BUILD_JOBS = $null
    CIMMERIA_LANE_SLOT = $null; CIMMERIA_LANE_TOKEN = $null
    LIVE_DB_TIER_SCRIPT = $tier; LIVE_DB_RELOAD_SCRIPT = $reload; STUB_CALLS = $calls; STUB_BUILD_EXIT = $null
}

# Runs pwsh with $Argv and $Env on top of $baseEnv and returns its exit code (-1 when the
# call outlives $TimeoutSec: the process tree is killed, so a hang fails rather than hangs).
function Invoke-Pwsh([string[]]$Argv, [hashtable]$Env = @{}, [int]$TimeoutSec = 120) {
    $psi = [System.Diagnostics.ProcessStartInfo]::new('pwsh')
    $psi.UseShellExecute = $false
    foreach ($a in $Argv) { $psi.ArgumentList.Add($a) }
    $all = $baseEnv.Clone()
    foreach ($k in $Env.Keys) { $all[$k] = $Env[$k] }
    foreach ($k in $all.Keys) {
        if ($null -eq $all[$k]) { [void]$psi.Environment.Remove($k) } else { $psi.Environment[$k] = [string]$all[$k] }
    }
    $p = [System.Diagnostics.Process]::Start($psi)
    if (-not $p.WaitForExit($TimeoutSec * 1000)) {
        & taskkill.exe /T /F /PID $p.Id *> $null
        return -1
    }
    $p.ExitCode
}

# Starts every case with no calls and no job lines from the case before.
function Reset-Case {
    Remove-Item -LiteralPath $calls, $jobsFile -Force -ErrorAction SilentlyContinue
}

function Read-Jobs {
    if (-not (Test-Path -LiteralPath $jobsFile)) { return @() }
    @(Get-Content -LiteralPath $jobsFile | Where-Object { $_.Trim() } | ForEach-Object { $_ | ConvertFrom-Json })
}

function Read-Calls {
    if (-not (Test-Path -LiteralPath $calls)) { return @() }
    @(Get-Content -LiteralPath $calls)
}

try {
    # --- 1. success: heavy --build-only, then db reload and run ---------------------------
    Reset-Case
    $rc = Invoke-Pwsh @('-NoProfile', '-File', $liveDb, 'chain_replay_tests::mission_701')
    Check ($rc -eq 0) "success exits 0 (got $rc)"
    $jobs = Read-Jobs
    Check ($jobs.Count -eq 2) "success writes two jobs.jsonl lines (got $($jobs.Count))"
    if ($jobs.Count -eq 2) {
        Check ($jobs[0].class -eq 'heavy' -and $jobs[0].cmd -match '--build-only') 'the first job is heavy and runs the tier with --build-only'
        Check ($jobs[1].class -eq 'db' -and $jobs[1].cmd -match '--in-lane') 'the second job is db and runs the in-lane phase'
    }
    $lines = Read-Calls
    Check ($lines.Count -eq 3 -and $lines[0] -eq 'tier --build-only' -and $lines[1] -eq 'reload' -and $lines[2] -eq 'tier chain_replay_tests::mission_701') `
        "the tier saw --build-only, then reload, then the filter alone (got: $($lines -join ' | '))"

    # --- 2. a leading --wireclient goes to both phases; the filter only to phase 2 -------
    Reset-Case
    $rc = Invoke-Pwsh @('-NoProfile', '-File', $liveDb, '--wireclient', 'chain_replay')
    Check ($rc -eq 0) "--wireclient run exits 0 (got $rc)"
    $lines = Read-Calls
    Check ($lines.Count -eq 3 -and $lines[0] -eq 'tier --wireclient --build-only' -and $lines[2] -eq 'tier --wireclient chain_replay') `
        "--wireclient reaches both phases (got: $($lines -join ' | '))"

    # --- 3. tier exit 3 under --build-only: exit 3, one line, no reload ---------------------
    Reset-Case
    $rc = Invoke-Pwsh @('-NoProfile', '-File', $liveDb, 'chain_replay') @{ STUB_BUILD_EXIT = '3' }
    Check ($rc -eq 3) "a failed build exits with the tier's code 3 (got $rc)"
    Check ((Read-Jobs).Count -eq 1) "a failed build writes one jobs.jsonl line (got $((Read-Jobs).Count))"
    Check (-not (Read-Calls | Where-Object { $_ -eq 'reload' })) 'a failed build never calls the reload'
} finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed -gt 0) { Write-Host "$($script:failed) check(s) failed"; exit 1 }
Write-Host 'all live-db phase checks passed'
exit 0
