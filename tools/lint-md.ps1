# Run markdownlint-cli2 against the repo. Warn-only — exit code is reported
# but never fails CI; locally it lets you choose to fix or not.
#
# Usage:
#   tools/lint-md.ps1                       # lint all *.md per .markdownlint-cli2.yaml
#   tools/lint-md.ps1 --fix                 # auto-fix what's auto-fixable
#   tools/lint-md.ps1 path/to/file.md       # lint a single file
#
# Companion: tools/lint-md.sh (Bash equivalent).

$ErrorActionPreference = 'Stop'

# Locate repo root relative to this script.
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $RepoRoot

# Prefer project-local node_modules if installed; fall back to npx, which
# fetches markdownlint-cli2 on demand. The version pin in package.json keeps
# both paths in sync.
$LocalBin = Join-Path $RepoRoot 'node_modules\.bin\markdownlint-cli2.cmd'
if (Test-Path $LocalBin) {
    & $LocalBin @args
    exit $LASTEXITCODE
}

$Npx = Get-Command npx -ErrorAction SilentlyContinue
if ($null -ne $Npx) {
    & $Npx.Source --yes markdownlint-cli2 @args
    exit $LASTEXITCODE
}

Write-Error @'
tools/lint-md.ps1: neither node_modules/.bin/markdownlint-cli2 nor npx is available.
Install Node.js (https://nodejs.org), then run 'npm install' from the repo root.
'@
exit 1
