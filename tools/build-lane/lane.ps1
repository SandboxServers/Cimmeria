# Cimmeria build lane, PowerShell 7 twin of lane.sh: run a cargo command inside the
# machine-wide counting semaphore, so Windows sessions never need a bash.
#
# Usage:  pwsh tools/build-lane/lane.ps1 [--exclusive] <command> [args...]
#
# Call it through `pwsh` (as above), not with `&` from inside a PowerShell session: the
# in-session parser swallows a bare `--` (`cargo test -p x -- --nocapture`), `pwsh` keeps it.
#
# Everything lane.sh's header describes applies here, with the same environment
# variables, files and defaults: LANE_ROOT, LANE_SLOTS and the SLOTS file, --exclusive,
# CARGO_BUILD_JOBS (cores / slots, floor 4), the per-worktree target dir and
# CIMMERIA_TARGET_ROOT / CIMMERIA_FORCE_DEV_DRIVE, sccache through sccache-wrap.rs
# (CIMMERIA_SCCACHE_DIR, SCCACHE_BIN, SCCACHE_CACHE_SIZE, SCCACHE_BASEDIRS), the disk
# guard (LANE_MIN_FREE_GB, exit 28), incremental pruning (LANE_PRUNE), quiet output
# through lane_summary.py (LANE_VERBOSE, CI, LANE_LOG_KEEP, LANE_LOG_DAYS), the job log
# for lane_stats.py (LANE_METRICS, LANE_METRICS_DIR) and the rm-worktree retiring mark
# (exit 75). Read lane.sh for the reasons; this file only notes where it differs.
#
# A bash lane and a PowerShell lane share the slots and exclude each other; how, and the
# one place the two can't fully cooperate (reclaiming a dead holder of the other kind),
# is in lane-slots.ps1.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot 'lane-slots.ps1')

$LaneDir = $PSScriptRoot
$LaneRoot = if ($env:LANE_ROOT) { $env:LANE_ROOT }
            elseif ($env:LOCALAPPDATA) { Join-Path $env:LOCALAPPDATA 'cimmeria-build' }
            else { Join-Path $HOME '.local/share/cimmeria-build' }
$LockDir = Join-Path $LaneRoot 'lane'
[void][System.IO.Directory]::CreateDirectory($LockDir)
$Slots = 2
if ($env:LANE_SLOTS) { $Slots = [int]$env:LANE_SLOTS }
else { try { $Slots = [int]([System.IO.File]::ReadAllText((Join-Path $LockDir 'SLOTS')).Trim()) } catch { } }

$Cmd = @($args)
$Exclusive = $false
if ($Cmd.Count -gt 0 -and $Cmd[0] -eq '--exclusive') { $Exclusive = $true; $Cmd = @($Cmd | Select-Object -Skip 1) }
if ($Cmd.Count -eq 0) { [Console]::Error.WriteLine('usage: lane.ps1 [--exclusive] <command> [args...]'); exit 2 }
$CmdStr = $Cmd -join ' '

