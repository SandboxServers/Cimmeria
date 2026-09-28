#!/usr/bin/env bash
# Load db/database.sql into the live-DB template database in the background, so the load
# overlaps the compile in CI's test-live-db and coverage-live-db jobs.
#
# Usage (from the repo root, with DATABASE_URL naming the template database):
#   tools/live-db-schema-load.sh start           # returns at once
#   tools/live-db-schema-load.sh wait [SECONDS]  # blocks until the load ends (default 300)
#
# `start` detaches the load from the step (setsid, nohup, no inherited stdio), so it keeps
# running after the step ends; a GitHub runner only reaps orphaned processes when the
# job ends. The load writes its log and exit code under $RUNNER_TEMP (or $TMPDIR).
# `wait` fails, printing the log, when the load failed or did not finish in time: a
# broken schema must fail the job with its error visible, not surface later as a
# confusing test failure.
set -euo pipefail

DIR="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
LOG="$DIR/live-db-schema.log"
RC="$DIR/live-db-schema.rc"
STARTED="$DIR/live-db-schema.started"

case "${1:-}" in
  start)
    : "${DATABASE_URL:?DATABASE_URL must name the template database}"
    rm -f "$LOG" "$RC"
    date +%s > "$STARTED"
    # setsid puts the load in its own session, out of the step's process group; Git
    # Bash has no setsid, so a local simulation runs it under nohup alone.
    detach=(nohup)
    command -v setsid > /dev/null && detach=(setsid nohup)
    "${detach[@]}" bash -c '
      echo "load started $(date -u +%H:%M:%S)" > "$2"
      psql "$1" -v ON_ERROR_STOP=1 -q -f db/database.sql >> "$2" 2>&1
      echo $? > "$3.tmp" && mv "$3.tmp" "$3"
    ' _ "$DATABASE_URL" "$LOG" "$RC" < /dev/null > /dev/null 2>&1 &
    echo "live-db-schema-load: started (pid $!), log $LOG"
    ;;
  wait)
    timeout="${2:-300}"
    waited=0
    until [ -f "$RC" ]; do
      if [ "$waited" -ge "$timeout" ]; then
        echo "live-db-schema-load: the schema load did not finish within ${timeout}s. Log so far:" >&2
        cat "$LOG" >&2 || true
        exit 1
      fi
      sleep 1
      waited=$((waited + 1))
    done
    rc="$(cat "$RC")"
    took=$(( $(date +%s) - $(cat "$STARTED") ))
    if [ "$rc" != 0 ]; then
      echo "live-db-schema-load: the schema load FAILED (psql exit $rc). Log:" >&2
      cat "$LOG" >&2
      exit 1
    fi
    echo "live-db-schema-load: loaded in ${took}s since start; waited ${waited}s here"
    ;;
  *)
    echo "usage: $0 start | wait [SECONDS]" >&2
    exit 2
    ;;
esac
