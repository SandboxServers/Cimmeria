#!/usr/bin/env bash
# Retire worktrees whose work has merged: the counterpart of mk-worktree.sh.
#
# Usage:
#   tools/build-lane/rm-worktree.sh [--dry-run] [--force] <name> [<name>...]
#   tools/build-lane/rm-worktree.sh [--dry-run] --merged
#
# <name> is the folder under .claude/worktrees/ (mk-worktree.sh names, agent-*, wf_*).
# --merged retires every worktree there whose branch's PR has merged and that nothing
# has touched for $RM_WORKTREE_MIN_IDLE minutes (default 30), so a session still sitting
# in a just-merged worktree keeps it. It then deletes target dirs on the Dev Drive that no
# registered worktree owns any more.
#
# For each worktree it deletes the build output ($CIMMERIA_TARGET_ROOT/<name> and
# <worktree>/target), unlinks the external/ junction, runs `git worktree remove`,
# deletes the local branch, and drops the worktree's own test database (sgw_<name>, the
# name reload-db.sh gives it) when the bundled Postgres is reachable. It refuses, unless --force, when:
#  * a lane job is building in that worktree right now (--force does not override this);
#  * the worktree has uncommitted changes, or is locked (agent worktrees are locked while
#    their agent runs; --force unlocks a stale lock);
#  * the branch's PR is open or was closed unmerged, or there is no PR and the branch
#    has commits origin/main doesn't (unpushed work). PR state comes from `gh`; without
#    it, only branches already on origin/main count as merged.
#
# external/ is removed with a non-recursive rmdir before anything else: a recursive
# delete can follow the junction and empty the real external/ directory.
set -uo pipefail

DRY=0; FORCE=0; MERGED=0; NAMES=()
for a in "$@"; do
  case "$a" in
    --dry-run) DRY=1 ;;
    --force) FORCE=1 ;;
    --merged) MERGED=1 ;;
    -h|--help) sed -n '2,26p' "$0"; exit 0 ;;
    -*) echo "unknown option: $a" >&2; exit 2 ;;
    *) NAMES+=("$a") ;;
  esac
