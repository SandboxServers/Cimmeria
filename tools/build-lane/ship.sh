#!/usr/bin/env bash
# The mechanical end of a packet in one call, with one line of output.
#
# Usage:
#   tools/build-lane/ship.sh pr -C <worktree> (-m MSG | -F FILE) [--title T] [--body-file F]
#                               [--paths P ...] [--base BRANCH] [--draft] [--force-with-lease] [-v]
#   tools/build-lane/ship.sh merge <PR> [--retire NAME] [--timeout 30m] [--queue-timeout 60m]
#                                  [--no-wait] [--allow-red-main] [-v]
#
#  * pr refuses (exit 2) unless -C is a registered worktree on a feature branch, then
#    stages, commits with $SHIP_TRAILERS, pushes and opens the PR (footer: $SHIP_PR_FOOTER).
#    --base stacks it on another PR's branch.
#  * merge waits for the PR's checks (Markdown/memory-only PRs: none), rebasing with
#    rebase-pr.sh only when GitHub says BEHIND or DIRTY, adds the PR to main's merge queue
#    and waits for it to land. Refuses code PRs while a main-red issue is open, and stacked
#    PRs; restacks PRs stacked on the merged branch. --retire retires the worktree with
#    rm-worktree.sh --no-prune.
# Exit codes: 0 ok, 1 conflict, failed check, main-red or queue-failed, 2 refused,
# 3 timeout (status=queued: still merges on its own), 4 git/gh failure, 5 merged but not
# retired. Details: ship.py and docs/agents/pre-pr-checks.md § Merge queue.
set -euo pipefail
exec "${PYTHON:-python}" "$(dirname "$0")/ship.py" "$@"
