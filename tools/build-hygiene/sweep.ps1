<#
.SYNOPSIS
  Removes stale build artifacts from every Cimmeria target dir on this machine.

.DESCRIPTION
  Cargo never deletes old artifacts from a target dir. Every change of Rust version,
  feature set or flags leaves another full copy of each affected crate behind, so target
  dirs grow without bound (the main checkout reached 171 GB, with 157 distinct builds of
  one crate). In the main checkout, every worktree under .claude/worktrees, and every
  per-worktree dir under $env:CIMMERIA_TARGET_ROOT (the Dev Drive, tools/dev-drive/),
  this script:

    1. keeps only artifacts built by the pinned toolchain (rust-toolchain.toml), and
       deletes artifacts not used for -Days days, with cargo-sweep
       (`cargo install --locked cargo-sweep`);
    2. prunes rustc's incremental caches, which cargo-sweep leaves alone and which are
       most of a warm target dir: the stale sessions rustc keeps next to the newest one
       in every unit dir, and whole unit dirs not compiled for -IncrementalHours hours.
       Neither ever makes cargo rebuild anything; a unit whose cache is gone compiles
       from scratch the next time it changes;
    3. deletes feature variants: build units (a crate built with another feature set,
       profile or dependency graph, for example before a rebase) that no build has read
       for -VariantHours hours, going by the files' last-access time. The most recently
       used variant of each package is always kept. A deleted variant that is needed
       again is rebuilt, from sccache for third-party crates. Skipped when the volume
       doesn't record last-access times (fsutil behavior query disablelastaccess).

  With -RemoveOrphans it also deletes Dev Drive target dirs whose worktree no longer
  exists, and with -RemoveLegacyTargets it deletes worktree-local target\ dirs once builds
  have moved to the Dev Drive. -Only limits it to the named worktrees.

  A target dir a lane job is building in (tools/build-lane/lane.sh) is skipped. Don't run
  it while anything else builds: cargo-sweep may delete files a running build is about to
  use.

.EXAMPLE
  pwsh tools/build-hygiene/sweep.ps1 -DryRun
  pwsh tools/build-hygiene/sweep.ps1 -Days 14
  pwsh tools/build-hygiene/sweep.ps1 -Only my-worktree -IncrementalHours 4
