#!/usr/bin/env bash
# Release an issue claim from the shared coordination registry.
#
# Run this when your PR for the issue merges, or when you abandon the task
# (finish the worktree cleanup first). Idempotent.
#
# Usage: tools/unclaim.sh <issue>
set -uo pipefail

ISSUE="${1:-}"
if [[ ! "$ISSUE" =~ ^[0-9]+$ ]]; then
    echo "usage: tools/unclaim.sh <issue>" >&2
    exit 2
fi

ROOT=$(git rev-parse --show-toplevel 2>/dev/null) || { echo "not inside a git checkout" >&2; exit 2; }
MAIN_ROOT=$(dirname "$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)" 2>/dev/null)
MAIN_ROOT=${MAIN_ROOT:-$ROOT}
CLAIMS="$MAIN_ROOT/.claude/worktrees/claims.tsv"

if [[ ! -f "$CLAIMS" ]]; then
    echo "no claims file — nothing to release" >&2
    exit 0
fi

if grep -q "^${ISSUE}[[:space:]]" "$CLAIMS"; then
    awk -F '\t' '$1 != issue' issue="$ISSUE" "$CLAIMS" > "$CLAIMS.tmp"
    mv "$CLAIMS.tmp" "$CLAIMS"
    echo "Released claim for #$ISSUE."
else
    echo "issue #$ISSUE was not claimed" >&2
fi

echo "Reminder: clean up the worktree from the main checkout once merged:"
echo "  git worktree remove --force .claude/worktrees/<slug>"
echo "  git branch -D <branch>"
echo "  git push origin --delete <branch>"