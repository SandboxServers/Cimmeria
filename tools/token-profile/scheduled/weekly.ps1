<#
.SYNOPSIS
  Weekly token-profile report: ingest, reconcile, validate and report the last 7 days.

.DESCRIPTION
  Runs `python tools/token-profile/scheduled weekly` (jobs.py): an incremental ingest from
  the main checkout with `gh pr list`, then reconcile and report over the 7 whole UTC days
  before today, and validate with the 10% wrong-share limit. Everything is written to
  <home>\reports\<YYYY-MM-DD>\ (home: $env:TOKEN_PROFILE_HOME, else
  %LOCALAPPDATA%\cimmeria-token-profile), never into the repo: each step's log, the reports,
  and weekly-summary.txt.

  Exits 0 when every step passed, 1 when one failed (the last line names it and why), 2 on
  a usage error, a held lock, or no Python. Arguments are passed through to jobs.py, for
  example --days 14 --until 2026-10-05 --skip-ingest.

  The database is $env:TOKEN_PROFILE_DB, else ~\token-profile.sqlite. Python is
  $env:TOKEN_PROFILE_PYTHON, else `python` on PATH, else `py -3`.

.EXAMPLE
  pwsh tools/token-profile/scheduled/weekly.ps1
  pwsh tools/token-profile/scheduled/weekly.ps1 --skip-ingest --days 14
#>
$ErrorActionPreference = 'Stop'

$home_ = if ($env:TOKEN_PROFILE_HOME) { $env:TOKEN_PROFILE_HOME } else { Join-Path $env:LOCALAPPDATA 'cimmeria-token-profile' }
# @(...) keeps a one-word command an array, so $python[0] is the word, not its first letter.
$python = @(if ($env:TOKEN_PROFILE_PYTHON) { $env:TOKEN_PROFILE_PYTHON }
            elseif (Get-Command python -ErrorAction SilentlyContinue) { 'python' }
            elseif (Get-Command py -ErrorAction SilentlyContinue) { 'py', '-3' })
if ($python.Count -eq 0) {
    New-Item -ItemType Directory -Force (Join-Path $home_ 'logs') | Out-Null
    $msg = "$(Get-Date -Format o) weekly: no Python found; set TOKEN_PROFILE_PYTHON"
    Add-Content (Join-Path $home_ 'logs\scheduled.log') $msg
    Write-Error $msg -ErrorAction Continue
    exit 2
}

$exe, $pre = $python[0], @($python | Select-Object -Skip 1)
& $exe @pre $PSScriptRoot weekly @args
exit $LASTEXITCODE
