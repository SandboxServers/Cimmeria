# Reload THIS worktree's own test database, then run the live-DB test tier
# (tools/test-live-db.ps1: every crate with live-DB tests, `--profile=ci-live-db`) with the
# given filter. PowerShell 7 twin of live-db-test.sh.
#
# Two build-lane calls, one after the other (design: docs/analysis/lane-classes/design.md
# § 4.4). Phase 1 compiles the tier in a heavy slot with --build-only, so a compile error
# ends the script before any reload is spent. Phase 2 reloads and runs the tier in a db
# slot, where the nextest run is a no-op build. The filter and extra args only go to phase 2.
#
# Usage (from a worktree root):
#   pwsh tools/build-lane/live-db-test.ps1 <nextest filter or test-name substring> [extra nextest args]
#   pwsh tools/build-lane/live-db-test.ps1 --wireclient <filter>   # the wireclient end-to-end tests only
# Example:
#   pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::mission_701
#
# Same command and profile as CI, per-slot clones included; only the template database
# (sgw_<worktree>) is per worktree. Test hooks: LIVE_DB_TIER_SCRIPT and LIVE_DB_RELOAD_SCRIPT
# replace the tier and reload scripts (defaults: tools/test-live-db.ps1, reload-db.ps1).
$TierScript = if ($env:LIVE_DB_TIER_SCRIPT) { $env:LIVE_DB_TIER_SCRIPT } else { Join-Path $PSScriptRoot '../test-live-db.ps1' }
$ReloadScript = if ($env:LIVE_DB_RELOAD_SCRIPT) { $env:LIVE_DB_RELOAD_SCRIPT } else { Join-Path $PSScriptRoot 'reload-db.ps1' }
if ($args.Count -ge 1 -and $args[0] -eq '--in-lane') {
    # Phase 2, run by lane.ps1 inside a db slot.
    $rest = @($args | Select-Object -Skip 1)
    $out = & pwsh -NoProfile -File $ReloadScript
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $out | Write-Output
    $url = $out | Where-Object { $_ -like 'DATABASE_URL=*' } | Select-Object -First 1
    $env:DATABASE_URL = $url.Substring('DATABASE_URL='.Length)
    & pwsh -NoProfile -File $TierScript @rest
    exit $LASTEXITCODE
}
if ($args.Count -lt 1) { [Console]::Error.WriteLine('live-db-test.ps1: filter required'); exit 2 }
# Phase 1: the leading --wireclient and --llvm-cov pick the binaries; the rest is phase 2's.
$lead = @()
while ($lead.Count -lt $args.Count -and ($args[$lead.Count] -eq '--wireclient' -or $args[$lead.Count] -eq '--llvm-cov')) {
    $lead += $args[$lead.Count]
}
& pwsh -NoProfile -File (Join-Path $PSScriptRoot 'lane.ps1') --class heavy pwsh -NoProfile -File $TierScript @lead --build-only
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
# Phase 2: reload and run, in a db slot.
& pwsh -NoProfile -File (Join-Path $PSScriptRoot 'lane.ps1') --class db pwsh -NoProfile -File $PSCommandPath --in-lane @args
exit $LASTEXITCODE
