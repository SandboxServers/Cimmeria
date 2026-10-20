<#
.SYNOPSIS
    Tests for the PowerShell build lane's job classes (lane-class.ps1 and lane.ps1's use of
    them) against a scratch LANE_ROOT. No Pester needed; exits non-zero on a failure. Runs
    no cargo and no bash, and never touches the real lane root.

        pwsh -NoProfile -File tools/build-lane/test-lane-classes.ps1
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lane-slots.ps1')
. (Join-Path $PSScriptRoot 'lane-class.ps1')

$script:failed = 0
function Check($cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

$lane = Join-Path $PSScriptRoot 'lane.ps1'
$fixture = Join-Path $PSScriptRoot 'fixtures/lane-classes.tsv'
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("lane-classes-test-" + [guid]::NewGuid().ToString('N'))
$lock = Join-Path $tmp 'lane'
$metrics = Join-Path $tmp 'metrics'
$jobsFile = Join-Path $metrics 'jobs.jsonl'
New-Item -ItemType Directory $lock | Out-Null
$children = @()

# probe.ps1 <out> <path>: the job the lane runs. It records the job's cargo jobs and lane
# token, and whether <path> exists while it runs ('none' skips that).
$probe = Join-Path $tmp 'probe.ps1'
$exit7 = Join-Path $tmp 'exit7.ps1'
@'
param([string]$Out, [string]$Path)
[ordered]@{
    exists = if ($Path -ne 'none') { Test-Path -LiteralPath $Path } else { $null }
    jobs = $env:CARGO_BUILD_JOBS
    token = $env:CIMMERIA_LANE_TOKEN
} | ConvertTo-Json | Set-Content -LiteralPath $Out
'@ | Set-Content -LiteralPath $probe
'exit 7' | Set-Content -LiteralPath $exit7

# Every lane call in this test runs with these; a $null value removes the variable.
$baseEnv = @{
    LANE_ROOT = $tmp; LANE_MIN_FREE_GB = '0'; LANE_VERBOSE = '1'; LANE_PRUNE = '0'
    RUSTC_WRAPPER = 'none'; LANE_METRICS = '1'; LANE_METRICS_DIR = $metrics
    LANE_SLOTS = $null; LANE_LIGHT_SLOTS = $null; LANE_DB_SLOTS = $null; LANE_LIGHT_JOBS = $null
    CARGO_BUILD_JOBS = $null; CIMMERIA_LANE_SLOT = $null; CIMMERIA_LANE_TOKEN = $null
}

# Runs `pwsh` with $Argv and $Env on top of $baseEnv. Exit is -1 and the process tree is
# killed when the call outlives $TimeoutSec, so a hung lane fails the test rather than
# hanging it.
function Invoke-Pwsh([string[]]$Argv, [hashtable]$Env = @{}, [int]$TimeoutSec = 60) {
    $psi = [System.Diagnostics.ProcessStartInfo]::new('pwsh')
    $psi.UseShellExecute = $false
    foreach ($a in $Argv) { $psi.ArgumentList.Add($a) }
    $all = $baseEnv.Clone()
    foreach ($k in $Env.Keys) { $all[$k] = $Env[$k] }
    foreach ($k in $all.Keys) {
        if ($null -eq $all[$k]) { [void]$psi.Environment.Remove($k) } else { $psi.Environment[$k] = [string]$all[$k] }
    }
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = [System.Diagnostics.Process]::Start($psi)
    if (-not $p.WaitForExit($TimeoutSec * 1000)) {
        & taskkill.exe /T /F /PID $p.Id *> $null
        return [pscustomobject]@{ Exit = -1; Secs = $sw.Elapsed.TotalSeconds }
    }
    [pscustomobject]@{ Exit = $p.ExitCode; Secs = $sw.Elapsed.TotalSeconds }
}

# A slot dir held by a live process (a sleeping pwsh), as a running job would hold it.
function New-Holder([string]$Dir) {
    $h = Start-Process pwsh -ArgumentList '-NoProfile', '-Command', 'Start-Sleep 300' -PassThru -WindowStyle Hidden
    $script:children += $h
    [void](New-AtomicDir $Dir ([ordered]@{ winpid = $h.Id; winstart = $h.StartTime.ToUniversalTime().ToString('o'); what = '00:00:00 other :: holder' }))
    $h
}

# Lets go of a holder made by New-Holder: its process ends and its slot dir goes.
function Release-Holder($Holder, [string]$Dir) {
    Stop-Process -Id $Holder.Id -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $Dir -Recurse -Force -ErrorAction SilentlyContinue
}

# The probe's record, after a job (empty fields when the job never ran the probe).
function Read-Probe([string]$Out) {
    if (Test-Path -LiteralPath $Out) { Get-Content -LiteralPath $Out -Raw | ConvertFrom-Json }
    else { [pscustomobject]@{ exists = $null; jobs = $null; token = $null } }
}

# The job-log lines written so far, parsed.
function Read-Jobs {
    if (-not (Test-Path -LiteralPath $jobsFile)) { return @() }
    @(Get-Content -LiteralPath $jobsFile | Where-Object { $_ } | ForEach-Object { $_ | ConvertFrom-Json })
}

try {
    # --- 1. the classifier against the shared fixture ---------------------------------
    $rows = @(Get-Content -LiteralPath $fixture | Where-Object { $_ -and -not $_.StartsWith('#') })
    Check ($rows.Count -ge 30) "the fixture has at least 30 rows (has $($rows.Count))"
    $bad = @()
    foreach ($r in $rows) {
        $parts = $r -split "`t", 2
        $got = Get-LaneClass ($parts[1] -split ' ')
        if ($got -ne $parts[0]) { $bad += "'$($parts[1])' is $got, want $($parts[0])" }
    }
    Check ($bad.Count -eq 0) "every fixture row classifies as its class $($bad -join '; ')"

    # --- 2. counts, slot paths and nesting, as units ----------------------------------
    # This process's own environment must not leak a count into the defaults checked here.
    Remove-Item Env:LANE_SLOTS, Env:LANE_LIGHT_SLOTS, Env:LANE_DB_SLOTS, Env:LANE_LIGHT_JOBS -ErrorAction SilentlyContinue
    Check ((Get-LaneClassCount $lock 'heavy') -eq 2 -and (Get-LaneClassCount $lock 'light') -eq 4 -and (Get-LaneClassCount $lock 'db') -eq 3) 'defaults: heavy 2 (no SLOTS file here), light 4, db 3'
    Set-Content -LiteralPath (Join-Path $lock 'LIGHT_SLOTS') -Value '5'
    Check ((Get-LaneClassCount $lock 'light') -eq 5) 'lane\LIGHT_SLOTS is read'
    $env:LANE_LIGHT_SLOTS = 'junk'
    Check ((Get-LaneClassCount $lock 'light') -eq 5) 'a junk LANE_LIGHT_SLOTS counts as unset (the file wins)'
    $env:LANE_LIGHT_SLOTS = '0'
    Check ((Get-LaneClassCount $lock 'light') -eq 0) 'LANE_LIGHT_SLOTS=0 is the kill switch (count 0)'
    Remove-Item Env:LANE_LIGHT_SLOTS
    Remove-Item -LiteralPath (Join-Path $lock 'LIGHT_SLOTS')
    $env:LANE_SLOTS = '0'
    Check ((Get-LaneClassCount $lock 'heavy') -eq 2) 'LANE_SLOTS=0 is unset for heavy (heavy never has 0 slots)'
    Remove-Item Env:LANE_SLOTS
    Check ((Get-LaneSlotPath $lock 'heavy' 3) -eq (Join-Path $lock 'slot.3') -and (Get-LaneSlotPath $lock 'light' 1) -eq (Join-Path $lock 'slot.light.1') -and (Get-LaneSlotPath $lock 'db' 2) -eq (Join-Path $lock 'slot.db.2')) 'slot dirs: slot.N, slot.light.N, slot.db.N'
    Check ((Get-LaneClassJobs 'heavy' 100) -eq 4 -and (Get-LaneClassJobs 'heavy' 1) -eq [math]::Max(4, [Environment]::ProcessorCount)) 'heavy jobs: max(4, cores / slots)'
    Check ((Get-LaneClassJobs 'light' 6) -eq 4) 'light jobs: 4 by default'
    $env:CIMMERIA_LANE_SLOT = Join-Path $lock 'slot.9'
    $env:CIMMERIA_LANE_TOKEN = 'abc'
    New-Item -ItemType Directory $env:CIMMERIA_LANE_SLOT | Out-Null
    [System.IO.File]::WriteAllText((Join-Path $env:CIMMERIA_LANE_SLOT 'token'), "other`n")
    Check ($null -eq (Test-LaneNested)) 'a different token is not nested'
    [System.IO.File]::WriteAllText((Join-Path $env:CIMMERIA_LANE_SLOT 'token'), "abc`n")
    Check ((Test-LaneNested) -eq $env:CIMMERIA_LANE_SLOT) 'the matching token is nested'
    Remove-Item Env:CIMMERIA_LANE_SLOT, Env:CIMMERIA_LANE_TOKEN

    # --- 3. a light job runs beside a busy heavy slot ---------------------------------
    $h1 = New-Holder (Join-Path $lock 'slot.1')
    $c2 = Join-Path $tmp 'c2.json'
    $r = Invoke-Pwsh @('-NoProfile', '-File', $lane, '--class', 'light', 'pwsh', '-NoProfile', '-File', $probe, $c2, (Join-Path $lock 'slot.light.1')) @{ LANE_SLOTS = '1' } 60
    Check ($r.Exit -eq 0 -and $r.Secs -lt 15) ("a light job runs with slot.1 held, in {0:N1}s (exit {1})" -f $r.Secs, $r.Exit)
    Check ((Read-Probe $c2).exists) 'slot.light.1 existed during the light job'
    Release-Holder $h1 (Join-Path $lock 'slot.1')

    # --- 4. a heavy job runs with the light slots held --------------------------------
    $h2 = New-Holder (Join-Path $lock 'slot.light.1')
    $c3 = Join-Path $tmp 'c3.json'
    $r = Invoke-Pwsh @('-NoProfile', '-File', $lane, 'pwsh', '-NoProfile', '-File', $probe, $c3, 'none') @{ LANE_SLOTS = '1'; LANE_LIGHT_SLOTS = '1' } 60
    Check ($r.Exit -eq 0 -and $r.Secs -lt 15) ("a heavy job runs with slot.light.1 held, in {0:N1}s (exit {1})" -f $r.Secs, $r.Exit)
    Release-Holder $h2 (Join-Path $lock 'slot.light.1')

    # --- 5. LANE_LIGHT_SLOTS=0 sends a light job to heavy -----------------------------
    $c4 = Join-Path $tmp 'c4.json'
    $r = Invoke-Pwsh @('-NoProfile', '-File', $lane, '--class', 'light', 'pwsh', '-NoProfile', '-File', $probe, $c4, (Join-Path $lock 'slot.1')) @{ LANE_SLOTS = '1'; LANE_LIGHT_SLOTS = '0' } 60
    Check ($r.Exit -eq 0 -and (Read-Probe $c4).exists) 'with LANE_LIGHT_SLOTS=0 the light job takes slot.1'
    $last = (Read-Jobs)[-1]
    Check ($last.class -eq 'heavy' -and $last.class_req -eq 'light') "and logs class=heavy, class_req=light (got $($last.class), $($last.class_req))"

    # --- 5b. a db job runs through the real lane on slot.db.1, and a db count of 0 sends it to heavy
    $c5 = Join-Path $tmp 'c5.json'
    $r = Invoke-Pwsh @('-NoProfile', '-File', $lane, '--class', 'db', 'pwsh', '-NoProfile', '-File', $probe, $c5, (Join-Path $lock 'slot.db.1')) @{} 60
    Check ($r.Exit -eq 0 -and (Read-Probe $c5).exists) 'a db job takes slot.db.1'
    $j = (Read-Jobs)[-1]
    Check ($j.class -eq 'db' -and $j.class_slots_total -eq 3 -and $j.jobs -eq 4) 'and logs class=db, class_slots_total=3, jobs=4'
    $c5b = Join-Path $tmp 'c5b.json'
    $r = Invoke-Pwsh @('-NoProfile', '-File', $lane, '--class', 'db', 'pwsh', '-NoProfile', '-File', $probe, $c5b, (Join-Path $lock 'slot.1')) @{ LANE_DB_SLOTS = '0'; LANE_SLOTS = '1' } 60
    Check ($r.Exit -eq 0 -and (Read-Probe $c5b).exists) 'with LANE_DB_SLOTS=0 the db job takes slot.1'
    $j = (Read-Jobs)[-1]
    Check ($j.class -eq 'heavy' -and $j.class_req -eq 'db') "and logs class=heavy, class_req=db (got $($j.class), $($j.class_req))"

    # --- 6. a lane call inside a lane job is nested: LANE_SLOTS=1 must not hang ----------
    $outer = @('-NoProfile', '-File', $lane, 'pwsh', '-NoProfile', '-File', $lane, 'pwsh', '-NoProfile', '-File', $exit7)
    $r = Invoke-Pwsh $outer @{ LANE_SLOTS = '1' } 45
    Check ($r.Exit -eq 7 -and $r.Secs -lt 30) ("a lane job that calls the lane again finishes in {0:N1}s and passes exit 7 out (exit {1})" -f $r.Secs, $r.Exit)
    $nestedLines = @(Read-Jobs | Where-Object { $_.nested })
    Check ($nestedLines.Count -eq 1 -and $nestedLines[0].slots -eq 0 -and $nestedLines[0].class -eq 'heavy') 'the inner call is logged nested: true, slots 0, class heavy'

    # --- 7. the job log carries the class fields ---------------------------------------
    $c7 = Join-Path $tmp 'c7.json'
    $r = Invoke-Pwsh @('-NoProfile', '-File', $lane, '--class', 'light', 'pwsh', '-NoProfile', '-File', $probe, $c7, 'none') @{} 60
    $j = (Read-Jobs)[-1]
    $names = @($j.PSObject.Properties.Name)
    $fields = 'class', 'class_req', 'class_slots_total', 'busy_class_at_start', 'nested'
    Check (@($fields | Where-Object { $names -notcontains $_ }).Count -eq 0) 'the jobs.jsonl line carries class, class_req, class_slots_total, busy_class_at_start and nested'
    Check ($j.class -eq 'light' -and $j.class_req -eq 'light' -and $j.class_slots_total -eq 4 -and $j.nested -eq $false -and $j.slots -eq 1) 'with values light, light, 4, false, slots 1'

    # --- 8. CARGO_BUILD_JOBS seen by a light job is 4 -----------------------------------
    $c8 = Join-Path $tmp 'c8.json'
    $r = Invoke-Pwsh @('-NoProfile', '-File', $lane, '--class', 'light', 'pwsh', '-NoProfile', '-File', $probe, $c8, 'none') @{} 60
    Check ((Read-Probe $c8).jobs -eq '4') "a light job sees CARGO_BUILD_JOBS=4 (got $((Read-Probe $c8).jobs))"

    # --- 9. flags: a bogus class and --exclusive with light exit 2 ---------------------
    $r = Invoke-Pwsh @('-NoProfile', '-File', $lane, '--class', 'bogus', 'pwsh', '-NoProfile', '-Command', 'exit 0') @{} 30
    Check ($r.Exit -eq 2) "--class bogus exits 2 (got $($r.Exit))"
    $r = Invoke-Pwsh @('-NoProfile', '-File', $lane, '--exclusive', '--class', 'light', 'pwsh', '-NoProfile', '-Command', 'exit 0') @{} 30
    Check ($r.Exit -eq 2) "--exclusive --class light exits 2 (got $($r.Exit))"
    $r = Invoke-Pwsh @('-NoProfile', '-File', $lane, '--class=light', 'pwsh', '-NoProfile', '-Command', 'exit 0') @{} 60
    Check ($r.Exit -eq 0) "--class=light is accepted (exit $($r.Exit))"
    $r = Invoke-Pwsh @('-NoProfile', '-File', $lane, '--class', 'LIGHT', 'pwsh', '-NoProfile', '-Command', 'exit 0') @{} 60
    Check ((Read-Jobs)[-1].class -ceq 'light') '--class LIGHT is logged as light'
} finally {
    foreach ($c in $children) { if (-not $c.HasExited) { Stop-Process -Id $c.Id -Force -ErrorAction SilentlyContinue } }
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed -gt 0) { Write-Host "$($script:failed) check(s) failed"; exit 1 }
Write-Host 'all lane-class checks passed'
exit 0
