#!/usr/bin/env bash
# Push one local branch to origin for `ops`, the bash twin of push-branch.ps1: same guards, output and exit codes.
#
# A plain push is `git push -u origin refs/heads/<b>:refs/heads/<b>`. With -Force it is the same with an explicit
# lease, `--force-with-lease=refs/heads/<b>:<sha>`, where <sha> is the remote tip the caller last saw
# (-ExpectRemoteSha). A bare --force is never run, and a lease without an expected value is never used: it trusts
# the remote-tracking ref, which any fetch refreshes, and would then overwrite commits pushed by someone else.
#
#   tools/build-lane/push-branch.sh -C <worktree> -Branch <name> [-Force -ExpectRemoteSha <40-hex>]
#
# Options take full names only (no abbreviations). Refuses (exit 2) with the same reasons as the .ps1. Prints
# status=ok branch=<name> sha=<sha>, or status=refused|failed reason=<why> (a failed push adds detail="<git's last
# output line>"). Exit codes: 0 pushed, 2 refused by a guard, 1 the push failed or a usage error (no status= line).
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

# Defaults: an empty -ExpectRemoteSha counts as not given, as the .ps1 default '' does.
OPT_ExpectRemoteSha=''

while [ $# -gt 0 ]; do
  tok=$1; shift
  case $tok in
    -*:*) name=${tok%%:*}; val=${tok#*:}
          [ "$(lower "$name")" != -force ] || usage_error "-Force takes no value" ;;
    -*)   name=$tok
          if [ "$(lower "$name")" = -force ]; then take_opt Force 1; continue; fi
          [ $# -gt 0 ] || usage_error "missing a value for $name"
          # taken verbatim, as the .ps1 binds it: a value such as -x is not a flag
          val=$1; shift ;;
    *)    usage_error "unexpected argument: $tok" ;;
  esac
  case $(lower "${name#-}") in
    c) take_opt C "$val" ;;
    branch) take_opt Branch "$val" ;;
    expectremotesha) take_opt ExpectRemoteSha "$val" ;;
    *) usage_error "unknown option: $name" ;;
  esac
done
[ -n "${SEEN_C:-}" ] || usage_error "missing -C"
[ -n "${SEEN_Branch:-}" ] || usage_error "missing -Branch"
[ -n "$OPT_C" ] || usage_error "-C is empty"
[ -n "$OPT_Branch" ] || usage_error "-Branch is empty"

# [A-Za-z0-9_] and the sha pattern are byte classes: compare them in the C locale.
starts_ok() { local LC_ALL=C; case $1 in [A-Za-z0-9_]*) return 0 ;; esac; return 1; }
is_sha40() { local LC_ALL=C; [[ $1 =~ ^[0-9a-f]{40}$ ]]; }

[ -d "$OPT_C" ] || stop_refused "not-a-directory: $OPT_C"
[ "$(git -C "$OPT_C" rev-parse --is-inside-work-tree 2>/dev/null || true)" = true ] || stop_refused "not-a-git-worktree: $OPT_C"
prefix=$(git -C "$OPT_C" rev-parse --show-prefix 2>/dev/null || true)
[ -z "$prefix" ] || stop_refused "not-the-worktree-root: $OPT_C"

if [ "$(git -C "$OPT_C" rev-parse --absolute-git-dir)" = "$(git -C "$OPT_C" rev-parse --path-format=absolute --git-common-dir)" ]; then
  stop_refused 'main-checkout: refuse to push from the main checkout'
fi

# symbolic-ref, not `rev-parse --abbrev-ref`: the latter answers `heads/<b>` when a tag has the same name.
headRef=$(git -C "$OPT_C" symbolic-ref -q HEAD 2>/dev/null) || stop_refused 'detached-head: a push of HEAD there names no branch'
current=${headRef#refs/heads/}

case $(lower "$OPT_Branch") in main|master) stop_refused "protected-branch: $OPT_Branch" ;; esac
starts_ok "$OPT_Branch" || stop_refused "bad-branch-name: $OPT_Branch (must start with a letter, digit or underscore: a leading + would force the push)"
git -C "$OPT_C" check-ref-format --branch "$OPT_Branch" >/dev/null 2>&1 || stop_refused "bad-branch-name: $OPT_Branch"
[ "$OPT_Branch" = "$current" ] || stop_refused "not-current-branch: $OPT_Branch is not the worktree's branch $current"

if [ -n "${SEEN_Force:-}" ] && ! is_sha40 "$OPT_ExpectRemoteSha"; then
  stop_refused 'force-needs-expect-remote-sha: -Force needs -ExpectRemoteSha <40 lowercase hex>, the remote tip the caller last saw'
fi
if [ -z "${SEEN_Force:-}" ] && [ -n "$OPT_ExpectRemoteSha" ]; then
  stop_refused 'expect-remote-sha-needs-force: -ExpectRemoteSha only goes with -Force'
fi

if [ -n "${SEEN_Force:-}" ]; then
  out=$(git -C "$OPT_C" push -u "--force-with-lease=refs/heads/$OPT_Branch:$OPT_ExpectRemoteSha" origin "refs/heads/$OPT_Branch:refs/heads/$OPT_Branch" 2>&1) && rc=0 || rc=$?
else
  out=$(git -C "$OPT_C" push -u origin "refs/heads/$OPT_Branch:refs/heads/$OPT_Branch" 2>&1) && rc=0 || rc=$?
fi
if [ "$rc" -ne 0 ]; then
  printf 'status=failed reason=git-push branch=%s detail=%s\n' "$OPT_Branch" "$(detail_of "$out")"
  exit 1
fi

sha=$(git -C "$OPT_C" rev-parse HEAD)
printf 'status=ok branch=%s sha=%s\n' "$OPT_Branch" "$sha"
exit 0
