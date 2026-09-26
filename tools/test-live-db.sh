#!/usr/bin/env bash
# Run the live-DB test tier: the lib tests of every crate that has live-DB tests, in ONE
# nextest invocation under the serialised `ci-live-db` profile. CI (test.yml's
# test-live-db job and coverage job) and tools/build-lane/live-db-test.sh call this.
#
# Usage:
#   DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw tools/test-live-db.sh [args...]
#   tools/test-live-db.sh --llvm-cov [args...]   # under `cargo llvm-cov --no-report nextest`
# Extra args pass through to nextest, e.g. a test-name substring or `--no-fail-fast`.
#
# Why a list: once cimmeria-services is split into several crates, running
# `-p cimmeria-services --lib` alone would run only the facade's tests and still pass,
# the #615 "green but empty" failure. Every crate with a `cimmeria-test-support`
# dev-dependency must be listed; the `live_db_wrapper_lists_every_test_support_crate`
# test in cimmeria-services checks that, and that this list matches test-live-db.ps1.
#
# Why one invocation: the profile's `threads-required = "num-test-threads"` (in
# .config/nextest.toml) serialises tests across every test binary of a single nextest
# run. Separate runs per crate would also be serial, but this keeps one JUnit report.
set -euo pipefail

# Crates whose lib tests include live-DB tests (`require_db_or_skip!`), one per line.
# cimmeria-test-support holds the gate itself and its tests.
LIVE_DB_CRATES=(
  cimmeria-resources
  cimmeria-services
  cimmeria-test-support
)

if [ -z "${DATABASE_URL:-}" ]; then
  echo "test-live-db: DATABASE_URL is not set, so every live-DB test would skip and pass." >&2
  echo "Point it at a database loaded from db/database.sql (the bundled Postgres listens on" >&2
  echo ":5433; tools/build-lane/reload-db.sh loads a per-worktree copy)." >&2
  exit 2
fi

packages=()
for crate in "${LIVE_DB_CRATES[@]}"; do
  packages+=(-p "$crate")
done

if [ "${1:-}" = "--llvm-cov" ]; then
  shift
  exec cargo llvm-cov --no-report nextest --profile=ci-live-db "${packages[@]}" --lib "$@"
fi
exec cargo nextest run --profile=ci-live-db "${packages[@]}" --lib "$@"
