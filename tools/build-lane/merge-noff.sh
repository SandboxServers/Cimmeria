#!/usr/bin/env bash
# Merge one local branch into the current branch of one worktree with --no-ff, for `ops` (bash twin of merge-noff.ps1).
#
# Usage: tools/build-lane/merge-noff.sh -C <worktree> -Branch <name> [-ExpectBranch <current>]
#
# A conflict or any failed merge step runs `git merge --abort` and exits 1, so the worktree is
# left as it was before the call. It aborts only the merge it started (none was in progress
# before, and MERGE_HEAD is the merged branch's tip); git's abort output is captured, so the
# status line is the only output. Hooks still run. Merge config that would change what is merged
# is overridden for the call: branch.<current>.mergeOptions is blanked (its -s, -X, --no-commit or
# --squash would reach the merge), merge.log, merge.branchdesc and merge.autoStash are off.
#
# Refuses (exit 2) when -C is not the root of a non-main worktree, a rebase, merge, cherry-pick or
# revert is in progress there (`operation-in-progress`: rebase-merge, rebase-apply, MERGE_HEAD,
# CHERRY_PICK_HEAD or REVERT_HEAD in its git dir), HEAD is detached, -Branch is not a valid name
# or no local branch of that name, -Branch is the current branch, the current branch is not
# -ExpectBranch (when given; compared case-sensitively), or the tree is dirty (any
# `git status --porcelain` output). A parameter-binding error (a missing or unknown option, or a
# second positional token) prints no status= line and exits 1.
# Prints status=ok merge=<sha> branch=<name>, status=up-to-date merge=<sha> branch=<name> when
# <name> is already contained in HEAD, or status=failed reason=<why> files="<conflicted paths>"
# detail="<git's last output line>" (reason=merge-aborted, or merge-not-aborted when a merge is
# left that this call did not start; both values are quoted with `%`, `"` and `=` percent-encoded,
# and a `,` inside a file name is %2C, so a file name plants no key=value token). Exit codes: 0
# merged or up to date, 2 refused by a guard, 1 the merge failed.
# Option names match the .ps1 parameters in any letter case; full names only, no abbreviations.
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

# Options: the C11 parse loop, with merge-noff.ps1's parameters.
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
    branch) take_opt Branch "$val" ;;
    expectbranch) take_opt ExpectBranch "$val" ;;
    *) usage_error "unknown option: $name" ;;
  esac
done
[ -n "${SEEN_C:-}" ] || usage_error "missing -C"
[ -n "$OPT_C" ] || usage_error "-C is empty"
[ -n "${SEEN_Branch:-}" ] || usage_error "missing -Branch"
[ -n "$OPT_Branch" ] || usage_error "-Branch is empty"
ExpectBranch=${OPT_ExpectBranch:-}   # an empty -ExpectBranch counts as not given

[ -d "$OPT_C" ] || stop_refused "not-a-directory: $OPT_C"
inside=$(git -C "$OPT_C" rev-parse --is-inside-work-tree 2>/dev/null) || inside=''
[ "$inside" = true ] || stop_refused "not-a-git-worktree: $OPT_C"
prefix=$(git -C "$OPT_C" rev-parse --show-prefix 2>/dev/null) || prefix=''
[ -z "$prefix" ] || stop_refused "not-the-worktree-root: $OPT_C"

abs_dir=$(git -C "$OPT_C" rev-parse --absolute-git-dir 2>/dev/null) || abs_dir=''
common_dir=$(git -C "$OPT_C" rev-parse --path-format=absolute --git-common-dir 2>/dev/null) || common_dir=''
if [ -n "$abs_dir" ] && [ "$abs_dir" = "$common_dir" ]; then
  stop_refused 'main-checkout: refuse to merge in the main checkout'
fi

# A rebase, merge, cherry-pick or revert already in progress is the caller's, not this script's:
# a merge whose index equals HEAD passes the dirty check, and `git merge` would then fail on it.
for state in rebase-merge rebase-apply MERGE_HEAD CHERRY_PICK_HEAD REVERT_HEAD; do
  [ ! -e "$abs_dir/$state" ] || stop_refused "operation-in-progress: $state exists; finish or abort it first"
