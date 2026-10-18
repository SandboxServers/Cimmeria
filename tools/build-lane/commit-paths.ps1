# Commit exactly the named files in one worktree, for `ops`. Never `git add .` or `-A`: the
# paths are staged one by one with --literal-pathspecs, so a glob cannot widen them.
#
#   pwsh tools/build-lane/commit-paths.ps1 -C <worktree> -Message <text> -Paths <a>,<b> [-Trailers <line>,...]
#
# Refuses (exit 2) when -C is not the root of a non-main worktree, HEAD is detached, a path is
# a glob, a directory, `.`, `..`, or starts with `-` or `:`, the index already holds other
# staged files, or `git diff --cached --check` finds whitespace errors. Git's hooks still run.
# Prints one line: status=ok sha=<sha> branch=<branch>, or status=refused|failed reason=<why>.
# Exit codes: 0 committed, 2 refused by a guard, 1 a git step failed.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$C,
    [Parameter(Mandatory)][string]$Message,
    [Parameter(Mandatory)][AllowEmptyString()][string[]]$Paths,
    [string[]]$Trailers = @()
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
if ((Get-FullPath $main) -ieq (Get-FullPath $C)) { Stop-Refused 'main-checkout: refuse to commit in the main checkout' }

& git -C $C symbolic-ref -q HEAD *> $null
if ($LASTEXITCODE -ne 0) { Stop-Refused 'detached-head: a commit there is held by no branch' }
$branch = (& git -C $C rev-parse --abbrev-ref HEAD).Trim()

if (-not $Message.Trim()) { Stop-Refused 'empty-message' }

$clean = @()
foreach ($raw in $Paths) {
    $q = $raw.Trim().Replace('\', '/')
    if (-not $q) { Stop-Refused 'empty-path' }
    if ($q -match '^[-:]') { Stop-Refused "option-or-magic-pathspec: $q" }
    if ($q -match '[*?\[\]]') { Stop-Refused "glob: $q" }
    if ($q.EndsWith('/') -or ($q -split '/' | Where-Object { $_ -in '.', '..' })) { Stop-Refused "not-an-exact-file: $q" }
    if (Test-Path -LiteralPath (Join-Path $C $q) -PathType Container) { Stop-Refused "directory: $q" }
    $clean += $q
}

$staged = @(& git -C $C diff --cached --name-only)
$extra = @($staged | Where-Object { $clean -notcontains $_ })
if ($extra.Count -gt 0) { Stop-Refused "other-staged-files: $($extra -join ', ')" }

& git -C $C --literal-pathspecs add -- @clean
if ($LASTEXITCODE -ne 0) {
    Write-Output 'status=failed reason=git-add'
    exit 1
}
if (@(& git -C $C diff --cached --name-only).Count -eq 0) { Stop-Refused 'nothing-to-commit' }

& git -C $C diff --cached --check
if ($LASTEXITCODE -ne 0) { Stop-Refused 'diff-check: whitespace errors (the paths stay staged)' }

$body = $Message.TrimEnd()
if ($Trailers.Count -gt 0) {
    $body += "`n`n" + (($Trailers | ForEach-Object { $_.Trim() }) -join "`n")
}
$file = [System.IO.Path]::GetTempFileName()
try {
    [System.IO.File]::WriteAllText($file, $body + "`n", [System.Text.UTF8Encoding]::new($false))
    & git -C $C commit -q -F $file
    $rc = $LASTEXITCODE
} finally {
    Remove-Item -LiteralPath $file -Force -ErrorAction SilentlyContinue
}
if ($rc -ne 0) {
    Write-Output 'status=failed reason=git-commit'
    exit 1
}
$sha = (& git -C $C rev-parse HEAD).Trim()
Write-Output "status=ok sha=$sha branch=$branch"
exit 0
