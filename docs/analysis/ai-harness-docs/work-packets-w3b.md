# ai-harness-docs: work packets, wave 3b (the #1797 takeover)

Continued from [work-packets.md](work-packets.md) (dispatch rules, quoting rule 8, contract C1-C9). Wave 3c (AH-11g to AH-11i) and wave 4 are in [work-packets-w3.md](work-packets-w3.md).

## Wave 3b and 3c: AH-11, the #1797 takeover and a Linux path for `ops`

The owner took over Aura's PR #1797 on 2026-10-18 rather than wait for another round, and asked for the shell scripts that let `ops` work first-class on Linux and WSL. This replaces the old AH-11 (paths A and B). Decisions: D-AH16 (revised), D-AH26 to D-AH34 in the [README](README.md#decisions). Evidence: F17 to F27.

**Waves.** W3b = AH-11a to AH-11f, all parallel, disjoint files (AH-11d and AH-11e both touch `ship.py`, in different hunks). **Merge AH-11e first**, so the PRs of AH-11b, AH-11c and AH-11d run its Linux job. AH-11a merges before AH-11f (AH-11f links the section AH-11a creates). W3c = AH-11g and AH-11h, after every W3b PR has merged. AH-11i, the closing reply on #1797, is an owner action after W3c merges.

### Rules for every AH-11 packet

Each packet below says "the AH-11 rules"; these are they.

1. Work only in your worktree, with absolute paths. Start every PowerShell call with `Set-Location <worktree>;`, and run every check in the foreground. Commit as soon as the checks pass.
2. PowerShell only: this workstation is Windows. Run a `.sh` file only through Git Bash, by its full path: `& "C:\Program Files\Git\bin\bash.exe" ...`. Never use the bare `bash` on PATH, which is the WSL launcher. Install nothing: no shellcheck, apt, pip or winget.
3. No `cargo` (no Rust here). Never run `git stash`, `git push`, `git reset`, `git worktree prune`, `gh pr` or `gh api`.
4. **Line endings.** Every `.sh` is LF, and `.gitattributes` forces it. Keep `#!/usr/bin/env bash` and `set -euo pipefail`. Every `.md` is CRLF in the working tree. Checks: `[IO.File]::ReadAllText('<abs .md>') -match "(?<!\r)\n"` must be `False`; `[IO.File]::ReadAllText('<abs .sh>').Contains("`r")` must be `False`. If the Edit tool leaves LF lines in a `.md`, convert the file with `[IO.File]` and absolute paths.
5. **File caps:** 500 lines soft, 700 hard. `tools/build-lane/test_ship.py` is at 698: add no lines to it. `tools/build-lane/ship.py` is at 557: its net growth must be zero or negative.
6. **Python:** stdlib only, with `from __future__ import annotations` and type hints like the neighbouring files. Build-lane tests: `Set-Location <worktree>; python -m unittest discover -s tools/build-lane -p "test_*.py"`. All must pass; record the skip count in the report.
7. Never hand-edit a `<!-- gen: -->` block. After editing `.claude/agents/*.md` or `.claude/skills/**`, run `python tools/agent-skills/sync.py`, commit what it regenerates, and check that `python tools/agent-skills/sync.py --check` exits 0.
8. No IPs, credentials or account names. Aura's public GitHub handle and noreply address are allowed where a packet gives them.
9. Commit once, with the subject given (AH-11a makes two commits; its steps say why). End the message with the attribution lines from your brief.

**How the `.sh` files are tested without a Linux box (D-AH31):**

- **Locally:** a Python `unittest` per script runs the `.sh` through Git Bash. It covers the Windows branch, and skips a Linux-only case with a reason. `bash -n` through Git Bash checks the syntax.
- **CI, authoritative:** AH-11e's Linux job runs `bash -n` on every script, shellcheck, and the same Python tests on `ubuntu-latest`, where the Linux branch runs. The merge queue also runs the build-lane Python tests on Linux in `build-and-test`.
- **WSL, best effort:** a packet may run its test file in WSL only if `wsl.exe -l -q` already lists `Ubuntu`. Install nothing, and write `WSL: not run` otherwise:
  ```powershell
  Set-Location <worktree>; $p = wsl.exe -d Ubuntu -- wslpath -a ((Get-Location).Path + '\tools\build-lane'); wsl.exe -d Ubuntu -- python3 -m unittest discover -s $p -p "<test file>" -v
  ```

### C7b. Platform wording, final long form

AH-11h puts this in place of the C7 long form. It is the reconciled decision 4 (D-AH16): one validated platform, and one agent, `ops`, with a CI-tested bash path.

```text
**Status: Windows with PowerShell 7 is fully supported. WSL, Linux, and Git Bash on Windows are not validated, but not unsupported.** The build lane, the ship scripts and every agent rule are validated on Windows with PowerShell 7. Every build-lane `.ps1` has a `.sh` twin, and CI runs shellcheck and the twins' tests on Linux, so the `ops` agent uses the `.sh` twins on a Linux or WSL host. "Not validated" means that no full session (orchestrator, packet workers, review, ship) has been run end to end on Linux or WSL, and that the packet workers still need PowerShell: on a host without it they return `blocked: no PowerShell (pwsh) on this host`. Expect the gaps below, and report new ones.
```

