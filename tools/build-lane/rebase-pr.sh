#!/usr/bin/env bash
# Rebase a PR branch onto origin/main with git alone; a worker is needed only for what
# this leaves behind.
#
# Usage: tools/build-lane/rebase-pr.sh [--push] [--json] [-v] [--no-fetch] [--onto REF] <PR|branch|worktree>
#
#  * Silent with exit 0 on a clean rebase. --push then pushes with --force-with-lease.
#  * Resolves only mechanical conflicts: generated doc blocks (main's side; regen.py owns
#    them) and Cargo.lock (main's copy plus `cargo update --workspace`).
#  * Anything else aborts, leaves the branch and its worktree untouched, prints
#    status=conflict with the semantic files, and exits 1.
# The rebase runs in a throwaway sparse worktree. Details: rebase_pr.py.
set -euo pipefail
# $PYTHON, else the first of python3, python, py that runs (lane.sh's order). Running it
# with -c '' skips the Windows Store python3 stub, which exists but is not Python.
py="${PYTHON:-}"
if [ -z "$py" ]; then
  for p in python3 python py; do
    if command -v "$p" >/dev/null 2>&1 && "$p" -c '' >/dev/null 2>&1; then py="$p"; break; fi
  done
fi
if [ -z "$py" ]; then
  echo 'status=refused reason="no Python: none of python3, python, py runs; install Python 3 or set PYTHON"'
  exit 2
fi
exec "$py" "$(dirname "$0")/rebase_pr.py" "$@"