function NowMs { [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() }
function Secs([long]$ms) { '{0}.{1:d3}' -f [math]::Floor($ms / 1000), ($ms % 1000) }

# --- build environment --------------------------------------------------------------
$Top = (git rev-parse --show-toplevel 2>$null)
if (-not $Top) { $Top = ConvertTo-LanePath (Get-Location).Path }
$Name = Split-Path -Leaf $Top

# The Dev Drive settings are user environment variables; a session started before they
# were set doesn't have them, so read the user's registry value.
foreach ($v in 'CIMMERIA_TARGET_ROOT', 'CIMMERIA_SCCACHE_DIR') {
    if (-not [Environment]::GetEnvironmentVariable($v)) {
        $u = [Environment]::GetEnvironmentVariable($v, 'User')
        if ($u) { [Environment]::SetEnvironmentVariable($v, $u) }
    }
}

$UseDevDrive = $false
$TRoot = $env:CIMMERIA_TARGET_ROOT
if ($TRoot -and (Test-Path -LiteralPath $TRoot -PathType Container)) {
    if ((Test-Path -LiteralPath (Join-Path $TRoot $Name) -PathType Container) -or
        -not (Test-Path -LiteralPath (Join-Path $Top 'target') -PathType Container) -or
        $env:CIMMERIA_FORCE_DEV_DRIVE -eq '1') { $UseDevDrive = $true }
}
if ($UseDevDrive) { $env:CARGO_TARGET_DIR = ConvertTo-LanePath (Join-Path $TRoot $Name) }
else { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue }   # <worktree>/target

$SccacheBin = $env:SCCACHE_BIN
if (-not $SccacheBin -and (Test-Path -LiteralPath (Join-Path $LaneRoot 'bin/sccache.exe'))) { $SccacheBin = Join-Path $LaneRoot 'bin/sccache.exe' }
if (-not $SccacheBin) { $c = Get-Command sccache -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1; if ($c) { $SccacheBin = $c.Source } }
if ($env:CARGO_INCREMENTAL -and $env:CARGO_INCREMENTAL -ne '0') { $SccacheBin = '' }   # sccache aborts under CARGO_INCREMENTAL=1
$UseSccache = $false

# Build sccache-wrap.rs once per source version, into the same hash-named dir lane.sh
# uses (sha1 of the file, first 12 hex digits), so both lanes share one wrapper.
function Get-SccacheWrapper {
    $src = Join-Path $LaneDir 'sccache-wrap.rs'
    if (-not (Test-Path -LiteralPath $src)) { return '' }
    $hash = (Get-FileHash -LiteralPath $src -Algorithm SHA1).Hash.ToLowerInvariant().Substring(0, 12)
    $dir = Join-Path $LaneRoot "bin/sccache-wrap/$hash"
    $exe = Join-Path $dir 'sccache.exe'
    if (-not (Test-Path -LiteralPath $exe)) {
        $tmp = Join-Path $dir "build.$PID"
        [void][System.IO.Directory]::CreateDirectory($tmp)
        Push-Location $LaneDir
        try {
            & rustc --edition 2021 -O -C debuginfo=0 -C linker=rust-lld -C linker-flavor=lld-link -o (Join-Path $tmp 'sccache.exe') $src *> (Join-Path $tmp 'log')
            $ok = $LASTEXITCODE -eq 0
        } catch { $ok = $false } finally { Pop-Location }
        if (-not $ok) {
            [Console]::Error.WriteLine("[lane] warning: could not build $src (see $tmp/log); using sccache directly")
            return ''
        }
        try { Move-Item -LiteralPath (Join-Path $tmp 'sccache.exe') -Destination $exe -Force -ErrorAction Stop } catch { }   # a lane that raced us may have won; fine
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $exe) { ConvertTo-LanePath $exe } else { '' }
}

if ($SccacheBin -and -not (Test-Path Env:RUSTC_WRAPPER)) {
    $UseSccache = $true
    $env:SCCACHE_DIR = if ($env:CIMMERIA_SCCACHE_DIR) { $env:CIMMERIA_SCCACHE_DIR } else { Join-Path $LaneRoot 'sccache-cache' }
    if (-not $env:SCCACHE_CACHE_SIZE) { $env:SCCACHE_CACHE_SIZE = '15G' }
    $env:SCCACHE_IDLE_TIMEOUT = '0'
    # Same prefixes as lane.sh, in the same C:/x form: the sccache server reads them once
    # when it starts, whichever lane starts it.
    if (-not $env:SCCACHE_BASEDIRS) {
        $common = git rev-parse --path-format=absolute --git-common-dir 2>$null
        $mainDir = if ($common) { Split-Path -Parent $common } else { '' }
        $dirs = @($TRoot, $mainDir) | Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Container) } |
            ForEach-Object { ConvertTo-LanePath ((Resolve-Path -LiteralPath $_).ProviderPath) }
        if ($dirs) { $env:SCCACHE_BASEDIRS = $dirs -join ';' }
    }
    $wrapper = Get-SccacheWrapper
    if ($wrapper) {
        $env:CIMMERIA_SCCACHE_REAL = ConvertTo-LanePath $SccacheBin
        $env:RUSTC_WRAPPER = $wrapper
    } else {
        $env:RUSTC_WRAPPER = $SccacheBin
    }
}
if (-not $env:CARGO_BUILD_JOBS) {
    $env:CARGO_BUILD_JOBS = [string][math]::Max(4, [math]::Floor([Environment]::ProcessorCount / $Slots))
}

