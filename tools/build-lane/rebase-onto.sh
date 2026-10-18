#!/usr/bin/env bash
# Rebase the current branch of one worktree onto a commit, for `ops`: the bash twin of rebase-onto.ps1.
#
# It runs `git rebase <onto>`, or `git rebase --onto <onto> <upstream>` when -Upstream is given (the
# commit the branch was cut from, so a stacked branch does not replay its parent's commits). It never
# fetches. A conflict, or any failed rebase step, runs `git rebase --abort` and exits 1, so the
# worktree is left as it was before the call. It aborts only the rebase it started (no rebase was in
# progress before, and the rebase's recorded branch and starting commit are this branch and its tip);
# git's abort output is discarded, so the status line is the only output. Hooks still run.
#
#   tools/build-lane/rebase-onto.sh -C <worktree> -Onto <40-hex> [-Upstream <40-hex>]
#
# Options take full names only (no abbreviations), in any letter case; `-Name value` and
# `-Name:value` both work. Merge commits on the branch are kept (`--rebase-merges`), so a `--no-ff`
# topology survives.
#
# Refuses (exit 2) when -C is not the root of a non-main worktree, a rebase, merge, cherry-pick or
# revert is in progress there (`operation-in-progress`: rebase-merge, rebase-apply, MERGE_HEAD,
# CHERRY_PICK_HEAD or REVERT_HEAD in its git dir, even with HEAD back on a branch and a clean tree),
# HEAD is detached, the tree is dirty (any `git status --porcelain` output), -Onto or -Upstream is not
# 40 lowercase hex digits or names no commit, -Upstream is not an ancestor of HEAD or is HEAD itself
# (nothing to replay: that would be a reset), or -Onto is already in HEAD while -Upstream is not in
# -Onto (the rebase would drop the commits between them). Rebase config such as rebase.updateRefs is
# overridden (--no-update-refs), so no other branch moves.
# Prints status=ok tip=<sha> branch=<name> after a rebase, status=up-to-date tip=<sha> branch=<name>
# when HEAD already contains -Onto and nothing moves, or status=failed reason=<why> branch=<name>
# detail="<git's last output line>" (reason=rebase-aborted, or rebase-not-aborted when a rebase is left
# that this call cannot prove it started; detail is quoted, with `%`, `"` and `=` percent-encoded, so it
# plants no key=value token).
# Exit codes: 0 rebased or up to date, 2 refused by a guard, 1 the rebase failed.
set -euo pipefail
# --- git-verb common (C11) ---
for v in GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_NAMESPACE GIT_OBJECT_DIRECTORY \
         GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_CONFIG_COUNT GIT_CONFIG_PARAMETERS GIT_SSH_COMMAND GIT_EXEC_PATH; do
  unset "$v"
done
stop_refused() { printf 'status=refused reason=%s\n' "$1"; exit 2; }
usage_error() { printf '%s: %s\n' "${0##*/}" "$1" >&2; exit 1; }
lower() { printf '%s' "$1" | LC_ALL=C tr '[:upper:]' '[:lower:]'; }
trim() { local s=$1; s=${s#"${s%%[![:space:]]*}"}; s=${s%"${s##*[![:space:]]}"}; printf '%s' "$s"; }
encode_field() { local s=$1 out='' c i; local LC_ALL=C
  for (( i = 0; i < ${#s}; i++ )); do c=${s:i:1}
    case $c in %|\"|=|[[:cntrl:]]) out+=$(printf '%%%02X' "'$c") ;; *) out+=$c ;; esac
  done; printf '%s' "$out"; }
detail_of() {  # $1 = git's combined output; prints "<encoded last error line>"
  local all=$1 line t l pick='' last=''
  while IFS= read -r line || [ -n "$line" ]; do t=$(trim "$line"); [ -n "$t" ] || continue; last=$t
    l=$(lower "$t"); case $l in *'failed to push some refs'*) ;; fatal:*|error:*|conflict*|'!'[[:space:]]*) pick=$t ;; esac
  done <<EOF
$all
EOF
  [ -n "$pick" ] || pick=$last
  pick=$(printf '%s' "$pick" | LC_ALL=C sed -E -e 's#://[^/@[:space:]]+@#://#g' -e 's/[[:cntrl:]]+/ /g')
  pick=$(trim "$pick"); pick=${pick:0:200}; [ -n "$pick" ] || pick=none
  printf '"%s"' "$(encode_field "$pick")"; }
take_opt() { local seen="SEEN_$1"; [ -z "${!seen:-}" ] || usage_error "-$1 given twice"
  printf -v "OPT_$1" '%s' "$2"; printf -v "SEEN_$1" '%s' 1; }
# --- end common ---

while [ $# -gt 0 ]; do
  tok=$1; shift
  case $tok in
    -*:*) name=${tok%%:*}; val=${tok#*:} ;;
    -*)   name=$tok
          [ $# -gt 0 ] || usage_error "missing a value for $name"
          # taken verbatim, as the .ps1 binds it: a value such as -x is not a flag
          val=$1; shift ;;
    *)    usage_error "unexpected argument: $tok" ;;
  esac
  case $(lower "${name#-}") in
    c) take_opt C "$val" ;;
    onto) take_opt Onto "$val" ;;
    upstream) take_opt Upstream "$val" ;;
    *) usage_error "unknown option: $name" ;;
  esac
done
# -C and -Onto are mandatory and refuse an empty value; an empty -Upstream counts as not given.
[ -n "${SEEN_C:-}" ] || usage_error "missing -C"
[ -n "${OPT_C:-}" ] || usage_error "-C is empty"
[ -n "${SEEN_Onto:-}" ] || usage_error "missing -Onto"
[ -n "${OPT_Onto:-}" ] || usage_error "-Onto is empty"
wt=$OPT_C
onto=$OPT_Onto
upstream=${OPT_Upstream:-}

