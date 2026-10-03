#!/usr/bin/env bash
# The mechanical end of a packet in one call, with one line of output.
#
# Usage:
#   tools/build-lane/ship.sh pr -C <worktree> (-m MSG | -F FILE) [--title T] [--body-file F]
#                               [--paths P ...] [--draft] [--force-with-lease] [-v]
#   tools/build-lane/ship.sh merge <PR> [--retire NAME] [--timeout 30m] [--no-wait] [-v]
#
#  * pr refuses (exit 2) unless -C is a registered worktree on a feature branch, then
#    stages, commits with $SHIP_TRAILERS, pushes and opens the PR (footer: $SHIP_PR_FOOTER).
#  * merge squash-merges: Markdown/memory-only PRs at once with --admin, code PRs after the
#    gating checks pass, rebasing with rebase-pr.sh only when GitHub says BEHIND or DIRTY.
#    --retire retires the worktree with rm-worktree.sh --no-prune.
# Exit codes: 0 ok, 1 conflict or failed check, 2 refused, 3 timeout, 4 git/gh failure,
# 5 merged but not retired. Details: ship.py.
set -euo pipefail
exec "${PYTHON:-python}" "$(dirname "$0")/ship.py" "$@"
