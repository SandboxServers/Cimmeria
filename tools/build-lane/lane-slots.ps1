# The build lane's slot protocol, for lane.ps1 and rm-worktree.ps1 (dot-source this file).
#
# lane.sh and lane.ps1 share one lock directory, $LANE_ROOT/lane, and must exclude each
# other. The protocol is lane.sh's:
#  * a held slot is the directory slot.N; whoever creates it holds it;
#  * `what` inside it reads "HH:MM:SS <worktree> :: <command>" (rm-worktree.* and the
#    busy message parse it);
#  * a holder whose process is dead may be broken by any lane.
#
# Two things have no exact PowerShell equivalent, and are done this way instead:
#
#  1. `mkdir` as the atomic test-and-set. New-Item and [IO.Directory]::CreateDirectory
#     both succeed on a directory that already exists, so two lanes could both "take" a
#     slot. lane.ps1 instead builds the slot complete under a private staging name and
#     renames it into place: MoveFile fails when the destination exists, and NTFS
#     arbitrates that against Git Bash's mkdir (NtCreateFile with FILE_CREATE) on the
#     same name, so exactly one of any mix of bash and PowerShell lanes wins. The
#     staging name starts with a dot, so the slot.* globs never see it, and the slot
#     appears with its holder files already in it.
#
#  2. The holder's pid. lane.sh writes `pid` = bash's $$, an MSYS pid: msys-2.0.dll hands
#     out its own pids from a shared counter, unrelated to Windows pids, and `kill -0`
#     only knows MSYS pids. So lane.ps1 never writes `pid` (lane.sh would find our
#     Windows pid "dead" and break a live slot); it writes `winpid` (and `winstart`, the
#     process start time, against pid reuse). lane.sh asks Windows PowerShell whether that
#     process still runs (its WINPID_PROBE) and breaks the slot only when it doesn't, so
#     mutual exclusion holds both ways and either lane reclaims a dead lane.ps1 holder.
#     The one gap: lane.ps1 cannot test an MSYS pid, so it breaks a lane.sh slot only when
#     no Git Bash / MSYS shell process is running at all (a live lane.sh holder is itself
#     a running bash.exe). Otherwise the bash lanes reclaim it.
#
# Breaking a dead holder's slot renames it to a private name first and deletes only what
# it moved, after checking the moved holder is the one it judged dead: another lane may
# have broken and retaken the slot in between, and a plain delete would remove theirs.

Set-StrictMode -Version Latest

# Forward-slash form (C:/x), the form lane.sh's `cygpath -m` gives native programs.
function ConvertTo-LanePath([string]$Path) { if ($Path) { $Path -replace '\\', '/' } else { $Path } }

# Create $Path atomically with $Files (name -> content) already inside. $true if this call
# created it, $false if it already existed (or the parent is missing).
function New-AtomicDir([string]$Path, [System.Collections.IDictionary]$Files = @{}) {
    $parent = Split-Path -Parent $Path
    $stage = Join-Path $parent (".stage-{0}-{1}" -f $PID, [guid]::NewGuid().ToString('N').Substring(0, 8))
    try {
        [void][System.IO.Directory]::CreateDirectory($stage)
        foreach ($k in $Files.Keys) {
            [System.IO.File]::WriteAllText((Join-Path $stage $k), [string]$Files[$k] + "`n")
        }
    } catch {
        Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
        return $false
    }
    try {
        # Win32 MoveFile, without MOVEFILE_REPLACE_EXISTING: fails if $Path exists.
        [System.IO.Directory]::Move($stage, $Path)
        return $true
    } catch {
        Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
        return $false
    }
}

