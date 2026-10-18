# ai-harness-docs: work packets, wave 3d (after #1824)

Continued from [work-packets-w3b.md](work-packets-w3b.md) (the AH-11 rules, C7b, C10) and [work-packets-w3.md](work-packets-w3.md) (W3c, W4). Decisions D-AH35 to D-AH41 and findings F28 to F36 are in the [README](README.md#decisions).

PR #1824 (`feat(ops): guarded git verbs for the ops agent`) merged on 2026-10-18 at 18:29Z as 9664e56f4, outside this campaign. It adds five PowerShell scripts that `ops.md` now names (`commit-paths.ps1`, `merge-noff.ps1`, `branch-at.ps1`, `push-branch.ps1`, `rebase-onto.ps1`), each with a `test-<name>.ps1`, and it rewrites three paragraphs of `ops.md`. It ships no `.sh` twins. This file holds:

- **AH-11a-r**: AH-11a's PR #1829 now conflicts with main in `ops.md` and `ops.toml`, and this packet resolves it;
- **W3d** (AH-11j to AH-11o): the `.sh` twins of the five verbs, so C10 holds for every script `ops.md` names;
- **AH-11p** (wave 3c): a parity test of `.sh` against `.ps1`, the CI wiring and the README rows.

## Merge order

| # | Step | Needs |
|---|---|---|
| 1 | AH-11e (#1825), the Linux CI job | W3b review-fixer done |
| 2 | AH-11b (#1826), AH-11c (#1827), AH-11d (#1828) | 1 |
| 3 | AH-11a-r, then AH-11a (#1829) | the W3b review-fixer has returned (it may commit in `ah-11a`) |
| 4 | AH-11f | 3 |
| - | #1824 | merged already, 9664e56f4 |
| 5 | W3d: AH-11j, AH-11l, AH-11m, AH-11n, AH-11o in parallel; AH-11k stacks on AH-11j's branch | 1 (the Linux job runs their tests); can be dispatched while 2 to 4 ship |
| 6 | W3c: AH-11g, AH-11h, AH-11p in parallel | every W3b PR and every W3d PR merged |
| 7 | AH-11i (owner), then W4: AH-15 (owner), AH-16 | 6 |

**The `ops.md` conflict rule** (D-AH39), whichever of AH-11a and #1824 lands second: keep #1824's text everywhere, append Aura's sentence `If the PowerShell tool is unavailable (a Linux or WSL host), return `blocked` and state that reason in the one-clause reason; never a bare `blocked`.` to the end of #1824's `Never run `git merge`...` paragraph, right after `Anything not on the allowed list: return `blocked`.`, and regenerate `.codex/agents/ops.toml` with `python tools/agent-skills/sync.py`. AH-11g later replaces that sentence with its "Pick the shell" paragraph.

---

## AH-11a-r: merge main into AH-11a (#1829)

Agent: packet-coder. Worktree `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ah-11a` (branch `ah-11a`, PR #1829, head 8d202b67f unless the W3b review-fixer committed after it). Size: about 12k working tokens. Dispatch only after the W3b review-fixer has returned. Commit subject: `Merge origin/main into ah-11a: ops.md on top of #1824 (AH-11a-r)`.

**Why a merge, not a rebase:** a merge commit needs no force-push, and it keeps Aura's commit 8613c4fb2 untouched with her as author. The merge queue squashes the PR anyway.

**Rules:** the AH-11 rules (w3b), plus:

- Edit only `.claude/agents/ops.md` and `.codex/agents/ops.toml`. If any other file conflicts, run `git merge --abort` and return `blocked` with the reason `unexpected conflict: <files>`.
- `.claude/agents/ops.md` keeps the line endings main's copy has. Check that `[IO.File]::ReadAllText('<abs ops.md>').Contains("`r")` gives the same answer as `git show origin/main:.claude/agents/ops.md` piped to a file in your scratch directory.

**Steps:**

1. `Set-Location <wt>; git status --porcelain` must be empty. Record `git rev-parse HEAD`.
2. `git fetch origin main`, then `git merge --no-ff origin/main`. It stops with `CONFLICT` in exactly `.claude/agents/ops.md` and `.codex/agents/ops.toml`.
3. Run `git checkout --theirs -- .claude/agents/ops.md .codex/agents/ops.toml`. In a merge, "theirs" is origin/main, so both files are now #1824's.
4. In `.claude/agents/ops.md`, use the Edit tool to find the one occurrence of ``Anything not on the allowed list: return `blocked`.`` (it ends the paragraph that starts ``Never run `git merge` ``). Replace it with the same text followed by one space and Aura's sentence, copied exactly from the conflict rule above.
5. Run `python tools/agent-skills/sync.py`, then check that `python tools/agent-skills/sync.py --check` exits 0.
6. `Select-String -Path <abs ops.md>,<abs ops.toml> -Pattern '^(<<<<<<<|=======|>>>>>>>)'` gives no hits.
7. `git add -- .claude/agents/ops.md .codex/agents/ops.toml`, then `git commit -m "<subject>" -m "<attribution lines>"`. This concludes the merge.

**Test type:** none; this is a conflict resolution. The checks are the proof.

**Checks:**

- `git diff origin/main -- .claude/agents/ops.md` changes exactly one line, the `Never run `git merge`` paragraph, and the only addition is Aura's sentence.
- `git log -1 --format=%P` lists two parents.
- `git log --format=%an origin/main..HEAD` still lists `aurablacklight`.
- `python -m unittest discover -s tools/agent-skills -p "test_*.py"` passes.

**Then (orchestrator, through `ops`):** run `pwsh -NoProfile -File tools/build-lane/push-branch.ps1 -C <ah-11a wt> -Branch ah-11a`. This is a plain push and a fast-forward. Then ship #1829 as step 3 of the merge order.

**Docs owed:** none new; AH-11a's rows stand.

---

## Wave 3d: the `.sh` twins of the ops git verbs

Six packets for one packet-review-fixer. AH-11j, AH-11l, AH-11m, AH-11n and AH-11o are parallel and touch disjoint files. AH-11k stacks on AH-11j's branch (`--base`): dispatch it once AH-11j has committed, in a worktree made from that branch. Each worktree is made from origin/main after 9664e56f4, so it contains the `.ps1` scripts and their tests.

### Rules for every W3d packet

These are the AH-11 rules (w3b, rules 1 to 9) plus:

1. **The `.ps1` is the spec.** Read `tools/build-lane/<name>.ps1` and `tools/build-lane/test-<name>.ps1` in your worktree before you write anything. Port:
   - every guard, in the same order;
   - the same `reason=` strings, word for word;
   - the same `status=` lines, the same exit codes (0 ok, 2 refused, 1 failed) and the same output fields (`sha=`, `branch=`, `files=`, `detail=`).

   Where this packet and the `.ps1` disagree, the `.ps1` wins; say so in the report. If the port exposes a bug in the `.ps1`, write it in the report as a finding and do not fix it.
2. **PowerShell comparisons ignore case unless they start with `c`.** `-match`, `-eq`, `-ne`, `-in`, `-contains`, `-notcontains` and `-like` are case-insensitive. `-cmatch`, `-ceq`, `-cne` and `-cnotmatch` are case-sensitive. Mirror each one:
   - for a case-insensitive compare, compare `$(lower "$a")` with `$(lower "$b")`;
   - bash `[[ $x =~ re ]]` and `case` are case-sensitive.

   Translate the regexes: `\d` → `[0-9]`, `\s` → `[[:space:]]`, `(?:` → `(`. Bash ERE has no lookaround, so split such a test into two tests.
3. **bash 3.2** (macOS) must parse and run the script. These are not allowed:
   - `${x,,}`, `mapfile` or `readarray`, `declare -A`, `[[ -v ]]` and `${x@Q}`;
   - expanding an empty array as `"${a[@]}"` under `set -u` (write `${a[@]+"${a[@]}"}`).

   `printf -v`, `${!name}`, `+=`, `(( ))` and `[[ ]]` are fine.
4. **`set -euo pipefail` must not end the script before it prints its status line.** Run every git call that can fail as one of:
   - `if git ...; then`;
   - `out=$(git ... 2>&1) && rc=0 || rc=$?`.
5. **Options** use the C11 parse loop below:
   - option names match the `.ps1` parameters, in any letter case;
   - `-Name value` and `-Name:value` both work;
   - every positional token, every unknown option and every repeated option is a usage error (exit 1, a message on stderr, no `status=` line), which is how PowerShell fails a binding.

   PowerShell's abbreviated names (`-Exp`) and `-Switch:$true` are **not** mirrored (D-AH40). A value token that starts with `-` and a letter is a usage error, as in PowerShell; write it as `-Name:-value`.
6. **Git Bash path conversion.** When Git Bash runs `git.exe`, it rewrites an argument that starts with `/`. So:
   - never pass a caller's value to git before the guard that refuses rooted paths;
   - do not set `MSYS_NO_PATHCONV`, because a `mktemp` path passed to `git -F` needs the conversion.
7. **Files.** Create exactly the files your packet names: the LF `.sh` and its Python test.
   - Do not edit the `.ps1`, its `test-*.ps1`, either README or `ops.md`. AH-11p and AH-11g own those.
   - The `.sh` starts `#!/usr/bin/env bash`. Its first comment line is a one-sentence summary that the generated tools catalog reads, as in `mk-worktree.sh`. Then `set -euo pipefail`.
   - After `git add`, run `git update-index --chmod=+x tools/build-lane/<name>.sh` so the file is executable on Linux.
8. **Tests:** see "Test shape" below. Port **every** `Check` in the `test-<name>.ps1` sections your packet owns, one assertion each, with the Check's text as the assertion `msg`. In the report, give the counts: Checks in the `.ps1` sections owned, assertions written.
9. **Context.** If you are near your budget with the tests unfinished, commit what passes and return `blocked context`. The orchestrator then sends a continuation (the RESUME loop rule).

### C11. The git-verb common block (verbatim in every twin)

Paste this block into each `.sh`, byte for byte, between the two marker lines, right after `set -euo pipefail`. AH-11p tests that the five copies are identical (D-AH37).

```bash
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
```

What each helper mirrors in the `.ps1`:

- the env loop and `stop_refused` mirror the `foreach ($v ...)` loop and `Stop-Refused`;
- `encode_field` mirrors `ConvertTo-Field`: `%`, `"`, `=`, bytes below 0x20 and 0x7f become `%XX`;
- `detail_of` mirrors `Get-Detail`:
  - lines are trimmed and empty ones dropped;
  - the last line starting with `fatal:`, `error:`, `CONFLICT` or `! ` wins (any case), except `failed to push some refs`; otherwise the last line;
  - `://user@` is masked, control runs become one space;
  - the result is cut to 200 characters, `none` if empty, then quoted.

If the `.ps1`'s `Get-Detail` or `ConvertTo-Field` differs from this list, follow the `.ps1`, change the block in your script, and say so in the report. The review-fixer then makes the other copies match.

**Parse loop** (per script; the option keys are that script's parameters):

```bash
while [ $# -gt 0 ]; do
  tok=$1; shift
  case $tok in
    -*:*) name=${tok%%:*}; val=${tok#*:}
          [ "$(lower "$name")" != -force ] || usage_error "-Force takes no value" ;;
    -*)   name=$tok
          if [ "$(lower "$name")" = -force ]; then take_opt Force 1; continue; fi   # push-branch only
          [ $# -gt 0 ] || usage_error "missing a value for $name"
          case $1 in -[A-Za-z]*) usage_error "missing a value for $name" ;; esac
          val=$1; shift ;;
    *)    usage_error "unexpected argument: $tok" ;;
  esac
  case $(lower "${name#-}") in
    c) take_opt C "$val" ;;
    expectbranch) take_opt ExpectBranch "$val" ;;
    # ... one line per parameter of the .ps1 ...
    *) usage_error "unknown option: $name" ;;
  esac
done
[ -n "${SEEN_C:-}" ] || usage_error "missing -C"   # one line per Mandatory parameter
```

**Empty values:**

- An empty value for a mandatory string is a usage error, as `[Parameter(Mandatory)]` refuses `''`. The exception is a parameter the `.ps1` marks `[AllowEmptyString()]`: commit-paths' `-Paths ''` goes on to the `empty-path` refusal.
- A whitespace-only `-Message` goes on to `empty-message`.
- An empty `-ExpectBranch` or `-ExpectRemoteSha` counts as not given, as the `.ps1` default `''` does.

**Platform-safe forms of the `.ps1` checks** (same reason strings):

| `.ps1` check | `.sh` form |
|---|---|
| `not-a-directory` | `[ -d "$OPT_C" ]` |
| `not-a-git-worktree` | `git -C "$OPT_C" rev-parse --is-inside-work-tree` does not print `true` |
| `not-the-worktree-root` | `git -C "$OPT_C" rev-parse --show-prefix` prints anything |
| `main-checkout` | `rev-parse --absolute-git-dir` equals `rev-parse --path-format=absolute --git-common-dir` (needs git 2.31) |
| `reparse-point` (commit-paths) | `[ -L "$OPT_C/<segment>" ]` for each leading segment of the path. If Git Bash's `-L` misses a junction (the junction test fails), compare `cd "<dir>" && pwd -P` with the logical path instead, and say so in the report |
| `outside-worktree` (commit-paths) | the same string check the `.ps1` makes, so the reasons match |

### Test shape (every W3d test file)

- **Type:** integration test of the script against a throwaway git repository (TESTING.md), with no network: the "remote" is a local bare repository.
- **File:** `tools/build-lane/test_<name>_sh.py` (underscores), stdlib only, with `from __future__ import annotations` and type hints, and under 500 lines.

```python
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from test_lane_stats import find_bash  # noqa: E402  Git Bash on Windows, PATH bash elsewhere
SCRIPT = HERE / "<name>.sh"
BASH = find_bash()

@unittest.skipUnless(BASH, "no bash: Git for Windows missing or LANE_TESTS_NO_BASH=1")
class <Name>ShTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="<name>-sh-"))
        empty = self.tmp / "empty.gitconfig"; empty.write_text("", encoding="utf-8")
        self.env = dict(os.environ, GIT_CONFIG_GLOBAL=str(empty), GIT_CONFIG_SYSTEM=str(empty))
        # build the same repo + worktree (+ bare origin) the test-<name>.ps1 builds
    def tearDown(self) -> None:
        shutil.rmtree(self.tmp, ignore_errors=True)
    def run_sh(self, *argv: str, env: dict[str, str] | None = None) -> tuple[int, str]:
        r = subprocess.run([BASH, SCRIPT.as_posix(), *argv], env=env or self.env,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                           text=True, encoding="utf-8", errors="replace")
        return r.returncode, r.stdout
```

- Give one test method to each `# ---` section of the `.ps1` test. Each method starts from a fresh fixture and replays the setup lines that section needs, so the methods do not depend on each other.
- Pass paths to the script as `Path.as_posix()`.
- Write files with `newline=""` and `\n` line endings. Keep `core.whitespace` and `core.autocrlf` exactly as the `.ps1` test sets them.
- **Hooks:** write the hook with LF endings, then `os.chmod(hook, 0o755)`.
- **Junction or symlink fixtures:** on `os.name == "nt"`, create with `subprocess.run(["cmd", "/c", "mklink", "/J", link, target])` and remove with `os.rmdir(link)` before `rmtree`. Elsewhere use `os.symlink` and `os.unlink`.
- A case that cannot run on one platform calls `self.skipTest("<reason>")`; it never passes silently.

**Why every W3d test fails when its change is reverted:**

- Delete the `.sh`, and every test method fails, because the script is not found and the exit code is 127.
- Remove any one guard, and the method of the section that tests it fails on the `status=refused reason=<guard>` assertion.

Each packet names the guards whose removal its report must prove. For each one, the coder comments out the guard, runs the test file, records the failing test names, and restores the guard. The script is unchanged in the commit.

### Checks for every W3d packet (in order, each with `Set-Location <wt>;`)

1. `& "C:\Program Files\Git\bin\bash.exe" -n tools/build-lane/<name>.sh` exits 0.
2. `python -m unittest discover -s tools/build-lane -p "test_<name>_sh*.py" -v`. All pass, with 0 skips on this Windows host.
3. The whole build-lane suite: `python -m unittest discover -s tools/build-lane -p "test_*.py"`. Report the skip count.
4. `[IO.File]::ReadAllText('<abs .sh>').Contains("`r")` is `False`. `git ls-files -s tools/build-lane/<name>.sh` shows mode `100755`.
5. WSL, best effort, as in the AH-11 test approach (w3b). Otherwise write `WSL: not run`.

**Docs owed (every W3d packet):** none in the packet. The tools catalog is generated from the `.sh` header, and AH-11p updates the hand-written README rows.

---

### AH-11j: `commit-paths.sh`, the guards and the happy path

Agent: packet-coder. Branch `ai-harness-docs/ah-11j-commit-paths-sh`, worktree `ah-11j`. Size: about 40k working tokens. Wave 3d. Commit subject: `feat(build-lane): commit-paths.sh, the bash twin of commit-paths.ps1 (AH-11j)`.

**Files:** create `tools/build-lane/commit-paths.sh` and `tools/build-lane/test_commit_paths_sh.py`.

**Steps:**

1. Read `commit-paths.ps1` (159 lines) and lines 1-131 of `test-commit-paths.ps1`.
2. Write `commit-paths.sh`. Its parameters are `-C`, `-Message`, `-Paths`, `-Trailers` and `-ExpectBranch`; `-C`, `-Message` and `-Paths` are mandatory.
3. `-Paths` is one string split on `,`. `-Trailers` is split on `;;` or a newline; strip `\r` from each trailer, trim it and drop empty ones.
4. Message: drop trailing whitespace with `${m%"${m##*[![:space:]]}"}`. Build the body as the `.ps1` does: the message, a blank line, then the trailers. Write it to `tmp=$(mktemp)`, remove it in `trap 'rm -f "$tmp"' EXIT`, and commit with `git -C "$OPT_C" commit -q -F "$tmp"`.
5. Port `Undo-Staging` and every failure path that calls it, so a failed check or commit leaves the index empty.
6. The no-policy whitespace warning goes to stderr as `WARNING: ...`, with the same text the `.ps1` writes through `Write-Warning`.
7. Write `test_commit_paths_sh.py` with one method per section in lines 59-131 of the `.ps1` test:
   - refusals leave HEAD where it was (including the rooted path, the space-separated list and an already-staged path);
   - the happy path stages and commits only the named files;
   - several paths and several trailers survive.

**Prove these guards (rule: why it fails when reverted):** `rooted-path`, `other-staged-files`, `empty-message`.

**Checks and docs:** as for every W3d packet.

### AH-11k: `commit-paths.sh`, the hard cases

Agent: packet-coder. Branch `ai-harness-docs/ah-11k-commit-paths-sh-more`, worktree `ah-11k`, made from AH-11j's branch and stacked on it (`--base ai-harness-docs/ah-11j-commit-paths-sh`). Size: about 35k working tokens. Wave 3d, after AH-11j has committed. Commit subject: `test(build-lane): commit-paths.sh hard cases, ported from test-commit-paths.ps1 (AH-11k)`.

**Files:** create `tools/build-lane/test_commit_paths_sh_more.py`. Edit `tools/build-lane/commit-paths.sh` only to fix a case these tests show is wrong, and list each such fix in the report.

**Steps:**

1. Read `commit-paths.ps1`, AH-11j's `commit-paths.sh` and `test_commit_paths_sh.py` (for the fixture), and lines 132 to the end of `test-commit-paths.ps1`.
2. Write `test_commit_paths_sh_more.py` with the same fixture and run helper (copy them; do not import from the other test file), and one method per section:
   - a whitespace error stops the commit and leaves nothing staged;
   - a failing hook leaves nothing staged;
   - a rename staged by someone else is not swept in;
   - a deleted file can be committed by name;
   - a deleted directory must not stage every deletion beneath it;
   - a junction (a symlink off Windows) inside the worktree must not carry a file from outside it;
   - `GIT_DIR` and `GIT_WORK_TREE` cannot retarget the commit;
   - the branch compare is case-sensitive;
   - whitespace is a refusal only where the repo sets a policy (`core.whitespace`, then a whitespace attribute);
   - `GIT_OBJECT_DIRECTORY` cannot redirect the objects;
   - a merge in progress is refused;
   - a conflicted cherry-pick or revert resolved to HEAD is still refused.

**Prove these guards:** `reparse-point`, `widened-pathspec`, `merge-in-progress`, and the env-clearing loop (remove `GIT_DIR` from it).

**Checks and docs:** as for every W3d packet, plus AH-11j's test file must still pass.

### AH-11l: `merge-noff.sh`

Agent: packet-coder. Branch `ai-harness-docs/ah-11l-merge-noff-sh`, worktree `ah-11l`. Size: about 40k working tokens. Wave 3d. Commit subject: `feat(build-lane): merge-noff.sh, the bash twin of merge-noff.ps1 (AH-11l)`.

**Files:** create `tools/build-lane/merge-noff.sh` and `tools/build-lane/test_merge_noff_sh.py`.

**Steps:**

1. Read `merge-noff.ps1` (153 lines) and `test-merge-noff.ps1` (253 lines).
2. Write `merge-noff.sh`. Its parameters are `-C`, `-Branch` (mandatory) and `-ExpectBranch`. Port, in order:
   - the `operation-in-progress` check (rebase-merge, rebase-apply, MERGE_HEAD, CHERRY_PICK_HEAD, REVERT_HEAD; read the `.ps1` for the exact set and how it finds the git dir);
   - the `branch-mismatch`, `bad-branch-name`, `no-such-local-branch`, `same-branch` and `dirty-tree` guards;
   - `up-to-date`;
   - the merge itself;
   - the abort that undoes only its own merge.
3. The merge overrides the repository's merge config only for this one call. Use the values the `.ps1` passes:

   ```bash
   GIT_CONFIG_COUNT=4 \
   GIT_CONFIG_KEY_0="branch.$current.mergeOptions" GIT_CONFIG_VALUE_0=' ' \
   GIT_CONFIG_KEY_1=merge.log GIT_CONFIG_VALUE_1=false \
   GIT_CONFIG_KEY_2=merge.branchdesc GIT_CONFIG_VALUE_2=false \
   GIT_CONFIG_KEY_3=merge.autoStash GIT_CONFIG_VALUE_3=false \
     git -C "$OPT_C" merge --no-ff --commit --no-squash --strategy=ort \
       -m "Merge branch '$OPT_Branch' into $current" "refs/heads/$OPT_Branch"
   ```

   If the `.ps1` uses other keys or values, use the `.ps1`'s.
4. On a conflict, print `files="<a>,<b>"` with each name passed through `encode_field`, plus a comma (as `%2C`) inside a name, exactly as the `.ps1` does.
5. Write `test_merge_noff_sh.py`, with one method per section of `test-merge-noff.ps1`.

**Prove these guards:** the `mergeOptions` override (a `branch.<b>.mergeOptions=-s ours` fixture still merges the content), `no-such-local-branch` for a tag of the same name, `operation-in-progress`, and the `files=` encoding of a conflicted name with `=` in it.

**Checks and docs:** as for every W3d packet.

### AH-11m: `branch-at.sh`

Agent: packet-coder. Branch `ai-harness-docs/ah-11m-branch-at-sh`, worktree `ah-11m`. Size: about 30k working tokens. Wave 3d. Commit subject: `feat(build-lane): branch-at.sh, the bash twin of branch-at.ps1 (AH-11m)`.

**Files:** create `tools/build-lane/branch-at.sh` and `tools/build-lane/test_branch_at_sh.py`.

**Steps:**

1. Read `branch-at.ps1` (106 lines) and `test-branch-at.ps1` (154 lines).
2. Write `branch-at.sh`. Its parameters are `-C`, `-Name` and `-Sha`, all mandatory. `-C` may be any repository or worktree: it needs only `git -C "$OPT_C" rev-parse --git-dir` for `not-a-git-repository`.
3. Port every name check with the `.ps1`'s letter case. Its reserved-name list and its `refs/` test are case-insensitive (`-contains`, `-match`), and the existing-branch compare refuses `Topic` against `topic`.
4. Write `test_branch_at_sh.py`, with one method per section of `test-branch-at.ps1`.

**Prove these guards:** the pseudo-ref name (`rebase_head` in lower case), `branch-exists` for a case-only difference, and the "already resolves" check (`stash`).

**Checks and docs:** as for every W3d packet.

### AH-11n: `push-branch.sh`

Agent: packet-coder. Branch `ai-harness-docs/ah-11n-push-branch-sh`, worktree `ah-11n`. Size: about 30k working tokens. Wave 3d. Commit subject: `feat(build-lane): push-branch.sh, the bash twin of push-branch.ps1 (AH-11n)`.

**Files:** create `tools/build-lane/push-branch.sh` and `tools/build-lane/test_push_branch_sh.py`.

**Steps:**

1. Read `push-branch.ps1` (101 lines) and `test-push-branch.ps1` (185 lines).
2. Write `push-branch.sh`. Its parameters are `-C`, `-Branch` (mandatory), the switch `-Force` (see the parse loop) and `-ExpectRemoteSha`.
3. Port, in the `.ps1`'s order:
   - `protected-branch` (main and master, compared as the `.ps1` compares them);
   - the two `bad-branch-name` checks (the leading-character rule);
   - `not-current-branch` (case-sensitive);
   - `force-needs-expect-remote-sha`, which requires 40 **lowercase** hex;
   - `expect-remote-sha-needs-force`.
4. The push is exactly `git -C "$OPT_C" push -u origin "refs/heads/$b:refs/heads/$b"`, and with `-Force` it adds `--force-with-lease="refs/heads/$b:$sha"`. No `--force` ever.
5. The tests push to a local bare repository set as `origin`, never the network.
6. Write `test_push_branch_sh.py`, with one method per section of `test-push-branch.ps1`.

**Prove these guards:** `bad-branch-name` for `+victim`, `force-needs-expect-remote-sha`, and the lease: a remote that moved after the given sha is `status=failed`.

**Checks and docs:** as for every W3d packet.

### AH-11o: `rebase-onto.sh`

Agent: packet-coder. Branch `ai-harness-docs/ah-11o-rebase-onto-sh`, worktree `ah-11o`. Size: about 40k working tokens. Wave 3d. Commit subject: `feat(build-lane): rebase-onto.sh, the bash twin of rebase-onto.ps1 (AH-11o)`.

**Files:** create `tools/build-lane/rebase-onto.sh` and `tools/build-lane/test_rebase_onto_sh.py`.

**Steps:**

1. Read `rebase-onto.ps1` (155 lines) and `test-rebase-onto.ps1` (240 lines).
2. Write `rebase-onto.sh`. Its parameters are `-C`, `-Onto` (mandatory) and `-Upstream`.
3. Port, in the `.ps1`'s order:
   - `operation-in-progress`, including a stale `rebase-merge` directory with HEAD back on the branch;
   - `detached-head` and `dirty-tree`;
   - both `sha-not-full-...` checks and both `not-a-commit` checks;
   - `upstream-not-ancestor`, `nothing-to-replay` and `would-drop-commits`.
4. The rebase runs the `.ps1`'s exact arguments (its lines 119-121). With `-Upstream`, that is `git rebase --no-update-refs --rebase-merges --onto <onto> <upstream>`; without it, `git rebase --no-update-refs --rebase-merges <onto>`. `--no-update-refs` needs git 2.38 or newer. Port the `status=up-to-date tip=<sha> branch=<b>` exit, which applies when `<onto>` is already an ancestor and no `-Upstream` was given.
5. Port the abort of its own conflicted rebase, and `reason=rebase-not-aborted` if that abort fails.
6. Write `test_rebase_onto_sh.py`, with one method per section of `test-rebase-onto.ps1`.

**Prove these guards:** `would-drop-commits`, `nothing-to-replay`, the stale `rebase-merge` refusal, and `rebase.updateRefs=true` ignored (a second branch at the old tip does not move).

**Checks and docs:** as for every W3d packet.

---

## AH-11p: parity, CI and the README rows (wave 3c)

Agent: packet-coder. Branch `ai-harness-docs/ah-11p-git-verb-parity`, worktree `ah-11p`. Size: about 35k working tokens. Wave 3c, parallel with AH-11g and AH-11h, after every W3b and W3d PR has merged (it needs AH-11e's `shell` job and all five twins). Commit subject: `test(build-lane): .sh and .ps1 git verbs agree; CI runs the twins on Windows (AH-11p)`.

**Rules:** the AH-11 rules and W3d rules 6 to 8. Edit only:

- the new `tools/build-lane/test_git_verb_parity.py`;
- `.github/workflows/build-lane-scripts.yml`;
- `tools/build-lane/README.md`;
- `tools/README.md` (hand-written rows only, outside every `<!-- gen: -->` block);
- the five `tools/build-lane/<name>.sh`, only to fix a shellcheck warning or a parity mismatch. Fix the `.sh`, never the `.ps1`: it is the spec.

**Steps:**

1. Create `test_git_verb_parity.py` with two test classes.
   - **`CommonBlockTests`** (runs everywhere): for the five `.sh` files, the text between `# --- git-verb common (C11) ---` and `# --- end common ---` is non-empty and identical in all five.
   - **`ParityTests`**, under `@unittest.skipUnless(os.name == "nt" and shutil.which("pwsh") and find_bash(), "parity needs Windows, pwsh and Git Bash")`. It holds a table of cases. Each case has a verb, a fixture function and the argv. For each case, build two fresh identical fixtures and run, one on each:
     - `pwsh -NoProfile -NonInteractive -File tools/build-lane/<verb>.ps1 <argv>`;
     - `<bash> tools/build-lane/<verb>.sh <argv>`.
   - Assert, with `subTest(verb=..., case=...)`:
     - equal exit codes;
     - equal first `status=` token, and equal `reason=` tokens up to the first `:` or space;
     - where either side prints no `status=` line, both exit 1 and neither prints one.
   - At least these cases:
     - **commit-paths:** ok; `-Paths /x` (rooted); `-Paths .`; `-Message ' '`; `-Paths a b` (usage); no `-Message` (usage); `-ExpectBranch` in the wrong case; `-Bogus x` (usage); `-Message -x` (usage).
     - **merge-noff:** ok; up-to-date; `no-such-local-branch`; `same-branch`; `dirty-tree`.
     - **branch-at:** ok; a short sha; `HEAD`; `Topic` against an existing `topic`; `refs/heads/x`.
     - **push-branch:** ok against a local bare origin; `main`; `-Force` alone; `-ExpectRemoteSha` without `-Force`; `+x`; a branch that is not current.
     - **rebase-onto:** ok; a short sha; `-Upstream` equal to HEAD; `dirty-tree`.
   - A mismatch is a finding. Fix the `.sh` to match the `.ps1`, and list each fix in the report.
2. `.github/workflows/build-lane-scripts.yml`:
   - In AH-11e's `shell` job, add `tools/build-lane/commit-paths.sh tools/build-lane/merge-noff.sh tools/build-lane/branch-at.sh tools/build-lane/push-branch.sh tools/build-lane/rebase-onto.sh` to the `shellcheck -S warning` line.
   - In the Windows `scripts` job, after the pwsh tests step, add:

   ```yaml
         - name: git-verb .sh twins under Git Bash, and parity with the .ps1
           shell: pwsh
           working-directory: tools/build-lane
           run: python -m unittest test_commit_paths_sh test_commit_paths_sh_more test_merge_noff_sh test_branch_at_sh test_push_branch_sh test_rebase_onto_sh test_git_verb_parity -v
   ```

   Match the file's indentation, and check that the job's `paths:` filters cover `tools/build-lane/**`.
3. `tools/build-lane/README.md`:
   - Change the five `| \`<name>.ps1\` |` rows to `| \`<name>.sh\` / \`.ps1\` |`, and add to each one: `the .sh takes full option names only (no abbreviations)`.
   - Replace `and CI does not run them, so run them after any change to those scripts:` with `CI runs them in the Windows build-lane job, with the .sh twins' Python tests under Git Bash and the parity test (test_git_verb_parity.py); the Linux job runs the .sh tests and shellcheck. Run them after any change to those scripts:`. Remove the backslashes (quoting rule 8).
4. `tools/README.md`: after the `build-lane/*.ps1` row (the hand-written rows near line 302), add:

   ```text
   | `build-lane/commit-paths.sh`, `merge-noff.sh`, `branch-at.sh`, `push-branch.sh`, `rebase-onto.sh` | Bash twins of the `ops` git verbs (`.ps1` above), same options, output and exit codes; `ops` runs them on a Linux or WSL host. `test_git_verb_parity.py` checks the two agree. |
   ```

5. Run shellcheck where it already exists. If `shellcheck` is not on PATH, do not install it: the PR's Linux job is the check, and the review-fixer fixes what it reports.

**Test type:** integration (a parity test across two implementations) plus a unit test of file content. **Reverted:**

- break one guard in one `.sh` (change a `reason=` string), and `ParityTests` fails for that case;
- change one byte inside one C11 block, and `CommonBlockTests` fails.

**Checks:**

- `python -m unittest discover -s tools/build-lane -p "test_*.py"`, 0 failures, with the skip count;
- `python -m unittest test_git_verb_parity -v` from `tools/build-lane`, with 0 skips on this host;
- `python tools/docs-gen/regen.py --check --skip-crate-graph`. The `gen:tools-catalog` block may be stale only by the five new `.sh` entries; do not edit it by hand, the regen workflow refreshes it;
- the CRLF check on both READMEs. `tools/README.md` and `tools/build-lane/README.md` are outside `docs/`, so keep each file's existing line endings, whatever they are.

**Docs owed:** the doc-update-map row for a build-lane script (the tools README rows); list it.
