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
#  * CARGO_BUILD_JOBS defaults to cores / slots (floor 4) so a full lane fits the CPU.
#  * sccache as RUSTC_WRAPPER when it is installed, with one shared cache
#    ($CIMMERIA_SCCACHE_DIR, default $LANE_ROOT/sccache-cache). It caches third-party
#    crates. Workspace crates build incrementally (the dev profile's default), and sccache
#    passes those through uncached. Worktrees used to force CARGO_INCREMENTAL=0 so sccache
#    could cache workspace crates as well, but sccache keys a crate by its absolute path,
#    so one worktree's crates never hit in another: the job log showed 0% hits while the
#    edit loop lost its incremental reuse. sccache refuses to run at all when
#    CARGO_INCREMENTAL is set to anything but 0, so a caller that sets it gets no sccache.
#  * Target dir: each worktree builds into its own target/ (cargo locks a target dir for
#    the whole build, so sharing one would serialise every worktree). When
#    CIMMERIA_TARGET_ROOT is set (a Dev Drive, see tools/dev-drive/), the target dir is
#    $CIMMERIA_TARGET_ROOT/<worktree name> instead.
#  * Job log: every job appends one JSON line to $LANE_ROOT/metrics/jobs.jsonl (wait and
#    run time, exit code, worktree, commit, settings, lowest free RAM, sccache hits and
#    misses). tools/build-lane/lane_stats.py reports on it. LANE_METRICS=0 turns it off.
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

# Microseconds since the epoch (EPOCHREALTIME is bash 5; its separator follows the locale).
now_us() { if [ -n "${EPOCHREALTIME:-}" ]; then echo "${EPOCHREALTIME/[.,]/}"; else echo "$(date +%s)000000"; fi; }

# --- build environment --------------------------------------------------------------
TOP="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
NAME="$(basename "$TOP")"

# The Dev Drive settings are user environment variables (tools/dev-drive/). A session
# started before they were set doesn't have them, so fall back to the registry.
user_env() {  # $1 = variable name; prints its HKCU\Environment value, if any
  command -v reg >/dev/null 2>&1 || return 0
  MSYS_NO_PATHCONV=1 reg query 'HKCU\Environment' /v "$1" 2>/dev/null \
    | sed -n "s/^ *$1 *REG_[A-Z_]* *//p" | tr -d '\r'
}
[ -z "${CIMMERIA_TARGET_ROOT:-}" ] && CIMMERIA_TARGET_ROOT="$(user_env CIMMERIA_TARGET_ROOT)"
[ -z "${CIMMERIA_SCCACHE_DIR:-}" ] && CIMMERIA_SCCACHE_DIR="$(user_env CIMMERIA_SCCACHE_DIR)"

# A worktree moves its builds to the Dev Drive the first time it builds without a warm
# local target/ (a new worktree). One that already has a local target/ stays put, so a
# job in flight never turns cold; CIMMERIA_FORCE_DEV_DRIVE=1 moves it anyway.
use_dev_drive=0
if [ -n "${CIMMERIA_TARGET_ROOT:-}" ] && [ -d "$CIMMERIA_TARGET_ROOT" ]; then
  if [ -d "$CIMMERIA_TARGET_ROOT/$NAME" ] || [ ! -d "$TOP/target" ] || [ "${CIMMERIA_FORCE_DEV_DRIVE:-0}" = 1 ]; then
    use_dev_drive=1
  fi
fi
if [ $use_dev_drive -eq 1 ]; then
  export CARGO_TARGET_DIR="$CIMMERIA_TARGET_ROOT/$NAME"
else
  unset CARGO_TARGET_DIR                     # <worktree>/target
fi
SCCACHE_BIN="${SCCACHE_BIN:-}"
[ -z "$SCCACHE_BIN" ] && [ -x "$LANE_ROOT/bin/sccache.exe" ] && SCCACHE_BIN="$LANE_ROOT/bin/sccache.exe"
[ -z "$SCCACHE_BIN" ] && SCCACHE_BIN="$(command -v sccache 2>/dev/null || true)"
use_sccache=0
if [ -n "${CARGO_INCREMENTAL:-}" ] && [ "$CARGO_INCREMENTAL" != 0 ]; then
  SCCACHE_BIN=""                             # sccache aborts under CARGO_INCREMENTAL=1
