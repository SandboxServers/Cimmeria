# Create one local branch at a given full commit SHA, for `ops`. It runs `git branch <name> <sha>`
# and nothing else: no checkout, no fetch, no push, and never an existing branch overwritten.
#
#   pwsh tools/build-lane/branch-at.ps1 -C <repo> -Name <branch> -Sha <40-hex>
#
# <repo> is the main checkout or any worktree of the repository. Refuses (exit 2) when <sha> is
# not 40 lowercase hex digits or names no commit, <name> is not a valid branch name, <name> is
# ambiguous (looks like a hex hash of 7 to 40 digits, names an existing tag, starts with `+`, `-` or `refs/`, is `@`, is HEAD, FETCH_HEAD, ORIG_HEAD, MERGE_HEAD or CHERRY_PICK_HEAD in any case, is all upper-case letters, digits and `_` (pseudo-ref shaped: REBASE_HEAD, AUTO_MERGE; the `*_HEAD` and AUTO_MERGE shapes in any case), or its first
# segment is a configured remote or `heads`, `tags` or `remotes`: a local `origin/main` would
# shadow the remote-tracking ref in later rev lookups), or a branch exists whose name equals <name> case-insensitively or is a path prefix of it or the reverse,
# or <name> already resolves to any ref (`git rev-parse --verify <name>`: refs/<name> such as
# `stash`, `bisect/*` or `notes/*`, a tag, a remote's HEAD, a pseudo-ref), which git's rev lookup
# would pick before refs/heads/<name>. Prints status=ok branch=<name> sha=<sha>, or
# status=refused reason=<why> / status=failed reason=<why> detail="<git's last output line>" (quoted,
# with `%`, `"` and `=` percent-encoded). Exit codes: 0 created, 2 refused, 1 git failed.
[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Mandatory)][string]$C,
    [Parameter(Mandatory)][string]$Name,
    [Parameter(Mandatory)][string]$Sha
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
# On NTFS a loose ref `Rebase_Head` and the file `.git/REBASE_HEAD` are the same name, so the
# all-caps rule is applied in any case to names without a slash.
if ($Name -match '^refs/' -or $Name -cmatch '^[A-Z][A-Z0-9_]*$' -or $Name -iin '@', 'HEAD', 'FETCH_HEAD', 'ORIG_HEAD', 'MERGE_HEAD', 'CHERRY_PICK_HEAD') { Stop-Refused "ambiguous-branch-name: $Name" }
if ($Name -imatch '^(?:[A-Z0-9_]*_HEAD|AUTO_MERGE)$') { Stop-Refused "ambiguous-branch-name: $Name is shaped like a git pseudo-ref in another case" }
$first = ($Name -split '/')[0]
$remotes = @(& git -C $C remote)
$reserved = @('heads', 'tags', 'remotes') + $remotes
if ($Name.Contains('/') -or $remotes -contains $Name) {
    if ($reserved -contains $first) { Stop-Refused "ambiguous-branch-name: $Name shadows a ref namespace or a remote" }
}
# Compare against every existing branch name, case-insensitively: on a case-insensitive
# filesystem a loose `refs/heads/Topic` shadows a packed `refs/heads/topic`, so a case variant
# would silently re-point the existing branch. A D/F prefix clash (`a` against `a/b`) is refused
# the same way.
$want = "refs/heads/$Name".ToLowerInvariant()
foreach ($ref in @(& git -C $C for-each-ref --format='%(refname)' refs/heads)) {
    $have = "$ref".Trim().ToLowerInvariant()
    if ($have -eq $want) { Stop-Refused "branch-exists: $Name is not overwritten (existing branch $ref, compared case-insensitively)" }
    if ($have.StartsWith("$want/") -or $want.StartsWith("$have/")) { Stop-Refused "branch-exists: $Name clashes with the existing branch $ref (one is a path prefix of the other)" }
}
# git's rev lookup tries <name>, refs/<name>, refs/tags/<name> and only then refs/heads/<name>:
# any name that already resolves (refs/stash, refs/bisect/*, a remote's HEAD, a pseudo-ref, or on
# NTFS a case variant of a loose one) would shadow the new branch in every later lookup.
& git -C $C rev-parse -q --verify "$Name" *> $null
if ($LASTEXITCODE -eq 0) { Stop-Refused "ambiguous-branch-name: $Name already resolves to an existing ref, which would shadow the branch" }

$out = & git -C $C branch $Name $Sha 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Output "status=failed reason=git-branch detail=$(Get-Detail $out)"
    exit 1
}
Write-Output "status=ok branch=$Name sha=$Sha"
exit 0
