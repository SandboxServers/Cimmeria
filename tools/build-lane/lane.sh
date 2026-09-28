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
#  * sccache when it is installed, with one shared cache ($CIMMERIA_SCCACHE_DIR, default
#    $LANE_ROOT/sccache-cache). It caches third-party crates. Workspace crates build
#    incrementally (the dev profile's default), and sccache passes those through uncached.
#    sccache refuses to run at all when CARGO_INCREMENTAL is set to anything but 0, so a
#    caller that sets it gets no sccache.
#    RUSTC_WRAPPER is not sccache itself but sccache-wrap.rs, built here on first use.
#    It hides CARGO_TARGET_DIR (and the other target-dir variables) from sccache, which
#    hashes every CARGO_* variable: with the per-worktree target dir in the key, every
#    worktree had its own cache and the lane scored ~0% hits (issue #1023).
#  * Disk guard: a job refuses to start when the target dir's drive has less than
#    LANE_MIN_FREE_GB free (default 10, 0 turns it off), after pruning this worktree's
#    stale incremental sessions, instead of letting cargo die with "os error 112".
#  * Incremental pruning: after each job, the worktree's stale incremental sessions are
#    deleted (rustc keeps the previous session of every unit next to the one it just
#    wrote, about 45% of the incremental dir). LANE_PRUNE=0 turns it off.
#  * Target dir: each worktree builds into its own target/ (cargo locks a target dir for
#    the whole build, so sharing one would serialise every worktree). When
#    CIMMERIA_TARGET_ROOT is set (a Dev Drive, see tools/dev-drive/), the target dir is
#    $CIMMERIA_TARGET_ROOT/<worktree name> instead.
#  * Job log: every job appends one JSON line to $LANE_ROOT/metrics/jobs.jsonl (wait and
#    run time, exit code, worktree, commit, settings, lowest free RAM, sccache hits and
#    misses, free disk at the start, MB pruned). tools/build-lane/lane_stats.py reports on
#    it. LANE_METRICS=0 turns it off.
#
# Lock layout: $LANE_ROOT/lane/slot.N directories (mkdir is atomic). A dead holder pid
# breaks its own slot.

set -u
LANE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
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
win_path() { cygpath -m "$1" 2>/dev/null || echo "$1"; }  # C:/x form for native programs

# Build sccache-wrap.rs (see the header) once per source version, with plain rustc. Prints
# the wrapper's path, or nothing if it can't be built (the lane then falls back to sccache
# itself, which works but misses across worktrees).
sccache_wrapper() {
  local src="$LANE_DIR/sccache-wrap.rs" ext="" hash dir exe tmp flags=()
  [ -f "$src" ] || return 0
  case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) ext=".exe"; flags=(-C linker=rust-lld -C linker-flavor=lld-link);; esac
  hash="$(sha1sum "$src" 2>/dev/null | cut -c1-12)"; [ -n "$hash" ] || return 0
  dir="$LANE_ROOT/bin/sccache-wrap/$hash"; exe="$dir/sccache$ext"
  if [ ! -x "$exe" ]; then
    tmp="$dir/build.$$"; mkdir -p "$tmp"
    if (cd "$LANE_DIR" && rustc --edition 2021 -O -C debuginfo=0 "${flags[@]}" -o "$tmp/sccache$ext" "$src") >"$tmp/log" 2>&1; then
      mv -f "$tmp/sccache$ext" "$exe" 2>/dev/null   # a lane that raced us may have won; fine
    else
      echo "[lane] warning: could not build $src (see $tmp/log); using sccache directly" >&2
      return 0
    fi
    rm -rf "$tmp"
  fi
  [ -x "$exe" ] && win_path "$exe"
}

if [ -n "$SCCACHE_BIN" ] && [ -z "${RUSTC_WRAPPER+set}" ]; then
  use_sccache=1
  export SCCACHE_DIR="${CIMMERIA_SCCACHE_DIR:-$LANE_ROOT/sccache-cache}"
  export SCCACHE_CACHE_SIZE="${SCCACHE_CACHE_SIZE:-40G}"
  export SCCACHE_IDLE_TIMEOUT=0
  # Path prefixes sccache strips before hashing. sccache 0.18 reads them once, when the
  # server starts, and applies them to C/C++ compiles only; the Rust fix is the wrapper.
  # Only existing absolute dirs: a bad entry stops the sccache server from starting.
  if [ -z "${SCCACHE_BASEDIRS:-}" ]; then
    sep=":"; case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) sep=";";; esac
    main_dir="$(cd "$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null || echo .)/.." 2>/dev/null && pwd)"
    for d in "${CIMMERIA_TARGET_ROOT:-}" "$main_dir"; do
      [ -n "$d" ] && [ -d "$d" ] || continue
      SCCACHE_BASEDIRS="${SCCACHE_BASEDIRS:+$SCCACHE_BASEDIRS$sep}$(win_path "$d")"
    done
    [ -n "${SCCACHE_BASEDIRS:-}" ] && export SCCACHE_BASEDIRS
  fi
  wrapper="$(sccache_wrapper)"
  if [ -n "$wrapper" ]; then
    export CIMMERIA_SCCACHE_REAL; CIMMERIA_SCCACHE_REAL="$(win_path "$SCCACHE_BIN")"
    export RUSTC_WRAPPER="$wrapper"
  else
    export RUSTC_WRAPPER="$SCCACHE_BIN"
  fi
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

# --- disk: incremental pruning and the low-disk guard --------------------------------
TARGET_DIR="$(cygpath -u "${CARGO_TARGET_DIR:-$TOP/target}" 2>/dev/null || echo "${CARGO_TARGET_DIR:-$TOP/target}")"
pruned_kb=0
free_start=""

