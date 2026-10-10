# Retire worktrees whose work has merged: the counterpart of mk-worktree.ps1, and the
# PowerShell 7 twin of rm-worktree.sh (same options, checks, output and exit codes).
#
# Usage:
#   pwsh tools/build-lane/rm-worktree.ps1 [--dry-run] [--force] [--prune] <name> [<name>...]
#   pwsh tools/build-lane/rm-worktree.ps1 [--dry-run] [--prune] --merged
#
# <name> is the folder under .claude/worktrees/. --merged retires every worktree there
# whose branch's PR has merged and that nothing has touched for $RM_WORKTREE_MIN_IDLE
# minutes (default 30), then deletes Dev Drive target dirs no registered worktree owns.
#
# For each worktree it deletes the build output ($CIMMERIA_TARGET_ROOT/<name> and
# <worktree>/target) and its lane job logs, unlinks the external/ junction, runs
# `git worktree remove`, deletes the local branch, and drops the worktree's test database
# (sgw_<name> and its slot clones) when the bundled Postgres is reachable. It refuses,
# unless --force, when the worktree has uncommitted changes or is locked, or its branch's
# PR is open, closed unmerged or absent with unpushed commits. A lane job building there
# right now always refuses it, --force or not.
#
# external/ is unlinked with a non-recursive rmdir before anything else (a recursive
# delete can follow the junction and empty the real external/), and the main checkout's
# external/ is checked afterwards. `git worktree prune` runs only with --prune: on
# 2026-10-03 a prune deleted the entries of about 22 live worktrees. See rm-worktree.sh
# for the reasons behind each check.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Continue'
# Escape embedded quotes (`DROP DATABASE "x"`) for every program, .cmd shims included;
# the default 'Windows' mode passes them unescaped to batch files.
$PSNativeCommandArgumentPassing = 'Standard'
. (Join-Path $PSScriptRoot 'lane-slots.ps1')

$Dry = $false; $Force = $false; $Merged = $false; $Prune = $false
$Names = [System.Collections.Generic.List[string]]::new()
foreach ($a in $args) {
    switch -CaseSensitive ($a) {
        '--dry-run' { $Dry = $true; continue }
        '--force' { $Force = $true; continue }
        '--merged' { $Merged = $true; continue }
        '--prune' { $Prune = $true; continue }
        '--no-prune' { $Prune = $false; continue }
        { $_ -in '-h', '--help' } {
            Get-Content -LiteralPath $PSCommandPath -TotalCount 23 | ForEach-Object { $_ -replace '^# ?', '' }
            exit 0
        }
        { $_.StartsWith('-') } { [Console]::Error.WriteLine("unknown option: $a"); exit 2 }
        default { $Names.Add($a) }
    }
}
if (-not $Merged -and $Names.Count -eq 0) {
    [Console]::Error.WriteLine('usage: rm-worktree.ps1 [--dry-run] [--force] [--prune] <name>... | --merged'); exit 2
}

# Paths in the form `git worktree list` prints them (C:/... on Windows), so they compare.
$Main = git rev-parse --path-format=absolute --git-common-dir
if (-not $Main) { exit 1 }
$Main = (Split-Path -Parent $Main) -replace '\\', '/'
$WtRoot = "$Main/.claude/worktrees"
$LaneRoot = if ($env:LANE_ROOT) { $env:LANE_ROOT }
            elseif ($env:LOCALAPPDATA) { Join-Path $env:LOCALAPPDATA 'cimmeria-build' }
            else { Join-Path $HOME '.local/share/cimmeria-build' }
$TRoot = $env:CIMMERIA_TARGET_ROOT
if (-not $TRoot) { $TRoot = [Environment]::GetEnvironmentVariable('CIMMERIA_TARGET_ROOT', 'User') }

function Remove-Tree([string]$Path) {
    if ($Dry) { Write-Output "  would: rm -rf $Path"; return }
    Remove-Item -LiteralPath $Path -Recurse -Force -ErrorAction SilentlyContinue
}

