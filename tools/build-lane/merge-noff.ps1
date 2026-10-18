# Merge one local branch into the current branch of one worktree with --no-ff, for `ops`.
# A conflict or any failed merge step runs `git merge --abort` and exits 1, so the worktree is
# left as it was before the call. It aborts only the merge it started (none was in progress
# before, and MERGE_HEAD is the merged branch's tip); git's abort output is captured, so the
# status line is the only output. Hooks still run. Merge config that would change what is merged
# is overridden for the call: branch.<current>.mergeOptions is blanked (its -s, -X, --no-commit or
# --squash would reach the merge), merge.log, merge.branchdesc and merge.autoStash are off.
#
#   pwsh tools/build-lane/merge-noff.ps1 -C <worktree> -Branch <name> [-ExpectBranch <current>]
#
# Refuses (exit 2) when -C is not the root of a non-main worktree, a rebase, merge, cherry-pick or
# revert is in progress there (`operation-in-progress`: rebase-merge, rebase-apply, MERGE_HEAD,
# CHERRY_PICK_HEAD or REVERT_HEAD in its git dir), HEAD is detached, -Branch
# is not a valid name or no local branch of that name, -Branch is the current branch, the current
# branch is not -ExpectBranch (when given; compared case-sensitively), or the tree is dirty (any
# `git status --porcelain` output). A parameter-binding error (a missing or unknown parameter,
# such as a second positional token) prints no status= line and exits 1.
# Prints status=ok merge=<sha> branch=<name>, status=up-to-date merge=<sha> branch=<name> when
# <name> is already contained in HEAD, or status=failed reason=<why> files="<conflicted paths>" detail="<git's last output line>"
# (reason=merge-aborted, or merge-not-aborted when a merge is left that this call did not start;
# both values are quoted with `%`, `"` and `=` percent-encoded, and a `,` inside a file name is
# %2C, so a file name plants no key=value token). Exit codes: 0 merged or up to date, 2 refused by
# a guard, 1 the merge failed.
[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Mandatory)][string]$C,
    [Parameter(Mandatory)][string]$Branch,
    [string]$ExpectBranch = ''
)
$ErrorActionPreference = 'Stop'

# Variables that redirect git to another repository, index, object store, injected config or transport;
# the guards below would be checked against one repository while git acts on another. GIT_CONFIG_GLOBAL
# and GIT_CONFIG_SYSTEM are NOT cleared: they are the caller's own config choice (the tests use them to
# isolate git from the developer's config).
foreach ($v in 'GIT_DIR', 'GIT_WORK_TREE', 'GIT_INDEX_FILE', 'GIT_COMMON_DIR', 'GIT_NAMESPACE', 'GIT_OBJECT_DIRECTORY', 'GIT_ALTERNATE_OBJECT_DIRECTORIES', 'GIT_CONFIG_COUNT', 'GIT_CONFIG_PARAMETERS', 'GIT_SSH_COMMAND', 'GIT_EXEC_PATH') {
    Remove-Item -LiteralPath "Env:$v" -ErrorAction SilentlyContinue
}

function Stop-Refused([string]$reason) {
    Write-Output "status=refused reason=$reason"
    exit 2
}