# --- quiet output --------------------------------------------------------------------
$Quiet = [Console]::IsOutputRedirected -and $env:LANE_VERBOSE -ne '1' -and $env:CI -ne 'true'
$LogDir = Join-Path $LaneRoot "logs/$Name"
$JobLog = ''; $FailuresFile = ''
function Initialize-JobLog {
    # Another lane's log pruning deletes empty log dirs; retry across that window.
    for ($i = 0; $i -lt 4; $i++) {
        try {
            [void][System.IO.Directory]::CreateDirectory($LogDir)
            [System.IO.File]::AppendAllText($script:JobLog, '')
            return
        } catch { if ($i -eq 3) { throw } }
    }
}
if ($Quiet) {
    $jobBase = Join-Path $LogDir ('{0}-{1}' -f (Get-Date -Format 'yyyyMMdd-HHmmss'), $PID)
    $JobLog = "$jobBase.log"; $FailuresFile = "$jobBase.failures.txt"
    Initialize-JobLog
    if (-not $env:NEXTEST_STATUS_LEVEL) { $env:NEXTEST_STATUS_LEVEL = 'fail' }
    if (-not $env:NEXTEST_SHOW_PROGRESS) { $env:NEXTEST_SHOW_PROGRESS = 'none' }
    if (-not $env:NEXTEST_FAILURE_OUTPUT) { $env:NEXTEST_FAILURE_OUTPUT = 'final' }
}

function Remove-OldLogs {
    $keep = 20; $days = 7
    if ($env:LANE_LOG_KEEP) { try { $keep = [int]$env:LANE_LOG_KEEP } catch { } }
    if ($env:LANE_LOG_DAYS) { try { $days = [int]$env:LANE_LOG_DAYS } catch { } }
    if ($keep -lt 1) { $keep = 1 }   # never the log of the job that just ran
    $logs = Join-Path $LaneRoot 'logs'
    if (-not (Test-Path -LiteralPath $logs)) { return }
    $cut = (Get-Date).AddDays(-$days)
    foreach ($f in @(Get-ChildItem -LiteralPath $logs -Directory -ErrorAction SilentlyContinue | Get-ChildItem -File -ErrorAction SilentlyContinue | Where-Object LastWriteTime -lt $cut)) {
        Remove-Item -LiteralPath $f.FullName -Force -ErrorAction SilentlyContinue
        try { [System.IO.Directory]::Delete($f.DirectoryName, $false) } catch { }   # only if now empty
    }
    $hour = (Get-Date).AddHours(-1)
    foreach ($d in @(Get-ChildItem -LiteralPath $logs -Directory -ErrorAction SilentlyContinue | Where-Object LastWriteTime -lt $hour)) {
        try { [System.IO.Directory]::Delete($d.FullName, $false) } catch { }
    }
    if (-not (Test-Path -LiteralPath $LogDir)) { return }
    foreach ($f in @(Get-ChildItem -LiteralPath $LogDir -Filter '*.log' -File | Sort-Object LastWriteTime -Descending | Select-Object -Skip $keep)) {
        Remove-Item -LiteralPath $f.FullName, ($f.FullName -replace '\.log$', '.failures.txt') -Force -ErrorAction SilentlyContinue
    }
}