$Psql = if ($env:PSQL) { $env:PSQL } else { "$Main/external/postgresql_server/bin/psql.exe" }
if (-not (Test-Path -LiteralPath $Psql)) {
    $c = Get-Command psql -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    $Psql = if ($c) { $c.Source } else { '' }
}
function Get-DbName([string]$n) { 'sgw_' + ($n -replace '[^A-Za-z0-9_]', '_').ToLowerInvariant() }
function Get-WorktreePaths { @(git worktree list --porcelain | Where-Object { $_.StartsWith('worktree ') } | ForEach-Object { $_.Substring(9) }) }

function Remove-TestDatabase([string]$n) {
    $db = Get-DbName $n
    if (-not $Psql) { return }
    # foo-bar and foo_bar both map to sgw_foo_bar: keep a database another worktree maps to.
    $others = @(Get-WorktreePaths | ForEach-Object { Split-Path -Leaf $_ } | Where-Object { $_ -ne $n })
    foreach ($o in $others) {
        if ((Get-DbName $o) -eq $db) { Write-Output "  kept database ${db}: worktree $o maps to it too"; return }
    }
    $taken = @($others | ForEach-Object { Get-DbName $_ })
    $port = if ($env:PGPORT) { $env:PGPORT } else { '5433' }
    $q = @('-h', 'localhost', '-p', $port, '-U', 'w-testing', '-d', 'postgres', '-tAq', '-v', 'ON_ERROR_STOP=1')
    $savedPw = $env:PGPASSWORD
    if (-not $env:PGPASSWORD) { $env:PGPASSWORD = 'w-testing' }
    try {
        # The database and its live-DB slot clones (<db>_0 ..), as tools/test-live-db.* make them.
        $list = @(& $Psql @q -c "SELECT datname FROM pg_database WHERE datname='$db' OR datname ~ '^${db}_[0-9]+`$' ORDER BY datname" 2>$null)
        foreach ($name in $list) {
            $name = "$name".Trim()
            if (-not $name) { continue }
            # A worktree named foo_1 owns sgw_foo_1, which looks like a slot clone of sgw_foo.
            if ($taken -contains $name) { Write-Output "  kept database ${name}: another worktree maps to it"; continue }
            if ($Dry) { Write-Output "  would: drop database $name"; continue }
            & $Psql @q -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname='$name' AND pid<>pg_backend_pid();" *> $null
            & $Psql @q -c "DROP DATABASE `"$name`";" *> $null
            if ($LASTEXITCODE -eq 0) { Write-Output "  dropped database $name" }
            else { [Console]::Error.WriteLine("  could not drop database $name") }
        }
    } finally { $env:PGPASSWORD = $savedPw }
}

# Each held slot's `what` reads "HH:MM:SS <worktree> :: <command>". A slot without one yet
# can't be attributed, so it counts as building (the lane checks the mark after `what`).
function Test-Building([string]$n) {
    foreach ($d in Get-SlotDirs (Join-Path $LaneRoot 'lane')) {
        $w = Read-SlotFile $d.FullName 'what'
        $fields = @($w -split '\s+' | Where-Object { $_ })
        if ($fields.Count -lt 2 -or $fields[1] -eq $n) { return $true }
    }
    $false
}

# MERGED, OPEN, CLOSED, NO_PR, ADVANCED, ON_MAIN or UNMERGED. ADVANCED: a merged PR whose branch
# has local commits past the head that merged.
function Get-MergeState([string]$br) {
    $st = ''; $head = ''
    if (Get-Command gh -ErrorAction SilentlyContinue) {
        $line = gh pr list --state all --head $br --limit 1 --json 'state,headRefOid' -q '.[0] | "\(.state) \(.headRefOid)"' 2>$null | Select-Object -First 1
        if ($line) { $st, $head = "$line".Trim() -split ' ', 2 }
        # No PR (jq renders `.[0].state` of [] as null). Not ON_MAIN: a fresh worker branch
        # with no commits yet is an ancestor of origin/main too, and --merged must keep it.
        if ($st -eq 'null') { $st = 'NO_PR' }
    }
    if ($st -eq 'MERGED') {
        git merge-base --is-ancestor $br $head 2>$null
        if ($LASTEXITCODE -ne 0) { return 'ADVANCED' }
    }
    if ($st) { return $st }
    git merge-base --is-ancestor $br origin/main 2>$null
    if ($LASTEXITCODE -eq 0) { 'ON_MAIN' } else { 'UNMERGED' }
}

$MinIdle = if ($env:RM_WORKTREE_MIN_IDLE) { [int]$env:RM_WORKTREE_MIN_IDLE } else { 30 }
# A commit (HEAD's commit time), a checkout (the HEAD file) or a build (the target dir) in
# the last $MinIdle minutes. Not the index or the reflog, which `git status`/`git gc` touch.
function Test-RecentlyUsed([string]$wt, [string]$n) {
    $gd = git -C $wt rev-parse --absolute-git-dir 2>$null
    if (-not $gd) { return $false }
    $cut = [DateTimeOffset]::UtcNow.AddMinutes(-$MinIdle)
    $ct = git -C $wt log -1 --format=%ct HEAD 2>$null
    if ($ct -and [DateTimeOffset]::FromUnixTimeSeconds([long]$ct) -gt $cut) { return $true }
    $h = Get-Item -LiteralPath (Join-Path $gd 'HEAD') -ErrorAction SilentlyContinue
    if ($h -and $h.LastWriteTimeUtc -gt $cut.UtcDateTime) { return $true }
    if ($TRoot -and (Test-Path -LiteralPath (Join-Path $TRoot $n))) {
        $t = Join-Path $TRoot $n
        $recent = @(Get-Item -LiteralPath $t) + @(Get-ChildItem -LiteralPath $t -Force -ErrorAction SilentlyContinue) +
                  @(Get-ChildItem -LiteralPath $t -Directory -Force -ErrorAction SilentlyContinue | Get-ChildItem -Force -ErrorAction SilentlyContinue)
        if ($recent | Where-Object { $_.LastWriteTimeUtc -gt $cut.UtcDateTime } | Select-Object -First 1) { return $true }
    }
    $false
}

# Mark first, check the slots after; the lane takes its slot first and checks the mark
# after, so one always sees the other. A dry run changes nothing, so it takes no mark.
function Set-RetiringMark([string]$n) {
    if ($Dry) { return $true }
    $lock = Join-Path $LaneRoot 'lane'
    [void][System.IO.Directory]::CreateDirectory($lock)
    New-AtomicDir (Join-Path $lock "retiring.$n")
}
function Clear-RetiringMark([string]$n) {
    if (-not $Dry) { try { [System.IO.Directory]::Delete((Join-Path $LaneRoot "lane/retiring.$n"), $false) } catch { } }
}

function Test-NonEmptyDir([string]$p) {
    (Test-Path -LiteralPath $p -PathType Container) -and [bool](Get-ChildItem -LiteralPath $p -Force -ErrorAction SilentlyContinue | Select-Object -First 1)
}

$script:Retired = 0; $script:Skipped = 0
function Skip([string]$msg) { Write-Output $msg; $script:Skipped++ }
function SkipErr([string]$msg) { [Console]::Error.WriteLine($msg); $script:Skipped++ }

function Invoke-Retire([string]$n, [string]$mode = '') {
    if (-not (Set-RetiringMark $n)) { Skip "skip ${n}: another rm-worktree is retiring it"; return }
    try { Invoke-RetireMarked $n $mode } finally { Clear-RetiringMark $n }
}

function Invoke-RetireMarked([string]$n, [string]$mode) {
    $wt = "$WtRoot/$n"
    $porcelain = @(git worktree list --porcelain)
    if ($porcelain -notcontains "worktree $wt") { Skip "skip ${n}: not a registered worktree under .claude/worktrees"; return }
    if ($mode -eq 'sweep' -and (Test-RecentlyUsed $wt $n)) {
        Skip "skip ${n}: used in the last $MinIdle min (name it explicitly to retire it now)"; return
    }
    if (Test-Building $n) { Skip "skip ${n}: a lane job is building there now"; return }
    $locked = $false; $in = $false
    foreach ($l in $porcelain) {
        if ($l -eq "worktree $wt") { $in = $true; continue }
        if ($l.StartsWith('worktree ')) { $in = $false }
        if ($in -and $l.StartsWith('locked')) { $locked = $true }
    }
    if ($locked -and -not $Force) { Skip "skip ${n}: locked, an agent may still be using it (--force unlocks it)"; return }
    $br = git -C $wt symbolic-ref --short -q HEAD 2>$null
    if ((git -C $wt status --porcelain 2>$null) -and -not $Force) { Skip "skip ${n}: uncommitted changes (commit them, or --force)"; return }
    $state = if ($br) { Get-MergeState $br } else { 'DETACHED' }
    if ($state -notin 'MERGED', 'ON_MAIN' -and -not $Force) {
        Skip "skip ${n}: branch $(if ($br) { $br } else { '(detached)' }) is $state (--force to retire anyway)"; return
    }

    Write-Output "retire $n ($(if ($br) { $br } else { 'detached' }), $state)"
    # The junction goes first, before any recursive delete runs near it. cmd's rmdir
    # without /s removes a junction (never its target) or an empty directory, nothing else.
    $mainExt = Test-NonEmptyDir "$Main/external"
    $ext = "$wt/external"
    if (Test-Path -LiteralPath $ext) {
        $winExt = $ext -replace '/', '\'
        if ($Dry) { Write-Output "  would: cmd /c rmdir $winExt" } else { & cmd /d /c rmdir "$winExt" 2>$null }
    }
    if (-not $Dry -and (Test-Path -LiteralPath $ext)) {
        SkipErr '  external/ is still there; stopping before git removes the worktree'; return
    }
    if ($mainExt -and -not (Test-NonEmptyDir "$Main/external")) {
        SkipErr "  the main checkout's external/ is gone or empty after unlinking ${n}'s; stopping"; return
    }
    if ($TRoot -and (Test-Path -LiteralPath (Join-Path $TRoot $n) -PathType Container)) { Remove-Tree (Join-Path $TRoot $n) }
    if (Test-Path -LiteralPath "$wt/target" -PathType Container) { Remove-Tree "$wt/target" }
    $logs = Join-Path $LaneRoot "logs/$n"
    if (Test-Path -LiteralPath $logs -PathType Container) { Remove-Tree $logs }
    if ($locked) { if ($Dry) { Write-Output "  would: git worktree unlock $wt" } else { git worktree unlock $wt } }
    $rmArgs = @('worktree', 'remove'); if ($Force) { $rmArgs += '--force' }
    if ($Dry) { Write-Output "  would: git $($rmArgs -join ' ') $wt" }
    else {
        git @rmArgs $wt
        if ($LASTEXITCODE -ne 0) { SkipErr '  git worktree remove failed; keeping the branch and the test database'; return }
    }
    if ($state -in 'MERGED', 'ON_MAIN' -and $br) {
        if ($Dry) { Write-Output "  would: git branch -D -q $br" } else { git branch -D -q $br }
    }
    Remove-TestDatabase $n
    $script:Retired++
}

Set-Location -LiteralPath $Main
# Without gh, merge state comes from origin/main; a stale one could call unmerged work merged.
git fetch -q origin
if ($LASTEXITCODE -ne 0) {
    [Console]::Error.WriteLine('git fetch origin failed; not deciding what has merged from a stale origin/main'); exit 1
}

if ($Merged) {
    foreach ($wt in Get-WorktreePaths) {
        if ($wt.StartsWith("$WtRoot/")) { Invoke-Retire (Split-Path -Leaf $wt) 'sweep' }
    }
    # Target dirs no registered worktree owns any more (the worktree was removed by hand).
    if ($TRoot -and (Test-Path -LiteralPath $TRoot -PathType Container)) {
        $owned = @(Get-WorktreePaths | ForEach-Object { Split-Path -Leaf $_ })
        foreach ($d in @(Get-ChildItem -LiteralPath $TRoot -Directory -ErrorAction SilentlyContinue)) {
            $n = $d.Name
            if ($owned -contains $n) { continue }
            if (-not (Set-RetiringMark $n)) { Write-Output "skip orphan target ${n}: another rm-worktree is on it"; continue }
            try {
                if (Test-Building $n) { Write-Output "skip orphan target ${n}: building"; continue }
                Write-Output "orphan target $n (no worktree)"; Remove-Tree $d.FullName
            } finally { Clear-RetiringMark $n }
        }
    }
} else {
    foreach ($n in $Names) { Invoke-Retire $n }
}

if (-not $Dry -and $Prune) { git worktree prune }
if ($Dry) { Write-Output "dry run: would retire $script:Retired, skip $script:Skipped" }
else { Write-Output "retired $script:Retired, skipped $script:Skipped" }
