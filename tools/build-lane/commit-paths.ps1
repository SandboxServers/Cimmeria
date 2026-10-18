# Commit exactly the named files in one worktree, for `ops`. Never `git add .` or `-A`: the
# paths are staged one by one with --literal-pathspecs, so a glob cannot widen them.
#
#   pwsh -NoProfile -File tools/build-lane/commit-paths.ps1 -C <worktree> -Message <text> `
#       -Paths <a>,<b> [-Trailers '<line>;;<line>'] [-ExpectBranch <name>]
#
# Run as `pwsh -File`, a comma list reaches the script as ONE string, so -Paths is split on `,`
# (a path holding a comma cannot be committed here) and -Trailers on `;;` or a newline; write
# the list without spaces after the commas and quote each trailer list as one argument. A path
# that starts with `-` must be written -Paths:<path>. In-process callers may pass real arrays.
#
# Refuses (exit 2) when -C is not the root of a non-main worktree, HEAD is detached, the branch
# is not -ExpectBranch (when given), a path is a glob, a directory, `.`, `..`, or starts with `-`
# or `:`, is rooted (a drive, a UNC share or a leading slash: give paths relative to -C) or
# resolves outside -C, lies below a junction or symlink, the index already holds ANY staged file
# (a rename counts as both of its paths; a named path staged earlier counts too), the add would
# stage a file that was not named (`widened-pathspec`, e.g. a deleted directory name), or
# `git diff --cached --check` finds whitespace errors AND the repository configures
# core.whitespace or a whitespace attribute; without such a policy that is only a warning on
# stderr, as plain `git commit` accepts it. A merge, cherry-pick or revert in progress (MERGE_HEAD,
# CHERRY_PICK_HEAD or REVERT_HEAD) is refused as `merge-in-progress`. -ExpectBranch is compared
# case-sensitively. Git's hooks still run (a hook is the repository's own gate). Because the index is empty on entry, a failed
# check or commit unstages what the add staged and leaves it empty again, exactly as it was found.
# Prints one line: status=ok sha=<sha> branch=<branch>, or status=refused|failed reason=<why>.
# Exit codes: 0 committed, 2 refused by a guard, 1 a git step failed. A parameter-binding error
# (a missing or unknown parameter) prints no status= line and exits 1.
[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Mandatory)][string]$C,
    [Parameter(Mandatory)][string]$Message,
    [Parameter(Mandatory)][AllowEmptyString()][string[]]$Paths,
    [string[]]$Trailers = @(),
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

# Puts the named paths, and anything else the add staged, back out of the index (the working tree
# is untouched). The index was empty on entry, so nothing the caller staged is lost.
function Undo-Staging {
    foreach ($q in $clean) { & git -C $C --literal-pathspecs restore --staged -- $q *> $null }
    foreach ($q in @(& git -C $C -c core.quotepath=off diff --cached --name-only --no-renames)) {
        & git -C $C --literal-pathspecs restore --staged -- $q *> $null
    }
}

if (-not (Test-Path -LiteralPath $C -PathType Container)) { Stop-Refused "not-a-directory: $C" }
$top = (& git -C $C rev-parse --show-toplevel 2>$null)
if ($LASTEXITCODE -ne 0) { Stop-Refused "not-a-git-worktree: $C" }
if ((Get-FullPath $top) -ine (Get-FullPath $C)) { Stop-Refused "not-the-worktree-root: $C" }

$main = (& git -C $C worktree list --porcelain | Select-Object -First 1) -replace '^worktree ', ''
if ((Get-FullPath $main) -ieq (Get-FullPath $C)) { Stop-Refused 'main-checkout: refuse to commit in the main checkout' }

$headRef = (& git -C $C symbolic-ref -q HEAD 2>$null)
if ($LASTEXITCODE -ne 0) { Stop-Refused 'detached-head: a commit there is held by no branch' }
# symbolic-ref, not `rev-parse --abbrev-ref`: the latter answers `heads/<b>` when a tag has the same name.
$branch = ($headRef.Trim() -replace '^refs/heads/', '')
if ($ExpectBranch -and $branch -cne $ExpectBranch) { Stop-Refused "branch-mismatch: HEAD is $branch, not $ExpectBranch" }

# A merge, cherry-pick or revert in progress would be concluded by `git commit` (a merge commit
# with a second parent), whatever the named paths hold.
foreach ($pending in 'MERGE_HEAD', 'CHERRY_PICK_HEAD', 'REVERT_HEAD') {
    & git -C $C rev-parse -q --verify $pending *> $null
    if ($LASTEXITCODE -eq 0) { Stop-Refused "merge-in-progress: $pending exists; finish or abort it first" }
}