#>
param(
    [int] $Days = 14,
    [double] $IncrementalHours = 24,
    [double] $VariantHours = 24,
    [string[]] $Only,
    [switch] $DryRun,
    [switch] $RemoveOrphans,
    [switch] $RemoveLegacyTargets
)
$ErrorActionPreference = 'Stop'
$Only = @($Only | ForEach-Object { $_ -split ',' } | Where-Object { $_ })   # `pwsh -File` passes "a,b" as one string
$main = Split-Path (Resolve-Path (git rev-parse --git-common-dir)).Path
if (-not (Get-Command cargo-sweep -ErrorAction SilentlyContinue)) { throw 'cargo-sweep not found: cargo install --locked cargo-sweep' }
$channel = (Select-String -Path (Join-Path $main 'rust-toolchain.toml') -Pattern '^channel\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
$host_triple = ((rustc -vV) | Select-String '^host: (.+)$').Matches[0].Groups[1].Value
$keep = "$channel-$host_triple"
# Always an array: `if` unwraps a one-element array to a string, and splatting a string
# hands cargo-sweep one argument per character.
$dry = @(if ($DryRun) { '--dry-run' })
$now = Get-Date

function Get-Size($p) {
    if (-not (Test-Path $p)) { return 0 }
    $o = robocopy $p NUL /L /S /NJH /BYTES /NFL /NDL /NC /NP /R:0 /W:0 2>$null
    [double](((($o | Select-String 'Bytes :').Line) -split '\s+')[3])
}
function Get-TreeBytes($p) {
    if (Test-Path $p -PathType Leaf) { return (Get-Item $p).Length }
    [double]((Get-ChildItem $p -File -Recurse -Force -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum)
}
# Deletes a file or dir and returns the bytes it held (what it would free, under -DryRun).
function Remove-Stale($p) {
    if (-not (Test-Path $p)) { return 0 }
    $b = Get-TreeBytes $p
    if (-not $DryRun) {
        if (Test-Path $p -PathType Container) { cmd /c "rmdir /s /q `"$p`"" 2>$null } else { Remove-Item $p -Force -ErrorAction SilentlyContinue }
    }
    $b
}

# Worktrees a lane job is building in right now: slot.N/what is "HH:MM:SS <worktree> :: <cmd>".
$laneDir = Join-Path ($env:LANE_ROOT ?? (Join-Path $env:LOCALAPPDATA 'cimmeria-build')) 'lane'
$building = @(Get-ChildItem $laneDir -Directory -Filter 'slot.*' -ErrorAction SilentlyContinue | ForEach-Object {
    $w = Join-Path $_.FullName 'what'
    if (Test-Path $w) { ((Get-Content $w -TotalCount 1) -split ' ')[1] }
})

# Feature-variant pruning needs last-access times: 1 and 3 mean the volume doesn't keep them.
$atime = (fsutil behavior query disablelastaccess 2>$null | Select-String '=\s*(\d)').Matches
$variantsOn = -not ($atime -and $atime[0].Groups[1].Value -in '1', '3')
if (-not $variantsOn) { Write-Warning 'last-access times are off (fsutil behavior query disablelastaccess): skipping feature variants' }

# A project for cargo-sweep is a dir with Cargo.toml and a target dir it can find.
$projects = @($main) + @(Get-ChildItem (Join-Path $main '.claude\worktrees') -Directory -ErrorAction SilentlyContinue | ForEach-Object FullName)
if ($Only) { $projects = @($projects | Where-Object { (Split-Path $_ -Leaf) -in $Only }) }
$targets = [ordered]@{}
foreach ($p in $projects) { if (Test-Path (Join-Path $p 'target')) { $targets[$p] = Join-Path $p 'target' } }
if ($env:CIMMERIA_TARGET_ROOT -and (Test-Path $env:CIMMERIA_TARGET_ROOT)) {
    foreach ($d in Get-ChildItem $env:CIMMERIA_TARGET_ROOT -Directory) {
        if ($Only -and $d.Name -notin $Only) { continue }
        $wt = if ($d.Name -eq (Split-Path $main -Leaf)) { $main } else { Join-Path $main ".claude\worktrees\$($d.Name)" }
        if (Test-Path $wt) { $targets["$wt (dev drive)"] = $d.FullName }
        elseif ($RemoveOrphans) {
            Write-Host "orphan (no worktree): $($d.FullName) - $([math]::Round((Get-Size $d.FullName)/1GB,1)) GB"
            if (-not $DryRun) { Remove-Item $d.FullName -Recurse -Force }
        }
    }
}

# rustc keeps the session it started from next to the one it just wrote, in every
# incremental unit dir, and loads only the newest finished one. A `-working` session is a
# compile in progress, or one that died.
function Remove-StaleIncremental($t) {
    $freed = 0
    foreach ($inc in Get-ChildItem $t -Directory -Recurse -Depth 2 -Filter incremental -ErrorAction SilentlyContinue) {
        foreach ($unit in Get-ChildItem $inc.FullName -Directory) {
            $sessions = @(Get-ChildItem $unit.FullName -Directory -Filter 's-*' | Sort-Object Name -Descending)
            $finished = @($sessions | Where-Object Name -notlike '*-working')
            $newest = if ($finished) { $finished[0].LastWriteTime } else { $unit.LastWriteTime }
            if ($newest -lt $now.AddHours(-$IncrementalHours)) { $freed += Remove-Stale $unit.FullName; continue }
            $stale = @($finished | Select-Object -Skip 1) + @($sessions | Where-Object { $_.Name -like '*-working' -and $_.LastWriteTime -lt $now.AddHours(-1) })
            foreach ($s in $stale) {
                $freed += Remove-Stale $s.FullName
                $lock = Join-Path $unit.FullName (($s.Name -replace '-[^-]+$', '') + '.lock')
                if (Test-Path $lock) { $freed += Remove-Stale $lock }
            }
        }
    }
    $freed
}

# A build unit is .fingerprint\<package>-<hash>, with its outputs in deps\*-<hash>* and
# build\<package>-<hash>. Cargo reads a unit's fingerprint files in every build that
# includes the unit, so their last-access time says when a build last used it.
function Remove-StaleVariants($t) {
    $freed = 0
    foreach ($fp in Get-ChildItem $t -Directory -Recurse -Depth 2 -Filter .fingerprint -Force -ErrorAction SilentlyContinue) {
        $profileDir = $fp.Parent.FullName
        $units = foreach ($u in Get-ChildItem $fp.FullName -Directory) {
            if ($u.Name -notmatch '^(.+)-([0-9a-f]{16})$') { continue }
            $files = @(Get-ChildItem $u.FullName -File -Force)
            $used = if ($files) { ($files | Measure-Object LastAccessTime -Maximum).Maximum } else { $u.LastWriteTime }
            [pscustomobject]@{ Dir = $u.FullName; Package = $Matches[1]; Hash = $Matches[2]; Used = $used }
        }
        foreach ($group in $units | Group-Object Package) {
            $old = $group.Group | Sort-Object Used -Descending | Select-Object -Skip 1 |
                Where-Object { $_.Used -lt $now.AddHours(-$VariantHours) }
            foreach ($u in $old) {
                $freed += Remove-Stale $u.Dir
                $freed += Remove-Stale (Join-Path $profileDir "build\$($u.Package)-$($u.Hash)")
                foreach ($f in Get-ChildItem (Join-Path $profileDir 'deps') -Filter "*-$($u.Hash)*" -ErrorAction SilentlyContinue) {
                    $freed += Remove-Stale $f.FullName
                }
            }
        }
    }
    $freed
}

$before = 0; $after = 0
foreach ($k in $targets.Keys) {
    $t = $targets[$k]
    $name = Split-Path ($k -replace ' \(dev drive\)$', '') -Leaf
    if ($name -in $building) { Write-Host "skipped (a lane job is building in it): $k"; continue }
    $b = Get-Size $t; $before += $b
    # cargo-sweep takes the project dir and honours CARGO_TARGET_DIR for where the target is.
    $proj = ($k -replace ' \(dev drive\)$', '')
    $env:CARGO_TARGET_DIR = $t
    try {
        cargo sweep @dry --toolchains $keep $proj | Out-Null
        cargo sweep @dry --time $Days $proj | Out-Null
    }
    finally { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue }
    $inc = Remove-StaleIncremental $t
    $var = if ($variantsOn) { Remove-StaleVariants $t } else { 0 }
    # Under -DryRun nothing was deleted, so "after" is the size minus what would go
    # (cargo-sweep's own share isn't counted then).
    $a = if ($DryRun) { $b - $inc - $var } else { Get-Size $t }
    $after += $a
    '{0,8:N1} GB -> {1,8:N1} GB  (incremental {2:N1} GB, variants {3:N1} GB)  {4}' -f ($b / 1GB), ($a / 1GB), ($inc / 1GB), ($var / 1GB), $k
}

if ($RemoveLegacyTargets -and $env:CIMMERIA_TARGET_ROOT) {
    foreach ($p in $projects) {
        $legacy = Join-Path $p 'target'
        if (Test-Path $legacy) {
            Write-Host "legacy target dir: $legacy - $([math]::Round((Get-Size $legacy)/1GB,1)) GB"
            if (-not $DryRun) { Remove-Item $legacy -Recurse -Force }
        }
    }
}
'{0:N1} GB -> {1:N1} GB total{2}' -f ($before / 1GB), ($after / 1GB), $(if ($DryRun) { ' (dry run: nothing deleted)' } else { '' })