done
if [ $MERGED -eq 0 ] && [ ${#NAMES[@]} -eq 0 ]; then
  echo "usage: rm-worktree.sh [--dry-run] [--force] <name>... | --merged" >&2; exit 2
fi

# Paths in the form `git worktree list` prints them (C:/... on Windows), so they compare.
MAIN="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"
WTROOT="$MAIN/.claude/worktrees"
LANE_ROOT="${LANE_ROOT:-${LOCALAPPDATA:-$HOME/.local/share}/cimmeria-build}"
LANE_ROOT="$(cygpath -u "$LANE_ROOT" 2>/dev/null || echo "$LANE_ROOT")"

# Same lookup as lane.sh: the Dev Drive root is a user environment variable, read from
# the registry when this shell predates it.
user_env() {
  command -v reg >/dev/null 2>&1 || return 0
  MSYS_NO_PATHCONV=1 reg query 'HKCU\Environment' /v "$1" 2>/dev/null \
    | sed -n "s/^ *$1 *REG_[A-Z_]* *//p" | tr -d '\r'
}
[ -z "${CIMMERIA_TARGET_ROOT:-}" ] && CIMMERIA_TARGET_ROOT="$(user_env CIMMERIA_TARGET_ROOT)"
TROOT=""
[ -n "${CIMMERIA_TARGET_ROOT:-}" ] && TROOT="$(cygpath -u "$CIMMERIA_TARGET_ROOT" 2>/dev/null || echo "$CIMMERIA_TARGET_ROOT")"

run() { if [ $DRY -eq 1 ]; then echo "  would: $*"; else "$@"; fi; }

# The worktree's test database, named as reload-db.sh names it. Only that exact name is
# dropped; hand-made databases (sgw_harset and the like) are never swept.
PSQL="${PSQL:-$MAIN/external/postgresql_server/bin/psql.exe}"
[ -x "$PSQL" ] || PSQL="$(command -v psql 2>/dev/null || true)"
db_name() { printf 'sgw_%s' "$(printf '%s' "$1" | tr -c 'A-Za-z0-9_' '_' | tr 'A-Z' 'a-z')"; }
drop_db() {  # $1 = worktree name
  local db other
  db="$(db_name "$1")"
  [ -n "$PSQL" ] || return 0
  # The mapping isn't one-to-one (foo-bar and foo_bar both give sgw_foo_bar): keep a
  # database that another registered worktree also maps to.
  while IFS= read -r other; do
    other="$(basename "$other")"
    if [ "$other" != "$1" ] && [ "$(db_name "$other")" = "$db" ]; then
      echo "  kept database $db: worktree $other maps to it too"; return 0
    fi
  done < <(git worktree list --porcelain | sed -n 's/^worktree //p')
  local q=(-h localhost -p "${PGPORT:-5433}" -U w-testing -d postgres -tAq -v ON_ERROR_STOP=1)
  local exists
  # psql.exe prints CRLF; an untrimmed \r makes every name "not exist".
  exists="$(PGPASSWORD="${PGPASSWORD:-w-testing}" "$PSQL" "${q[@]}" -c "SELECT 1 FROM pg_database WHERE datname='$db'" 2>/dev/null | tr -d '\r')"
  [ "$exists" = 1 ] || return 0
  if [ $DRY -eq 1 ]; then echo "  would: drop database $db"; return 0; fi
  PGPASSWORD="${PGPASSWORD:-w-testing}" "$PSQL" "${q[@]}" \
    -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname='$db' AND pid<>pg_backend_pid();" >/dev/null 2>&1
  if PGPASSWORD="${PGPASSWORD:-w-testing}" "$PSQL" "${q[@]}" -c "DROP DATABASE \"$db\";" >/dev/null 2>&1; then
    echo "  dropped database $db"
  else
    echo "  could not drop database $db" >&2
  fi
}

# lane.sh writes "HH:MM:SS <worktree name> :: <command>" into each held slot, right after
# the mkdir that takes it. A slot caught between the two can't be attributed yet, so it
# counts as building here: lane.sh only checks for the retiring mark after `what` exists.
building() {
  local d w
  for d in "$LANE_ROOT"/lane/slot.*/; do
    [ -d "$d" ] || continue
    w="$(awk '{print $2; exit}' "$d/what" 2>/dev/null)"
    [ -z "$w" ] || [ "$w" = "$1" ] && return 0
  done
  return 1
}

# Prints MERGED, OPEN, CLOSED, ON_MAIN or UNMERGED for a branch.
merge_state() {
  local br="$1" st=""
  if command -v gh >/dev/null 2>&1; then
    st="$(gh pr list --state all --head "$br" --limit 1 --json state -q '.[0].state' 2>/dev/null)"
  fi
  if [ -n "$st" ]; then echo "$st"; return; fi
  if git merge-base --is-ancestor "$br" origin/main 2>/dev/null; then echo ON_MAIN; else echo UNMERGED; fi
}

MIN_IDLE="${RM_WORKTREE_MIN_IDLE:-30}"
# True when the worktree saw a commit (HEAD's commit time), a checkout (the HEAD file)
# or a build (the target dir) in the last $MIN_IDLE minutes. Not the index or the
# reflog: `git status` rewrites the index and `git gc` rewrites every worktree's reflog.
# Uncommitted edits are caught by the dirty check instead.
recently_used() {  # $1 = worktree path, $2 = name
  local gd now t
  gd="$(git -C "$1" rev-parse --absolute-git-dir 2>/dev/null)" || return 1
  now="$(date +%s)"
  for t in "$(git -C "$1" log -1 --format=%ct HEAD 2>/dev/null)" "$(stat -c %Y "$gd/HEAD" 2>/dev/null)"; do
    [ -n "$t" ] && [ $(( now - t )) -lt $(( MIN_IDLE * 60 )) ] && return 0
  done
  [ -n "$TROOT" ] && [ -d "$TROOT/$2" ] && [ -n "$(find "$TROOT/$2" -maxdepth 2 -mmin "-$MIN_IDLE" -print -quit 2>/dev/null)" ] && return 0
  return 1
}

# A lane job could take a slot between the building check and the delete. So the
# worktree is marked first ($LANE_ROOT/lane/retiring.<name>) and the slots checked after,
# while lane.sh takes its slot first and checks for the mark after: one of the two always
# sees the other, and lane.sh refuses to build into a marked worktree. A dry run changes
# nothing, so it takes no mark.
mark() {  # $1 = name; fails when another rm-worktree.sh is already retiring it
  [ $DRY -eq 1 ] && return 0
  mkdir -p "$LANE_ROOT/lane" && mkdir "$LANE_ROOT/lane/retiring.$1" 2>/dev/null
}
unmark() { [ $DRY -eq 1 ] || rmdir "$LANE_ROOT/lane/retiring.$1" 2>/dev/null; }

retired=0; skipped=0
retire() {  # $1 = worktree name, $2 = "sweep" when called by --merged
  if ! mark "$1"; then
    echo "skip $1: another rm-worktree.sh is retiring it"; skipped=$((skipped+1)); return
  fi
  retire_marked "$@"
  unmark "$1"
}
retire_marked() {
  local name="$1" wt="$WTROOT/$1" br state
  if ! git worktree list --porcelain | grep -qx "worktree $wt"; then
    echo "skip $name: not a registered worktree under .claude/worktrees"; skipped=$((skipped+1)); return
  fi
  if [ "${2:-}" = sweep ] && recently_used "$wt" "$name"; then
    echo "skip $name: used in the last $MIN_IDLE min (name it explicitly to retire it now)"; skipped=$((skipped+1)); return
  fi
  if building "$name"; then
    echo "skip $name: a lane job is building there now"; skipped=$((skipped+1)); return
  fi
  # Agent worktrees are locked while their agent runs; a stale lock needs --force.
  local locked
  locked="$(git worktree list --porcelain | awk -v w="worktree $wt" '$0==w{f=1;next} /^worktree /{f=0} f&&/^locked/{print "yes"}')"
  if [ -n "$locked" ] && [ $FORCE -eq 0 ]; then
    echo "skip $name: locked, an agent may still be using it (--force unlocks it)"; skipped=$((skipped+1)); return
  fi
  br="$(git -C "$wt" symbolic-ref --short -q HEAD || true)"
  if [ -n "$(git -C "$wt" status --porcelain 2>/dev/null)" ] && [ $FORCE -eq 0 ]; then
    echo "skip $name: uncommitted changes (commit them, or --force)"; skipped=$((skipped+1)); return
  fi
  if [ -n "$br" ]; then state="$(merge_state "$br")"; else state="DETACHED"; fi
  case "$state" in
    MERGED|ON_MAIN) ;;
    *) if [ $FORCE -eq 0 ]; then
         echo "skip $name: branch ${br:-(detached)} is $state (--force to retire anyway)"; skipped=$((skipped+1)); return
       fi ;;
  esac

  echo "retire $name (${br:-detached}, $state)"
  [ -n "$TROOT" ] && [ -d "$TROOT/$name" ] && run rm -rf "$TROOT/$name"
  if [ -e "$wt/external" ]; then
    run cmd //c rmdir "$(cygpath -w "$wt/external")"
  fi
  [ -d "$wt/target" ] && run rm -rf "$wt/target"
  if [ $DRY -eq 0 ] && [ -e "$wt/external" ]; then
    echo "  external/ is still there; stopping before git removes the worktree" >&2; skipped=$((skipped+1)); return
  fi
  [ -n "$locked" ] && run git worktree unlock "$wt"
  local rm_args=(remove); [ $FORCE -eq 1 ] && rm_args+=(--force)
  if ! run git worktree "${rm_args[@]}" "$wt"; then
    echo "  git worktree remove failed; keeping the branch and the test database" >&2; skipped=$((skipped+1)); return
  fi
  case "$state" in
    MERGED|ON_MAIN) [ -n "$br" ] && run git branch -D -q "$br" ;;
  esac
  drop_db "$name"
  retired=$((retired+1))
}

