# Rebase the current branch of one worktree onto a commit, for `ops`. It runs `git rebase <onto>`,
# or `git rebase --onto <onto> <upstream>` when -Upstream is given (the commit the branch was cut
# from, so a stacked branch does not replay its parent's commits). It never fetches. A conflict,
# or any failed rebase step, runs `git rebase --abort` (only while a rebase is in progress) and
# exits 1, so the worktree is left as it was before the call. Hooks still run.
#
#   pwsh tools/build-lane/rebase-onto.ps1 -C <worktree> -Onto <40-hex> [-Upstream <40-hex>]
#
# Merge commits on the branch are kept (`--rebase-merges`), so a `--no-ff` topology survives.
#
# Refuses (exit 2) when -C is not the root of a non-main worktree, HEAD is detached, the tree is
# dirty (any `git status --porcelain` output), -Onto or -Upstream is not 40 lowercase hex digits
# or names no commit, -Upstream is not an ancestor of HEAD or is HEAD itself (nothing to replay:
# that would be a reset), or -Onto is already in HEAD while -Upstream is not in -Onto (the
# rebase would drop the commits between them). Rebase config such as rebase.updateRefs is
# overridden (--no-update-refs), so no other branch moves.
# Prints status=ok tip=<sha> branch=<name> after a rebase, status=up-to-date tip=<sha> branch=<name>
# when HEAD already contains -Onto and nothing moves, or status=failed reason=<why> detail=<git's last output line>.
# Exit codes: 0 rebased or up to date, 2 refused by a guard, 1 the rebase failed (aborted).
[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Mandatory)][string]$C,
    [Parameter(Mandatory)][string]$Onto,
    [string]$Upstream = ''
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

# One line saying why git failed: the last error-looking line of its output (fatal:, error:, a
# rejected ref, a CONFLICT), else the last non-empty line. URL credentials are masked.
function Get-Detail($lines) {
    $all = @($lines | ForEach-Object { "$_".Trim() } | Where-Object { $_ })
    $errs = @($all | Where-Object { $_ -match '^(?:fatal:|error:|CONFLICT|!\s)' -and $_ -notmatch 'failed to push some refs' })
    $pick = if ($errs.Count -gt 0) { $errs[-1] } else { $all | Select-Object -Last 1 }
    $text = ("$pick" -replace '://[^/@\s]+@', '://' -replace '[\x00-\x1f\x7f]+', ' ').Trim()
    if ($text.Length -gt 200) { $text = $text.Substring(0, 200) }
    if ($text) { $text } else { 'none' }
}

if (-not (Test-Path -LiteralPath $C -PathType Container)) { Stop-Refused "not-a-directory: $C" }
$top = (& git -C $C rev-parse --show-toplevel 2>$null)
if ($LASTEXITCODE -ne 0) { Stop-Refused "not-a-git-worktree: $C" }
if ((Get-FullPath $top) -ine (Get-FullPath $C)) { Stop-Refused "not-the-worktree-root: $C" }

$main = (& git -C $C worktree list --porcelain | Select-Object -First 1) -replace '^worktree ', ''
if ((Get-FullPath $main) -ieq (Get-FullPath $C)) { Stop-Refused 'main-checkout: refuse to rebase the main checkout' }

& git -C $C symbolic-ref -q HEAD *> $null
if ($LASTEXITCODE -ne 0) { Stop-Refused 'detached-head: a rebase there is held by no branch' }
$branch = (& git -C $C rev-parse --abbrev-ref HEAD).Trim()

$dirty = (& git -C $C status --porcelain | Out-String).Trim()
if ($dirty) { Stop-Refused 'dirty-tree: commit or remove these first' }

if ($Onto -cnotmatch '^[0-9a-f]{40}$') { Stop-Refused "sha-not-full-40-hex: $Onto" }
if ($Upstream -and $Upstream -cnotmatch '^[0-9a-f]{40}$') { Stop-Refused "sha-not-full-40-hex: $Upstream" }
$ontoSha = (& git -C $C rev-parse --verify --quiet "$Onto^{commit}" | Out-String).Trim()
if (-not $ontoSha) { Stop-Refused "not-a-commit: $Onto" }

if ($Upstream) {
    $upSha = (& git -C $C rev-parse --verify --quiet "$Upstream^{commit}" | Out-String).Trim()
    if (-not $upSha) { Stop-Refused "not-a-commit: $Upstream" }
    & git -C $C merge-base --is-ancestor $upSha HEAD
    if ($LASTEXITCODE -ne 0) { Stop-Refused "upstream-not-ancestor: $Upstream is not in $branch" }
    # HEAD is its own ancestor; --onto <x> HEAD replays nothing and would move the branch to <x>.
    $replay = (& git -C $C rev-list --count "$upSha..HEAD" | Out-String).Trim()
    if ($upSha -eq (& git -C $C rev-parse HEAD).Trim() -or $replay -eq '0') { Stop-Refused "nothing-to-replay: $Upstream is the tip of $branch" }
    # An -Onto already inside HEAD only makes sense when -Upstream sits at or below it; otherwise
    # --onto <old commit> <newer commit> silently drops everything between the two.
    & git -C $C merge-base --is-ancestor $ontoSha HEAD
    if ($LASTEXITCODE -eq 0) {
        & git -C $C merge-base --is-ancestor $upSha $ontoSha
        if ($LASTEXITCODE -ne 0) { Stop-Refused "would-drop-commits: $Upstream is not in $Onto, so the commits between them would be lost" }
    }
    $rebaseArgs = @('rebase', '--no-update-refs', '--rebase-merges', '--onto', $ontoSha, $upSha)
} else {
    $rebaseArgs = @('rebase', '--no-update-refs', '--rebase-merges', $ontoSha)
}

& git -C $C merge-base --is-ancestor $ontoSha HEAD
if ($LASTEXITCODE -eq 0 -and -not $Upstream) {
    $tip = (& git -C $C rev-parse HEAD).Trim()
    Write-Output "status=up-to-date tip=$tip branch=$branch"
    exit 0
}

$out = & git -C $C @rebaseArgs 2>&1
if ($LASTEXITCODE -ne 0) {
    $gitDir = (& git -C $C rev-parse --absolute-git-dir).Trim()
    if ((Test-Path -LiteralPath (Join-Path $gitDir 'rebase-merge')) -or (Test-Path -LiteralPath (Join-Path $gitDir 'rebase-apply'))) {
        & git -C $C rebase --abort
    }
    Write-Output "status=failed reason=rebase-aborted branch=$branch detail=$(Get-Detail $out)"
    exit 1
}

$tip = (& git -C $C rev-parse HEAD).Trim()
Write-Output "status=ok tip=$tip branch=$branch"
exit 0