function Get-FullPath([string]$p) {
    [System.IO.Path]::GetFullPath($p).TrimEnd('\', '/').Replace('\', '/')
}

# Percent-encodes `%`, `"`, `=` and control characters, so a value taken from git (a message, a
# file name) can neither close its quoted field nor plant a `key=value` token such as `status=ok`.
function ConvertTo-Field([string]$s) {
    $sb = [System.Text.StringBuilder]::new()
    foreach ($ch in $s.ToCharArray()) {
        if ('%"='.IndexOf($ch) -ge 0 -or [int]$ch -lt 0x20 -or [int]$ch -eq 0x7f) { [void]$sb.AppendFormat('%{0:X2}', [int]$ch) } else { [void]$sb.Append($ch) }
    }
    $sb.ToString()
}

# One quoted line saying why git failed: the last error-looking line of its output (fatal:,
# error:, a rejected ref, a CONFLICT), else the last non-empty line. URL credentials are masked.
function Get-Detail($lines) {
    $all = @($lines | ForEach-Object { "$_".Trim() } | Where-Object { $_ })
    $errs = @($all | Where-Object { $_ -match '^(?:fatal:|error:|CONFLICT|!\s)' -and $_ -notmatch 'failed to push some refs' })
    $pick = if ($errs.Count -gt 0) { $errs[-1] } else { $all | Select-Object -Last 1 }
    $text = ("$pick" -replace '://[^/@\s]+@', '://' -replace '[\x00-\x1f\x7f]+', ' ').Trim()
    if ($text.Length -gt 200) { $text = $text.Substring(0, 200) }
    if (-not $text) { $text = 'none' }
    '"' + (ConvertTo-Field $text) + '"'
}

if (-not (Test-Path -LiteralPath $C -PathType Container)) { Stop-Refused "not-a-directory: $C" }
$top = (& git -C $C rev-parse --show-toplevel 2>$null)
if ($LASTEXITCODE -ne 0) { Stop-Refused "not-a-git-worktree: $C" }
if ((Get-FullPath $top) -ine (Get-FullPath $C)) { Stop-Refused "not-the-worktree-root: $C" }

$main = (& git -C $C worktree list --porcelain | Select-Object -First 1) -replace '^worktree ', ''
if ((Get-FullPath $main) -ieq (Get-FullPath $C)) { Stop-Refused 'main-checkout: refuse to merge in the main checkout' }

# A rebase, merge, cherry-pick or revert already in progress is the caller's, not this script's:
# a merge whose index equals HEAD passes the dirty check, and `git merge` would then fail on it.
$gitDir = (& git -C $C rev-parse --absolute-git-dir).Trim()
foreach ($state in 'rebase-merge', 'rebase-apply', 'MERGE_HEAD', 'CHERRY_PICK_HEAD', 'REVERT_HEAD') {
    if (Test-Path -LiteralPath (Join-Path $gitDir $state)) { Stop-Refused "operation-in-progress: $state exists; finish or abort it first" }
}

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

# git parses branch.<current>.mergeOptions before argv, and `-s` / `-X` accumulate, so a
# `--strategy=ort` on the command line alone does not pin the strategy: `-s ours` there would
# still record the merge while dropping the branch, and `--no-commit` would leave it open. That
# key is overridden for this call with a blank value (git splits it into no options), passed as a
# GIT_CONFIG_KEY_n / GIT_CONFIG_VALUE_n pair rather than `-c`: `-c` splits at the first `=`, and a
# branch name may hold one (`a=b`), which would leave the key untouched. `--strategy=ort` covers
# pull.twohead; `--commit` and `--no-squash` restate the defaults. `-m` writes the subject
# `Merge branch '<b>' into <current>` (`--no-edit` would name refs/heads/<b>).
$srcSha = (& git -C $C rev-parse "refs/heads/$Branch").Trim()
$override = [ordered]@{ "branch.$current.mergeOptions" = ' '; 'merge.log' = 'false'; 'merge.branchdesc' = 'false'; 'merge.autoStash' = 'false' }
$i = 0
foreach ($key in $override.Keys) {
    Set-Item -LiteralPath "Env:GIT_CONFIG_KEY_$i" -Value $key
    Set-Item -LiteralPath "Env:GIT_CONFIG_VALUE_$i" -Value $override[$key]
    $i++
}
$env:GIT_CONFIG_COUNT = "$i"
try {
    $out = & git -C $C merge --no-ff --commit --no-squash --strategy=ort -m "Merge branch '$Branch' into $current" "refs/heads/$Branch" 2>&1
    $mergeExit = $LASTEXITCODE
} finally {
    Remove-Item -LiteralPath 'Env:GIT_CONFIG_COUNT' -ErrorAction SilentlyContinue
    for ($j = 0; $j -lt $i; $j++) { Remove-Item -LiteralPath "Env:GIT_CONFIG_KEY_$j", "Env:GIT_CONFIG_VALUE_$j" -ErrorAction SilentlyContinue }
}
if ($mergeExit -ne 0) {
    $names = @(((& git -C $C diff --name-only -z --diff-filter=U) -join "`n") -split "`0" | Where-Object { $_ })
    $files = '"' + (@($names | ForEach-Object { (ConvertTo-Field $_) -replace ',', '%2C' }) -join ',') + '"'
    # Abort only the merge this call started: none was in progress before (checked above), and
    # MERGE_HEAD is the tip of the branch being merged. Git's abort output is captured.
    $reason = 'merge-aborted'
    $mergeHead = (& git -C $C rev-parse -q --verify MERGE_HEAD 2>$null)
    if ($LASTEXITCODE -eq 0) {
        if ("$mergeHead".Trim() -ceq $srcSha) {
            $null = & git -C $C merge --abort 2>&1
            if ($LASTEXITCODE -ne 0) { $reason = 'merge-not-aborted' }
        } else {
            $reason = 'merge-not-aborted'
        }
    }
    Write-Output "status=failed reason=$reason files=$files detail=$(Get-Detail $out)"
    exit 1
}

$sha = (& git -C $C rev-parse HEAD).Trim()
Write-Output "status=ok merge=$sha branch=$Branch"
exit 0
