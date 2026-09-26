#!/usr/bin/env bash
# Reload THIS worktree's own test database, then run the live-DB test tier
# (tools/test-live-db.sh: every crate with live-DB tests, `--profile=ci-live-db`) with the
# given filter, inside one build-lane slot.
#
# Usage (from a worktree root):
#   tools/build-lane/live-db-test.sh <nextest filter or test-name substring> [extra nextest args]
# Example:
#   tools/build-lane/live-db-test.sh chain_replay_tests::mission_701
#
# Same command, profile and serialisation as CI; only the database is per worktree.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
FILTER="${1:?filter required}"; shift || true
exec "$HERE/lane.sh" bash -c '
  set -e
  HERE="$1"; shift
  out="$("$HERE/reload-db.sh")"; echo "$out"
  export "$(echo "$out" | grep ^DATABASE_URL=)"
  bash "$HERE/../test-live-db.sh" "$@"
' _ "$HERE" "$FILTER" "$@"
