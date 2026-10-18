#!/usr/bin/env bash
# Create one local branch at a given full commit SHA, for `ops`: the bash twin of branch-at.ps1.
#
#   tools/build-lane/branch-at.sh -C <repo> -Name <branch> -Sha <40-hex>
#
# It runs `git branch <name> <sha>` and nothing else: no checkout, no fetch, no push, and never an
# existing branch overwritten. <repo> is the main checkout or any worktree of the repository.
# Refuses (exit 2) when <sha> is not 40 lowercase hex digits or names no commit, <name> is not a
# valid branch name, <name> is ambiguous (looks like a hex hash of 7 to 40 digits, names an existing
# tag, starts with `+`, `-` or `refs/`, is `@`, is HEAD, FETCH_HEAD, ORIG_HEAD, MERGE_HEAD or
# CHERRY_PICK_HEAD in any case, is all upper-case letters, digits and `_` (pseudo-ref shaped:
# REBASE_HEAD, AUTO_MERGE; the `*_HEAD` and AUTO_MERGE shapes in any case), or its first segment is
# a configured remote or `heads`, `tags` or `remotes`), or a branch exists whose name equals <name>
# case-insensitively or is a path prefix of it or the reverse, or <name> already resolves to any ref
# (`git rev-parse --verify <name>`). Prints status=ok branch=<name> sha=<sha>, or
# status=refused reason=<why> / status=failed reason=<why> detail="<git's last output line>" (quoted,
# with `%`, `"` and `=` percent-encoded). Exit codes: 0 created, 2 refused, 1 git failed.
# Options take full names only, in any letter case (D-AH40); see the AH-11 rules for the parse loop.
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

# Case-sensitive ERE match in the C locale: $1 is the text, $2 the regex.
matches() { local LC_ALL=C; [[ $1 =~ $2 ]]; }

while [ $# -gt 0 ]; do
  tok=$1; shift
  case $tok in
    -*:*) name=${tok%%:*}; val=${tok#*:} ;;
    -*)   name=$tok
          [ $# -gt 0 ] || usage_error "missing a value for $name"
          case $1 in -[A-Za-z]*) usage_error "missing a value for $name" ;; esac
          val=$1; shift ;;
    *)    usage_error "unexpected argument: $tok" ;;
  esac
  case $(lower "${name#-}") in
    c) take_opt C "$val" ;;
    name) take_opt Name "$val" ;;
    sha) take_opt Sha "$val" ;;
    *) usage_error "unknown option: $name" ;;
  esac
done
[ -n "${OPT_C:-}" ] || usage_error "missing or empty -C"
[ -n "${OPT_Name:-}" ] || usage_error "missing or empty -Name"
[ -n "${OPT_Sha:-}" ] || usage_error "missing or empty -Sha"

if [ ! -d "$OPT_C" ]; then stop_refused "not-a-directory: $OPT_C"; fi
if ! git -C "$OPT_C" rev-parse --git-dir >/dev/null 2>&1; then stop_refused "not-a-git-repository: $OPT_C"; fi

if ! matches "$OPT_Sha" '^[0-9a-f]{40}$'; then stop_refused "sha-not-full-40-hex: $OPT_Sha"; fi
if ! git -C "$OPT_C" rev-parse --verify --quiet "$OPT_Sha^{commit}" >/dev/null 2>&1; then stop_refused "no-such-commit: $OPT_Sha"; fi