cd "$MAIN" || exit 1
# Without gh, merge state comes from origin/main; a stale one could call unmerged work merged.
if ! git fetch -q origin; then
  echo "git fetch origin failed; not deciding what has merged from a stale origin/main" >&2; exit 1
fi

if [ $MERGED -eq 1 ]; then
  while IFS= read -r wt; do
    case "$wt" in "$WTROOT"/*) retire "$(basename "$wt")" sweep ;; esac
  done < <(git worktree list --porcelain | sed -n 's/^worktree //p')
  # Target dirs no registered worktree owns any more (the worktree was removed by hand).
  if [ -n "$TROOT" ] && [ -d "$TROOT" ]; then
    owned=" $(git worktree list --porcelain | sed -n 's/^worktree //p' | xargs -r -n1 basename | tr '\n' ' ') "
    for d in "$TROOT"/*/; do
      [ -d "$d" ] || continue
      n="$(basename "$d")"
      case "$owned" in *" $n "*) continue ;; esac
      if ! mark "$n"; then echo "skip orphan target $n: another rm-worktree.sh is on it"; continue; fi
      if building "$n"; then echo "skip orphan target $n: building"; unmark "$n"; continue; fi
      echo "orphan target $n (no worktree)"; run rm -rf "$TROOT/$n"
      unmark "$n"
    done
  fi
else
  for n in "${NAMES[@]}"; do retire "$n"; done
fi

[ $DRY -eq 1 ] || git worktree prune
if [ $DRY -eq 1 ]; then echo "dry run: would retire $retired, skip $skipped"; else echo "retired $retired, skipped $skipped"; fi
