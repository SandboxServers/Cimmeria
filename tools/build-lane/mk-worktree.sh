#!/usr/bin/env bash
# Create a worktree off origin/main under .claude/worktrees/<name>, ready to build.
#
# Usage: tools/build-lane/mk-worktree.sh <branch> <name>
#
#  * Links the gitignored external/ tree in: a junction under Git Bash on Windows, a symlink
#    elsewhere (crates/entity/build.rs compiles Detour from ../../external/recast, so a bare
#    worktree cannot build).
#  * When CIMMERIA_TARGET_ROOT points at a Dev Drive (tools/dev-drive/), seeds the new
#    worktree's target dir from the warmest existing one with ReFS block cloning, so the
#    first build is incremental instead of cold, at almost no disk cost.
set -euo pipefail
BRANCH="${1:?branch}"; NAME="${2:?name}"
MAIN="$(dirname "$(cd "$(git rev-parse --git-common-dir)" && pwd)")"
WT="$MAIN/.claude/worktrees/$NAME"
cd "$MAIN"
git fetch -q origin
if [ -d "$WT" ]; then echo "exists: $WT"; exit 0; fi
git worktree add -q -b "$BRANCH" "$WT" origin/main
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    WIN_WT="$(cygpath -w "$WT")"
    WIN_MAIN="$(cygpath -w "$MAIN")"
    powershell -NoProfile -Command "New-Item -ItemType Junction -Path '$WIN_WT\\external' -Target '$WIN_MAIN\\external' | Out-Null"
    echo "created $WT on branch $BRANCH (external/ junctioned)"
    ;;
  *)
    ln -s "$MAIN/external" "$WT/external"
    echo "created $WT on branch $BRANCH (external/ symlinked)"
    [ -e "$MAIN/external" ] || echo "warning: $MAIN/external does not exist yet; populate it (docs/agents/development-workflow.md#create-a-worktree-that-builds)" >&2
    ;;
esac

# The Dev Drive seeding is Windows-only (ReFS block cloning through PowerShell).
if [ -n "${CIMMERIA_TARGET_ROOT:-}" ] && command -v pwsh >/dev/null 2>&1; then
  pwsh -NoProfile -File "$MAIN/tools/dev-drive/Copy-WarmTarget.ps1" -TargetRoot "$CIMMERIA_TARGET_ROOT" -Name "$NAME" \
    || echo "warning: target seeding failed; the first build will be cold" >&2
fi