case $OPT_Name in -*|+*) stop_refused "bad-branch-name: $OPT_Name" ;; esac
if matches "$OPT_Name" '^[0-9a-f]{7,40}$'; then stop_refused "ambiguous-branch-name: $OPT_Name looks like a commit hash"; fi
if git -C "$OPT_C" show-ref --verify --quiet "refs/tags/$OPT_Name"; then stop_refused "ambiguous-branch-name: $OPT_Name is an existing tag"; fi
if ! git -C "$OPT_C" check-ref-format --branch "$OPT_Name" >/dev/null 2>&1; then stop_refused "bad-branch-name: $OPT_Name"; fi
# Git's top-level pseudo-refs (REBASE_HEAD, AUTO_MERGE, REVERT_HEAD, BISECT_HEAD ...) are all-caps
# and win over refs/heads/ in a rev lookup. Ordinary lower-case names such as `feat` stay valid.
# On NTFS a loose ref `Rebase_Head` and the file `.git/REBASE_HEAD` are the same name, so the
# all-caps rule is applied in any case to names without a slash.
name_l=$(lower "$OPT_Name")
if matches "$name_l" '^refs/' || matches "$OPT_Name" '^[A-Z][A-Z0-9_]*$'; then stop_refused "ambiguous-branch-name: $OPT_Name"; fi
case $name_l in @|head|fetch_head|orig_head|merge_head|cherry_pick_head) stop_refused "ambiguous-branch-name: $OPT_Name" ;; esac
if matches "$name_l" '^[a-z0-9_]*_head$' || [ "$name_l" = auto_merge ]; then stop_refused "ambiguous-branch-name: $OPT_Name is shaped like a git pseudo-ref in another case"; fi

# A name with a slash, or a remote's name, must not start with a ref namespace or a remote name.
first_l=$(lower "${OPT_Name%%/*}")
is_remote=0; first_hit=0
case $first_l in heads|tags|remotes) first_hit=1 ;; esac
remotes=$(git -C "$OPT_C" remote 2>/dev/null) || remotes=''
while IFS= read -r r; do
  if [ -z "$r" ]; then continue; fi
  rl=$(lower "$r")
  if [ "$rl" = "$name_l" ]; then is_remote=1; fi
  if [ "$rl" = "$first_l" ]; then first_hit=1; fi
done <<EOF
$remotes
EOF
if [ "$is_remote" = 1 ] || [ "${OPT_Name#*/}" != "$OPT_Name" ]; then
  if [ "$first_hit" = 1 ]; then stop_refused "ambiguous-branch-name: $OPT_Name shadows a ref namespace or a remote"; fi
fi

# Compare against every existing branch name, case-insensitively: on a case-insensitive
# filesystem a loose `refs/heads/Topic` shadows a packed `refs/heads/topic`, so a case variant
# would silently re-point the existing branch. A D/F prefix clash (`a` against `a/b`) is refused
# the same way.
want=$(lower "refs/heads/$OPT_Name")
refs=$(git -C "$OPT_C" for-each-ref --format='%(refname)' refs/heads 2>/dev/null) || refs=''
while IFS= read -r ref; do
  if [ -z "$ref" ]; then continue; fi
  have=$(lower "$ref")
  if [ "$have" = "$want" ]; then stop_refused "branch-exists: $OPT_Name is not overwritten (existing branch $ref, compared case-insensitively)"; fi
  case $have in "$want"/*) stop_refused "branch-exists: $OPT_Name clashes with the existing branch $ref (one is a path prefix of the other)" ;; esac
  case $want in "$have"/*) stop_refused "branch-exists: $OPT_Name clashes with the existing branch $ref (one is a path prefix of the other)" ;; esac
done <<EOF
$refs
EOF
# git's rev lookup tries <name>, refs/<name>, refs/tags/<name> and only then refs/heads/<name>:
# any name that already resolves (refs/stash, refs/bisect/*, a remote's HEAD, a pseudo-ref, or on
# NTFS a case variant of a loose one) would shadow the new branch in every later lookup.
if git -C "$OPT_C" rev-parse -q --verify "$OPT_Name" >/dev/null 2>&1; then
  stop_refused "ambiguous-branch-name: $OPT_Name already resolves to an existing ref, which would shadow the branch"
fi

out=$(git -C "$OPT_C" branch "$OPT_Name" "$OPT_Sha" 2>&1) && rc=0 || rc=$?
if [ "$rc" -ne 0 ]; then
  printf 'status=failed reason=git-branch detail=%s\n' "$(detail_of "$out")"
  exit 1
fi
printf 'status=ok branch=%s sha=%s\n' "$OPT_Name" "$OPT_Sha"
exit 0