free_gb() {  # free GB on the drive that holds $1 (or its nearest existing parent)
  local d="$1"
  while [ ! -d "$d" ] && [ "$d" != "$(dirname "$d")" ]; do d="$(dirname "$d")"; done
  df -Pk "$d" 2>/dev/null | awk 'NR == 2 { print int($4 / 1048576) }'
}

other_job_here() {  # true if a slot this job doesn't hold is building this worktree
  local d s mine
  for d in "$LOCKDIR"/slot.*; do
    [ -f "$d/what" ] || continue
    mine=0; for s in "${held[@]:-}"; do [ "$s" = "$d" ] && mine=1; done
    [ $mine -eq 0 ] && grep -q " $NAME :: " "$d/what" 2>/dev/null && return 0
  done
  return 1
}

# rustc keeps the session it started from next to the one it just wrote, in every
# incremental unit dir (<target>/<profile>/incremental/<crate>-<hash>/s-*), and deletes it
# only when that unit next compiles. It loads only the newest finished session, so the
# older ones are dead weight: about 45% of a worktree's incremental dir. A `-working`
# session is a compile in progress, or one that died; only an hour-old one is removed.
# Never touches a worktree another lane job is building.
prune_incremental() {
  [ "${LANE_PRUNE:-1}" != 0 ] && [ -d "$TARGET_DIR" ] || return 0
  other_job_here && return 0
  local stale=() line unit prev="" kb
  # Session names start with a fixed-width base-36 timestamp, so a reverse sort lists each
  # unit's sessions newest first.
  while IFS= read -r line; do
    unit="${line%/*}"
    [ "$unit" = "$prev" ] && stale+=("$line" "${line%-*}.lock")
    prev="$unit"
  done < <(cd "$TARGET_DIR" && find . -mindepth 4 -maxdepth 5 -type d -path '*/incremental/*/s-*' ! -name '*-working' 2>/dev/null | sort -r)
  while IFS= read -r line; do
    stale+=("$line" "${line%-*}.lock")
  done < <(cd "$TARGET_DIR" && find . -mindepth 4 -maxdepth 5 -type d -path '*/incremental/*/s-*-working' -mmin +60 2>/dev/null)
  [ ${#stale[@]} -eq 0 ] && return 0
  kb="$(cd "$TARGET_DIR" && printf '%s\0' "${stale[@]}" | xargs -0 du -sk 2>/dev/null | awk '{ s += $1 } END { print s + 0 }')"
  (cd "$TARGET_DIR" && printf '%s\0' "${stale[@]}" | xargs -0 rm -rf 2>/dev/null)
  pruned_kb=$(( pruned_kb + ${kb:-0} ))
}

# Cargo that runs out of disk dies part-way with "os error 112" and can leave a target dir
# that needs a clean. On 2026-09-28 the Dev Drive filled twice and failed every agent's
# build, so a job refuses to start instead. It first prunes this worktree's stale
# incremental sessions, which is always safe.
disk_guard() {
  local min="${LANE_MIN_FREE_GB:-10}" free
  free="$(free_gb "$TARGET_DIR")"; free_start="$free"
  [ "$min" = 0 ] || [ -z "$free" ] || [ "$free" -ge "$min" ] && return 0
  prune_incremental
  free="$(free_gb "$TARGET_DIR")"; free_start="$free"
  [ -z "$free" ] || [ "$free" -ge "$min" ] && return 0
  cat >&2 <<EOF
[lane] refusing to start: ${free} GB free on the drive that holds $(win_path "$TARGET_DIR"),
[lane] below LANE_MIN_FREE_GB=${min}. Cargo would fail part-way with "os error 112".
[lane] This job did not run. Free space, then run it again:
[lane]   bash tools/build-lane/rm-worktree.sh --merged    # retire merged worktrees (target dir, test DB)
[lane]   pwsh tools/build-hygiene/sweep.ps1 -DryRun        # then without -DryRun, while nothing builds:
[lane]                                                     # stale incremental caches and old feature variants
[lane] LANE_MIN_FREE_GB=0 turns this check off.
EOF
  exit 28
}
disk_guard

t_start="$(now_us)"
echo "[lane] acquired ${#held[@]}/$SLOTS slot(s) after ${waited}s; target=${CARGO_TARGET_DIR:-$TOP/target}; free=${free_start:-?}GB; jobs=$CARGO_BUILD_JOBS; incremental=${CARGO_INCREMENTAL:-default} :: $*" >&2

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
  line+=",\"min_free_mb\":$(json_num "${min_kb:+$((min_kb / 1024))}"),\"mem_total_mb\":$(json_num "${total_kb:+$((total_kb / 1024))}")"
  line+=",\"disk_free_gb\":$(json_num "$free_start"),\"pruned_mb\":$((pruned_kb / 1024))}"
  # mkdir is the lock; a holder that died leaves it behind, so give up waiting after ~5 s.
  while ! mkdir "$METRICS_DIR/.lock" 2>/dev/null; do i=$((i + 1)); [ $i -ge 50 ] && break; sleep 0.1; done
  printf '%s\n' "$line" >> "$METRICS_DIR/jobs.jsonl"
  rmdir "$METRICS_DIR/.lock" 2>/dev/null
}

"$@"
rc=$?
t_end="$(now_us)"
run_ms=$(( (t_end - t_start) / 1000 )); wait_ms=$(( (t_start - t_request) / 1000 ))
prune_incremental
pruned_note=""; [ "$pruned_kb" -gt 0 ] && pruned_note="; pruned $((pruned_kb / 1024)) MB of stale incremental sessions"
echo "[lane] released (exit $rc, ran $(secs $run_ms)s$pruned_note)" >&2
[ "$METRICS" != 0 ] && record_job "$rc" "$wait_ms" "$run_ms"
exit $rc