done

# symbolic-ref, not `rev-parse --abbrev-ref`: the latter answers `heads/<b>` when a tag has the same name.
head_ref=$(git -C "$OPT_C" symbolic-ref -q HEAD 2>/dev/null) || stop_refused 'detached-head: a merge there is held by no branch'
current=${head_ref#refs/heads/}
if [ -n "$ExpectBranch" ] && [ "$current" != "$ExpectBranch" ]; then
  stop_refused "branch-mismatch: HEAD is $current, not $ExpectBranch"
fi

git -C "$OPT_C" check-ref-format --branch "$OPT_Branch" >/dev/null 2>&1 || stop_refused "bad-branch-name: $OPT_Branch"
git -C "$OPT_C" show-ref --verify --quiet "refs/heads/$OPT_Branch" || stop_refused "no-such-local-branch: $OPT_Branch"
# -eq in the .ps1 ignores case, so this compare does too.
[ "$(lower "$OPT_Branch")" != "$(lower "$current")" ] || stop_refused "same-branch: $OPT_Branch is the current branch"

dirty=$(git -C "$OPT_C" status --porcelain 2>/dev/null) || dirty=''
[ -z "$dirty" ] || stop_refused 'dirty-tree: commit or remove these first'

if git -C "$OPT_C" merge-base --is-ancestor "refs/heads/$OPT_Branch" HEAD >/dev/null 2>&1; then
  sha=$(git -C "$OPT_C" rev-parse HEAD)
  printf 'status=up-to-date merge=%s branch=%s\n' "$sha" "$OPT_Branch"
  exit 0
fi

# git parses branch.<current>.mergeOptions before argv, and `-s` / `-X` accumulate, so a
# `--strategy=ort` on the command line alone does not pin the strategy. That key is overridden
# for this call with a blank value, passed as GIT_CONFIG_KEY_n / GIT_CONFIG_VALUE_n. `--strategy=ort`
# covers pull.twohead; `--commit` and `--no-squash` restate the defaults. `-m` writes the subject
# `Merge branch '<b>' into <current>`.
src_sha=$(git -C "$OPT_C" rev-parse "refs/heads/$OPT_Branch")
out=$(GIT_CONFIG_COUNT=4 \
  GIT_CONFIG_KEY_0="branch.$current.mergeOptions" GIT_CONFIG_VALUE_0=' ' \
  GIT_CONFIG_KEY_1=merge.log GIT_CONFIG_VALUE_1=false \
  GIT_CONFIG_KEY_2=merge.branchdesc GIT_CONFIG_VALUE_2=false \
  GIT_CONFIG_KEY_3=merge.autoStash GIT_CONFIG_VALUE_3=false \
  git -C "$OPT_C" merge --no-ff --commit --no-squash --strategy=ort \
    -m "Merge branch '$OPT_Branch' into $current" "refs/heads/$OPT_Branch" 2>&1) && mrc=0 || mrc=$?

if [ "$mrc" -ne 0 ]; then
  files=''
  while IFS= read -r -d '' name; do
    [ -n "$name" ] || continue
    enc=$(encode_field "$name"); enc=${enc//,/%2C}
    files="${files:+$files,}$enc"
  done < <(git -C "$OPT_C" diff --name-only -z --diff-filter=U 2>/dev/null)
  # Abort only the merge this call started: none was in progress before (checked above), and
  # MERGE_HEAD is the tip of the branch being merged. Git's abort output is captured.
  reason=merge-aborted
  if mh=$(git -C "$OPT_C" rev-parse -q --verify MERGE_HEAD 2>/dev/null); then
    if [ "$mh" = "$src_sha" ]; then
      git -C "$OPT_C" merge --abort >/dev/null 2>&1 || reason=merge-not-aborted
    else
      reason=merge-not-aborted
    fi
  fi
  printf 'status=failed reason=%s files="%s" detail=%s\n' "$reason" "$files" "$(detail_of "$out")"
  exit 1
fi

sha=$(git -C "$OPT_C" rev-parse HEAD)
printf 'status=ok merge=%s branch=%s\n' "$sha" "$OPT_Branch"
exit 0
