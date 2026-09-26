#!/usr/bin/env bash
# Wipe and reload THIS worktree's own test database from ./db/database.sql, on the
# bundled Postgres at localhost:5433. Prints the DATABASE_URL to use.
#
# Database name: sgw_<worktree dir name> (non [A-Za-z0-9_] characters become `_`); the
# main checkout keeps `sgw`. Override with CIMMERIA_TEST_DB=<name>. One database per
# worktree lets live-DB runs from different worktrees proceed concurrently without one
# reload landing under another's tests.
set -euo pipefail
if [ ! -f db/database.sql ]; then
  echo "reload-db: run from a repo/worktree root (db/database.sql not found in $(pwd))" >&2; exit 2
fi
TOP="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
MAIN="$(dirname "$(cd "$(git rev-parse --git-common-dir)" && pwd)")"
NAME="$(basename "$TOP" | tr -c 'A-Za-z0-9_\n' '_' | tr 'A-Z' 'a-z')"
if [ "$TOP" = "$MAIN" ]; then DB="sgw"; else DB="sgw_${NAME}"; fi
DB="${CIMMERIA_TEST_DB:-$DB}"
PSQL="${PSQL:-$MAIN/external/postgresql_server/bin/psql.exe}"
[ -x "$PSQL" ] || PSQL="$(command -v psql)"
export PGPASSWORD="${PGPASSWORD:-w-testing}"
HOSTARGS=(-h localhost -p "${PGPORT:-5433}" -U w-testing)
LOG="${TMPDIR:-/tmp}/reload-db-$DB.log"
start=$(date +%s)
"$PSQL" "${HOSTARGS[@]}" -d postgres -q -v ON_ERROR_STOP=1 \
  -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname='$DB' AND pid<>pg_backend_pid();" \
  -c "DROP DATABASE IF EXISTS \"$DB\";" \
  -c "CREATE DATABASE \"$DB\" OWNER \"w-testing\";" > "$LOG" 2>&1 || {
  echo "reload-db: drop/create of $DB FAILED:" >&2; tail -20 "$LOG" >&2; exit 1; }
"$PSQL" "${HOSTARGS[@]}" -d "$DB" -q -v ON_ERROR_STOP=1 -f db/database.sql >> "$LOG" 2>&1 || {
  echo "reload-db: schema load into $DB FAILED. Last 40 lines of $LOG:" >&2
  tail -40 "$LOG" >&2; exit 1; }
n=$("$PSQL" "${HOSTARGS[@]}" -d "$DB" -tAc 'select count(*) from resources.content_chains')
echo "reload-db: OK in $(( $(date +%s) - start ))s into $DB from $(pwd) ($n content chains)"
echo "DATABASE_URL=postgres://w-testing:w-testing@localhost:${PGPORT:-5433}/$DB"