### C10. `ops` shell by platform (AH-11g)

| Platform line in the agent's environment | Tool | Commands |
|---|---|---|
| `win32` | PowerShell | `pwsh tools/build-lane/<script>.ps1 ...` (as today); never Bash |
| anything else (`linux`, `darwin`) | Bash | `bash tools/build-lane/<script>.sh ...`, same arguments |
| the needed tool is missing | none | `blocked`, with the reason `no PowerShell tool on win32` or `no Bash tool on <platform>` |

The `.sh` and `.ps1` twins take the same options and print the same output with the same exit codes (tools/build-lane/README.md). On Linux, `ops` never runs a `.ps1` under Linux `pwsh`: `mk-worktree.ps1` makes a junction, and `ship.ps1` calls `python` (F25).

---

### AH-11a: take over #1797 (cherry-pick, then trim)

Agent: packet-coder. Branch `ai-harness-docs/ah-11a-takeover`, worktree name `ah-11a`. Size: about 25k working tokens. Wave 3b.

**Rules:** the AH-11 rules, plus:

- **CLAUDE.md stays at or under 4,500 bytes**: `(Get-Item <wt>\CLAUDE.md).Length`. It is 4,194 before the cherry-pick.
- Leave `.claude/agents/ops.md` and `.codex/agents/ops.toml` exactly as the cherry-pick leaves them; AH-11g rewrites that line.
- Two commits: the cherry-pick (Aura's, unchanged) and your trim commit.

**Steps:**

1. Make sure Aura's commit is present: `Set-Location <wt>; git cat-file -t f6f180ef82e8dbf0de9c2dd3adab816896020210` must print `commit`. If it doesn't, run `Set-Location <wt>; git fetch -q origin docs/wsl-host-reality` and try again. If it is still missing, return `blocked`.
2. `Set-Location <wt>; git cherry-pick f6f180ef82e8dbf0de9c2dd3adab816896020210`. It applied cleanly on origin/main 81c3c829c. On a conflict, run `git cherry-pick --abort` and return `blocked` with the conflicted file names. Then `git log -1 --format=%an` must print `aurablacklight`, so she stays the commit's author.
3. Trim, with the Edit tool. Each step: find the exact text, then replace only that.

   **3.1** `docs/agents/development-workflow.md`, § WSL and Linux hosts. Find the whole paragraph that starts `**Status: not supported for interactive sessions. Use the Windows checkout.**` and ends `tracked as an open question in the PR that added this section.` Replace it with:
   ```text
   **Status: Windows with PowerShell 7 is fully supported. WSL, Linux, and Git Bash on Windows are not validated, but not unsupported.** The build lane, the ship scripts and every agent rule are validated on Windows with PowerShell 7. The build-lane scripts have `.sh` twins and CI builds on Linux, but nobody runs development sessions there routinely, so expect the gaps below and report new ones. Agents use PowerShell; an agent on a host without it returns `blocked: no PowerShell (pwsh) on this host`. The documented build path is still Windows with PowerShell 7 ([building.md](../building.md#prerequisites)); this section records what you hit elsewhere, so you don't rediscover it.
   ```
   **3.2** Same section. Find the bullet that starts `- **No documented path.**` (the whole line). Replace it with:
   ```text
   - **Not validated.** [building.md](../building.md#prerequisites) and the `lane-build` skill name Windows with PowerShell 7 as the validated platform. Every `tools/build-lane/*.ps1` has a `.sh` twin ([README](../../tools/build-lane/README.md)), but nobody runs them routinely on a developer host.
   ```
   **3.3** Find `(now with the reason stated)`. Replace it with `and says why`.

   **3.4** Find `Link it by hand with `ln -s <main-checkout>/external <worktree>/external` (Windows worktrees use a junction instead), and remove the symlink before `git worktree remove`.` Replace it with:
   ```text
   Link it by hand with a symlink, as [Create a worktree that builds](#create-a-worktree-that-builds) says, and retire the worktree with `rm-worktree.sh`, which removes the link before `git worktree remove`.
   ```
   **3.5** Find the whole sub-bullet that starts `  - `rm-worktree.sh` is Windows-shaped.` Replace it with:
   ```text
     - `rm-worktree.sh` does run on Linux and WSL: it falls back from `cygpath` and `cmd`, and removes a symlinked `external/` with `rm -f`. `ship.sh merge --retire` never reaches it while `ship.sh` stops at `python`, so run `bash tools/build-lane/rm-worktree.sh <name>` yourself.
   ```
   **3.6** Find `waits or times out ([details](../../.claude/agent-memory/main-session/reference_loopback_exporter_test_seams_2026_10_04.md)).` Replace it with:
   ```text
   waits or times out. Point such a test at `[::1]:9` instead, which is refused at once (other `127/8` addresses are too, but macOS has no `127.0.0.2` by default).
   ```
   **3.7** Find `That is an observation, not a supported configuration.` Replace it with `That is an observation, not a validated configuration.`

   **3.8** Find `Interactively, `claude --agent hands-on` is the documented escape hatch.` Replace it with:
   ```text
   Interactively, `claude --agent hands-on` is the documented escape hatch. No setting limits the default persona to interactive sessions, so it stays on for every launch, and the session banner (`.claude/hooks/mode_banner.sh`) names the persona at the start of each one.
   ```
   **3.9** Same file, § Builds, worktrees and test databases. Find `Development builds run natively on Windows, from PowerShell or Git Bash.` Replace it with:
   ```text
   Development builds are validated on Windows with PowerShell 7; Git Bash, WSL and Linux are not validated, but not unsupported ([WSL and Linux hosts](#wsl-and-linux-hosts)).
   ```
   **3.10** `CLAUDE.md`. Find `Windows-native, on the toolchain`. Replace it with `Windows with PowerShell 7 is the validated platform, on the toolchain`.

   **3.11** `CLAUDE.md`. Find `WSL/Linux hosts are not supported for interactive sessions; what breaks, and how headless launches differ:`. Replace it with `WSL, Linux and Git Bash are not validated, but not unsupported; what breaks, and how headless launches differ:`.

   **3.12** `docs/guides/autonomous-agent-kickoff.md`. Find `and WSL is not a supported interactive host.` Replace it with `and WSL is not a validated host.` Then find `Which of the two wins on Linux is an open maintainer question.` and replace it with `Until this guide is revised, CLAUDE.md's lane rule wins.`
4. Run the checks, then commit the trim with the subject `docs(agents): WSL and Linux hosts to the platform decision (AH-11a)`. Above the attribution lines, add this trailer: `Co-authored-by: aurablacklight <89038013+aurablacklight@users.noreply.github.com>`.
5. Write the PR body to `<wt>\.claude\reports\ai-harness-docs\AH-11a-pr-body.md`, with exactly this text:
   ```text
   Takes over #1797 by @aurablacklight, at the owner's request.

   - Commit 1 is her commit f6f180ef, cherry-picked unchanged onto main, so she stays its author.
   - Commit 2 trims it to the platform decision: Windows with PowerShell 7 is fully supported; WSL, Linux, and Git Bash on Windows are not validated, but not unsupported. It drops the open-question sentence, states the loopback fact inline instead of linking into agent memory, points the by-hand `external/` link at the existing worktree section, and corrects the `rm-worktree.sh` bullet: it runs on Linux, and `ship.sh` stopped before reaching it.

   The Linux fixes her list names (mk-worktree.sh, ship.sh, rebase-pr.sh, an old `gh`) land in the AH-11b to AH-11e PRs of the ai-harness-docs campaign. Supersedes #1797. It is closed with a reply once the `ops` Linux path lands.

   Doc-update-map: the CLAUDE.md row.

   Co-authored-by: aurablacklight <89038013+aurablacklight@users.noreply.github.com>
   ```

**Test type:** none (docs only). The behaviour this documents is tested by AH-11b to AH-11e.

**Checks:**

- `Select-String` the section and the kickoff note for `not supported`, `open question` and `agent-memory`: no hits.
- The CLAUDE.md size check.
- The CRLF check on the three `.md` files.
- `python tools/agent-skills/sync.py --check` exits 0. The cherry-pick changed `ops.md` and `ops.toml` together.
- `git log --format="%h %an %s" origin/main..HEAD` lists two commits, and the first author is `aurablacklight`.

**Ship (for the orchestrator):** ops runs `ship.ps1 pr -C <wt> -m "docs(agents): WSL and Linux hosts to the platform decision (AH-11a)" --title "docs(agents): WSL and Linux hosts (takes over #1797 by @aurablacklight) (AH-11a)" --body-file <wt>\.claude\reports\ai-harness-docs\AH-11a-pr-body.md`. The PR is docs-only, so it merges without waiting for CI.

**Docs owed:** the doc-update-map row for CLAUDE.md; list it.

### AH-11b: `mk-worktree.sh` links `external/` on Linux

Agent: packet-coder. Branch `ai-harness-docs/ah-11b-mk-worktree`, worktree name `ah-11b`. Size: about 30k working tokens. Wave 3b. Commit subject: `fix(build-lane): mk-worktree.sh symlinks external/ off Windows instead of dying at cygpath (AH-11b)`.

**Rules:** the AH-11 rules. Edit only `tools/build-lane/mk-worktree.sh`, and create `tools/build-lane/test_mk_worktree_sh.py`.

**The bug (F17):** on Linux and WSL, `cygpath` does not exist. Under `set -e` the script exits 127 right after `git worktree add`, so the worktree exists but has no `external/`, and it cannot build.

**Steps:**

1. In `mk-worktree.sh`, change the header bullet `#  * Junctions the gitignored external/ tree in (...` to `#  * Links the gitignored external/ tree in: a junction under Git Bash on Windows, a symlink elsewhere (...`, keeping the rest of that bullet.
2. Replace the four lines from `WIN_WT="$(cygpath -w "$WT")"` through `echo "created $WT on branch $BRANCH (external/ junctioned)"`, and the `if [ -n "${CIMMERIA_TARGET_ROOT:-}" ]` block, with:
   ```bash
   case "$(uname -s)" in
     MINGW*|MSYS*|CYGWIN*)
       WIN_WT="$(cygpath -w "$WT")"
       WIN_MAIN="$(cygpath -w "$MAIN")"
       powershell -NoProfile -Command "New-Item -ItemType Junction -Path '$WIN_WT\\external' -Target '$WIN_MAIN\\external' | Out-Null"
       echo "created $WT on branch $BRANCH (external/ junctioned)"
       ;;
     *)
       ln -s "$MAIN/external" "$WT/external"
       echo "created $WT on branch $BRANCH (external/ symlinked)"
       [ -e "$MAIN/external" ] || echo "warning: $MAIN/external does not exist yet; populate it (docs/agents/development-workflow.md#create-a-worktree-that-builds)" >&2
       ;;
   esac

   # The Dev Drive seeding is Windows-only (ReFS block cloning through PowerShell).
   if [ -n "${CIMMERIA_TARGET_ROOT:-}" ] && command -v pwsh >/dev/null 2>&1; then
     pwsh -NoProfile -File "$MAIN/tools/dev-drive/Copy-WarmTarget.ps1" -TargetRoot "$CIMMERIA_TARGET_ROOT" -Name "$NAME" \
       || echo "warning: target seeding failed; the first build will be cold" >&2
   fi
   ```
3. Create `tools/build-lane/test_mk_worktree_sh.py`, an **integration test** (TESTING.md: a real git repository and the real script, in a temp dir). Shape:
   ```python
   """Tests for mk-worktree.sh in a throwaway repository with a bare origin; nothing here
   touches a real worktree.

       python -m unittest discover -s tools/build-lane -p "test_*.py"
   """
   # imports: os, shutil, subprocess, sys, tempfile, unittest, Path
   # sys.path.insert(0, HERE); from test_lane_stats import find_bash; from test_rebase_pr import Repo, sh
   SCRIPT = HERE / "mk-worktree.sh"

   @unittest.skipUnless(find_bash(), "no bash")
   class MkWorktreeShTests(unittest.TestCase):
       # setUp: tmp = mkdtemp(prefix="test-mkwt-"); repo = Repo(tmp); main/external/keep.txt = "real external\n";
       #   repo.commit(repo.main, {".gitignore": ".claude/worktrees/\nexternal\n", "README.md": "# Demo\n"}); repo.push_main()
       # run(*argv): subprocess.run([find_bash(), SCRIPT.as_posix(), *argv], cwd=repo.main, capture_output=True,
       #   text=True, env=dict(os.environ) minus CIMMERIA_TARGET_ROOT)
       # tearDown: remove <wt>/external first (cmd /c rmdir on Windows, rm -f elsewhere), as test_ship.py's
       #   tearDown does, so rmtree never follows the link; then shutil.rmtree(tmp, ignore_errors=True)
   ```
   The tests:
   - `test_links_external_and_the_worktree_builds_from_it`: run `probe/one one`. Expect exit 0 (print stderr on failure), and `.claude/worktrees/one/external/keep.txt` reads `real external\n`. On Windows, expect `(external/ junctioned)` in stdout; elsewhere expect `(external/ symlinked)` and `os.path.islink(<wt>/external)`.
   - `test_missing_main_external_warns_but_links` (`skipIf(os.name == "nt")`, reason "the Linux branch; CI runs it"): delete `main/external` first. Expect exit 0, `warning:` in stderr, and the link still created (`os.path.islink`).
   - `test_existing_worktree_is_left_alone`: run twice. The second run prints `exists:` and exits 0.

   **Why it fails when the fix is reverted:** on Linux the old script exits 127 at `cygpath`, so the first two tests fail in CI. The first test also guards the Windows branch locally.
4. `bash -n`: `Set-Location <wt>; & "C:\Program Files\Git\bin\bash.exe" -n tools/build-lane/mk-worktree.sh` exits 0.

**Checks:** step 4; the build-lane tests (rule 6), where the new file's Windows case passes and the Linux-only case skips; the LF check on the `.sh`; and the WSL run, best effort, for `test_mk_worktree_sh.py`.

**Docs owed:** none in this packet. AH-11h updates development-workflow.md. List "doc-update-map: build-lane script behaviour, docs in AH-11h".

### AH-11c: `ship.sh` and `rebase-pr.sh` find `python3`

Agent: packet-coder. Branch `ai-harness-docs/ah-11c-py-wrappers`, worktree name `ah-11c`. Size: about 25k working tokens. Wave 3b. Commit subject: `fix(build-lane): ship.sh and rebase-pr.sh find python3, python or py (AH-11c)`.

**Rules:** the AH-11 rules. Edit only `tools/build-lane/ship.sh` and `tools/build-lane/rebase-pr.sh`, and create `tools/build-lane/test_py_wrappers_sh.py`. Leave the `.ps1` twins alone: on Windows `python` is right, and ops on Linux uses the `.sh` (C10).

**The bug (F17):** both scripts end in `exec "${PYTHON:-python}" ...`. A host with only `python3` (Ubuntu, WSL) exits 127.

**Steps:**

1. In both files, replace the last line (`exec "${PYTHON:-python}" "$(dirname "$0")/<ship.py|rebase_pr.py>" "$@"`) with this block, keeping each file's own `.py` name:
   ```bash
   # $PYTHON, else the first of python3, python, py that runs (lane.sh's order). Running it
   # with -c '' skips the Windows Store python3 stub, which exists but is not Python.
   py="${PYTHON:-}"
   if [ -z "$py" ]; then
     for p in python3 python py; do
       if command -v "$p" >/dev/null 2>&1 && "$p" -c '' >/dev/null 2>&1; then py="$p"; break; fi
     done
   fi
   if [ -z "$py" ]; then
     echo 'status=refused reason="no Python: none of python3, python, py runs; install Python 3 or set PYTHON"'
     exit 2
   fi
   exec "$py" "$(dirname "$0")/ship.py" "$@"
   ```
   Exit 2 with a `status=refused` line is how both tools already report a refused precondition. ship.py's docstring: exit 2 is refused. rebase_pr.py's: exit 2 is a bad target or precondition.
2. Create `tools/build-lane/test_py_wrappers_sh.py`, a **unit test of the wrapper** (TESTING.md): real bash and fake interpreters on PATH. Shape:
   - `@unittest.skipUnless(find_bash(), "no bash")` (from `test_lane_stats`). In `setUp`, make a temp `bin` dir and a helper `shim(name, body)` that writes `bin/<name>` as `#!/bin/sh\n<body>\n` and `chmod 0o755`. Run each script as `[find_bash(), (HERE / "<script>").as_posix(), *argv]`, with `env = dict(os.environ, PATH=str(bin) + os.pathsep + os.environ["PATH"])` and `PYTHON` removed from env.
   - `real = Path(sys.executable).as_posix()`.
   - `test_ship_prefers_python3_over_python`: shim `python3` = `exec "<real>" "$@"`, and shim `python` = `echo wrong-python >&2; exit 97`. Run `ship.sh pr -C <tmp>/gone -m x`. Expect exit 2 and `status=refused` in stdout. **Reverted**, the script runs the `python` shim and exits 97.
   - `test_rebase_pr_prefers_python3_over_python`: the same shims; `rebase-pr.sh --help` exits 0.
   - `test_python_env_wins`: shim `mypy3` = `echo picked >&2; exec "<real>" "$@"`, with `PYTHON=<bin>/mypy3` (posix path). Expect `picked` in stderr.
   - `test_no_working_python_refuses`: shims `python3`, `python` and `py`, each `exit 1`. `ship.sh pr -C x -m x` exits 2 with `no Python` in stdout. **Reverted**, it runs the `python` shim (exit 1).
   - If a test fails only on Windows, because Git Bash does not see an extensionless shim, add `@unittest.skipIf(os.name == "nt", "Git Bash ignores extensionless shims; the Linux CI job runs it")` to that test and say so in the report. Do not change the scripts to suit the shim.
3. `bash -n` both scripts through Git Bash (rule 2).

**Checks:** step 3; the build-lane tests (rule 6), where the existing `WrapperTests` in test_ship.py must still pass; the LF check; and the WSL run, best effort, for `test_py_wrappers_sh.py`.

**Docs owed:** none in this packet. AH-11h updates development-workflow.md.

### AH-11d: `ship.py` works with an old `gh`

Agent: packet-coder. Branch `ai-harness-docs/ah-11d-gh-compat`, worktree name `ah-11d`. Size: about 40k working tokens. Wave 3b. Commit subject: `fix(build-lane): ship.py falls back for gh without pr checks --json or a working pr edit (AH-11d)`.

**Rules:** the AH-11 rules. Edit `tools/build-lane/ship.py` (only the import, `wait_for_checks` and `restack`), and create `tools/build-lane/gh_compat.py` and `tools/build-lane/test_ship_gh_compat.py`. Do not edit `test_ship.py`: it is at 698 lines.

**The bugs (F18):**

- With Ubuntu's apt `gh` 2.45, `gh pr checks --json` fails with `unknown flag: --json`. `wait_for_checks` treats the failure as "no checks yet" and polls until its timeout (exit 3) on every code PR.
- `gh pr edit --base` fails on an old `gh` with a GraphQL error about classic projects. `restack` ignores the failure (`check=False`), so a restacked child keeps its stale base.

**Steps:**

1. Create `tools/build-lane/gh_compat.py`. Write it to this shape (the docstrings are required):
   ```python
   """Fallbacks for a gh older than ship.py assumes: Ubuntu's apt gh 2.45 has no
   `gh pr checks --json`, and its `gh pr edit` fails on a classic-projects GraphQL field.
   Each function takes ship.py's `gh(*args, cwd=..., check=...)`, as ship_queue.py does."""

   from __future__ import annotations

   import json

   OLD_GH = ("unknown flag", "unknown shorthand flag")
   FAILED = {"FAILURE", "TIMED_OUT", "ACTION_REQUIRED", "STARTUP_FAILURE", "STALE", "ERROR"}


   def bucket(item: dict) -> str:
       """`gh pr checks`' bucket (pass, fail, pending, skipping, cancel) for one
       statusCheckRollup entry: a CheckRun (status, conclusion) or a StatusContext (state)."""
       if item.get("__typename") == "StatusContext" or "context" in item:
           state = (item.get("state") or "").upper()
           return "pass" if state == "SUCCESS" else "fail" if state in FAILED else "pending"
       if (item.get("status") or "").upper() != "COMPLETED":
           return "pending"
       conclusion = (item.get("conclusion") or "").upper()
       if conclusion in ("SUCCESS", "NEUTRAL"):
           return "pass"
       if conclusion == "SKIPPED":
           return "skipping"
       if conclusion == "CANCELLED":
           return "cancel"
       return "fail" if conclusion in FAILED else "pending"


   def checks(gh, pr: str, repo: str) -> list[dict]:
       """[{name, bucket}] for the PR's checks; [] while none are reported."""
       p = gh("pr", "checks", pr, "--json", "name,bucket", cwd=repo, check=False)
       if p.returncode != 0 and any(h in (p.stderr or "") + (p.stdout or "") for h in OLD_GH):
           p = gh("pr", "view", pr, "--json", "statusCheckRollup", cwd=repo, check=False)
           try:
               items = json.loads(p.stdout or "{}").get("statusCheckRollup") or []
           except json.JSONDecodeError:
               return []
           return [{"name": i.get("name") or i.get("context") or "", "bucket": bucket(i)} for i in items]
       try:
           return json.loads(p.stdout or "[]")
       except json.JSONDecodeError:
           return []   # "no checks reported" yet


   def set_base(gh, pr: str, base: str, repo: str) -> bool:
       """Retarget a PR, through the REST API when `gh pr edit` fails (an old gh)."""
       if gh("pr", "edit", pr, "--base", base, cwd=repo, check=False).returncode == 0:
           return True
       return gh("api", "-X", "PATCH", f"repos/{{owner}}/{{repo}}/pulls/{pr}", "-f", f"base={base}",
                 cwd=repo, check=False).returncode == 0
   ```
   `{owner}/{repo}` is `gh api`'s own placeholder for the current repository, which `ship_queue.py` already uses. The f-string's doubled braces print it literally.
2. `ship.py`:
   - After `import ship_queue  # noqa: E402`, add `import gh_compat  # noqa: E402`.
   - In `wait_for_checks`, replace the five lines from `p = gh("pr", "checks", pr, "--json", "name,bucket", cwd=repo, check=False)` through `checks = []   # "no checks reported" yet` with `checks = gh_compat.checks(gh, pr, repo)`.
   - In `restack`, replace `gh("pr", "edit", n, "--base", "main", cwd=repo, check=False)` with `gh_compat.set_base(gh, n, "main", repo)`. Ignore the result, as the current code does.
   - Then `(Get-Content <wt>\tools\build-lane\ship.py).Count` must be at most 557.
3. Create `tools/build-lane/test_ship_gh_compat.py`:
   - **Unit tests:** `BucketTests(unittest.TestCase)`, one assertion per row:
     - CheckRun COMPLETED with SUCCESS or NEUTRAL → `pass`;
     - SKIPPED → `skipping`;
     - CANCELLED → `cancel`;
     - FAILURE, TIMED_OUT or ACTION_REQUIRED → `fail`;
     - status IN_PROGRESS or QUEUED → `pending`;
     - StatusContext SUCCESS → `pass`; PENDING or EXPECTED → `pending`; FAILURE or ERROR → `fail`.

     `SetBaseTests(unittest.TestCase)` uses a fake `gh` callable that records its args and returns `subprocess.CompletedProcess(args, rc, "", "")`, with scripted return codes:
     - edit ok → one call and `True`;
     - edit rc 1, then api rc 0 → the second call is `("api", "-X", "PATCH", "repos/{owner}/{repo}/pulls/8", "-f", "base=main")` and the result is `True`;
     - both fail → `False`.
   - **Integration tests**, ship.py against a fake old `gh`: `from test_ship import FAKE_GH, GREEN, MergeCase`. MergeCase defines no tests, so none run twice. Define `OLD_GH_PRELUDE`, a Python source string that runs before `FAKE_GH`'s body. It loads `FAKE_GH_STATE`, appends `sys.argv[1:]` to `st["calls"]` and saves it, then answers and exits for four calls:
     - `pr checks ... --json` → stderr `unknown flag: --json`, exit 1;
     - `pr edit` → stderr `GraphQL: Projects (classic) is being deprecated in favor of the new Projects experience`, exit 1;
     - `api` with `PATCH` → stdout `{}`, exit 0;
     - `pr view` with `statusCheckRollup` → pops a frame from `st["rollup"]` the way `FAKE_GH` pops `checks` (`seq.pop(0) if len(seq) > 1 else seq[0]`), and prints `{"statusCheckRollup": frame}`.

     Every other call falls through to `FAKE_GH`, which records its own call. In `OldGhCase(MergeCase).setUp`, after `super().setUp()`:
     - write `bin/old_gh.py` as `OLD_GH_PRELUDE + FAKE_GH`;
     - stop and replace the GH patch in place: `self.patches[0].stop(); self.patches[0] = mock.patch.object(ship, "GH", [sys.executable, str(<bin>/"old_gh.py")]); self.patches[0].start()`. A second stacked patch would leak into later tests when tearDown stops them in order.
   - `test_checks_fall_back_to_the_status_rollup`: `add_pr(["src.rs"])`, and set `st["rollup"]` to two frames: first every GATING name as a CheckRun `IN_PROGRESS`, then every one `COMPLETED`/`SUCCESS`. Run `self.merge("--timeout", "5s")`. Expect exit 0, `["pr", "merge", "7", "--squash"]` among the calls, and at least one `pr view` call with `statusCheckRollup`. **Reverted** (ship.py calling `pr checks --json` directly): `checks` is always `[]`, so it times out with exit 3.
   - `test_a_failed_rollup_check_stops_the_merge`: one frame, with `cargo clippy -D warnings` `COMPLETED`/`FAILURE` and the other gating checks `SUCCESS`. Expect exit 1, `status=ci-failed`, and `checks=cargo clippy -D warnings`.
   - `test_restack_retargets_through_the_rest_api`: patch `ship.ship_queue.stacked_children` to return `[{"number": 8, "baseRefName": "feat"}]`, and `ship.REBASE` to `[sys.executable, "-c", "pass"]`. Call `ship.restack(str(self.repo.main), "feat", "0" * 40)`. Expect `(["8"], [])`, and that the calls include `["api", "-X", "PATCH", "repos/{owner}/{repo}/pulls/8", "-f", "base=main"]`. **Reverted**, only the failing `pr edit` is called.

**Checks:** the build-lane tests (rule 6). All of `test_ship.py` must still pass, which shows nothing changes on a current `gh`. Plus the `ship.py` line count.

**Docs owed:** none in this packet. AH-11h updates development-workflow.md.

### AH-11e: Linux CI job for the build-lane shell scripts

Agent: packet-coder. Branch `ai-harness-docs/ah-11e-linux-ci`, worktree name `ah-11e`. Size: about 20k working tokens. Wave 3b, **merged first**. Commit subject: `ci(build-lane): shellcheck and the shell-twin tests on Linux (AH-11e)`.

**Rules:** the AH-11 rules. Edit only `.github/workflows/build-lane-scripts.yml`, the `CONDITIONAL` tuple in `tools/build-lane/ship.py`, the check table in `docs/agents/pre-pr-checks.md`, and `tools/build-lane/README.md`.

**Why (F19):** the build-lane Python tests run on Linux only inside `build-and-test`, which on a PR runs only with the `full-ci` label. It always runs in the merge queue. `build-lane-scripts.yml` is Windows-only, and nothing runs shellcheck on `tools/build-lane/`. The launcher's `release-scripts` job (launcher-desktop.yml) is the precedent: `ubuntu-latest`, with shellcheck preinstalled.

**Steps:**

1. `.github/workflows/build-lane-scripts.yml`:
   - Change the header comment to say that it covers the PowerShell twins on Windows, and the `.sh` twins (bash -n, shellcheck and the Python tests that drive them) on Linux.
   - Add this second job after `scripts:`, at the same indentation:
   ```yaml
     shell:
       name: build-lane shell scripts (Linux)
       runs-on: ubuntu-latest
       timeout-minutes: 15
       steps:
         - uses: actions/checkout@v7
           with:
             persist-credentials: false
         - name: bash -n (every script parses)
           run: for f in tools/build-lane/*.sh; do bash -n "$f" || exit 1; done
         - name: shellcheck (warnings on the scripts ops runs, errors on all)
           run: |
             shellcheck -S warning tools/build-lane/mk-worktree.sh tools/build-lane/ship.sh tools/build-lane/rebase-pr.sh
             shellcheck -S error tools/build-lane/*.sh
         - name: build-lane Python tests (they drive the .sh twins with bash)
           run: python3 -m unittest discover -s tools/build-lane -p "test_*.py"
   ```
2. `tools/build-lane/ship.py`: in `CONDITIONAL`, add `"build-lane shell scripts (Linux)",` on the line after `"build-lane PowerShell tests",`. That is +1 line. AH-11d shrinks the file by 3.
3. `docs/agents/pre-pr-checks.md`: after the `build-lane PowerShell tests` row (line 20), add a row in the same format:
   ```text
   | `build-lane shell scripts (Linux)` | [build-lane-scripts.yml](../../.github/workflows/build-lane-scripts.yml) | **Yes** when it runs, enforced by `ship.py merge` | `tools/build-lane/**`: `bash -n`, shellcheck and the build-lane Python tests on Linux |
   ```
4. `tools/build-lane/README.md`: after the sentence that ends `are their tests.`, add: ` CI runs the \`.sh\` twins on Linux (bash -n, shellcheck and the Python tests), and the \`.ps1\` tests on Windows ([build-lane-scripts.yml](../../.github/workflows/build-lane-scripts.yml)).` Remove the backslashes (quoting rule 8 of work-packets.md).

**Test type:** CI configuration. It is proven by the job's own first run on this PR, which the reviewer reads with `gh pr checks`. You cannot run shellcheck locally (rule 2). If the job is red on a shellcheck finding in a script this packet does not own, the review-fixer fixes that script in this worktree, within the fix-round limit. A finding in `mk-worktree.sh`, `ship.sh` or `rebase-pr.sh` goes to that packet's worktree.

**Checks:**

- The YAML parses: `Set-Location <wt>; python -c "import sys,yaml; yaml.safe_load(open(sys.argv[1]))" .github/workflows/build-lane-scripts.yml`. If PyYAML is missing, skip this check and say so; install nothing.
- The build-lane tests (rule 6).
- The CRLF check on both `.md` files.

**Docs owed:** pre-pr-checks.md (the check table) and tools/build-lane/README.md; list both.

### AH-11f: platform wording outside #1797's files

Agent: packet-coder. Branch `ai-harness-docs/ah-11f-platform-wording`, worktree name `ah-11f`. Size: about 25k working tokens. Wave 3b; merge after AH-11a. Commit subject: `docs: one platform statement: Windows with PowerShell 7 supported, others not validated (AH-11f)`.

**Rules:** the AH-11 rules. Edit only the files in the table, plus the `.agents/skills/lane-build/SKILL.md` mirror that sync.py regenerates. Do not touch development-workflow.md, CLAUDE.md, rules-and-gotchas.md or the kickoff guide: AH-11a and AH-11h own them.

**Steps:** for each row, Grep the old text and replace only that sentence or clause. Links point at `#wsl-and-linux-hosts`, which AH-11a creates; the orchestrator merges AH-11a first.

| # | File | Old text (find it) | New text |
|---|---|---|---|
| 1 | `CONTRIBUTING.md` | `(Windows-native builds on a pinned toolchain, and the build lane agents use)` | `(Windows with PowerShell 7 as the validated platform, a pinned toolchain, and the build lane agents use)` |
| 2 | `docs/architecture/build-system.md` line 141 | `### 11. No WSL builds` | `### 11. Platforms, and the retired WSL cross-compile`. Then Grep the repo for `#11-no-wsl-builds` (none on 2026-10-18) and fix any hit to `#11-platforms-and-the-retired-wsl-cross-compile`. |
| 3 | same file, line 143 | `Development builds run natively on Windows (PowerShell or Git Bash, driven by Claude Code).` | `Development builds are validated natively on Windows with PowerShell 7. WSL, Linux, and Git Bash on Windows are not validated, but not unsupported ([WSL and Linux hosts](../agents/development-workflow.md#wsl-and-linux-hosts)).` |
| 4 | `docs/building.md` line 25 | `Development builds run natively on Windows (PowerShell or Git Bash). CI builds on Linux, but there is no WSL or cross-compile path for development.` | `Development builds are validated on Windows with PowerShell 7. CI builds on Linux. WSL, Linux, and Git Bash on Windows are not validated, but not unsupported ([WSL and Linux hosts](agents/development-workflow.md#wsl-and-linux-hosts)); the old WSL cross-compile path is retired.` |
| 5 | `docs/guides/getting-started.md` line 37 | `There is no WSL build path.` | `Windows with PowerShell 7 is the validated setup; WSL and Linux are not validated, but not unsupported ([WSL and Linux hosts](../agents/development-workflow.md#wsl-and-linux-hosts)).` |
| 6 | `.github/copilot-instructions.md` line 40 | `development builds run natively on Windows;` | `development builds are validated on Windows with PowerShell 7 (WSL, Linux and Git Bash are not validated, but not unsupported);` |
| 7 | `.github/instructions/rust-services.instructions.md` line 31 | `Builds run natively on Windows on the toolchain` | `Builds are validated on Windows with PowerShell 7 (WSL, Linux and Git Bash are not validated, but not unsupported), on the toolchain` |
| 8 | `.claude/skills/lane-build/SKILL.md` line 57 | `- **Builds are Windows-native.** The WSL cross-compile is retired; CI builds on Linux runners.` | `- **Windows with PowerShell 7 is the validated platform.** WSL, Linux and Git Bash are not validated, but not unsupported; the WSL cross-compile is retired, and CI builds on Linux runners.` Then run `python tools/agent-skills/sync.py`. |

Then:

- Grep the repo (`*.md`, `*.yml`, `*.py`, `*.ps1`) for `nopjmp` and `Linux pipeline`. Expect no hits (F10) and record the result. If there is a hit, delete that sentence.
- Leave `.cargo/config.toml`, `ship.py`, `lane-slots.ps1` and the `pre-pr-checks.md` "macOS / Linux / WSL" labels unchanged. They state facts (the default MSVC target, which shell runs what), not a support policy.
- If a row's old text is not found, apply the C7 meaning to whatever the sentence now says, and list it as a deviation.

**Test type:** none (docs only).

**Checks:** the CRLF check on every edited `.md`, and `python tools/agent-skills/sync.py --check`.

**Docs owed:** the doc-update-map row for build commands and invariants (copilot-instructions); list it.