fi
if [ -n "$SCCACHE_BIN" ] && [ -z "${RUSTC_WRAPPER+set}" ]; then
  use_sccache=1
  export RUSTC_WRAPPER="$SCCACHE_BIN"
  export SCCACHE_DIR="${CIMMERIA_SCCACHE_DIR:-$LANE_ROOT/sccache-cache}"
  export SCCACHE_CACHE_SIZE="${SCCACHE_CACHE_SIZE:-40G}"
  export SCCACHE_IDLE_TIMEOUT=0
fi
# Split the cores across the slots so a full lane doesn't oversubscribe the CPU
# (e.g. 32 cores / 4 slots = 8 jobs per build), with a floor of 4.
if [ -z "${CARGO_BUILD_JOBS:-}" ]; then
  cores="$(nproc 2>/dev/null || echo 8)"
  CARGO_BUILD_JOBS=$(( cores / SLOTS )); [ "$CARGO_BUILD_JOBS" -lt 4 ] && CARGO_BUILD_JOBS=4
fi
export CARGO_BUILD_JOBS

# --- acquire ------------------------------------------------------------------------
held=()
sampler_pid=""
release() {
  for s in "${held[@]:-}"; do [ -n "$s" ] && rm -rf "$s"; done
  [ -n "$sampler_pid" ] && kill "$sampler_pid" 2>/dev/null
}
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

t_request="$(now_us)"
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

# rm-worktree.sh marks a worktree before it checks the slots; this checks for the mark
# after taking a slot and writing its `what` (rm-worktree.sh counts a slot without one as
# busy), so a build never starts in a worktree that is being deleted.
if [ -d "$LOCKDIR/retiring.$NAME" ]; then
  echo "[lane] $NAME is being retired by rm-worktree.sh; not building" >&2; exit 75
fi

t_start="$(now_us)"
echo "[lane] acquired ${#held[@]}/$SLOTS slot(s) after ${waited}s; target=${CARGO_TARGET_DIR:-$TOP/target}; jobs=$CARGO_BUILD_JOBS; incremental=${CARGO_INCREMENTAL:-default} :: $*" >&2

# --- job log ------------------------------------------------------------------------
METRICS="${LANE_METRICS:-1}"
METRICS_DIR="${LANE_METRICS_DIR:-$LANE_ROOT/metrics}"
cmd_str="$*"
json_str() {  # $1 as a JSON string
  local s="$1"
  s="${s//\\/\\\\}"; s="${s//\"/\\\"}"; s="${s//$'\t'/\\t}"; s="${s//$'\r'/\\r}"; s="${s//$'\n'/\\n}"
  printf '"%s"' "$s"
}
json_bool() { [ "$1" -eq 1 ] && printf true || printf false; }
json_num() { if [ -n "${1:-}" ]; then printf '%s' "$1"; else printf null; fi; }
secs() { printf '%d.%03d' $(( $1 / 1000 )) $(( $1 % 1000 )); }  # ms -> "s.mmm"
meminfo_kb() { local k v r; while read -r k v r; do [ "$k" = "$1:" ] && { echo "$v"; return; }; done < /proc/meminfo; }
sccache_counts() {  # "<hits> <misses>" from the sccache server (shared by every build in flight)
  "$SCCACHE_BIN" --show-stats 2>/dev/null | tr -d '\r' \
    | awk '/^Cache hits +[0-9]+$/ {h=$3} /^Cache misses +[0-9]+$/ {m=$3} END {print h, m}'
}

