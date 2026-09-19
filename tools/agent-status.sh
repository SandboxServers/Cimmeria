#!/usr/bin/env bash
# One-shot agent coordination snapshot.
#
# Prints: issue claims, worktrees (flagging any outside the convention),
# open PRs per issue, local-main health (drift + dirty files + orphaned
# commits), running cargo/rustc, and a warnings section. Read this before
# picking work or before building, so you don't duplicate a claim, branch
# off a polluted main, or run cargo concurrently with another agent.
#
# Usage: tools/agent-status.sh [--quiet]
set -uo pipefail

ROOT=$(git rev-parse --show-toplevel 2>/dev/null) || {
    echo "not inside a git checkout" >&2
    exit 1
}
MAIN_ROOT=$(dirname "$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)" 2>/dev/null)
MAIN_ROOT=${MAIN_ROOT:-$ROOT}
cd "$ROOT"

CLAIMS="$MAIN_ROOT/.claude/worktrees/claims.tsv"
VERBOSE=1
[[ "${1:-}" == "--quiet" ]] && VERBOSE=0

echo "Cimmeria agent coordination — $(date +%Y-%m-%dT%H:%M:%S)"
echo

# ── 1. Claims registry ────────────────────────────────────────────────
echo "== ISSUE CLAIMS ($CLAIMS)"
if [[ -f "$CLAIMS" ]]; then
    # columns: issue, slug, agent, branch, note
    awk -F '\t' 'NF >= 4 { printf "  #%s  %-28s agent=%s branch=%s", $1, $2, $3, $4; if (NF >= 5) printf "  (%s)", $5; printf "\n" }' "$CLAIMS"
    if ! grep -q '[^[:space:]]' "$CLAIMS"; then echo "  (empty)"; fi
else
    echo "  no claims file yet — nothing claimed"
fi
echo

# ── 2. Worktrees ──────────────────────────────────────────────────────
echo "== WORKTREES"
git worktree list --porcelain | awk '
  /^worktree / { wt=$0; sub(/^worktree /,"",wt) }
  /^branch /   { br=$0; sub(/^branch /,"",br); sub("refs/heads/","",br)
                 ok = (wt ~ /\/\.claude\/worktrees\//)
                 flag = ok ? "" : "  <-- OUTSIDE .claude/worktrees/ convention"
                 printf "  %-60s  %-45s %s\n", wt, br, flag }
'
echo

# ── 3. Open PRs per issue ─────────────────────────────────────────────
echo "== OPEN PRs (issue tags)"
if command -v gh >/dev/null 2>&1; then
    gh pr list --state open --limit 100 --json number,title,headRefName \
        --jq '.[] | "  #\(.number)  \(.headRefName): \(.title)"' 2>/dev/null \
        | head -40
    [[ ${PIPESTATUS[0]} -ne 0 ]] && echo "  (gh pr list failed)"
else
    echo "  (gh not available)"
fi
echo

# ── 4. Main checkout health ───────────────────────────────────────────
MAIN=$(git worktree list --porcelain \
    | awk '/^worktree /{wt=$0;sub(/^worktree /,"",wt)} /^branch refs\/heads\/main$/{print wt}')
if [[ -n "$MAIN" ]]; then
    echo "== MAIN CHECKOUT ($MAIN)"
    if [[ -d "$MAIN/.git" || -f "$MAIN/.git" ]]; then :; else
        echo "  (main worktree dir missing?)"
    fi
    AHEAD=$(git -C "$MAIN" rev-list --count origin/main..HEAD 2>/dev/null || echo "?")
    BEHIND=$(git -C "$MAIN" rev-list --count HEAD..origin/main 2>/dev/null || echo "?")
    echo "  vs origin/main: ahead=$AHEAD behind=$BEHIND"
    if [[ "$AHEAD" =~ ^[0-9]+$ ]] && (( AHEAD > 0 )); then
        echo "  commits on local main NOT on origin (rescue if they are real work):"
        git -C "$MAIN" log --oneline origin/main..HEAD 2>/dev/null | sed 's/^/    /'
    fi
    DIRTY=$(git -C "$MAIN" status --porcelain 2>/dev/null)
    if [[ -n "$DIRTY" ]]; then
        echo "  dirty/untracked in main checkout:"
        echo "$DIRTY" | sed 's/^/    /' | head -20
    else
        echo "  main checkout is clean"
    fi
else
    echo "== main worktree not found (unusual)"
fi
echo

# ── 5. Running cargo/rustc ────────────────────────────────────────────
echo "== CARGO/RUSTC running"
RUNNING=$(pgrep -af 'cargo|rustc' 2>/dev/null | grep -v "pgrep -af" || true)
if [[ -n "$RUNNING" ]]; then
    echo "  BUSY — wait for these to finish before building (rule: no concurrent cargo):"
    echo "$RUNNING" | head -8 | sed 's/^/    /'
else
    echo "  none — safe to build"
fi
echo

# ── 6. Cross-checks ───────────────────────────────────────────────────
echo "== CHECKS"
WARN=0
if [[ -f "$CLAIMS" ]]; then
    tail -n +2 "$CLAIMS" 2>/dev/null | while IFS=$'\t' read -r iss slug agent branch note; do
        [[ -z "$iss" ]] && continue
        if [[ ! -d "$MAIN_ROOT/.claude/worktrees/$slug" ]]; then
            echo "  WARN: #$iss claimed (slug=$slug, agent=$agent) but no worktree $MAIN_ROOT/.claude/worktrees/$slug"
            WARN=1
        fi
    done
fi
worktree_list=$(git worktree list --porcelain)
for slug in "$MAIN_ROOT"/.claude/worktrees/*/; do
    [[ -e "$slug" ]] || continue
    base=$(basename "$slug")
    if [[ "$base" != "claims.tsv" ]] && ! grep -q "$base" "$CLAIMS" 2>/dev/null; then
        branch_of=$(printf '%s\n' "$worktree_list" | awk -v wt="$MAIN_ROOT/.claude/worktrees/$base" '
            /^worktree /{cur=$0;sub(/^worktree /,"",cur)} /^branch /{if(cur==wt){b=$0;sub(/^branch refs\/heads\//,"",b);print b}}')
        echo "  NOTE: worktree $base ($branch_of) has no claims.tsv entry"
    fi
done
[[ "$WARN" -eq 0 ]] && echo "  no warnings"