# Staging and broken-slot dirs left by a lane.ps1 that was killed mid-rename.
function Clear-StaleStaging([string]$LockDir) {
    foreach ($d in @(Get-ChildItem -LiteralPath $LockDir -Directory -Force -ErrorAction SilentlyContinue | Where-Object Name -Match '^\.(stage|broken)-')) {
        if ($d.Name -match '^\.(?:stage|broken)-(\d+)-' -and -not (Get-Process -Id ([int]$Matches[1]) -ErrorAction SilentlyContinue)) {
            Remove-Item -LiteralPath $d.FullName -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

function Read-SlotFile([string]$Slot, [string]$Name) {
    try { ([System.IO.File]::ReadAllText((Join-Path $Slot $Name))).Trim() } catch { '' }
}

# True when some running process could be a Git Bash / MSYS shell, i.e. a lane.sh holder.
# WSL's bash.exe (System32, WindowsApps) runs lane.sh against a Linux $HOME, never this
# lock dir, so it doesn't count. A shell whose path we can't read counts (be safe).
function Test-MsysShellRunning {
    $sys = $env:SystemRoot
    foreach ($p in @(Get-Process -Name bash, sh -ErrorAction SilentlyContinue)) {
        $path = $null
        try { $path = $p.Path } catch { }
        if (-not $path) { return $true }
        if ($sys -and $path.StartsWith($sys, [StringComparison]::OrdinalIgnoreCase)) { continue }
        if ($path -match '\\WindowsApps\\') { continue }
        return $true
    }
    $false
}

# 'alive', 'dead' or 'unknown' (no holder files yet: a lane.sh between its mkdir and its
# `pid` write; treat as busy, as lane.sh does).
function Get-SlotHolderState([string]$Slot) {
    $winpid = Read-SlotFile $Slot 'winpid'
    if ($winpid -match '^\d+$') {
        $p = Get-Process -Id ([int]$winpid) -ErrorAction SilentlyContinue
        if (-not $p) { return 'dead' }
        $start = Read-SlotFile $Slot 'winstart'
        if ($start) {
            try {
                $rec = [datetime]::Parse($start, $null, [System.Globalization.DateTimeStyles]::RoundtripKind)
                if ([math]::Abs(($p.StartTime.ToUniversalTime() - $rec.ToUniversalTime()).TotalSeconds) -gt 2) { return 'dead' }   # pid reused
            } catch { }
        }
        return 'alive'
    }
    $msyspid = Read-SlotFile $Slot 'pid'
    if ($msyspid) {
        if (Test-MsysShellRunning) { return 'alive' } else { return 'dead' }
    }
    'unknown'
}

# lane.sh's try_slot: take $Slot, or break it if its holder is dead and then take it.
function Enter-LaneSlot([string]$Slot, [string]$What) {
    $me = Get-Process -Id $PID
    $files = [ordered]@{ winpid = $PID; winstart = $me.StartTime.ToUniversalTime().ToString('o'); what = $What }
    if (New-AtomicDir $Slot $files) { return $true }
    $dead = Get-SlotIdentity $Slot
    if ((Get-SlotHolderState $Slot) -ne 'dead') { return $false }
    # Move it aside, then make sure what moved is the holder judged dead; if another lane
    # broke and retook the slot in between, put theirs back and leave it alone.
    $aside = Join-Path (Split-Path -Parent $Slot) (".broken-{0}-{1}" -f $PID, [guid]::NewGuid().ToString('N').Substring(0, 8))
    try { [System.IO.Directory]::Move($Slot, $aside) } catch { return $false }
    if ((Get-SlotIdentity $aside) -ne $dead) {
        try { [System.IO.Directory]::Move($aside, $Slot) } catch { }
        return $false
    }
    $who = (Read-SlotFile $aside 'winpid'), (Read-SlotFile $aside 'pid') -ne '' | Select-Object -First 1
    [Console]::Error.WriteLine("[lane] breaking stale slot $(Split-Path -Leaf $Slot) held by dead pid $who")
    Remove-Item -LiteralPath $aside -Recurse -Force -ErrorAction SilentlyContinue
    New-AtomicDir $Slot $files
}

# Who holds a slot, as one string: its holder files' contents.
function Get-SlotIdentity([string]$Slot) {
    (('pid', 'winpid', 'winstart', 'what') | ForEach-Object { Read-SlotFile $Slot $_ }) -join '|'
}

function Exit-LaneSlot([string]$Slot) {
    Remove-Item -LiteralPath $Slot -Recurse -Force -ErrorAction SilentlyContinue
}

function Get-SlotDirs([string]$LockDir) {
    @(Get-ChildItem -LiteralPath $LockDir -Directory -Filter 'slot.*' -ErrorAction SilentlyContinue)
}
