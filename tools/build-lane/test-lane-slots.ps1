<#
.SYNOPSIS
    Tests for the PowerShell build lane's slot protocol (lane-slots.ps1) and lane.ps1's
    slot accounting, against a scratch LANE_ROOT. No Pester needed; exits non-zero on a
    failure. Runs no cargo and no bash.

        pwsh -NoProfile -File tools/build-lane/test-lane-slots.ps1
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lane-slots.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("lane-test-" + [guid]::NewGuid().ToString('N'))
$lock = Join-Path $tmp 'lane'
New-Item -ItemType Directory $lock | Out-Null
$lane = Join-Path $PSScriptRoot 'lane.ps1'
$children = @()
try {
    # --- the atomic create ---------------------------------------------------------
    $d = Join-Path $lock 'x'
    Check (New-AtomicDir $d @{ what = 'one' }) 'first create wins'
    Check (-not (New-AtomicDir $d @{ what = 'two' })) 'second create of the same name loses'
    Check ((Read-SlotFile $d 'what') -eq 'one') 'the loser did not touch the winner''s files'
    Check (@(Get-ChildItem $lock -Force -Filter '.stage-*').Count -eq 0) 'no staging dirs left behind'

    # A dir made the way Git Bash's mkdir makes it (no rename) also blocks the create.
    $m = Join-Path $lock 'bashmade'
    [void][System.IO.Directory]::CreateDirectory($m)
    Check (-not (New-AtomicDir $m)) 'a plain mkdir-made dir blocks the create'

    # Many racers, one name: exactly one wins.
    $race = Join-Path $lock 'race'
    $lib = Join-Path $PSScriptRoot 'lane-slots.ps1'
    $wins = 1..16 | ForEach-Object -ThrottleLimit 16 -Parallel {
        . $using:lib
        New-AtomicDir $using:race @{ what = "$_" }
    }
    Check (@($wins | Where-Object { $_ }).Count -eq 1) '16 parallel racers: exactly one wins'

    # --- holder liveness -------------------------------------------------------------
    $slot = Join-Path $lock 'slot.1'
    $live = Start-Process pwsh -ArgumentList '-NoProfile', '-Command', 'Start-Sleep 120' -PassThru -WindowStyle Hidden
    $children += $live
    $liveStart = $live.StartTime.ToUniversalTime().ToString('o')
    [void](New-AtomicDir $slot ([ordered]@{ winpid = $live.Id; winstart = $liveStart; what = '00:00:00 other :: cargo build' }))
    Check ((Get-SlotHolderState $slot) -eq 'alive') 'a live Windows-pid holder is alive'
    Check (-not (Enter-LaneSlot $slot '00:00:00 me :: x')) 'a live holder''s slot is not taken'
    Check ((Read-SlotFile $slot 'winpid') -eq "$($live.Id)") 'and is left alone'

    Exit-LaneSlot $slot
    [void](New-AtomicDir $slot ([ordered]@{ winpid = $live.Id; winstart = '2001-01-01T00:00:00.0000000Z'; what = 'x' }))
    Check ((Get-SlotHolderState $slot) -eq 'dead') 'a reused pid (start time differs) counts as dead'

    Exit-LaneSlot $slot
    $gone = Start-Process pwsh -ArgumentList '-NoProfile', '-Command', 'exit' -PassThru -WindowStyle Hidden
    $gone.WaitForExit()
    [void](New-AtomicDir $slot ([ordered]@{ winpid = $gone.Id; what = '00:00:00 other :: cargo build' }))
    Check ((Get-SlotHolderState $slot) -eq 'dead') 'an exited holder is dead'
    Check (Enter-LaneSlot $slot '00:00:00 me :: x') 'a dead holder''s slot is broken and taken'
    Check ((Read-SlotFile $slot 'winpid') -eq "$PID") 'and now names this process'
    Check ((Read-SlotFile $slot 'what') -eq '00:00:00 me :: x') 'with this job''s what'
    Check (-not (Test-Path (Join-Path $slot 'pid'))) 'never a `pid` file, which lane.sh would kill -0'

    # A lane.sh holder: `pid` is an MSYS pid, testable only through "is any MSYS shell up".
    Exit-LaneSlot $slot
    [void](New-AtomicDir $slot ([ordered]@{ pid = 1119; what = '00:00:00 other :: cargo build' }))
    function Test-MsysShellRunning { $true }
    Check ((Get-SlotHolderState $slot) -eq 'alive') 'a bash holder counts as alive while an MSYS shell runs'
    Check (-not (Enter-LaneSlot $slot 'x')) 'and its slot is not taken'
    function Test-MsysShellRunning { $false }
    Check ((Get-SlotHolderState $slot) -eq 'dead') 'a bash holder with no MSYS shell running is dead'
    Check (Enter-LaneSlot $slot 'x') 'and its slot is reclaimed'

    # mkdir done, `pid` not yet written: busy, as lane.sh treats it.
    Exit-LaneSlot $slot
    [void][System.IO.Directory]::CreateDirectory($slot)
    Check ((Get-SlotHolderState $slot) -eq 'unknown') 'a slot with no holder files yet is unknown'
    Check (-not (Enter-LaneSlot $slot 'x')) 'and is not taken'
    Exit-LaneSlot $slot

    # --- lane.ps1: SLOTS respected, retiring mark, dead holder reclaimed --------------
    $env:LANE_ROOT = $tmp; $env:LANE_SLOTS = '2'; $env:LANE_METRICS = '0'
    $env:LANE_MIN_FREE_GB = '0'; $env:LANE_VERBOSE = '1'; $env:SCCACHE_BIN = ''; $env:LANE_PRUNE = '0'
    Remove-Item Env:SCCACHE_BIN -ErrorAction SilentlyContinue
    $env:RUSTC_WRAPPER = 'none'   # keep the lane from building the sccache wrapper
    $marks = Join-Path $tmp 'marks'
    New-Item -ItemType Directory $marks | Out-Null
    # Each job records its start and end; the lane may run at most two at once.
    $job = "`$f = Join-Path '$marks' ([guid]::NewGuid()); [IO.File]::WriteAllText(`$f, [DateTime]::UtcNow.Ticks.ToString()); Start-Sleep -Milliseconds 2500; [IO.File]::AppendAllText(`$f, ' ' + [DateTime]::UtcNow.Ticks)"
    $jobs = 1..4 | ForEach-Object {
        Start-Process pwsh -ArgumentList '-NoProfile', '-File', $lane, 'pwsh', '-NoProfile', '-Command', $job -PassThru -WindowStyle Hidden
    }
    $jobs | ForEach-Object { $_.WaitForExit() }
    $spans = Get-ChildItem $marks | ForEach-Object { $a = (Get-Content $_.FullName).Split(' '); [pscustomobject]@{ s = [long]$a[0]; e = [long]$a[1] } }
    $peak = 0
    foreach ($s in $spans) { $peak = [math]::Max($peak, @($spans | Where-Object { $_.s -le $s.s -and $_.e -gt $s.s }).Count) }
    Check (@($spans).Count -eq 4) 'four lane jobs ran'
    Check ($peak -eq 2) "SLOTS=2: at most two ran at once (peak $peak)"
    Check (@(Get-SlotDirs $lock).Count -eq 0) 'every slot released afterwards'

    $env:LANE_SLOTS = '1'
    # A holder killed mid-job: its slot is reclaimed by the next lane.
    $holder = Start-Process pwsh -ArgumentList '-NoProfile', '-File', $lane, 'pwsh', '-NoProfile', '-Command', 'Start-Sleep 60' -PassThru -WindowStyle Hidden
    $children += $holder
    for ($i = 0; $i -lt 100 -and -not (Test-Path (Join-Path $lock 'slot.1/winpid')); $i++) { Start-Sleep -Milliseconds 100 }
    Check ((Read-SlotFile (Join-Path $lock 'slot.1') 'winpid') -eq "$($holder.Id)") 'the holder took slot.1'
    Stop-Process -Id $holder.Id -Force; $holder.WaitForExit()
    Check (Test-Path (Join-Path $lock 'slot.1')) 'a killed holder leaves its slot behind'
    $out = & pwsh -NoProfile -File $lane --exclusive pwsh -NoProfile -Command 'exit 7' 2>&1 | Out-String
    Check ($LASTEXITCODE -eq 7) "the next lane ran and passed the exit code through (got $LASTEXITCODE)"
    Check ($out -match 'breaking stale slot slot\.1 held by dead pid') 'and said it broke the stale slot'

    # rm-worktree's retiring mark stops a build in that worktree (exit 75).
    $name = Split-Path -Leaf (git rev-parse --show-toplevel)   # the lane names the cwd's worktree
    New-Item -ItemType Directory (Join-Path $lock "retiring.$name") | Out-Null
    & pwsh -NoProfile -File $lane pwsh -NoProfile -Command 'exit 0' 2>&1 | Out-Null
    Check ($LASTEXITCODE -eq 75) 'a worktree being retired is not built (exit 75)'
    Check (@(Get-SlotDirs $lock).Count -eq 0) 'and the slot it took is released'
} finally {
    $children | ForEach-Object { if (-not $_.HasExited) { Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue } }
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
if ($script:failed) { Write-Host "$script:failed failed"; exit 1 }
Write-Host 'all passed'
