# Reload THIS worktree's own test database, then run the live-DB test tier
# (tools/test-live-db.ps1: every crate with live-DB tests, `--profile=ci-live-db`) with the
# given filter, inside one build-lane slot. PowerShell 7 twin of live-db-test.sh.
#
# Usage (from a worktree root):
#   pwsh tools/build-lane/live-db-test.ps1 <nextest filter or test-name substring> [extra nextest args]
# Example:
#   pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::mission_701
#
# Same command and profile as CI, per-slot clones included; only the template database
# (sgw_<worktree>) is per worktree.
if ($args.Count -ge 1 -and $args[0] -eq '--in-lane') {
    # Second half, run by lane.ps1 inside the slot.
    $rest = @($args | Select-Object -Skip 1)
    $out = & pwsh -NoProfile -File (Join-Path $PSScriptRoot 'reload-db.ps1')
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $out | Write-Output
    $url = $out | Where-Object { $_ -like 'DATABASE_URL=*' } | Select-Object -First 1
    $env:DATABASE_URL = $url.Substring('DATABASE_URL='.Length)
    & pwsh -NoProfile -File (Join-Path $PSScriptRoot '../test-live-db.ps1') @rest
    exit $LASTEXITCODE
}
if ($args.Count -lt 1) { [Console]::Error.WriteLine('live-db-test.ps1: filter required'); exit 2 }
& pwsh -NoProfile -File (Join-Path $PSScriptRoot 'lane.ps1') pwsh -NoProfile -File $PSCommandPath --in-lane @args
exit $LASTEXITCODE
