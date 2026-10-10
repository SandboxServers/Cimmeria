# Create a worktree off origin/main under .claude/worktrees/<name>, ready to build.
# PowerShell 7 twin of mk-worktree.sh.
#
# Usage: pwsh tools/build-lane/mk-worktree.ps1 <branch> <name>
#
#  * Junctions the gitignored external/ tree in (crates/entity/build.rs compiles Detour
#    from ../../external/recast, so a bare worktree cannot build).
#  * When CIMMERIA_TARGET_ROOT points at a Dev Drive (tools/dev-drive/), seeds the new
#    worktree's target dir from the warmest existing one with ReFS block cloning, so the
#    first build is incremental instead of cold, at almost no disk cost.
$ErrorActionPreference = 'Stop'
if ($args.Count -lt 2) { [Console]::Error.WriteLine('usage: mk-worktree.ps1 <branch> <name>'); exit 2 }
$Branch = [string]$args[0]; $Name = [string]$args[1]

$common = git rev-parse --path-format=absolute --git-common-dir
if ($LASTEXITCODE -ne 0) { exit 1 }
$Main = Split-Path -Parent $common
$Wt = Join-Path $Main ".claude/worktrees/$Name"
Set-Location -LiteralPath $Main
git fetch -q origin
if ($LASTEXITCODE -ne 0) { exit 1 }
if (Test-Path -LiteralPath $Wt) { Write-Output "exists: $Wt"; exit 0 }
git worktree add -q -b $Branch $Wt origin/main
if ($LASTEXITCODE -ne 0) { exit 1 }
New-Item -ItemType Junction -Path (Join-Path $Wt 'external') -Target (Join-Path $Main 'external') | Out-Null
Write-Output "created $Wt on branch $Branch (external/ junctioned)"

if ($env:CIMMERIA_TARGET_ROOT) {
    & pwsh -NoProfile -File (Join-Path $Main 'tools/dev-drive/Copy-WarmTarget.ps1') -TargetRoot $env:CIMMERIA_TARGET_ROOT -Name $Name
    if ($LASTEXITCODE -ne 0) { [Console]::Error.WriteLine('warning: target seeding failed; the first build will be cold') }
}
