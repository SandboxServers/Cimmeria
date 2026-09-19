#!/usr/bin/env bash
# Preflight gate before starting work on a GitHub issue.
#
# Refuses to proceed when the issue is closed, already claimed in the
# shared claims registry, has a worktree under .claude/worktrees/, or has
# an open PR referencing it. Run this FIRST; claim.sh runs it for you.
#
# Why: agents have repeatedly picked issues that were already closed
# (issues auto-close on PR merge) or already being worked on by another
# agent's worktree — the wasted work is the coordination failure this
# kit exists to prevent.
#
# Usage: tools/check-issue.sh <issue-number>
set -uo pipefail

ISSUE="${1:-}"
if [[ ! "$ISSUE" =~ ^[0-9]+$ ]]; then
    echo "usage: tools/check-issue.sh <issue-number>" >&2
    exit 2
fi

ROOT=$(git rev-parse --show-toplevel 2>/dev/null) || {
    echo "not inside a git checkout" >&2
    exit 2
}
MAIN_ROOT=$(dirname "$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)" 2>/dev/null)
MAIN_ROOT=${MAIN_ROOT:-$ROOT}
CLAIMS="$MAIN_ROOT/.claude/worktrees/claims.tsv"

fail() { echo "REFUSE — $*" >&2; exit 1; }

if ! command -v gh >/dev/null 2>&1; then
    echo "WARN: gh not available; skipping remote issue/PR checks" >&2
else
    state=$(gh issue view "$ISSUE" --json state --jq .state 2>/dev/null)
    if [[ -z "$state" ]]; then
        fail "issue #$ISSUE not found or gh api failed"
    elif [[ "$state" != "OPEN" ]]; then
        fail "issue #$ISSUE is ${state} on GitHub — check if a merged PR already resolved it"
    fi
fi

if [[ -f "$CLAIMS" ]] && grep -q "^${ISSUE}[[:space:]]" "$CLAIMS"; then
    row=$(grep "^${ISSUE}[[:space:]]" "$CLAIMS" | head -1)
    fail "issue #$ISSUE is already claimed in $CLAIMS: $row"
fi

if [[ -d "$MAIN_ROOT/.claude/worktrees/issue-$ISSUE" ]]; then
    fail "a worktree already exists at $MAIN_ROOT/.claude/worktrees/issue-$ISSUE — another agent is on it"
fi

if command -v gh >/dev/null 2>&1; then
    inflight=$(gh pr list --state open --search "#$ISSUE" --json number,title \
        --jq '.[] | "#\(.number) \(.title)"' 2>/dev/null | head -1)
    if [[ -n "$inflight" ]]; then
        fail "an open PR already references #$ISSUE: $inflight"
    fi
fi

echo "PASS — issue #$ISSUE is open, unclaimed, and has no in-flight PR or worktree."
echo "Next: tools/claim.sh $ISSUE <type> <slug>  (type=fix|feat|docs|chore|test|refactor)"
exit 0