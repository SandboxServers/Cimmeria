#!/usr/bin/env bash
# Run the live-DB test tier: the lib tests of every crate that has live-DB tests, in ONE
# nextest invocation under the `ci-live-db` profile, each live-DB test on its own database
# clone. CI (test.yml's test-live-db and coverage-live-db jobs) and
# tools/build-lane/live-db-test.sh call this.
#
# Usage:
#   DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw tools/test-live-db.sh [args...]
#   tools/test-live-db.sh --llvm-cov [args...]   # under `cargo llvm-cov --no-report nextest`
#   tools/test-live-db.sh [--llvm-cov] --build-only   # compile only; no database needed
# Extra args pass through to nextest, e.g. a test-name substring or `--no-fail-fast`.
#
# Why a list: once cimmeria-services is split into several crates, running
# `-p cimmeria-services --lib` alone would run only the facade's tests and still pass,
# the #615 "green but empty" failure. Every crate with a `cimmeria-test-support`
# dev-dependency must be listed; the `live_db_wrapper_lists_every_test_support_crate`
# test in cimmeria-services checks that, and that this list matches test-live-db.ps1.
#
# Why one invocation: the profile's `live-db` test group (in .config/nextest.toml) runs
# the tests whose name contains `live_db` N at a time, one per database clone, across
# every test binary of a single nextest run, and the rest in parallel. The group and its
# slots span one run only, so per-crate runs would have to go one after another; one run
# also keeps one JUnit report.
set -euo pipefail

# Crates whose lib tests include live-DB tests (`require_db_or_skip!`), one per line.
# cimmeria-test-support holds the gate itself and its tests. cimmeria-wire has no
# live-DB tests yet; it dev-depends on cimmeria-test-support (for LogCapture), so
# the guard requires it here, and its lib tests ran in this tier before the split.
LIVE_DB_CRATES=(
  cimmeria-resources
  cimmeria-auth
  cimmeria-cell-cover
  cimmeria-services
  cimmeria-test-support
  cimmeria-wire
  cimmeria-cell-catalog
  cimmeria-names
  cimmeria-minigame
  cimmeria-base-session
  cimmeria-cell-world
  cimmeria-base-methods
  cimmeria-base-world-entry
  cimmeria-cell-combat
  cimmeria-base
  cimmeria-base-crafting
  cimmeria-cell-content
  cimmeria-cell-console
  cimmeria-cell-interactions
  cimmeria-cell-methods
  cimmeria-cell-pets
  cimmeria-cell-duel
  cimmeria-cell-org
  cimmeria-cell-effect-scripts
  cimmeria-cell
)

# Flags, in either order, before any nextest args: --llvm-cov runs under
# `cargo llvm-cov --no-report nextest`; --build-only compiles the test binaries with
# exactly the arguments the real run uses (so the real run does not recompile) and
# stops, with no database needed. CI builds first while the schema loads in the
# background (tools/live-db-schema-load.sh), then runs this again to test.
llvm_cov=0
build_only=0
while :; do
  case "${1:-}" in
    --llvm-cov) llvm_cov=1; shift ;;
    --build-only) build_only=1; shift ;;
    *) break ;;
  esac
done

packages=()
for crate in "${LIVE_DB_CRATES[@]}"; do
  packages+=(-p "$crate")
done
if [ $llvm_cov -eq 1 ]; then
  nextest=(cargo llvm-cov --no-report nextest)
else
  nextest=(cargo nextest run)
fi
nextest+=(--profile=ci-live-db "${packages[@]}" --lib)

if [ $build_only -eq 1 ]; then
  if [ $llvm_cov -eq 1 ]; then
    # cargo-llvm-cov takes --no-run as its own (deprecated) flag and rejects it with
    # --no-report, so build by running no tests: a filterset leaves the build unchanged.
    exec "${nextest[@]}" -E 'none()' --no-tests=pass "$@"
  fi
  exec "${nextest[@]}" --no-run "$@"
fi

if [ -z "${DATABASE_URL:-}" ]; then
  echo "test-live-db: DATABASE_URL is not set, so every live-DB test would skip and pass." >&2
  echo "Point it at a database loaded from db/database.sql (the bundled Postgres listens on" >&2
  echo ":5433; tools/build-lane/reload-db.sh loads a per-worktree copy)." >&2
  exit 2
fi

# Per-slot databases. The profile's `live-db` test group runs up to N live-DB tests at
# once, and each test uses the clone for its group slot, <db>_<slot> (the resolver is
# `database_url()` in crates/test-support/src/live_db_slot.rs). N is read from the group's
# `max-threads` in .config/nextest.toml, so the clones and the group cannot drift apart.
# The database DATABASE_URL names is the template: it must hold the loaded schema, and
# nothing may be connected to it while it is cloned.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SLOTS="$(sed -nE 's/^live-db = \{ max-threads = ([0-9]+) \}\r?$/\1/p' "$ROOT/.config/nextest.toml" | head -1)"
if [ -z "$SLOTS" ]; then
  echo "test-live-db: no 'live-db = { max-threads = N }' line in .config/nextest.toml" >&2
  exit 2
fi
PSQL="${PSQL:-$(command -v psql || true)}"
[ -n "$PSQL" ] || PSQL="$ROOT/external/postgresql_server/bin/psql.exe"
url_head="${DATABASE_URL%%\?*}"
TEMPLATE_DB="${url_head##*/}"
ADMIN_URL="${url_head%/*}/postgres"
start=$(date +%s)
# Stale clones from an earlier run (or a larger N) go first: the template's exact name
# plus a numeric suffix. The template's own sessions are not touched (it may be a dev
# server's database); CREATE DATABASE fails with "being accessed by other users" if any
# are open.
sql=()
while IFS= read -r stale; do
  [ -n "$stale" ] && sql+=("DROP DATABASE \"$stale\" WITH (FORCE);")
done < <("$PSQL" "$ADMIN_URL" -tAq -c \
  "SELECT datname FROM pg_database WHERE datname ~ '^${TEMPLATE_DB}_[0-9]+\$'" | tr -d '\r')
for ((k = 0; k < SLOTS; k++)); do
  sql+=("CREATE DATABASE \"${TEMPLATE_DB}_$k\" TEMPLATE \"$TEMPLATE_DB\";")
done
args=()
for s in "${sql[@]}"; do args+=(-c "$s"); done
if ! "$PSQL" "$ADMIN_URL" -q -v ON_ERROR_STOP=1 "${args[@]}" > /dev/null; then
  echo "test-live-db: cloning $TEMPLATE_DB failed. Close every session on it (a running" >&2
  echo "server, a psql shell) or point DATABASE_URL at a worktree database." >&2
  exit 1
fi
echo "test-live-db: cloned $TEMPLATE_DB into ${TEMPLATE_DB}_0..${TEMPLATE_DB}_$((SLOTS - 1)) in $(( $(date +%s) - start ))s"

# Reporter settings come from the caller. Under the build lane's quiet mode (an agent,
# through live-db-test.sh) the NEXTEST_* variables report failures only and the lane
# summarises; CI runs this script directly, so its log keeps nextest's full output.
exec "${nextest[@]}" "$@"
