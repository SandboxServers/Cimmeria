<#
.SYNOPSIS
  Removes stale build artifacts from every Cimmeria target dir on this machine.

.DESCRIPTION
  Cargo never deletes old artifacts from a target dir. Every change of Rust version,
  feature set or flags leaves another full copy of each affected crate behind, so target
  dirs grow without bound (the main checkout reached 171 GB, with 157 distinct builds of
  one crate). This script uses cargo-sweep (`cargo install --locked cargo-sweep`) to:

    1. keep only artifacts built by the pinned toolchain (rust-toolchain.toml), and
    2. delete artifacts not used for -Days days,

  in the main checkout, every worktree under .claude/worktrees, and every per-worktree
  dir under $env:CIMMERIA_TARGET_ROOT (the Dev Drive, tools/dev-drive/). With
  -RemoveOrphans it also deletes Dev Drive target dirs whose worktree no longer exists,
  and with -RemoveLegacyTargets it deletes worktree-local target\ dirs once builds have
  moved to the Dev Drive.

  Safe to run while nothing is building. Don't run it during a build: cargo-sweep may
  delete files a running build is about to use.

.EXAMPLE
  pwsh tools/build-hygiene/sweep.ps1 -DryRun
  pwsh tools/build-hygiene/sweep.ps1 -Days 14
#>
param(
    [int] $Days = 14,
    [switch] $DryRun,
    [switch] $RemoveOrphans,
    [switch] $RemoveLegacyTargets
)
$ErrorActionPreference = 'Stop'
$main = Split-Path (Resolve-Path (git rev-parse --git-common-dir)).Path
if (-not (Get-Command cargo-sweep -ErrorAction SilentlyContinue)) { throw 'cargo-sweep not found: cargo install --locked cargo-sweep' }
$channel = (Select-String -Path (Join-Path $main 'rust-toolchain.toml') -Pattern '^channel\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
$host_triple = ((rustc -vV) | Select-String '^host: (.+)$').Matches[0].Groups[1].Value
$keep = "$channel-$host_triple"
$dry = if ($DryRun) { @('--dry-run') } else { @() }

function Get-Size($p) {
    if (-not (Test-Path $p)) { return 0 }
    $o = robocopy $p NUL /L /S /NJH /BYTES /NFL /NDL /NC /NP /R:0 /W:0 2>$null
    [double](((($o | Select-String 'Bytes :').Line) -split '\s+')[3])
}

# A project for cargo-sweep is a dir with Cargo.toml and a target dir it can find.
$projects = @($main) + @(Get-ChildItem (Join-Path $main '.claude\worktrees') -Directory -ErrorAction SilentlyContinue | ForEach-Object FullName)
$targets = [ordered]@{}
foreach ($p in $projects) { if (Test-Path (Join-Path $p 'target')) { $targets[$p] = Join-Path $p 'target' } }
if ($env:CIMMERIA_TARGET_ROOT -and (Test-Path $env:CIMMERIA_TARGET_ROOT)) {
    foreach ($d in Get-ChildItem $env:CIMMERIA_TARGET_ROOT -Directory) {
        $wt = if ($d.Name -eq (Split-Path $main -Leaf)) { $main } else { Join-Path $main ".claude\worktrees\$($d.Name)" }
        if (Test-Path $wt) { $targets["$wt (dev drive)"] = $d.FullName }
        elseif ($RemoveOrphans) {
            Write-Host "orphan (no worktree): $($d.FullName) - $([math]::Round((Get-Size $d.FullName)/1GB,1)) GB"
            if (-not $DryRun) { Remove-Item $d.FullName -Recurse -Force }
        }
    }
}

$before = 0; $after = 0
foreach ($k in $targets.Keys) {
    $t = $targets[$k]
    $b = Get-Size $t; $before += $b
    # cargo-sweep takes the project dir and honours CARGO_TARGET_DIR for where the target is.
    $proj = ($k -replace ' \(dev drive\)$', '')
    $env:CARGO_TARGET_DIR = $t
    try {
        cargo sweep @dry --toolchains $keep $proj | Out-Null
        cargo sweep @dry --time $Days $proj | Out-Null
    }
    finally { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue }
    # cargo-sweep leaves rustc's incremental caches alone, and they are usually the
    # largest part of a warm target dir. Drop per-crate session dirs not written for
    # -Days days (rustc rebuilds a missing cache on the next build).
    foreach ($inc in Get-ChildItem $t -Directory -Recurse -Depth 2 -Filter incremental -ErrorAction SilentlyContinue) {
        foreach ($crate in Get-ChildItem $inc.FullName -Directory) {
            if ($crate.LastWriteTime -lt (Get-Date).AddDays(-$Days)) {
                if (-not $DryRun) { cmd /c "rmdir /s /q `"$($crate.FullName)`"" }
            }
        }
    }
    $a = Get-Size $t; $after += $a
    '{0,8:N1} GB -> {1,8:N1} GB  {2}' -f ($b / 1GB), ($a / 1GB), $k
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
