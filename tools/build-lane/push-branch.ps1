# Push one local branch to origin for `ops`. A plain push is `git push -u origin refs/heads/<b>:refs/heads/<b>`.
# With -Force it is the same with an explicit lease, `--force-with-lease=refs/heads/<b>:<sha>`, where
# <sha> is the remote tip the caller last saw (-ExpectRemoteSha). A bare --force is never run, and a
# lease without an expected value is never used: it trusts the remote-tracking ref, which any fetch
# refreshes, and would then overwrite commits pushed by someone else.
#
#   pwsh tools/build-lane/push-branch.ps1 -C <worktree> -Branch <name> [-Force -ExpectRemoteSha <40-hex>]
#
# Refuses (exit 2) when -C is not the root of a non-main worktree, HEAD is detached, -Branch is not a
# valid branch name or does not start with a letter, digit or `_`, -Branch is main or master (the same protected names ship.py uses), -Branch
# is not the worktree's current branch (compared case-sensitively), -Force is given without a 40 lowercase hex
# -ExpectRemoteSha, or -ExpectRemoteSha is given without -Force. Git's own refusals (a rejected non-fast-forward,
# a lease that no longer matches) are a failed push. Prints status=ok branch=<name> sha=<sha>, or
# status=refused|failed reason=<why>. Exit codes: 0 pushed, 2 refused by a guard, 1 the push failed.
# A parameter-binding error (a missing or unknown parameter) prints no status= line and exits 1.
[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Mandatory)][string]$C,
    [Parameter(Mandatory)][string]$Branch,
    [switch]$Force,
    [string]$ExpectRemoteSha = ''
)
$ErrorActionPreference = 'Stop'

# Variables that redirect git to another repository, index, config or transport; the guards below
# would be checked against one repository while git acts on another.
foreach ($v in 'GIT_DIR', 'GIT_WORK_TREE', 'GIT_INDEX_FILE', 'GIT_COMMON_DIR', 'GIT_NAMESPACE', 'GIT_CONFIG_COUNT', 'GIT_CONFIG_PARAMETERS', 'GIT_SSH_COMMAND', 'GIT_EXEC_PATH') {
    Remove-Item -LiteralPath "Env:$v" -ErrorAction SilentlyContinue
}

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

$headRef = (& git -C $C symbolic-ref -q HEAD 2>$null)
if ($LASTEXITCODE -ne 0) { Stop-Refused 'detached-head: a push of HEAD there names no branch' }
# symbolic-ref, not `rev-parse --abbrev-ref`: the latter answers `heads/<b>` when a tag has the same name.
$current = ($headRef.Trim() -replace '^refs/heads/', '')

if ($Branch -in 'main', 'master') { Stop-Refused "protected-branch: $Branch" }
if ($Branch -cnotmatch '^[A-Za-z0-9_]') { Stop-Refused "bad-branch-name: $Branch (must start with a letter, digit or underscore: a leading + would force the push)" }
& git -C $C check-ref-format --branch $Branch *> $null
if ($LASTEXITCODE -ne 0) { Stop-Refused "bad-branch-name: $Branch" }
if ($Branch -cne $current) { Stop-Refused "not-current-branch: $Branch is not the worktree's branch $current" }

if ($Force -and $ExpectRemoteSha -cnotmatch '^[0-9a-f]{40}$') {
    Stop-Refused 'force-needs-expect-remote-sha: -Force needs -ExpectRemoteSha <40 lowercase hex>, the remote tip the caller last saw'
}
if (-not $Force -and $ExpectRemoteSha) { Stop-Refused 'expect-remote-sha-needs-force: -ExpectRemoteSha only goes with -Force' }

if ($Force) {
    $pushArgs = @('push', '-u', "--force-with-lease=refs/heads/${Branch}:$ExpectRemoteSha", 'origin', "refs/heads/${Branch}:refs/heads/$Branch")
} else {
    $pushArgs = @('push', '-u', 'origin', "refs/heads/${Branch}:refs/heads/$Branch")
}
$null = & git -C $C @pushArgs 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Output "status=failed reason=git-push branch=$Branch"
    exit 1
}

$sha = (& git -C $C rev-parse HEAD).Trim()
Write-Output "status=ok branch=$Branch sha=$sha"
exit 0
