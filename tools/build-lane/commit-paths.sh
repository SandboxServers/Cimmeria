#!/usr/bin/env bash
# Commit exactly the named files in one worktree, for ops: the bash twin of commit-paths.ps1.
#
#   tools/build-lane/commit-paths.sh -C <worktree> -Message <text> -Paths <a>,<b> \
#       [-Trailers '<line>;;<line>'] [-ExpectBranch <name>]
#
# Never `git add .` or -A: the paths are staged one by one with --literal-pathspecs, so a glob cannot
# widen them. The guards, their reason= strings and the output are those of commit-paths.ps1, which
# is the spec (its header lists every refusal). Prints one line: status=ok sha=<sha> branch=<branch>,
# or status=refused|failed reason=<why>. Exit codes: 0 committed, 2 refused by a guard, 1 a git step
# failed. A bad option (missing, unknown, repeated or positional) prints no status= line and exits 1.
# The index is empty on entry; a failed check or commit unstages what the add staged.
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

# Prints the non-empty lines of $1 joined by ", " (for the reason strings).
join_list() {
  local out='' s
  while IFS= read -r s; do
    [ -n "$s" ] || continue
    out="${out:+$out, }$s"
  done <<EOF
$1
EOF
  printf '%s' "$out"
}

# Puts the named paths, and anything else the add staged, back out of the index (the working tree
# is untouched). The index was empty on entry, so nothing the caller staged is lost.
undo_staging() {
  local q
  for q in ${clean[@]+"${clean[@]}"}; do
    git -C "$OPT_C" --literal-pathspecs restore --staged -- "$q" >/dev/null 2>&1 || true
  done
  while IFS= read -r q; do
    [ -n "$q" ] || continue
    git -C "$OPT_C" --literal-pathspecs restore --staged -- "$q" >/dev/null 2>&1 || true
  done <<EOF
$(git -C "$OPT_C" -c core.quotepath=off diff --cached --name-only --no-renames 2>/dev/null || true)
EOF
}

# Parse loop (C11 template). Option names match the .ps1 parameters, in any letter case.
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
    message) take_opt Message "$val" ;;
    paths) take_opt Paths "$val" ;;
    trailers) take_opt Trailers "$val" ;;
    expectbranch) take_opt ExpectBranch "$val" ;;
    *) usage_error "unknown option: $name" ;;
  esac
done
# Mandatory: -C, -Message and -Paths. An empty -C or -Message is refused as a binding error, as
# [Parameter(Mandatory)] refuses ''; -Paths may be empty (it reaches the empty-path refusal).
[ -n "${OPT_C:-}" ] || usage_error "missing -C"
[ -n "${OPT_Message:-}" ] || usage_error "missing -Message"
[ -n "${SEEN_Paths:-}" ] || usage_error "missing -Paths"
OPT_Paths=${OPT_Paths:-}
OPT_Trailers=${OPT_Trailers:-}
OPT_ExpectBranch=${OPT_ExpectBranch:-}

[ -d "$OPT_C" ] || stop_refused "not-a-directory: $OPT_C"
inside=$(git -C "$OPT_C" rev-parse --is-inside-work-tree 2>/dev/null) || inside=''
[ "$inside" = true ] || stop_refused "not-a-git-worktree: $OPT_C"
prefix=$(git -C "$OPT_C" rev-parse --show-prefix 2>/dev/null) || prefix=''
[ -z "$prefix" ] || stop_refused "not-the-worktree-root: $OPT_C"

if [ "$(git -C "$OPT_C" rev-parse --absolute-git-dir)" = "$(git -C "$OPT_C" rev-parse --path-format=absolute --git-common-dir)" ]; then
  stop_refused 'main-checkout: refuse to commit in the main checkout'
fi

