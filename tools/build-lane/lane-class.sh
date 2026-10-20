# shellcheck shell=bash
# The build lane's job classes, for lane.sh (sourced, not run). The PowerShell twin is
# lane-class.ps1; both read fixtures/lane-classes.tsv, which holds the rules equal. Design:
# docs/analysis/lane-classes/design.md (§ 4.1-4.6).
#
# The lane has three classes. Each is its own counting semaphore in $LOCKDIR (set by the
# caller), and each uses the slot protocol of lane.sh:
#
#   heavy   slot.N         compiling cargo, and anything the classifier can't name.
#                          Count: LANE_SLOTS, else lane/SLOTS, else 2.
#   light   slot.light.N   rustfmt, cargo fmt/metadata/..., cargo check -p X.
#                          Count: LANE_LIGHT_SLOTS, else lane/LIGHT_SLOTS, else 4.
#   db      slot.db.N      the live-DB run phase; only by an explicit --class db.
#                          Count: LANE_DB_SLOTS, else lane/DB_SLOTS, else 3.
#
# A light or db count of 0 sends that class's jobs to heavy (the kill switch). A count
# that doesn't parse counts as unset.

# Prints a count from a text value: a non-negative integer of up to 4 digits that is at
# least $2 (heavy: 1). Prints nothing for junk.
lane_count_value() {  # $1 = text, $2 = minimum
  local n
  [[ "$1" =~ ^[[:space:]]*([0-9]{1,4})[[:space:]]*$ ]] || return 0
  n=$((10#${BASH_REMATCH[1]}))
  [ "$n" -ge "$2" ] && echo "$n"
  return 0
}

# Prints the class a command runs in, from its words alone: heavy or light (never db).
# $@ is the command as lane.sh received it, the program first.
lane_class() {
  local exe sub w j pkg=0 all=0
  local -a rest
  [ $# -eq 0 ] && { echo heavy; return 0; }
  exe="$(printf '%s' "${1##*[\\/]}" | tr 'A-Z' 'a-z')"; exe="${exe%.exe}"
  if [ "$exe" = rustfmt ]; then echo light; return 0; fi
  if [ "$exe" != cargo ]; then echo heavy; return 0; fi
  shift
  case "${1:-}" in +*) shift;; esac             # cargo +toolchain
  # Global options: --color, --config, -Z and -C take a value; any other dash word is one.
  while [ $# -gt 0 ]; do
    case "$1" in
      --color|--config|-Z|-C) shift; [ $# -gt 0 ] && shift ;;
      -*) shift ;;
      *) break ;;
    esac
  done
  if [ $# -eq 0 ]; then echo light; return 0; fi   # cargo --version, and bare cargo
  sub="$1"; shift; rest=("$@")
  case "$sub" in
    fmt|metadata|tree|version|locate-project|pkgid|verify-project|search|help|update|generate-lockfile|fetch|hakari|machete)
      echo light; return 0 ;;
    check|c) ;;
    *) echo heavy; return 0 ;;
  esac
  # cargo check is light only for one named package: -p X, --package X or --package=X,
  # and no --workspace or --all.
  for ((j = 0; j < ${#rest[@]}; j++)); do
    w="${rest[j]}"
    if [ "$w" = -p ] || [ "$w" = --package ]; then
      if [ $((j + 1)) -lt ${#rest[@]} ]; then case "${rest[j+1]}" in -*) ;; *) pkg=1;; esac; fi
    else
      case "$w" in --package=?*) pkg=1;; --workspace|--all) all=1;; esac
    fi
  done
  if [ $pkg -eq 1 ] && [ $all -eq 0 ]; then echo light; else echo heavy; fi
  return 0
}

# Prints the slot count of a class: LANE_<X>_SLOTS, else <LOCKDIR>/<X>SLOTS, else the default.
lane_class_count() {  # $1 = class
  local env file dflt min=0 n
  case "$1" in
    light) env=LANE_LIGHT_SLOTS; file=LIGHT_SLOTS; dflt=4 ;;
    db)    env=LANE_DB_SLOTS; file=DB_SLOTS; dflt=3 ;;
    *)     env=LANE_SLOTS; file=SLOTS; dflt=2; min=1 ;;
  esac
  n="$(lane_count_value "${!env:-}" "$min")"
  [ -n "$n" ] || n="$(lane_count_value "$(cat "$LOCKDIR/$file" 2>/dev/null)" "$min")"
  echo "${n:-$dflt}"
}

