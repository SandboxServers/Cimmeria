# Run the live-DB test tier: the lib tests of every crate that has live-DB tests, in ONE
# nextest invocation under the `ci-live-db` profile, each live-DB test on its own database
# clone. PowerShell twin of
# tools/test-live-db.sh; see that file for why the list exists and why it is one run.
#
# Usage:
#   $env:DATABASE_URL = 'postgres://w-testing:w-testing@localhost:5433/sgw'
#   tools/test-live-db.ps1 [args...]
#   tools/test-live-db.ps1 --llvm-cov [args...]   # under `cargo llvm-cov --no-report nextest`
#   tools/test-live-db.ps1 [--llvm-cov] --build-only   # compile only; no database needed
# Extra args pass through to nextest, e.g. a test-name substring or `--no-fail-fast`.
#
# The `live_db_wrapper_lists_every_test_support_crate` test in cimmeria-services checks
# that this list matches test-live-db.sh and covers every crate with a
# `cimmeria-test-support` dev-dependency.

$ErrorActionPreference = 'Stop'

# Crates whose lib tests include live-DB tests (`require_db_or_skip!`), one per line.
# cimmeria-test-support holds the gate itself and its tests. cimmeria-wire has no
# live-DB tests yet; it dev-depends on cimmeria-test-support (for LogCapture), so
# the guard requires it here, and its lib tests ran in this tier before the split.
$LiveDbCrates = @(
    'cimmeria-resources'
    'cimmeria-auth'
    'cimmeria-cell-cover'
    'cimmeria-services'
    'cimmeria-test-support'
    'cimmeria-wire'
    'cimmeria-cell-catalog'
    'cimmeria-minigame'
    'cimmeria-base-session'
    'cimmeria-cell-world'
    'cimmeria-base-methods'
    'cimmeria-base-world-entry'
    'cimmeria-cell-combat'
    'cimmeria-base'
    'cimmeria-cell-content'
    'cimmeria-cell-console'
    'cimmeria-cell-interactions'
    'cimmeria-cell-methods'
    'cimmeria-cell-pets'
    'cimmeria-cell-duel'
    'cimmeria-cell'
)

# Flags, in either order, before any nextest args: --llvm-cov and --build-only, as in
# test-live-db.sh.
$rest = @($args)
$llvmCov = $false
$buildOnly = $false
while ($rest.Count -gt 0 -and ($rest[0] -eq '--llvm-cov' -or $rest[0] -eq '--build-only')) {
    if ($rest[0] -eq '--llvm-cov') { $llvmCov = $true } else { $buildOnly = $true }
    $rest = @($rest | Select-Object -Skip 1)
}
$packages = foreach ($crate in $LiveDbCrates) { '-p'; $crate }
$nextest = if ($llvmCov) { @('llvm-cov', '--no-report', 'nextest') } else { @('nextest', 'run') }
$nextest += @('--profile=ci-live-db') + $packages + @('--lib')

if ($buildOnly) {
    # Under llvm-cov, --no-run is cargo-llvm-cov's own flag and clashes with
    # --no-report; run no tests instead (see test-live-db.sh).
    if ($llvmCov) { & cargo @nextest -E 'none()' --no-tests=pass @rest } else { & cargo @nextest --no-run @rest }
    exit $LASTEXITCODE
}

if ([string]::IsNullOrEmpty($env:DATABASE_URL)) {
    [Console]::Error.WriteLine("test-live-db: DATABASE_URL is not set, so every live-DB test would skip and pass. " +
        "Point it at a database loaded from db/database.sql (the bundled Postgres listens on :5433).")
    exit 2
}

# Per-slot databases: clone the template (the database DATABASE_URL names) into
# <db>_0 .. <db>_<N-1>, N being the `live-db` group's max-threads in .config/nextest.toml.
# Same steps as test-live-db.sh; see there.
$root = Split-Path -Parent $PSScriptRoot
$slotLine = Get-Content (Join-Path $root '.config/nextest.toml') |
    Where-Object { $_ -match '^live-db = \{ max-threads = ([0-9]+) \}$' } | Select-Object -First 1
if (-not $slotLine) {
    [Console]::Error.WriteLine("test-live-db: no 'live-db = { max-threads = N }' line in .config/nextest.toml")
    exit 2
}
$slots = [int]($slotLine -replace '^live-db = \{ max-threads = ([0-9]+) \}$', '$1')
$psql = if ($env:PSQL) { $env:PSQL } elseif (Get-Command psql -ErrorAction SilentlyContinue) { 'psql' } else {
    Join-Path $root 'external/postgresql_server/bin/psql.exe' }
$urlHead = ($env:DATABASE_URL -split '\?')[0]
$templateDb = $urlHead.Substring($urlHead.LastIndexOf('/') + 1)
$adminUrl = $urlHead.Substring(0, $urlHead.LastIndexOf('/')) + '/postgres'
$stale = & $psql $adminUrl -tAq -c "SELECT datname FROM pg_database WHERE datname ~ '^${templateDb}_[0-9]+`$'"
$sqlArgs = @()
foreach ($name in @($stale | Where-Object { $_ })) { $sqlArgs += @('-c', "DROP DATABASE `"$($name.Trim())`" WITH (FORCE);") }
for ($k = 0; $k -lt $slots; $k++) { $sqlArgs += @('-c', "CREATE DATABASE `"${templateDb}_$k`" TEMPLATE `"$templateDb`";") }
& $psql $adminUrl -q -v ON_ERROR_STOP=1 @sqlArgs | Out-Null
if ($LASTEXITCODE -ne 0) {
    [Console]::Error.WriteLine("test-live-db: cloning $templateDb failed. Close every session on it " +
        "(a running server, a psql shell) or point DATABASE_URL at a worktree database.")
    exit 1
}
Write-Host "test-live-db: cloned $templateDb into ${templateDb}_0..${templateDb}_$($slots - 1)"

& cargo @nextest @rest
exit $LASTEXITCODE
