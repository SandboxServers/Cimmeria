# Merge one local branch into the current branch of one worktree with --no-ff, for `ops`.
# A conflict or any failed merge step runs `git merge --abort` (only when a merge is in
# progress) and exits 1, so the worktree is left as it was before the call. Hooks still run.
#
#   pwsh tools/build-lane/merge-noff.ps1 -C <worktree> -Branch <name> [-ExpectBranch <current>]
#
# Refuses (exit 2) when -C is not the root of a non-main worktree, HEAD is detached, -Branch
# is not a valid name or no local branch of that name, -Branch is the current branch, the current
# branch is not -ExpectBranch (when given; compared case-sensitively), or the tree is dirty (any
# `git status --porcelain` output). A parameter-binding error (a missing or unknown parameter,
# such as a second positional token) prints no status= line and exits 1.
# Prints status=ok merge=<sha> branch=<name>, status=up-to-date merge=<sha> branch=<name> when
# <name> is already contained in HEAD, or status=failed reason=<why> files=<conflicted paths>.
# Exit codes: 0 merged or up to date, 2 refused by a guard, 1 the merge failed (aborted).
[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Mandatory)][string]$C,
    [Parameter(Mandatory)][string]$Branch,
    [string]$ExpectBranch = ''
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
if ((Get-FullPath $main) -ieq (Get-FullPath $C)) { Stop-Refused 'main-checkout: refuse to merge in the main checkout' }

$headRef = (& git -C $C symbolic-ref -q HEAD 2>$null)
if ($LASTEXITCODE -ne 0) { Stop-Refused 'detached-head: a merge there is held by no branch' }
# symbolic-ref, not `rev-parse --abbrev-ref`: the latter answers `heads/<b>` when a tag has the same name.
$current = ($headRef.Trim() -replace '^refs/heads/', '')
if ($ExpectBranch -and $current -cne $ExpectBranch) { Stop-Refused "branch-mismatch: HEAD is $current, not $ExpectBranch" }

& git -C $C check-ref-format --branch $Branch *> $null
if ($LASTEXITCODE -ne 0) { Stop-Refused "bad-branch-name: $Branch" }
& git -C $C show-ref --verify --quiet "refs/heads/$Branch"
if ($LASTEXITCODE -ne 0) { Stop-Refused "no-such-local-branch: $Branch" }
if ($Branch -eq $current) { Stop-Refused "same-branch: $Branch is the current branch" }

$dirty = (& git -C $C status --porcelain | Out-String).Trim()
if ($dirty) { Stop-Refused 'dirty-tree: commit or remove these first' }

& git -C $C merge-base --is-ancestor "refs/heads/$Branch" HEAD
if ($LASTEXITCODE -eq 0) {
    $sha = (& git -C $C rev-parse HEAD).Trim()
    Write-Output "status=up-to-date merge=$sha branch=$Branch"
    exit 0
}

$null = & git -C $C merge --no-ff --no-edit "refs/heads/$Branch" 2>&1
if ($LASTEXITCODE -ne 0) {
    $files = (& git -C $C diff --name-only --diff-filter=U | Out-String).Trim() -replace "`r?`n", ','
    & git -C $C rev-parse -q --verify MERGE_HEAD *> $null
    if ($LASTEXITCODE -eq 0) { & git -C $C merge --abort }
    Write-Output "status=failed reason=merge-aborted files=$files"
    exit 1
}

$sha = (& git -C $C rev-parse HEAD).Trim()
Write-Output "status=ok merge=$sha branch=$Branch"
exit 0
