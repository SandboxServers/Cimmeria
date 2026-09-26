#!/usr/bin/env bash
# Cimmeria build lane: run a cargo command inside a machine-wide counting semaphore.
#
# Usage:  tools/build-lane/lane.sh [--exclusive] <command> [args...]
#
# Why: a full link of the server or test binaries can use tens of GB of RAM, and several
# Claude sessions and agents build on one workstation at once. Every cargo call that
# compiles goes through this lane, so at most LANE_SLOTS builds run at a time.
#
# What it sets up for the wrapped command:
#  * Slots: LANE_SLOTS (default: the number in $LANE_ROOT/lane/SLOTS, else 2). `--exclusive`
#    takes every slot, for full-workspace builds and measurements.
#  * CARGO_BUILD_JOBS (default 10) caps rustc parallelism per build.
#  * sccache as RUSTC_WRAPPER when it is installed, with one shared cache
#    ($CIMMERIA_SCCACHE_DIR, default $LANE_ROOT/sccache-cache).
#  * Linked git worktrees (agents, campaign workers) build with CARGO_INCREMENTAL=0.
#    sccache cannot cache incremental compilations, and a short-lived worktree gains
#    little from an incremental cache. Without it, crates a worker did not touch come
#    straight from sccache, and the worktree's target/ skips the incremental cache
#    (the largest part of a warm target dir). The main checkout keeps incremental.
#    Override with CARGO_INCREMENTAL=1 in the caller's environment.
#  * Target dir: each worktree builds into its own target/ (cargo locks a target dir for
#    the whole build, so sharing one would serialise every worktree). When
#    CIMMERIA_TARGET_ROOT is set (a Dev Drive, see tools/dev-drive/), the target dir is
#    $CIMMERIA_TARGET_ROOT/<worktree name> instead.
#
# Lock layout: $LANE_ROOT/lane/slot.N directories (mkdir is atomic). A dead holder pid
# breaks its own slot.

set -u
LANE_ROOT="${LANE_ROOT:-${LOCALAPPDATA:-$HOME/.local/share}/cimmeria-build}"
LANE_ROOT="$(cygpath -u "$LANE_ROOT" 2>/dev/null || echo "$LANE_ROOT")"
LOCKDIR="$LANE_ROOT/lane"; mkdir -p "$LOCKDIR"
SLOTS="${LANE_SLOTS:-$(cat "$LOCKDIR/SLOTS" 2>/dev/null || echo 2)}"

exclusive=0
if [ "${1:-}" = "--exclusive" ]; then exclusive=1; shift; fi
if [ $# -eq 0 ]; then echo "usage: lane.sh [--exclusive] <command> [args...]" >&2; exit 2; fi

# --- build environment --------------------------------------------------------------
TOP="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
NAME="$(basename "$TOP")"
# Compare canonical paths: under Git Bash, --absolute-git-dir prints C:/... while pwd
# prints /c/..., so a raw string comparison would call the main checkout a worktree.
canon() { (cd "$1" 2>/dev/null && pwd -P) || true; }
GIT_DIR_ABS="$(canon "$(git rev-parse --absolute-git-dir 2>/dev/null || echo /nonexistent)")"
GIT_COMMON="$(canon "$(git rev-parse --git-common-dir 2>/dev/null || echo /nonexistent)")"
is_worktree=0
[ -n "$GIT_DIR_ABS" ] && [ -n "$GIT_COMMON" ] && [ "$GIT_DIR_ABS" != "$GIT_COMMON" ] && is_worktree=1

if [ -n "${CIMMERIA_TARGET_ROOT:-}" ]; then
  export CARGO_TARGET_DIR="$CIMMERIA_TARGET_ROOT/$NAME"
else
  unset CARGO_TARGET_DIR                     # <worktree>/target
fi
if [ $is_worktree -eq 1 ] && [ -z "${CARGO_INCREMENTAL:-}" ]; then
  export CARGO_INCREMENTAL=0
fi
SCCACHE_BIN="${SCCACHE_BIN:-}"
[ -z "$SCCACHE_BIN" ] && [ -x "$LANE_ROOT/bin/sccache.exe" ] && SCCACHE_BIN="$LANE_ROOT/bin/sccache.exe"
[ -z "$SCCACHE_BIN" ] && SCCACHE_BIN="$(command -v sccache 2>/dev/null || true)"
if [ -n "$SCCACHE_BIN" ] && [ -z "${RUSTC_WRAPPER+set}" ]; then
  export RUSTC_WRAPPER="$SCCACHE_BIN"
  export SCCACHE_DIR="${CIMMERIA_SCCACHE_DIR:-$LANE_ROOT/sccache-cache}"
  export SCCACHE_CACHE_SIZE="${SCCACHE_CACHE_SIZE:-40G}"
  export SCCACHE_IDLE_TIMEOUT=0
fi
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-10}"

# --- acquire ------------------------------------------------------------------------
held=()
release() { for s in "${held[@]:-}"; do [ -n "$s" ] && rm -rf "$s"; done; }
trap 'release' EXIT INT TERM HUP

try_slot() {  # $1 = slot dir, rest = command (for the "what" note); returns 0 if acquired
  local slot="$1"; shift
  if mkdir "$slot" 2>/dev/null; then echo "$$" > "$slot/pid"; echo "$(date '+%H:%M:%S') $NAME :: $*" > "$slot/what"; return 0; fi
  local pid; pid="$(cat "$slot/pid" 2>/dev/null || true)"
  if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then
    echo "[lane] breaking stale slot $(basename "$slot") held by dead pid $pid" >&2; rm -rf "$slot"
    if mkdir "$slot" 2>/dev/null; then echo "$$" > "$slot/pid"; echo "$(date '+%H:%M:%S') $NAME :: $*" > "$slot/what"; return 0; fi
  fi
  return 1
}

waited=0
while :; do
  if [ $exclusive -eq 1 ]; then
    got=0
    for i in $(seq 1 "$SLOTS"); do
      if try_slot "$LOCKDIR/slot.$i" "$@"; then held+=("$LOCKDIR/slot.$i"); got=$((got+1)); fi
    done
    [ "$got" -eq "$SLOTS" ] && break
    release; held=()
  else
    for i in $(seq 1 "$SLOTS"); do
      if try_slot "$LOCKDIR/slot.$i" "$@"; then held+=("$LOCKDIR/slot.$i"); break; fi
    done
    [ ${#held[@]} -gt 0 ] && break
  fi
  if [ $((waited % 60)) -eq 0 ]; then
    echo "[lane] all $SLOTS slots busy: $(for d in "$LOCKDIR"/slot.*; do [ -f "$d/what" ] && printf '[%s] ' "$(head -c 120 "$d/what")"; done) ... ${waited}s" >&2
  fi
  sleep 5; waited=$((waited+5))
done

echo "[lane] acquired ${#held[@]}/$SLOTS slot(s) after ${waited}s; target=${CARGO_TARGET_DIR:-$TOP/target}; jobs=$CARGO_BUILD_JOBS; incremental=${CARGO_INCREMENTAL:-default} :: $*" >&2
"$@"
rc=$?
echo "[lane] released (exit $rc)" >&2
exit $rc