if (-not $Message.Trim()) { Stop-Refused 'empty-message' }

$clean = @()
foreach ($raw in ($Paths | ForEach-Object { $_ -split ',' })) {
    $q = $raw.Trim().Replace('\', '/')
    if (-not $q) { Stop-Refused 'empty-path' }
    if ($q -match '^[-:]') { Stop-Refused "option-or-magic-pathspec: $q" }
    if ($q -match '^[A-Za-z]:' -or $q.StartsWith('/') -or [System.IO.Path]::IsPathRooted($q)) { Stop-Refused "rooted-path: $q (give the path relative to -C)" }
    if ($q -match '[*?\[\]]') { Stop-Refused "glob: $q" }
    if ($q.EndsWith('/') -or ($q -split '/' | Where-Object { $_ -in '.', '..' })) { Stop-Refused "not-an-exact-file: $q" }
    $full = Get-FullPath ([System.IO.Path]::Combine($C, $q))
    if (-not $full.StartsWith((Get-FullPath $C) + '/', [System.StringComparison]::OrdinalIgnoreCase)) { Stop-Refused "outside-worktree: $q" }
    if (Test-Path -LiteralPath $full -PathType Container) { Stop-Refused "directory: $q" }
    # A junction or symlink above the file would carry a file that lives outside -C into the commit.
    $walk = Get-FullPath $C
    $segments = @($q -split '/')
    for ($i = 0; $i -lt $segments.Count - 1; $i++) {
        $walk = "$walk/$($segments[$i])"
        $item = Get-Item -LiteralPath $walk -Force -ErrorAction SilentlyContinue
        if (-not $item) { break }
        if ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) { Stop-Refused "reparse-point: $q passes through the junction or symlink $($segments[$i])" }
    }
    $clean += $q
}

$staged = @(& git -C $C diff --cached --name-only --no-renames)
if ($staged.Count -gt 0) { Stop-Refused "other-staged-files: $($staged -join ', ') (the index must be empty so a failure can restore it)" }

& git -C $C --literal-pathspecs add -- @clean
if ($LASTEXITCODE -ne 0) {
    Undo-Staging
    Write-Output 'status=failed reason=git-add'
    exit 1
}
$stagedNow = @(& git -C $C -c core.quotepath=off diff --cached --name-only --no-renames)
if ($stagedNow.Count -eq 0) { Stop-Refused 'nothing-to-commit' }
# A literal pathspec still matches a directory prefix, so naming a deleted directory stages every
# deletion beneath it. Only the files that were named may be staged.
$widened = @($stagedNow | Where-Object { $clean -inotcontains $_ })
if ($widened.Count -gt 0) {
    Undo-Staging
    Stop-Refused "widened-pathspec: the paths would also stage $($widened -join ', '); name each file (the paths were unstaged again)"
}

# Plain `git commit` accepts trailing whitespace, so this is a refusal only where the repository
# declares a whitespace policy (core.whitespace or a whitespace attribute); elsewhere it warns.
& git -C $C diff --cached --check
if ($LASTEXITCODE -ne 0) {
    $policy = (& git -C $C config --get core.whitespace 2>$null)
    $attrs = @(& git -C $C check-attr whitespace -- @clean | Where-Object { $_ -notmatch ': whitespace: unspecified$' })
    if ($policy -or $attrs.Count -gt 0) {
        Undo-Staging
        Stop-Refused 'diff-check: whitespace errors (the paths were unstaged again)'
    }
    Write-Warning 'warning=diff-check: whitespace errors in the staged changes (the repository sets no whitespace policy, so the commit goes ahead)'
}

$lines = @($Trailers | ForEach-Object { $_ -split ';;|\r?\n' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
$body = $Message.TrimEnd()
if ($lines.Count -gt 0) { $body += "`n`n" + ($lines -join "`n") }
$file = [System.IO.Path]::GetTempFileName()
try {
    [System.IO.File]::WriteAllText($file, $body + "`n", [System.Text.UTF8Encoding]::new($false))
    & git -C $C commit -q -F $file
    $rc = $LASTEXITCODE
} finally {
    Remove-Item -LiteralPath $file -Force -ErrorAction SilentlyContinue
}
if ($rc -ne 0) {
    Undo-Staging
    Write-Output 'status=failed reason=git-commit'
    exit 1
}
$sha = (& git -C $C rev-parse HEAD).Trim()
Write-Output "status=ok sha=$sha branch=$branch"
exit 0
