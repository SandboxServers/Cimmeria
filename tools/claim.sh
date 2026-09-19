#!/usr/bin/env bash
# Claim a GitHub issue in the shared coordination registry and set up the
# dedicated worktree for it, in one deterministic step.
#
# Usage: tools/claim.sh <issue> <type> [slug] [note]
#   <issue>  GitHub issue number (must pass tools/check-issue.sh)
#   <type>   branch prefix: fix | feat | docs | chore | test | refactor
#   [slug]   worktree dir name under .claude/worktrees/ (default: issue-<issue>)
#   [note]   free text: subsystem, harness, expected files (optional)
#
# Creates: .claude/worktrees/<slug> on branch <type>/<issue>-<slug> from
# origin/main, symlinks external/, and records the claim in
# .claude/worktrees/claims.tsv (gitignored, shared on disk).
#
# Never run against the main checkout — this creates a worktree only.
set -uo pipefail

ISSUE="${1:-}"
TYPE="${2:-}"
SLUG="${3:-issue-$ISSUE}"
NOTE="${4:-}"

if [[ ! "$ISSUE" =~ ^[0-9]+$ ]] || [[ -z "$TYPE" ]]; then
    echo "usage: tools/claim.sh <issue> <type> [slug] [note]" >&2
    exit 2
fi
case "$TYPE" in fix|feat|docs|chore|test|refactor) ;; *)
    echo "type must be one of: fix | feat | docs | chore | test | refactor" >&2
    exit 2
;; esac

ROOT=$(git rev-parse --show-toplevel 2>/dev/null) || { echo "not inside a git checkout" >&2; exit 2; }
MAIN_ROOT=$(dirname "$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)" 2>/dev/null)
MAIN_ROOT=${MAIN_ROOT:-$ROOT}
cd "$ROOT"

"$ROOT/tools/check-issue.sh" "$ISSUE" || exit 1

WT="$MAIN_ROOT/.claude/worktrees/$SLUG"
if [[ -e "$WT" ]]; then
    echo "REFUSE — $WT already exists" >&2
    exit 1
fi

BRANCH="$TYPE/$ISSUE-$SLUG"

mkdir -p "$MAIN_ROOT/.claude/worktrees"
git worktree add -b "$BRANCH" "$WT" origin/main || exit 1
ln -s "$MAIN_ROOT/external" "$WT/external" 2>/dev/null || true

# Record the claim (columns: issue, slug, agent, branch, note, ts).
printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$ISSUE" "$SLUG" "${USER:-unknown}" "$BRANCH" "${NOTE:-}" \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$MAIN_ROOT/.claude/worktrees/claims.tsv"

echo "Claimed #$ISSUE:"
echo "  worktree: $WT"
echo "  branch:   $BRANCH"
echo
echo "Next: cd $WT && git status -sb"
echo "When the PR is merged or the task abandoned, release the claim:"
echo "  tools/unclaim.sh $ISSUE"