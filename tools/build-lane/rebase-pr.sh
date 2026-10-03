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
exec "${PYTHON:-python}" "$(dirname "$0")/rebase_pr.py" "$@"
