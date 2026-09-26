# Run the live-DB test tier: the lib tests of every crate that has live-DB tests, in ONE
# nextest invocation under the serialised `ci-live-db` profile. PowerShell twin of
# tools/test-live-db.sh; see that file for why the list exists and why it is one run.
#
# Usage:
#   $env:DATABASE_URL = 'postgres://w-testing:w-testing@localhost:5433/sgw'
#   tools/test-live-db.ps1 [args...]
#   tools/test-live-db.ps1 --llvm-cov [args...]   # under `cargo llvm-cov --no-report nextest`
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
    'cimmeria-services'
    'cimmeria-test-support'
    'cimmeria-wire'
)

if ([string]::IsNullOrEmpty($env:DATABASE_URL)) {
    [Console]::Error.WriteLine("test-live-db: DATABASE_URL is not set, so every live-DB test would skip and pass. " +
        "Point it at a database loaded from db/database.sql (the bundled Postgres listens on :5433).")
    exit 2
}

$packages = foreach ($crate in $LiveDbCrates) { '-p'; $crate }
$rest = @($args)

if ($rest.Count -gt 0 -and $rest[0] -eq '--llvm-cov') {
    $rest = @($rest | Select-Object -Skip 1)
    & cargo llvm-cov --no-report nextest --profile=ci-live-db @packages --lib @rest
} else {
    & cargo nextest run --profile=ci-live-db @packages --lib @rest
}
exit $LASTEXITCODE
