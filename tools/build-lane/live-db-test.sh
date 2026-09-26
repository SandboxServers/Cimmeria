#!/usr/bin/env bash
# Reload THIS worktree's own test database, then run the live-DB nextest profile for
# cimmeria-services with the given filter, inside one build-lane slot.
#
# Usage (from a worktree root):
#   tools/build-lane/live-db-test.sh <nextest filter or test-name substring> [extra nextest args]
# Example:
#   tools/build-lane/live-db-test.sh chain_replay_tests::mission_701
#
# Same command, profile and serialisation as CI (`--profile=ci-live-db`); only the
# database is per worktree.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
FILTER="${1:?filter required}"; shift || true
exec "$HERE/lane.sh" bash -c '
  set -e
  HERE="$1"; shift
  out="$("$HERE/reload-db.sh")"; echo "$out"
  export "$(echo "$out" | grep ^DATABASE_URL=)"
  cargo nextest run --profile=ci-live-db -p cimmeria-services --lib "$@"
' _ "$HERE" "$FILTER" "$@"