[ -d "$wt" ] || stop_refused "not-a-directory: $wt"
inside=$(git -C "$wt" rev-parse --is-inside-work-tree 2>/dev/null) || inside=''
[ "$inside" = true ] || stop_refused "not-a-git-worktree: $wt"
prefix=$(git -C "$wt" rev-parse --show-prefix 2>/dev/null) || prefix=''
[ -z "$prefix" ] || stop_refused "not-the-worktree-root: $wt"

# The main checkout's git dir is its common dir; a linked worktree's git dir is under .git/worktrees.
gitdir=$(git -C "$wt" rev-parse --absolute-git-dir)
commondir=$(git -C "$wt" rev-parse --path-format=absolute --git-common-dir)
[ "$gitdir" != "$commondir" ] || stop_refused 'main-checkout: refuse to rebase the main checkout'

# A rebase, merge, cherry-pick or revert already in progress is the caller's, not this script's.
# A stale rebase-merge directory with HEAD back on the branch (an interrupted rebase, then
# `checkout -f`) passes the detached and dirty checks; `git rebase` would then fail on it and an
# abort would reset the branch to that old rebase's starting commit.
for state in rebase-merge rebase-apply MERGE_HEAD CHERRY_PICK_HEAD REVERT_HEAD; do
  if [ -e "$gitdir/$state" ]; then stop_refused "operation-in-progress: $state exists; finish or abort it first"; fi
done

# symbolic-ref, not `rev-parse --abbrev-ref`: the latter answers `heads/<b>` when a tag has the same name.
headRef=$(git -C "$wt" symbolic-ref -q HEAD 2>/dev/null) || stop_refused 'detached-head: a rebase there is held by no branch'
branch=${headRef#refs/heads/}

dirty=$(git -C "$wt" status --porcelain)
[ -z "$dirty" ] || stop_refused 'dirty-tree: commit or remove these first'

[[ $onto =~ ^[0-9a-f]{40}$ ]] || stop_refused "sha-not-full-40-hex: $onto"
if [ -n "$upstream" ] && ! [[ $upstream =~ ^[0-9a-f]{40}$ ]]; then stop_refused "sha-not-full-40-hex: $upstream"; fi
ontoSha=$(git -C "$wt" rev-parse --verify --quiet "$onto^{commit}" 2>/dev/null) || ontoSha=''
[ -n "$ontoSha" ] || stop_refused "not-a-commit: $onto"

if [ -n "$upstream" ]; then
  upSha=$(git -C "$wt" rev-parse --verify --quiet "$upstream^{commit}" 2>/dev/null) || upSha=''
  [ -n "$upSha" ] || stop_refused "not-a-commit: $upstream"
  if ! git -C "$wt" merge-base --is-ancestor "$upSha" HEAD; then
    stop_refused "upstream-not-ancestor: $upstream is not in $branch"
  fi
  # HEAD is its own ancestor; --onto <x> HEAD replays nothing and would move the branch to <x>.
  replay=$(git -C "$wt" rev-list --count "$upSha..HEAD")
  headsha=$(git -C "$wt" rev-parse HEAD)
  if [ "$upSha" = "$headsha" ] || [ "$replay" = 0 ]; then
    stop_refused "nothing-to-replay: $upstream is the tip of $branch"
  fi
  # An -Onto already inside HEAD only makes sense when -Upstream sits at or below it; otherwise
  # --onto <old commit> <newer commit> silently drops everything between the two.
  if git -C "$wt" merge-base --is-ancestor "$ontoSha" HEAD; then
    if ! git -C "$wt" merge-base --is-ancestor "$upSha" "$ontoSha"; then
      stop_refused "would-drop-commits: $upstream is not in $onto, so the commits between them would be lost"
    fi
  fi
  rebase_args=(rebase --no-update-refs --rebase-merges --onto "$ontoSha" "$upSha")
else
  rebase_args=(rebase --no-update-refs --rebase-merges "$ontoSha")
fi

if git -C "$wt" merge-base --is-ancestor "$ontoSha" HEAD && [ -z "$upstream" ]; then
  tip=$(git -C "$wt" rev-parse HEAD)
  printf 'status=up-to-date tip=%s branch=%s\n' "$tip" "$branch"
  exit 0
fi

before=$(git -C "$wt" rev-parse HEAD)
rc=0
out=$(git -C "$wt" "${rebase_args[@]}" 2>&1) || rc=$?
if [ "$rc" -ne 0 ]; then
  # Abort only a rebase this call started: none was in progress before (checked above), and the
  # one left records this branch and its pre-rebase tip. Git's abort output is discarded.
  reason=rebase-aborted
  for state in rebase-merge rebase-apply; do
    dir=$gitdir/$state
    [ -e "$dir" ] || continue
    headName=$(head -n 1 "$dir/head-name" 2>/dev/null) || headName=''
    origHead=$(head -n 1 "$dir/orig-head" 2>/dev/null) || origHead=''
    headName=$(trim "$headName"); origHead=$(trim "$origHead")
    if [ "$headName" = "$headRef" ] && [ "$origHead" = "$before" ]; then
      git -C "$wt" rebase --abort >/dev/null 2>&1 || reason=rebase-not-aborted
    else
      reason=rebase-not-aborted
    fi
  done
  printf 'status=failed reason=%s branch=%s detail=%s\n' "$reason" "$branch" "$(detail_of "$out")"
  exit 1
fi

tip=$(git -C "$wt" rev-parse HEAD)
printf 'status=ok tip=%s branch=%s\n' "$tip" "$branch"
exit 0