# A Python that runs (on Windows `python3` is often the Store's stub, which doesn't).
function Find-Python {
    foreach ($p in 'python3', 'python', 'py') {
        if (Get-Command $p -CommandType Application -ErrorAction SilentlyContinue) {
            try { & $p -c '' *> $null; if ($LASTEXITCODE -eq 0) { return $p } } catch { }
        }
    }
    ''
}

# --- acquire ------------------------------------------------------------------------
$Held = [System.Collections.Generic.List[string]]::new()
$Sampler = $null
$Rc = 1
$SavedOutputEncoding = $null
try {
    Clear-StaleStaging $LockDir
    $what = '{0} {1} :: {2}' -f (Get-Date -Format 'HH:mm:ss'), $Name, $CmdStr
    $tRequest = NowMs
    $waited = 0
    while ($true) {
        if ($Exclusive) {
            for ($i = 1; $i -le $Slots; $i++) {
                $s = Join-Path $LockDir "slot.$i"
                if (Enter-LaneSlot $s $what) { $Held.Add($s) }
            }
            if ($Held.Count -eq $Slots) { break }
            foreach ($s in $Held) { Exit-LaneSlot $s }
            $Held.Clear()
        } else {
            for ($i = 1; $i -le $Slots; $i++) {
                $s = Join-Path $LockDir "slot.$i"
                if (Enter-LaneSlot $s $what) { $Held.Add($s); break }
            }
            if ($Held.Count -gt 0) { break }
        }
        if ($waited % 60 -eq 0) {
            $busy = foreach ($d in Get-SlotDirs $LockDir) {
                $w = Read-SlotFile $d.FullName 'what'
                if ($w) { '[{0}]' -f ($w.Substring(0, [math]::Min(120, $w.Length))) }
            }
            [Console]::Error.WriteLine("[lane] all $Slots slots busy: $($busy -join ' ') ... ${waited}s")
        }
        Start-Sleep -Seconds 5; $waited += 5
    }

    # rm-worktree marks a worktree before it checks the slots; this checks for the mark
    # after taking a slot and writing `what`, so one of the two always sees the other.
    if (Test-Path -LiteralPath (Join-Path $LockDir "retiring.$Name")) {
        [Console]::Error.WriteLine("[lane] $Name is being retired by rm-worktree; not building")
        $Rc = 75; exit $Rc   # `exit` still runs the finally below
    }

    # --- disk: incremental pruning and the low-disk guard ----------------------------
    $TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { "$Top/target" }
    $script:PrunedKb = 0
    $FreeStart = $null

    function Get-FreeGb([string]$Path) {
        $d = $Path
        while ($d -and -not (Test-Path -LiteralPath $d -PathType Container)) { $d = Split-Path -Parent $d }
        if (-not $d) { return $null }
        try { [long][math]::Floor([System.IO.DriveInfo]::new([System.IO.Path]::GetPathRoot((Resolve-Path -LiteralPath $d).ProviderPath)).AvailableFreeSpace / 1GB) } catch { $null }
    }

    function Test-OtherJobHere {
        foreach ($d in Get-SlotDirs $LockDir) {
            if ($Held -contains $d.FullName) { continue }
            if ((Read-SlotFile $d.FullName 'what').Contains(" $Name :: ")) { return $true }
        }
        $false
    }

    # lane.sh's prune_incremental: in each <target>/[<triple>/]<profile>/incremental/<unit>,
    # keep the newest finished session (names sort by their base-36 timestamp) and delete
    # the older ones with their .lock files; delete `-working` sessions over an hour old.
    function Invoke-PruneIncremental {
        if ($env:LANE_PRUNE -eq '0' -or -not (Test-Path -LiteralPath $TargetDir -PathType Container)) { return }
        if (Test-OtherJobHere) { return }
        $units = @(Get-ChildItem -LiteralPath $TargetDir -Directory -ErrorAction SilentlyContinue | ForEach-Object {
            $_ | Get-ChildItem -Directory -Filter 'incremental' -ErrorAction SilentlyContinue
            $_ | Get-ChildItem -Directory -ErrorAction SilentlyContinue | Get-ChildItem -Directory -Filter 'incremental' -ErrorAction SilentlyContinue
        } | Get-ChildItem -Directory -ErrorAction SilentlyContinue)
        $stale = [System.Collections.Generic.List[string]]::new()
        $hourAgo = (Get-Date).AddHours(-1)
        foreach ($u in $units) {
            $sessions = @($u | Get-ChildItem -Directory -Filter 's-*' -ErrorAction SilentlyContinue)
            [string[]]$names = @($sessions | Where-Object { -not $_.Name.EndsWith('-working') } | ForEach-Object Name)
            [Array]::Sort($names, [StringComparer]::Ordinal); [Array]::Reverse($names)
            $done = @($names | ForEach-Object { Get-Item -LiteralPath (Join-Path $u.FullName $_) })
            $old = @($done | Select-Object -Skip 1) + @($sessions | Where-Object { $_.Name.EndsWith('-working') -and $_.LastWriteTime -lt $hourAgo })
            foreach ($s in $old) {
                $stale.Add($s.FullName)
                $stale.Add((Join-Path $u.FullName (($s.Name -replace '-[^-]*$', '') + '.lock')))
            }
        }
        if ($stale.Count -eq 0) { return }
        $bytes = 0L
        foreach ($p in $stale) {
            $m = Get-ChildItem -LiteralPath $p -Recurse -File -Force -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum
            if ($m -and $m.Sum) { $bytes += [long]$m.Sum }
            $f = Get-Item -LiteralPath $p -Force -ErrorAction SilentlyContinue
            if ($f -and -not $f.PSIsContainer) { $bytes += $f.Length }
        }
        foreach ($p in $stale) { Remove-Item -LiteralPath $p -Recurse -Force -ErrorAction SilentlyContinue }
        $script:PrunedKb += [long][math]::Floor($bytes / 1KB)
    }

    $min = 10
    if ($null -ne $env:LANE_MIN_FREE_GB -and $env:LANE_MIN_FREE_GB -ne '') { $min = [int]$env:LANE_MIN_FREE_GB }
    $FreeStart = Get-FreeGb $TargetDir
    if ($min -ne 0 -and $null -ne $FreeStart -and $FreeStart -lt $min) {
        Invoke-PruneIncremental
        $FreeStart = Get-FreeGb $TargetDir
        if ($null -ne $FreeStart -and $FreeStart -lt $min) {
            [Console]::Error.WriteLine(@"
[lane] refusing to start: $FreeStart GB free on the drive that holds $(ConvertTo-LanePath $TargetDir),
[lane] below LANE_MIN_FREE_GB=$min. Cargo would fail part-way with "os error 112".
[lane] This job did not run. Free space, then run it again:
[lane]   pwsh tools/build-lane/rm-worktree.ps1 --merged    # retire merged worktrees (target dir, test DB)
[lane]   pwsh tools/build-hygiene/sweep.ps1 -DryRun         # then without -DryRun, while nothing builds:
[lane]                                                      # stale incremental caches and old feature variants
[lane] LANE_MIN_FREE_GB=0 turns this check off.
"@)
            $Rc = 28; exit $Rc
        }
    }

    $tStart = NowMs
    $freeShown = if ($null -ne $FreeStart) { $FreeStart } else { '?' }
    $incr = if ($env:CARGO_INCREMENTAL) { $env:CARGO_INCREMENTAL } else { 'default' }
    $acquired = "[lane] acquired $($Held.Count)/$Slots slot(s) after ${waited}s; target=$TargetDir; free=${freeShown}GB; jobs=$($env:CARGO_BUILD_JOBS); incremental=$incr :: $CmdStr"
    if ($Quiet) {
        Initialize-JobLog
        [System.IO.File]::WriteAllText($JobLog, "$acquired`n")
        [Console]::Error.WriteLine("[lane] running; log: $(ConvertTo-LanePath $JobLog)")
    } else {
        [Console]::Error.WriteLine($acquired)
    }

    # --- job log ----------------------------------------------------------------------
    $Metrics = $env:LANE_METRICS -ne '0'
    $MetricsDir = if ($env:LANE_METRICS_DIR) { $env:LANE_METRICS_DIR } else { Join-Path $LaneRoot 'metrics' }
    function Get-SccacheCounts {
        $h = $null; $m = $null
        try {
            foreach ($l in @(& $SccacheBin --show-stats 2>$null)) {
                if ($l -match '^Cache hits +(\d+)\s*$') { $h = [long]$Matches[1] }
                if ($l -match '^Cache misses +(\d+)\s*$') { $m = [long]$Matches[1] }
            }
        } catch { }
        , @($h, $m)
    }
    if ($Metrics) {
        [void][System.IO.Directory]::CreateDirectory($MetricsDir)
        $startIso = (Get-Date -Format 'yyyy-MM-ddTHH:mm:sszzz') -replace '([+-]\d\d):(\d\d)$', '$1$2'
        $commit = git rev-parse --short=10 HEAD 2>$null
        $branch = git rev-parse --abbrev-ref HEAD 2>$null
        $busyAtStart = @(Get-SlotDirs $LockDir).Count - $Held.Count
        $sc0 = if ($UseSccache) { Get-SccacheCounts } else { @($null, $null) }
        # Lowest free RAM during the job, every 2 s (lane.sh reads MemFree from /proc/meminfo).
        $memState = [hashtable]::Synchronized(@{ Min = $null; Stop = $false })
        $Sampler = Start-ThreadJob -ArgumentList $memState -ScriptBlock {
            param($st)
            while (-not $st.Stop) {
                try {
                    $kb = [long](Get-CimInstance -ClassName Win32_OperatingSystem -Property FreePhysicalMemory).FreePhysicalMemory
                    if ($null -eq $st.Min -or $kb -lt $st.Min) { $st.Min = $kb }
                } catch { }
                for ($i = 0; $i -lt 20 -and -not $st.Stop; $i++) { Start-Sleep -Milliseconds 100 }
            }
        }
    }

    # --- run --------------------------------------------------------------------------
    $exe = $Cmd[0]
    $rest = @($Cmd | Select-Object -Skip 1)
    try {
        if ($Quiet) {
            # Native output is decoded with the console's code page; cargo writes UTF-8.
            try { $SavedOutputEncoding = [Console]::OutputEncoding; [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false) } catch { $SavedOutputEncoding = $null }
            & $exe @rest *>> $JobLog
        } else {
            & $exe @rest
        }
        $Rc = if ($null -ne $LASTEXITCODE) { $LASTEXITCODE } elseif ($?) { 0 } else { 1 }
    } catch {
        $msg = "[lane] could not run ${exe}: $($_.Exception.Message)"
        if ($Quiet) { [System.IO.File]::AppendAllText($JobLog, "$msg`n") } else { [Console]::Error.WriteLine($msg) }
        $Rc = 127
    }
    $tEnd = NowMs
    $runMs = $tEnd - $tStart; $waitMs = $tStart - $tRequest
    Invoke-PruneIncremental
    $prunedNote = if ($script:PrunedKb -gt 0) { "; pruned $([math]::Floor($script:PrunedKb / 1024)) MB of stale incremental sessions" } else { '' }
    if ($Quiet) {
        [System.IO.File]::AppendAllText($JobLog, "[lane] released (exit $Rc, ran $(Secs $runMs)s$prunedNote)`n")
        $py = Find-Python
        $ok = $false
        if ($py) {
            $pyArgs = @((Join-Path $LaneDir 'lane_summary.py'), '--log', (ConvertTo-LanePath $JobLog), '--exit', $Rc,
                        '--failures', (ConvertTo-LanePath $FailuresFile), '--ran', (Secs $runMs))
            if ($script:PrunedKb -ge 1024) { $pyArgs += '--note', "pruned=$([math]::Floor($script:PrunedKb / 1024))MB" }
            & $py @pyArgs
            $ok = $LASTEXITCODE -eq 0
        }
        if (-not $ok) {
            "[lane] status=$(if ($Rc -eq 0) { 'ok' } else { 'failed' }) exit=$Rc ran=$(Secs $runMs)s (no summary: lane_summary.py did not run)"
            if ($Rc -ne 0) { Get-Content -LiteralPath $JobLog -Tail 40 }
            "log: $(ConvertTo-LanePath $JobLog)"
        }
        Remove-OldLogs
    } else {
        [Console]::Error.WriteLine("[lane] released (exit $Rc, ran $(Secs $runMs)s$prunedNote)")
    }

    if ($Metrics) {
        $minKb = $null; $totalKb = $null
        if ($Sampler) {
            $memState.Stop = $true
            [void](Wait-Job $Sampler -Timeout 5); Remove-Job $Sampler -Force; $Sampler = $null
            $minKb = $memState.Min
            try { $totalKb = [long](Get-CimInstance -ClassName Win32_OperatingSystem -Property TotalVisibleMemorySize).TotalVisibleMemorySize } catch { }
        }
        $dh = $null; $dm = $null
        if ($UseSccache) {
            $sc1 = Get-SccacheCounts
            # The counters restart with the server; skip the delta rather than go negative.
            if ($null -ne $sc0[0] -and $null -ne $sc1[0] -and $sc1[0] -ge $sc0[0]) { $dh = $sc1[0] - $sc0[0] }
            if ($null -ne $sc0[1] -and $null -ne $sc1[1] -and $sc1[1] -ge $sc0[1]) { $dm = $sc1[1] - $sc0[1] }
        }
        $job = [ordered]@{
            v = 1; t = [long][math]::Floor($tStart / 1000); start = $startIso
            worktree = $Name; branch = [string]$branch; commit = [string]$commit
            cmd = $CmdStr; exit = $Rc; wait_s = [double](Secs $waitMs); run_s = [double](Secs $runMs)
            exclusive = $Exclusive; slots = $Held.Count; slots_total = $Slots; busy_at_start = $busyAtStart
            jobs = [int]$env:CARGO_BUILD_JOBS; incremental = $incr
            dev_drive = $UseDevDrive; target = $TargetDir
            sccache = $UseSccache; sccache_hits = $dh; sccache_misses = $dm
            min_free_mb = $(if ($null -ne $minKb) { [long][math]::Floor($minKb / 1024) } else { $null })
            mem_total_mb = $(if ($null -ne $totalKb) { [long][math]::Floor($totalKb / 1024) } else { $null })
            disk_free_gb = $FreeStart; pruned_mb = [long][math]::Floor($script:PrunedKb / 1024)
            quiet = [bool]$Quiet; log = $(if ($JobLog) { ConvertTo-LanePath $JobLog } else { '' })
        }
        $line = $job | ConvertTo-Json -Compress
        # The mkdir lock lane.sh uses, taken the atomic way; a holder that died leaves it
        # behind, so give up waiting after ~5 s.
        $lock = Join-Path $MetricsDir '.lock'
        $got = $false
        for ($i = 0; $i -lt 50; $i++) { if (New-AtomicDir $lock) { $got = $true; break }; Start-Sleep -Milliseconds 100 }
        try { [System.IO.File]::AppendAllText((Join-Path $MetricsDir 'jobs.jsonl'), "$line`n") } catch { }
        if ($got) { try { [System.IO.Directory]::Delete($lock, $false) } catch { } }
    }
} finally {
    if ($null -ne $SavedOutputEncoding) { try { [Console]::OutputEncoding = $SavedOutputEncoding } catch { } }
    if ($Sampler) { $memState.Stop = $true; Remove-Job $Sampler -Force -ErrorAction SilentlyContinue }
    foreach ($s in $Held) { Exit-LaneSlot $s }
}
exit $Rc