headref=$(git -C "$OPT_C" symbolic-ref -q HEAD 2>/dev/null) || stop_refused 'detached-head: a commit there is held by no branch'
branch=${headref#refs/heads/}
if [ -n "$OPT_ExpectBranch" ] && [ "$branch" != "$OPT_ExpectBranch" ]; then
  stop_refused "branch-mismatch: HEAD is $branch, not $OPT_ExpectBranch"
fi

# A merge, cherry-pick or revert in progress would be concluded by `git commit`, whatever the paths hold.
for pending in MERGE_HEAD CHERRY_PICK_HEAD REVERT_HEAD; do
  if git -C "$OPT_C" rev-parse -q --verify "$pending" >/dev/null 2>&1; then
    stop_refused "merge-in-progress: $pending exists; finish or abort it first"
  fi
done

[ -n "$(trim "$OPT_Message")" ] || stop_refused 'empty-message'

# -Paths is split on ','. The trailing ',' makes an empty last item (`a.txt,`) reach empty-path.
clean=()
paths_rest="$OPT_Paths,"
while case $paths_rest in *,*) true ;; *) false ;; esac; do
  raw=${paths_rest%%,*}; paths_rest=${paths_rest#*,}
  q=$(trim "$raw"); q=${q//\\//}
  [ -n "$q" ] || stop_refused 'empty-path'
  case $q in -*|:*) stop_refused "option-or-magic-pathspec: $q" ;; esac
  case $q in [A-Za-z]:*|/*) stop_refused "rooted-path: $q (give the path relative to -C)" ;; esac
  case $q in *[*?]*|*'['*|*']'*) stop_refused "glob: $q" ;; esac
  case "/$q/" in */./*|*/../*) stop_refused "not-an-exact-file: $q" ;; esac
  case $q in */) stop_refused "not-an-exact-file: $q" ;; esac
  wtn=${OPT_C//\\//}; wtn=${wtn%/}
  full="$wtn/$q"
  case $(lower "$full") in "$(lower "$wtn")"/*) ;; *) stop_refused "outside-worktree: $q" ;; esac
  [ ! -d "$full" ] || stop_refused "directory: $q"
  # A junction or symlink above the file would carry a file that lives outside -C into the commit.
  walk=$wtn; below=$q
  while :; do
    case $below in */*) ;; *) break ;; esac
    seg=${below%%/*}; below=${below#*/}
    walk="$walk/$seg"
    [ ! -L "$walk" ] || stop_refused "reparse-point: $q passes through the junction or symlink $seg"
  done
  clean+=("$q")
done

staged=$(git -C "$OPT_C" diff --cached --name-only --no-renames 2>/dev/null) || staged=''
if [ -n "$staged" ]; then
  stop_refused "other-staged-files: $(join_list "$staged") (the index must be empty so a failure can restore it)"
fi

if ! git -C "$OPT_C" --literal-pathspecs add -- ${clean[@]+"${clean[@]}"}; then
  undo_staging
  printf 'status=failed reason=git-add\n'
  exit 1
fi
staged_now=$(git -C "$OPT_C" -c core.quotepath=off diff --cached --name-only --no-renames 2>/dev/null) || staged_now=''
[ -n "$staged_now" ] || stop_refused 'nothing-to-commit'
# A literal pathspec still matches a directory prefix, so naming a deleted directory stages every
# deletion beneath it. Only the files that were named may be staged.
widened=''
while IFS= read -r f; do
  [ -n "$f" ] || continue
  found=''
  for q in ${clean[@]+"${clean[@]}"}; do
    if [ "$(lower "$q")" = "$(lower "$f")" ]; then found=1; fi
  done
  if [ -z "$found" ]; then widened="${widened:+$widened, }$f"; fi
done <<EOF
$staged_now
EOF
if [ -n "$widened" ]; then
  undo_staging
  stop_refused "widened-pathspec: the paths would also stage $widened; name each file (the paths were unstaged again)"
fi

# Plain `git commit` accepts trailing whitespace, so this is a refusal only where the repository
# declares a whitespace policy (core.whitespace or a whitespace attribute); elsewhere it warns.
if ! git -C "$OPT_C" diff --cached --check; then
  policy=$(git -C "$OPT_C" config --get core.whitespace 2>/dev/null) || policy=''
  attrs=$(git -C "$OPT_C" check-attr whitespace -- ${clean[@]+"${clean[@]}"} | grep -v ': whitespace: unspecified$' || true)
  if [ -n "$policy" ] || [ -n "$attrs" ]; then
    undo_staging
    stop_refused 'diff-check: whitespace errors (the paths were unstaged again)'
  fi
  printf '%s\n' 'WARNING: warning=diff-check: whitespace errors in the staged changes (the repository sets no whitespace policy, so the commit goes ahead)' >&2
fi

# Trailers: split on ';;' or newlines, strip CR, trim, drop empty ones. Then the message (trailing
# whitespace dropped), a blank line and the trailer lines, as the .ps1 builds it.
trailers_text=${OPT_Trailers//;;/$'\n'}
lines=()
while IFS= read -r line || [ -n "$line" ]; do
  line=${line%$'\r'}; line=$(trim "$line")
  if [ -n "$line" ]; then lines+=("$line"); fi
done <<EOF
$trailers_text
EOF
block=''
for l in ${lines[@]+"${lines[@]}"}; do block+="$l"$'\n'; done
msg=$OPT_Message
msg=${msg%"${msg##*[![:space:]]}"}
body=$msg
if [ -n "$block" ]; then body+=$'\n\n'"${block%$'\n'}"; fi
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
printf '%s\n' "$body" > "$tmp"
if ! git -C "$OPT_C" commit -q -F "$tmp"; then
  undo_staging
  printf 'status=failed reason=git-commit\n'
  exit 1
fi
sha=$(git -C "$OPT_C" rev-parse HEAD)
printf 'status=ok sha=%s branch=%s\n' "$sha" "$branch"
exit 0