if [ "$METRICS" != 0 ]; then
  mkdir -p "$METRICS_DIR"
  start_iso="$(date '+%Y-%m-%dT%H:%M:%S%z')"
  commit="$(git rev-parse --short=10 HEAD 2>/dev/null || true)"
  branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)"
  busy=0; for d in "$LOCKDIR"/slot.*; do [ -d "$d" ] && busy=$((busy + 1)); done
  busy=$((busy - ${#held[@]}))               # slots other builds held when this one started
  h0=""; m0=""; [ $use_sccache -eq 1 ] && read -r h0 m0 <<<"$(sccache_counts)"
  memfile="$METRICS_DIR/.minfree.$$"
  if [ -r /proc/meminfo ]; then                # lowest free RAM during the job, every 2 s
    ( min=""; while :; do
        v="$(meminfo_kb MemFree)"
        if [ -n "$v" ] && { [ -z "$min" ] || [ "$v" -lt "$min" ]; }; then min=$v; echo "$min" > "$memfile"; fi
        sleep 2
      done ) &
    sampler_pid=$!
  fi
fi

record_job() {  # $1 = exit code, $2 = wait ms, $3 = run ms
  local min_kb="" total_kb="" h1="" m1="" dh="" dm="" line i=0
  if [ -n "$sampler_pid" ]; then
    kill "$sampler_pid" 2>/dev/null; wait "$sampler_pid" 2>/dev/null; sampler_pid=""
    min_kb="$(cat "$memfile" 2>/dev/null)"; rm -f "$memfile"
    total_kb="$(meminfo_kb MemTotal)"
  fi
  if [ $use_sccache -eq 1 ]; then
    read -r h1 m1 <<<"$(sccache_counts)"
    # The counters restart with the server; skip the delta rather than go negative.
    [ -n "$h0" ] && [ -n "$h1" ] && [ "$h1" -ge "$h0" ] && dh=$((h1 - h0))
    [ -n "$m0" ] && [ -n "$m1" ] && [ "$m1" -ge "$m0" ] && dm=$((m1 - m0))
  fi
  line="{\"v\":1,\"t\":$(( t_start / 1000000 )),\"start\":$(json_str "$start_iso")"
  line+=",\"worktree\":$(json_str "$NAME"),\"branch\":$(json_str "$branch"),\"commit\":$(json_str "$commit")"
  line+=",\"cmd\":$(json_str "$cmd_str"),\"exit\":$1,\"wait_s\":$(secs "$2"),\"run_s\":$(secs "$3")"
  line+=",\"exclusive\":$(json_bool $exclusive),\"slots\":${#held[@]},\"slots_total\":$SLOTS,\"busy_at_start\":$busy"
  line+=",\"jobs\":$CARGO_BUILD_JOBS,\"incremental\":$(json_str "${CARGO_INCREMENTAL:-default}")"
  line+=",\"dev_drive\":$(json_bool $use_dev_drive),\"target\":$(json_str "${CARGO_TARGET_DIR:-$TOP/target}")"
  line+=",\"sccache\":$(json_bool $use_sccache),\"sccache_hits\":$(json_num "$dh"),\"sccache_misses\":$(json_num "$dm")"
  line+=",\"min_free_mb\":$(json_num "${min_kb:+$((min_kb / 1024))}"),\"mem_total_mb\":$(json_num "${total_kb:+$((total_kb / 1024))}")}"
  # mkdir is the lock; a holder that died leaves it behind, so give up waiting after ~5 s.
  while ! mkdir "$METRICS_DIR/.lock" 2>/dev/null; do i=$((i + 1)); [ $i -ge 50 ] && break; sleep 0.1; done
  printf '%s\n' "$line" >> "$METRICS_DIR/jobs.jsonl"
  rmdir "$METRICS_DIR/.lock" 2>/dev/null
}

"$@"
rc=$?
t_end="$(now_us)"
run_ms=$(( (t_end - t_start) / 1000 )); wait_ms=$(( (t_start - t_request) / 1000 ))
echo "[lane] released (exit $rc, ran $(secs $run_ms)s)" >&2
[ "$METRICS" != 0 ] && record_job "$rc" "$wait_ms" "$run_ms"
exit $rc