# Sets the globals class_req (the class asked for: the flag, else the classifier, heavy with
# --exclusive), class (the class that runs it: heavy when that class's count is 0) and
# class_slots. $1 = 1 if --class was given, $2 = its value, $3 = 1 with --exclusive, then
# the command.
lane_resolve() {
  local given="$1" flag="$2" excl="$3"
  shift 3
  if [ "$given" -eq 1 ]; then class_req="$flag"
  elif [ "$excl" -eq 1 ]; then class_req=heavy
  else class_req="$(lane_class "$@")"; fi
  class="$class_req"; class_slots="$(lane_class_count "$class")"
  if [ "$class_slots" -eq 0 ]; then class=heavy; class_slots="$(lane_class_count heavy)"; fi
}

# Prints the path of slot <i> of a class: slot.<i>, slot.light.<i> or slot.db.<i>.
lane_slot_path() {  # $1 = class, $2 = index
  case "$1" in
    light) echo "$LOCKDIR/slot.light.$2" ;;
    db)    echo "$LOCKDIR/slot.db.$2" ;;
    *)     echo "$LOCKDIR/slot.$2" ;;
  esac
}

# Prints the slot dirs of one class, one per line (a glob of slot.* would list every class).
lane_class_dirs() {  # $1 = class
  local d n re
  case "$1" in
    light) re='^slot\.light\.[0-9]+$' ;;
    db)    re='^slot\.db\.[0-9]+$' ;;
    *)     re='^slot\.[0-9]+$' ;;
  esac
  for d in "$LOCKDIR"/slot.*; do
    n="${d##*/}"
    if [[ "$n" =~ $re ]] && [ -d "$d" ]; then echo "$d"; fi
  done
  return 0
}

# Prints the CARGO_BUILD_JOBS of a job that didn't set one. Heavy shares the cores among
# the $2 heavy slots (floor 4); light and db use LANE_LIGHT_JOBS (default 4).
lane_class_jobs() {  # $1 = class, $2 = heavy slot count
  local cores n
  if [ "$1" = heavy ]; then
    cores="$(nproc 2>/dev/null || echo 8)"; n=$((cores / $2))
    [ "$n" -lt 4 ] && n=4
    echo "$n"
  else
    n="$(lane_count_value "${LANE_LIGHT_JOBS:-}" 1)"; echo "${n:-4}"
  fi
  return 0
}

# Prints the slot dir this lane call runs inside, or nothing. A call is nested when
# CIMMERIA_LANE_SLOT names a dir whose `token` file holds CIMMERIA_LANE_TOKEN: the token,
# not the path, stops a detached grandchild of a finished job from riding a slot that
# someone else now holds.
lane_nested() {
  local slot="${CIMMERIA_LANE_SLOT:-}" held
  [ -n "$slot" ] && [ -n "${CIMMERIA_LANE_TOKEN:-}" ] && [ -d "$slot" ] || return 0
  held="$(tr -d ' \t\r\n' 2>/dev/null < "$slot/token")"
  [ -n "$held" ] && [ "$held" = "$CIMMERIA_LANE_TOKEN" ] && echo "$slot"
  return 0
}

# Prints a fresh token for a job's slot: 32 hex digits.
lane_token() { od -An -tx1 -N16 /dev/urandom | tr -d ' \n'; }
