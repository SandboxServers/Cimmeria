# Push one local branch to origin for `ops`. A plain push is `git push -u origin <branch>`. With
# -Force it is `git push -u --force-with-lease origin <branch>`: a bare --force is never run.
#
#   pwsh tools/build-lane/push-branch.ps1 -C <worktree> -Branch <name> [-Force]
#
# Refuses (exit 2) when -C is not the root of a non-main worktree, HEAD is detached, -Branch is not a
# valid branch name, -Branch is main or master (the same protected names ship.py uses), or -Branch
# is not the worktree's current branch. Git's own refusals (a rejected non-fast-forward, a lease
# that no longer matches) are a failed push. Prints status=ok branch=<name> sha=<sha>, or
# status=refused|failed reason=<why>. Exit codes: 0 pushed, 2 refused by a guard, 1 the push failed.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$C,
    [Parameter(Mandatory)][string]$Branch,
    [switch]$Force
)
$ErrorActionPreference = 'Stop'

function Stop-Refused([string]$reason) {
    Write-Output "status=refused reason=$reason"
    exit 2
}

function Get-FullPath([string]$p) {
    [System.IO.Path]::GetFullPath($p).TrimEnd('\', '/').Replace('\', '/')
}

if (-not (Test-Path -LiteralPath $C -PathType Container)) { Stop-Refused "not-a-directory: $C" }
$top = (& git -C $C rev-parse --show-toplevel 2>$null)
if ($LASTEXITCODE -ne 0) { Stop-Refused "not-a-git-worktree: $C" }
if ((Get-FullPath $top) -ine (Get-FullPath $C)) { Stop-Refused "not-the-worktree-root: $C" }

$main = (& git -C $C worktree list --porcelain | Select-Object -First 1) -replace '^worktree ', ''
if ((Get-FullPath $main) -ieq (Get-FullPath $C)) { Stop-Refused 'main-checkout: refuse to push from the main checkout' }

& git -C $C symbolic-ref -q HEAD *> $null
if ($LASTEXITCODE -ne 0) { Stop-Refused 'detached-head: a push of HEAD there names no branch' }
$current = (& git -C $C rev-parse --abbrev-ref HEAD).Trim()

if ($Branch -in 'main', 'master') { Stop-Refused "protected-branch: $Branch" }
& git -C $C check-ref-format --branch $Branch *> $null
if ($LASTEXITCODE -ne 0) { Stop-Refused "bad-branch-name: $Branch" }
if ($Branch -ne $current) { Stop-Refused "not-current-branch: $Branch is not the worktree's branch $current" }

if ($Force) {
    $pushArgs = @('push', '-u', '--force-with-lease', 'origin', $Branch)
} else {
    $pushArgs = @('push', '-u', 'origin', $Branch)
}
$null = & git -C $C @pushArgs 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Output "status=failed reason=git-push branch=$Branch"
    exit 1
}

$sha = (& git -C $C rev-parse HEAD).Trim()
Write-Output "status=ok branch=$Branch sha=$sha"
exit 0
