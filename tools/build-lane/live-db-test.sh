#!/usr/bin/env bash
# Reload THIS worktree's own test database, then run the live-DB test tier
# (tools/test-live-db.sh: every crate with live-DB tests, `--profile=ci-live-db`) with the
# given filter. Two lane jobs, one after the other (design: docs/analysis/lane-classes/
# design.md § 4.4): first the test binaries are built in a heavy slot with --build-only (no
# database, no reload), then the reload and the run go in a db slot. A compile error stops
# before the reload. A non-zero build exit ends the script with that code.
#
# Usage (from a worktree root):
#   tools/build-lane/live-db-test.sh <nextest filter or test-name substring> [extra nextest args]
#   tools/build-lane/live-db-test.sh --wireclient <filter>   # the wireclient end-to-end tests only
# Example:
#   tools/build-lane/live-db-test.sh chain_replay_tests::mission_701
#
# Same command and profile as CI, per-slot clones included; only the template database
# (sgw_<worktree>) is per worktree. Test hooks: LIVE_DB_TIER_SCRIPT and LIVE_DB_RELOAD_SCRIPT
# replace the tier and reload scripts (the .ps1 twin reads the same names).
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
TIER="${LIVE_DB_TIER_SCRIPT:-$HERE/../test-live-db.sh}"
RELOAD="${LIVE_DB_RELOAD_SCRIPT:-$HERE/reload-db.sh}"

# Leading --wireclient and --llvm-cov go to the tier in both phases; the rest is the filter.
lead=()
while [ "${1:-}" = --wireclient ] || [ "${1:-}" = --llvm-cov ]; do lead+=("$1"); shift; done
if [ $# -eq 0 ] && [ ${#lead[@]} -eq 0 ]; then echo 'live-db-test.sh: filter required' >&2; exit 2; fi

# Phase 1, heavy: compile the test binaries (the same binaries as --no-run; CI builds so too).
"$HERE/lane.sh" --class heavy bash "$TIER" ${lead[@]+"${lead[@]}"} --build-only || exit $?

# Phase 2, db: reload this worktree's database, then run the tier with the filter.
exec "$HERE/lane.sh" --class db bash -c '
  set -e
  out="$(bash "$1")"; echo "$out"
  export "$(echo "$out" | grep ^DATABASE_URL=)"
  shift; bash "$@"
' _ "$RELOAD" "$TIER" ${lead[@]+"${lead[@]}"} "$@"
