<#
.SYNOPSIS
  Daily per-PR stats sweep: post or refresh the stats comment of every PR merged in the last 3 days.

.DESCRIPTION
  Runs `python tools/token-profile/scheduled pr-sweep` (jobs.py): an incremental ingest from
  the main checkout with `gh pr list`, then `pr_stats --backfill --post --restart` over the
  PRs merged since 3 whole UTC days before today, at 6 PRs a minute. pr_stats finds each
  PR's comment by its marker: none, it creates one; a stale one, it edits in place; a
  current one, it leaves alone. So a rerun posts nothing new, and a PR whose numbers moved
  since the last ingest is brought up to date.

  The sweep keeps its own state file (<home>\pr-sweep-state.json), apart from the
  backfill's. Logs go to <home>\logs\pr-sweep-<YYYY-MM-DD>*.log and the summary to
  pr-sweep-<YYYY-MM-DD>-summary.txt (home: $env:TOKEN_PROFILE_HOME, else
  %LOCALAPPDATA%\cimmeria-token-profile).

  Exits 0 when every step passed, 1 when one failed (the last line names it), 2 on a usage
  error, a held lock, or no Python. Arguments pass through to jobs.py: --days N, --rate 6/min,
  --dry-run (post nothing), --skip-ingest.

.EXAMPLE
  pwsh tools/token-profile/scheduled/pr-sweep.ps1
  pwsh tools/token-profile/scheduled/pr-sweep.ps1 --dry-run --days 7
#>
$ErrorActionPreference = 'Stop'

$home_ = if ($env:TOKEN_PROFILE_HOME) { $env:TOKEN_PROFILE_HOME } else { Join-Path $env:LOCALAPPDATA 'cimmeria-token-profile' }
# @(...) keeps a one-word command an array, so $python[0] is the word, not its first letter.
$python = @(if ($env:TOKEN_PROFILE_PYTHON) { $env:TOKEN_PROFILE_PYTHON }
            elseif (Get-Command python -ErrorAction SilentlyContinue) { 'python' }
            elseif (Get-Command py -ErrorAction SilentlyContinue) { 'py', '-3' })
if ($python.Count -eq 0) {
    New-Item -ItemType Directory -Force (Join-Path $home_ 'logs') | Out-Null
    $msg = "$(Get-Date -Format o) pr-sweep: no Python found; set TOKEN_PROFILE_PYTHON"
    Add-Content (Join-Path $home_ 'logs\scheduled.log') $msg
    Write-Error $msg -ErrorAction Continue
    exit 2
}

$exe, $pre = $python[0], @($python | Select-Object -Skip 1)
& $exe @pre $PSScriptRoot pr-sweep @args
exit $LASTEXITCODE
