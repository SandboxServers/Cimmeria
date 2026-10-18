# Create one local branch at a given full commit SHA, for `ops`. It runs `git branch <name> <sha>`
# and nothing else: no checkout, no fetch, no push, and never an existing branch overwritten.
#
#   pwsh tools/build-lane/branch-at.ps1 -C <repo> -Name <branch> -Sha <40-hex>
#
# <repo> is the main checkout or any worktree of the repository. Refuses (exit 2) when <sha> is
# not 40 lowercase hex digits or names no commit, <name> is not a valid branch name, <name> is
# ambiguous (looks like a hex hash of 7 to 40 digits, names an existing tag, starts with `+`, `-` or `refs/`, is `@`, is HEAD, FETCH_HEAD, ORIG_HEAD, MERGE_HEAD or CHERRY_PICK_HEAD in any case, is all upper-case letters, digits and `_` (pseudo-ref shaped: REBASE_HEAD, AUTO_MERGE), or its first
# segment is a configured remote or `heads`, `tags` or `remotes`: a local `origin/main` would
# shadow the remote-tracking ref in later rev lookups), or refs/heads/<name> already exists. Prints status=ok branch=<name> sha=<sha>, or
# status=refused reason=<why> / status=failed reason=<why>. Exit codes: 0 created, 2 refused,
# 1 git failed.
[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Mandatory)][string]$C,
    [Parameter(Mandatory)][string]$Name,
    [Parameter(Mandatory)][string]$Sha
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

if (-not (Test-Path -LiteralPath $C -PathType Container)) { Stop-Refused "not-a-directory: $C" }
& git -C $C rev-parse --git-dir *> $null
if ($LASTEXITCODE -ne 0) { Stop-Refused "not-a-git-repository: $C" }

if ($Sha -cnotmatch '^[0-9a-f]{40}$') { Stop-Refused "sha-not-full-40-hex: $Sha" }
& git -C $C rev-parse --verify --quiet "$Sha^{commit}" *> $null
if ($LASTEXITCODE -ne 0) { Stop-Refused "no-such-commit: $Sha" }

if ($Name.StartsWith('-') -or $Name.StartsWith('+')) { Stop-Refused "bad-branch-name: $Name" }
if ($Name -cmatch '^[0-9a-f]{7,40}$') { Stop-Refused "ambiguous-branch-name: $Name looks like a commit hash" }
& git -C $C show-ref --verify --quiet "refs/tags/$Name"
if ($LASTEXITCODE -eq 0) { Stop-Refused "ambiguous-branch-name: $Name is an existing tag" }
& git -C $C check-ref-format --branch $Name *> $null
if ($LASTEXITCODE -ne 0) { Stop-Refused "bad-branch-name: $Name" }
# Git's top-level pseudo-refs (REBASE_HEAD, AUTO_MERGE, REVERT_HEAD, BISECT_HEAD ...) are all-caps
# and win over refs/heads/ in a rev lookup. Ordinary lower-case names such as `feat` stay valid.
if ($Name -match '^refs/' -or $Name -cmatch '^[A-Z][A-Z0-9_]*$' -or $Name -iin '@', 'HEAD', 'FETCH_HEAD', 'ORIG_HEAD', 'MERGE_HEAD', 'CHERRY_PICK_HEAD') { Stop-Refused "ambiguous-branch-name: $Name" }
$first = ($Name -split '/')[0]
$remotes = @(& git -C $C remote)
$reserved = @('heads', 'tags', 'remotes') + $remotes
if ($Name.Contains('/') -or $remotes -contains $Name) {
    if ($reserved -contains $first) { Stop-Refused "ambiguous-branch-name: $Name shadows a ref namespace or a remote" }
}
& git -C $C show-ref --verify --quiet "refs/heads/$Name"
if ($LASTEXITCODE -eq 0) { Stop-Refused "branch-exists: $Name is not overwritten" }

$null = & git -C $C branch $Name $Sha 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Output "status=failed reason=git-branch"
    exit 1
}
Write-Output "status=ok branch=$Name sha=$Sha"
exit 0
