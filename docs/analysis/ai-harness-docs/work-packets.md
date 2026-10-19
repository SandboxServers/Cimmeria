# ai-harness-docs: work packets

Ledger: [README.md](README.md). Each packet below is self-contained: it carries its own rules and the contract text it needs. A packet-coder skips CLAUDE.md, so it must not rely on it.

## Dispatch rules (for the orchestrator)

1. **Ship this ledger first.** `ops`: `ship.ps1 pr -C <ledger worktree> -m "docs(ai-harness-docs): campaign ledger and packets"`, then merge. Packet worktrees are cut from `origin/main` afterwards, so they contain this file. Until then, give briefs the absolute path of this file in the ledger worktree.
2. One worktree per packet: `pwsh tools/build-lane/mk-worktree.ps1 ai-harness-docs/<packet>-<slug> <name>`. Branch and name are listed in each packet. A stacked packet's worktree is cut from its base branch, and ops ships it with `--base <branch>`.
3. Brief (under 25 lines) = worktree path, this file's path and the packet heading, the report path `<worktree>\.claude\reports\ai-harness-docs\<packet>-<role>.md`, the commit attribution lines, and the return line `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. Add: "PowerShell only; run every check in the foreground with `Set-Location <worktree>;` at the start of the call; commit as soon as the checks pass."
4. **Coder loop (D-AH2, D-AH3):** a coder that returns `blocked` with the reason `context` gets a fresh packet-coder with "continuation N of 3" and its worknote path. After the third stop, dispatch one `rust-gameserver-dev` (tools packets) or `documentation-writer` (docs packets) with the worknote. Review findings never go back to a coder. **Superseded 2026-10-18 (D-AH43):** fix notes now go to a *fresh* packet-coder; see [work-packets-w4.md C12](work-packets-w4.md#c12-the-review-loop-d-ah43-replaces-c6).
5. **Review (D-AH4, D-AH5):** one `packet-review-fixer` per wave. Its return ends `verdicts=<packet>:<ship|recheck|decision>,...`. On `recheck`, dispatch a fresh review-fixer for that packet alone (pass 2, then 3). After pass 3 without `ship`, or on `decision`, dispatch `architect` for a ruling note in `worknotes/<packet>.md`, then a fresh review-fixer to apply it. (W1 runs before AH-07 lands, so the W1 review-fixer may return the old "ship / needs coordinator decision" wording: treat that as `ship` / `decision`.) **Superseded 2026-10-18 (D-AH43):** `packet-review-fixer` is retired; every review from W3c on runs the loop in [work-packets-w4.md C12](work-packets-w4.md#c12-the-review-loop-d-ah43-replaces-c6).
6. Waves: W1 = AH-01, AH-02, AH-03 (stacked on AH-02 after AH-02 reports `go`), AH-05. W2 = AH-06..AH-09, after W1 merges. W3 = AH-12, AH-13, AH-14 (stacked on AH-13), after W2 merges (all merged). W3b = AH-11a..AH-11f, the #1797 takeover and the Linux scripts, merging AH-11e first and AH-11a before AH-11f ([work-packets-w3b.md](work-packets-w3b.md)); W3c = AH-11g and AH-11h, after W3b merges; then AH-11i, the owner's closing reply on #1797 ([work-packets-w3.md](work-packets-w3.md)). W4 = AH-15 (owner), then AH-16.
7. Packets in a wave touch disjoint files, except where a packet is stacked. Do not reorder file ownership.
8. **Quoting convention.** Exact text to insert is shown inside a code span. Inside such a span, `\`` stands for a plain backtick and `\|` for a plain `|`: write them without the backslash.

## Contract

### C1. `.claude/hooks/py.sh` (exact file, LF line endings; `.gitattributes` forces `*.sh` to LF)

```sh
# Interpreter launcher for the Python hooks: finds python3, python or py, then execs it.
# Sourced, not run, so it costs no extra process:  set -- <script> <args>; . "<repo>/.claude/hooks/py.sh"
# Windows (Git Bash: OSTYPE msys or cygwin) tries python first, because the Microsoft Store
# python3.exe alias stub exists where python.org installs ship no python3.exe; elsewhere python3 first.
# `hash` is a builtin that finds and remembers the command, so exec needs no second PATH search.
# With no interpreter it warns on stderr and exits 1: a visible, non-blocking hook error (exit 2 would
# block every tool call). Known gap: a Windows host with only the Store stub execs the stub, which fails.
case "${OSTYPE:-}" in
  msys*|cygwin*) _cimm_py_order='python py python3' ;;
  *) _cimm_py_order='python3 python' ;;
esac
for _cimm_py in $_cimm_py_order; do
  if hash "$_cimm_py" 2>/dev/null; then
    exec "$_cimm_py" "$@"
  fi
done
echo "cimmeria hook: no python3, python or py on PATH, so ${1:-a hook} did not run and its guard is off" >&2
exit 1
```

### C2. Hook command shapes (AH-03 rewires exactly these four)

| Site | New `command` value (the text after `"command": ` in JSON, or between the YAML single quotes) |
|---|---|
| `.claude/settings.json` SessionStart `compact` | `f="$CLAUDE_PROJECT_DIR/.claude/hooks/resume_card.py"; [ -n "$CLAUDE_PROJECT_DIR" ] && [ -f "$f" ] \|\| exit 0; set -- "$f"; . "$CLAUDE_PROJECT_DIR/.claude/hooks/py.sh"` |
| `.claude/settings.json` PreToolUse `Bash\|PowerShell` | `f="$CLAUDE_PROJECT_DIR/.claude/hooks/shell_guard.py"; [ -n "$CLAUDE_PROJECT_DIR" ] && [ -f "$f" ] \|\| exit 0; set -- "$f" --harness claude; . "$CLAUDE_PROJECT_DIR/.claude/hooks/py.sh"` |
| `.claude/agents/orchestrator.md` frontmatter | `f="$CLAUDE_PROJECT_DIR/.claude/hooks/orchestrator_guard.py"; [ -n "$CLAUDE_PROJECT_DIR" ] && [ -f "$f" ] \|\| exit 0; set -- "$f"; . "$CLAUDE_PROJECT_DIR/.claude/hooks/py.sh"` |
| `.codex/hooks.json` PreToolUse `Bash` | `r="$(git rev-parse --show-toplevel 2>/dev/null)"; f="$r/.claude/hooks/shell_guard.py"; [ -f "$f" ] \|\| exit 0; set -- "$f" --harness codex; . "$r/.claude/hooks/py.sh"` |

In JSON every `"` inside the value is written `\"`, as in the current file. In the table, `\|` stands for a plain `|`.

New first SessionStart entry in `.claude/settings.json` (no `matcher`, so it fires on startup, resume, clear and compact):

```json
      {
        "hooks": [
          {
            "type": "command",
            "command": "f=\"$CLAUDE_PROJECT_DIR/.claude/hooks/mode_banner.sh\"; [ -n \"$CLAUDE_PROJECT_DIR\" ] && [ -f \"$f\" ] || exit 0; . \"$f\"",
            "timeout": 5
          }
        ]
      },
```

### C3. `.claude/hooks/mode_banner.sh` (exact file, LF)

```sh
# SessionStart mode banner: tells the user and the model which persona this session runs as.
# Pure POSIX sh with builtins only (no Python, no forks), so it still works where the Python hooks cannot.
# First SessionStart entry in .claude/settings.json, with no matcher, so it also runs after a compaction.
payload=''
while IFS= read -r line || [ -n "$line" ]; do
  payload="$payload$line"
done
agent=''
case "$payload" in
  *'"agent_type":"'*) rest=${payload#*'"agent_type":"'}; agent=${rest%%'"'*} ;;
  *'"agent_type": "'*) rest=${payload#*'"agent_type": "'}; agent=${rest%%'"'*} ;;
esac
case "$agent" in
  *[!A-Za-z0-9_-]*) agent='' ;;
esac
guards=''
if ! hash python3 2>/dev/null && ! hash python 2>/dev/null && ! hash py 2>/dev/null; then
  guards=' WARNING: no python3, python or py on PATH, so the shell and orchestrator guards are off.'
fi
if [ -z "$agent" ] || [ "$agent" = orchestrator ]; then
  msg="Mode: orchestrator, this repo's default persona (it dispatches agents and has no shell). For direct work start claude --agent hands-on. How the AI workflow works: README.md#how-ai-development-works-here.$guards"
  ctx="Session mode: you run as the orchestrator persona unless this session was started with --agent. When the user asks for direct work, mention once that claude --agent hands-on starts a full-tool session, then still dispatch the right agent for the task.$guards"
  printf '{"systemMessage":"%s","hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"%s"}}\n' "$msg" "$ctx"
else
  printf '{"systemMessage":"%s"}\n' "Mode: $agent (started with --agent).$guards"
fi
exit 0
```

### C4. `orchestrator_guard.py` worktree helper (AH-01)

```python
def under_worktree_dirs(root, target, dirs):
    """True when target lies under <root>/.claude/worktrees/<name>/ followed by one of the dirs."""
    try:
        parts = target.relative_to(Path(os.path.normcase(os.path.realpath(root)))).parts
    except ValueError:
        return False
    if len(parts) < 4 or parts[:2] != (".claude", "worktrees"):
        return False
    rest = parts[3:]
    return any(rest[: len(prefix)] == prefix for prefix in dirs)
```

`verdict()` allows Edit or Write when `under_dirs(...)` **or** `under_worktree_dirs(root, target, WRITABLE_DIRS)` holds, and a Read when `under_dirs(...)` **or** `under_worktree_dirs(root, target, READABLE_DIRS)` holds. `WRITE_REASON` gains: `(in the main checkout or a campaign worktree under .claude/worktrees/<name>/)`.

### C5. `tools/docs-gen/harness_tables.py` API (AH-05)

```python
"""Render the AI-harness hook and agent tables for regen.py (gen:harness-hooks, gen:harness-agents)."""
INLINE_REASONS = {"agent-board": "Agent board: registers the session and shows unread board activity. External tool, skipped when it is not installed."}
@dataclass(frozen=True)
class Hook:
    event: str      # "SessionStart", "PreToolUse", ...
    matcher: str    # "" when the entry has none
    where: str      # ".claude/settings.json", ".codex/hooks.json" or "agent: <name>"
    runs: str       # script file name, e.g. "shell_guard.py", or "agent-board"
    reason: str
def hook_script(command: str) -> str | None   # first .claude/hooks/<name>.py|.sh in command that is not py.sh; "agent-board" if the command names .agent-board; else None
def script_reason(path: Path) -> str          # .py: first line of the module docstring (ast.get_docstring); .sh: first line starting "# ", without "# "; "" when none
def json_hooks(root: Path, rel: str, where: str) -> list[Hook]
def agent_hooks(root: Path) -> list[Hook]     # .claude/agents/*.md frontmatter `hooks:` blocks, sorted by agent file name
def collect_hooks(root: Path) -> list[Hook]   # settings.json, then agent frontmatter, then .codex/hooks.json
def render_hooks(root: Path) -> str
def render_agents(root: Path) -> str
```

- `render_hooks` returns `"\n" + table + "\n"`. The table header is `| Event | Matcher | Configured in | Runs | Why |`, with `|---|---|---|---|---|`. A row reads `| SessionStart | (all) | \`.claude/settings.json\` | \`mode_banner.sh\` | <reason> |`. An empty matcher is `(all)`, and `|` inside the matcher is written `\|`. A row whose reason is `""` shows `(no reason: add a docstring)`.
- `render_agents` returns `"\n" + table + "\n"`. Header: `| Agent | Model | Effort | Max turns | Role |`. Rows are sorted by `name`. A missing model is `inherit`, and a missing effort or maxTurns is `-`. Role is the description up to and including its first `. ` (the whole description when there is none), with `|` escaped.
- Frontmatter parsing: the lines between the first two `---` lines. A top-level `key: value` line (no leading space or `-`) is a field. A value in double quotes is `json.loads`-ed, and one in single quotes has the quotes stripped. Inside `hooks:`, a 2-space `<Event>:` line starts an event, and `matcher:` and `command:` lines (any indent, an optional leading `- `) give the matcher and the command. Each command makes one Hook.
- Registration in `tools/docs-gen/regen.py`, the same shape as `gen_tools_catalog`:

```python
def _harness_tables():
    return sys.modules.get("harness_tables") or _load("harness_tables", pathlib.Path(__file__).with_name("harness_tables.py"))


def gen_harness_hooks(ctx: Context, args: list[str], after: str) -> str:
    return _harness_tables().render_hooks(ctx.root)


def gen_harness_agents(ctx: Context, args: list[str], after: str) -> str:
    return _harness_tables().render_agents(ctx.root)
```

Add `"harness-hooks": gen_harness_hooks, "harness-agents": gen_harness_agents,` to `GENERATORS`.

### C6. Cycle vocabulary (W2 and W3 use these words exactly)

**Superseded 2026-10-18 by [work-packets-w4.md C12](work-packets-w4.md#c12-the-review-loop-d-ah43-replaces-c6) (D-AH43).** Kept as the record of what W2 and W3 shipped.

| Count | Limit | Meaning |
|---|---|---|
| Fix round | 5 per check | One rerun of a lane check inside one session |
| Coder session | 3 per packet | One packet-coder run. A context stop leaves a worknote, and the next fresh session continues from it. |
| Review pass | 3 per packet | One packet-review-fixer session over that packet |

The escalation ladder:

- Coder: 3 Haiku sessions → 1 Sonnet implementer (`rust-gameserver-dev` for code and tools, `documentation-writer` for docs) → `architect` re-slices. Opus never implements.
- Reviewer: verdict `ship | recheck | decision`, at most 3 passes. Then `architect` (or the Opus advisor for protocol or RE) writes a ruling note with no code, and a fresh Sonnet review-fixer applies it. An owner call is BlockedDecision.

### C7. Platform wording

- **Long form** (development-workflow.md § WSL and Linux hosts): "**Status: Windows with PowerShell 7 is fully supported. WSL, Linux, and Git Bash on Windows are not validated, but not unsupported.** The build lane, the ship scripts and every agent rule are validated on Windows with PowerShell 7. The build-lane scripts have `.sh` twins and CI builds on Linux, but nobody runs development sessions there routinely, so expect the gaps below and report new ones. Agents use PowerShell; an agent on a host without it returns `blocked: no PowerShell (pwsh) on this host`."
- **Short form**: "Windows with PowerShell 7 is fully supported; WSL, Linux, and Git Bash on Windows are not validated, but not unsupported."
- Anchor for links: `docs/agents/development-workflow.md#wsl-and-linux-hosts`.
- **Revised 2026-10-18 (D-AH16):** the long form's last sentence changes once `ops` has its bash path. AH-11a writes the long form above; AH-11h replaces it with C7b in [work-packets-w3b.md](work-packets-w3b.md#c7b-platform-wording-final-long-form). The short form does not change.

### C8. Numbers (W3 cites these and nothing else; the sources are in-repo)

| Measure | Opus implements (before 2026-10-10) | Haiku packets, Sonnet review | Source |
|---|---|---|---|
| Cost per 1,000 changed lines | $7.44 | $3.03 (2.5x cheaper) | campaign-metrics findings.md |
| Cost per accepted change | $18.65 | $2.35 | same |
| Cost per merged PR, mean / median | $22.01 / $18.18 (248 PRs, $5,727.50) | $2.67 / $0.91 (185 PRs, $594.06) | same |
| Spend by tier | Opus 93% | Opus 75%, Sonnet 22%, Haiku 2% | same |

Facts to cite, with their reasons:

- **Caveat**, always stated: the work differs. Haiku campaigns implement from a finished spec, and the spec's cost sits in the Opus coordinator. The 2.5x per 1,000 lines is the fair comparison, not the per-PR figures.
- **Packet size:** 9 of 34 Haiku sessions hit the 90k cap before the re-slice, and 0 of 36 after. About 35-40k of a Haiku session is fixed overhead, which is why a packet is about 40k working tokens.
- **Fresh sessions:** Haiku 5.5 bills 5x per request above a 100k prompt, which is why the coder loop starts a fresh session instead of growing one.
- **Escalation:** 16.9% of Haiku-regime packets escalated (44 of 261). 0.60 retries per accepted packet. 4.8% first-pass completion, because almost every packet needed a review fix, which is why review is a built-in step.
- **Opus doesn't code:** $107.43 went to oversteps (Opus coding in the main session $76.48; Opus implementing a Haiku packet $27.43), of which $53.72 to $84.20 was recoverable at the intended tier.
- **Guards:** one week of telemetry (2026-10-03..10) showed 326 direct `cargo` calls and about 10,000 grep or cat calls through the shell (shell_guard.py docstring).
- **Context:** the median main-session request carried 349k tokens of context. Parked workers' cache rewrites were 13.2% of all spend, and one-per-request mechanical calls 11.2% (development-workflow.md § Worker lifetime), which is why there are no teammates, no parking, and batched mechanical steps.

### C9. Anchors

- README section heading: `## How AI development works here`, giving `README.md#how-ai-development-works-here`.
- Reference doc: `docs/agents/ai-harness.md`, with headings `## Settings`, `## Agents`, `## Skills`, `## Hooks and guards`, `## Board and memory`, `## Review bots`, `## Cycle and escalation`, `## Keeping this page current`.

---

## Wave 1: tooling

### AH-01: orchestrator_guard allows worktree ledgers

Agent: packet-coder. Branch `ai-harness-docs/ah-01-guard-worktree`, worktree name `ah-01`. Size: about 15k working tokens. Commit subject: `fix(hooks): orchestrator guard allows a campaign worktree's ledger paths (AH-01)`.

**Rules for this packet:**

- Work only in your worktree, with absolute paths. Start every PowerShell call with `Set-Location <worktree>;`, and run every check in the foreground.
- PowerShell only, never bash. This packet compiles no Rust, so it runs no `cargo` and no lane.
- Never run `git stash`, `git push`, `git worktree prune` or `gh pr`.
- Edit only the files listed below. Python files keep their existing line endings.
- Files stay under 700 lines.
- No IPs, credentials or account names.
- Commit once at the end with the brief's message.

**Why:** the orchestrator's session cwd is the main checkout, so it couldn't write `.claude/worktrees/<name>/docs/analysis/...` (README F4, D-AH12).

**Files:** `.claude/hooks/orchestrator_guard.py` and `tools/agent-skills/test_hooks.py`.

**Steps:**

1. In `orchestrator_guard.py`, add `under_worktree_dirs` from contract C4 right after `under_dirs` (line 88-94).
2. In `verdict()` (lines 111-131), change the Edit/Write line to `return None if any(under_dirs(r, target, WRITABLE_DIRS) or under_worktree_dirs(r, target, WRITABLE_DIRS) for r in roots) else WRITE_REASON`. Make the same change to the Read check, with `READABLE_DIRS`.
3. Add to `WRITE_REASON` the clause from C4, before " Dispatch".
4. Update the module docstring's list of allowed paths to match. Read lines 1-19 first.
5. In `test_hooks.py`, insert these tests in `OrchestratorGuardTest`, **directly after `test_allows_write_to_reports_dir`** (it ends at line 211), not at the end of the class:
   - `test_allows_worktree_ledger_from_main_checkout`: create `self.project/.claude/worktrees/x`. With no `cwd` in the payload, Write `str(worktree/"docs/analysis/x/README.md")`, Write `.../.claude/state/active-campaign` and Edit `.../.claude/reports/x/r.md` each return 0. A Read of a 500-line `worktree/docs/analysis/x/big.md` with no limit also returns 0 (use `write_lines`).
   - `test_denies_worktree_code_from_main_checkout`: with no cwd, Edit `worktree/crates/foo.rs` returns 2, and a 500-line `worktree/crates/big.rs` Read with no limit returns 2.
   - `test_denies_escape_through_worktrees_dotdot`: Write `str(self.project/".claude/worktrees/x/../../../crates/foo.rs")` returns 2. Also Write to `.claude/worktrees/docs/analysis/a.md`, which has no worktree-name segment: `parts[3:]` is `("analysis","a.md")`, so it returns 2.

**Test type:** unit test (TESTING.md §1, Python unittest). The first test fails with the change reverted, because today that path is denied with exit 2.

**Checks (in order):**

```powershell
Set-Location <worktree>; python -m unittest discover -s tools/agent-skills -p "test_*.py"
Set-Location <worktree>; python tools/agent-skills/sync.py --check
```

**Docs owed:** none in this packet; AH-13 documents the guard.

### AH-02: `py.sh` launcher, tests and timing

Agent: packet-coder. Branch `ai-harness-docs/ah-02-py-launcher`, worktree name `ah-02`. Size: about 30k working tokens. Commit subject: `feat(hooks): py.sh interpreter launcher with tests and timing harness (AH-02)`.

**Rules for this packet:**

- Work only in your worktree, with absolute paths. Start every PowerShell call with `Set-Location <worktree>;`, and run every check in the foreground.
- PowerShell is your shell. You run Git Bash only as the subject of the measurement in step 4, always as `& "$env:ProgramFiles\Git\bin\bash.exe"`, never a bare `bash`: on Windows a bare `bash` can resolve to WSL's System32 bash.
- No `cargo`, no lane. Never run `git stash`, `git push`, `git worktree prune` or `gh pr`.
- `*.sh` files must be LF. After writing `py.sh`, check with `[IO.File]::ReadAllText('<abs path>').Contains("`r")` that the result is `False`.
- `docs/**/*.md` must be CRLF. Write `hook-timing.md`, then convert it with `$t=[IO.File]::ReadAllText($p) -replace "`r?`n","`r`n"; [IO.File]::WriteAllText($p,$t)`, using absolute paths.
- Files stay under 700 lines. No IPs, credentials or account names.
- Commit once at the end with the brief's message.

**Files:** new `.claude/hooks/py.sh` (C1 verbatim), new `tools/agent-skills/test_hook_launcher.py`, new `tools/agent-skills/measure_hooks.ps1`, and new `docs/analysis/ai-harness-docs/hook-timing.md`. Do **not** touch settings.json, orchestrator.md or .codex/hooks.json: AH-03 does the wiring.

**Steps:**

1. Write `py.sh` exactly as C1.
2. Write `test_hook_launcher.py`:
   - Module docstring: `"""Tests for .claude/hooks/py.sh, the interpreter launcher every Python hook goes through."""`.
   - `REPO = Path(__file__).resolve().parents[2]`, and `PY_SH = REPO / ".claude/hooks/py.sh"`.
   - `posix_shell()`: on `os.name == "nt"`, return `Path(os.environ.get("ProgramFiles", r"C:\Program Files"))/"Git"/"bin"/"bash.exe"` if it is a file, else `None`. Otherwise return `shutil.which("sh")`.
   - `run_launcher(args, path, ostype=None)`: runs `[SHELL, "-c", (f"OSTYPE={ostype}; " if ostype else "") + '. "$PY_SH"', "sh", *args]` with `env = {"PATH": path, "PY_SH": PY_SH.as_posix(), **{k: os.environ[k] for k in ("SYSTEMROOT", "TEMP", "TMP") if k in os.environ}}`, `capture_output=True, text=True`.
   - Class `PyLauncherTest`, decorated `@unittest.skipIf(SHELL is None, "no POSIX shell")`, with these tests:
     - `test_execs_python_with_script_and_args`: write a temp `echo_args.py` that prints `json.dumps(sys.argv[1:])`, then run with `[script, "--harness", "claude"]` and `path=os.path.dirname(sys.executable)`. Expect exit 0 and stdout JSON `["--harness", "claude"]`.
     - `test_missing_interpreter_warns_and_exits_1`: an empty temp dir as `path`. Expect returncode 1, `"no python3, python or py"` in stderr, and the script name in stderr.
     - `test_prefers_python3_off_windows`: `@unittest.skipIf(os.name == "nt", ...)`. Make a temp bin with executable fake `python3` and `python` scripts (`#!/bin/sh\necho <name>\n`, `os.chmod(p, 0o755)`), and set `path = bin + os.pathsep + "/usr/bin:/bin"`. Stdout is `python3`.
     - `test_prefers_python_on_msys`: same fakes, `ostype="msys"`. Stdout is `python`.
3. Write `measure_hooks.ps1`:
   - A comment-based help block whose `.SYNOPSIS` is `Times a hook command the way Claude Code runs it: through Git Bash, with a JSON payload on stdin.`
   - `param([Parameter(Mandatory)][string]$Command, [string]$Payload = '{}', [int]$Runs = 30, [int]$Warmup = 3, [string]$Shell = '')`.
   - Resolve `$Shell`: if empty, use `Join-Path $env:ProgramFiles 'Git\bin\bash.exe'` on `$IsWindows`, else `/bin/sh`. Throw if it is missing.
   - Set `$env:CLAUDE_PROJECT_DIR` to `git rev-parse --show-toplevel`.
   - Loop `$Warmup + $Runs` times. Each iteration does `$sw=[Diagnostics.Stopwatch]::StartNew(); $null = $Payload | & $Shell -c $Command 2>&1; $sw.Stop()` and keeps the times after the warmup.
   - Print one compressed JSON object: `runs`, `median_ms`, `p90_ms`, `min_ms` (rounded to 0.1) and `exit` (the last `$LASTEXITCODE`).
   - Under 80 lines.
4. Measure, on this Windows machine, from the worktree. Run each pair twice, in the order before, after, before, after. Use these payloads:
   - shell_guard: `{"tool_name":"PowerShell","tool_input":{"command":"git status"},"cwd":"<worktree with forward slashes>"}`
   - resume_card: `{"hook_event_name":"SessionStart","source":"compact"}`

   | Label | `-Command` |
   |---|---|
   | bare-python | `python -c pass` |
   | shell_guard before | the current settings.json shell_guard command (C2 site 2, but ending `; python "$f" --harness claude`) |
   | shell_guard after | C2 site 2 |
   | resume_card before | `f="$CLAUDE_PROJECT_DIR/.claude/hooks/resume_card.py"; [ -n "$CLAUDE_PROJECT_DIR" ] && [ -f "$f" ] \|\| exit 0; python "$f"` |
   | resume_card after | C2 site 1 |

   Pass each command in single quotes, so PowerShell expands nothing. First check quoting with `-Command 'echo "$CLAUDE_PROJECT_DIR"' -Runs 1 -Warmup 0`, which must print a path.
5. Write `docs/analysis/ai-harness-docs/hook-timing.md`. Include: date, machine note (Windows, Git Bash version from `& "$env:ProgramFiles\Git\bin\bash.exe" --version` first line, `python --version`), the raw JSON lines, and a table of median before, median after, added ms and added %, with each pair's two rounds averaged. End with a verdict line: `Verdict: go` if the added median is at most 5 ms **and** at most 10% for both hooks, otherwise `Verdict: no-go`.
6. If the verdict is `no-go`: still commit, and return `blocked` with the reason `no-go: +<ms> ms / +<pct>%`.

**Test type:** unit test (Python unittest). With `py.sh` removed or its order reversed, the exec and order tests fail. With `exit 2` instead of `exit 1`, the missing-interpreter test fails.

**Checks (in order):**

```powershell
Set-Location <worktree>; python -m unittest discover -s tools/agent-skills -p "test_*.py"
Set-Location <worktree>; python tools/agent-skills/sync.py --check
```

**Docs owed:** none beyond hook-timing.md. AH-13 documents the launcher, and AH-14's regen run covers the tools catalog.

### AH-03: wire the launcher, add the mode banner

Agent: packet-coder. **Stacked on `ai-harness-docs/ah-02-py-launcher`; dispatch only after AH-02 returns `done` with verdict `go`.** Branch `ai-harness-docs/ah-03-wire-banner`, worktree name `ah-03`, cut from the AH-02 branch. Size: about 30k working tokens. Commit subject: `feat(hooks): route Python hooks through py.sh and add the session mode banner (AH-03)`.

**Rules for this packet:**

- Work only in your worktree, with absolute paths. Start every PowerShell call with `Set-Location <worktree>;`, and run every check in the foreground.
- PowerShell only. Run no bare `bash`; the tests find Git Bash themselves.
- No `cargo`, no lane. Never run `git stash`, `git push`, `git worktree prune` or `gh pr`.
- `*.sh` must be LF; check it as in AH-02. `.claude/agents/*.md` keep their current line endings. Edit them with the Edit tool and don't rewrite the whole file.
- After editing `.claude/agents/orchestrator.md`, run `python tools/agent-skills/sync.py` and commit what it regenerates.
- Files stay under 700 lines. No IPs, credentials or account names.
- Commit once at the end.

**Files:** `.claude/settings.json`, `.claude/agents/orchestrator.md` (frontmatter `command:` line only), `.codex/hooks.json`, new `.claude/hooks/mode_banner.sh`, `tools/agent-skills/test_hook_launcher.py`, `tools/agent-skills/test_hooks.py` (class `WiringTest` only), and anything `sync.py` regenerates.

**Steps:**

1. Write `mode_banner.sh` exactly as in C3.
2. `.claude/settings.json`: insert the C2 banner entry as the **first** element of `"SessionStart"`, keeping 2-space JSON indentation. Replace the resume_card and shell_guard `command` values with C2 sites 1 and 2. Leave the board entry and `"agent"` unchanged. Afterwards check that the file parses: `Get-Content <abs> -Raw | ConvertFrom-Json`.
3. In the `orchestrator.md` frontmatter, replace the text between the single quotes on the `command:` line with C2 site 3.
4. `.codex/hooks.json`: set the PreToolUse `command` to C2 site 4.
5. In `test_hook_launcher.py`, add `BANNER = REPO / ".claude/hooks/mode_banner.sh"` and a class `ModeBannerTest` with the same skip as `PyLauncherTest`. Run the banner with `[SHELL, "-c", '. "$BANNER"']`, `input=<payload>` and env `PATH`, `BANNER` (as posix), plus `SYSTEMROOT`/`TEMP`/`TMP`. Tests:
   - `test_default_session_names_orchestrator_and_hands_on`: payload `{"session_id":"s","source":"startup"}`, PATH `dirname(sys.executable)`. Stdout is valid JSON; `systemMessage` contains `Mode: orchestrator` and `claude --agent hands-on`; `hookSpecificOutput.additionalContext` exists; `WARNING` does not appear.
   - `test_agent_flag_names_that_agent`: payload `{"agent_type":"hands-on"}`. `systemMessage` starts `Mode: hands-on`, and there is no `hookSpecificOutput`.
   - `test_unsafe_agent_type_falls_back_and_stays_valid_json`: payload `{"agent_type":"x;rm -rf"}`. The output parses as JSON and names orchestrator.
   - `test_warns_when_no_python`: PATH set to an empty temp dir. `systemMessage` contains `WARNING: no python3, python or py`.
6. In `test_hooks.py` `WiringTest`, add these two tests and keep the existing ones as they are:
   - `test_banner_is_the_first_session_start_entry`: entry 0 has no `matcher`, and its command contains `mode_banner.sh` and `|| exit 0`.
   - `test_every_python_hook_goes_through_the_launcher`: collect every command from settings.json SessionStart and PreToolUse, from the orchestrator frontmatter (`re.search(r"command: (.+)", front)`), and from `.codex/hooks.json`. Every command containing `.py` must contain `py.sh` and must not contain `; python `.

**Test type:** unit test (Python unittest). Removing the banner entry, or putting back a bare `python "$f"` anywhere, fails a WiringTest. Deleting the `hash` check fails `test_warns_when_no_python`.

**Checks (in order):**

```powershell
Set-Location <worktree>; python -m unittest discover -s tools/agent-skills -p "test_*.py"
Set-Location <worktree>; python tools/agent-skills/sync.py --check
```

Then do one live check and record it in your report: the shell_guard command from C2 site 2, run via `measure_hooks.ps1 -Runs 1 -Warmup 0` with a payload whose command is `cargo build`, must print `exit` 2.

**Docs owed:** none here; AH-13 and AH-14 document the banner and the launcher.

### AH-05: `harness_tables.py` generators

Agent: packet-coder. Branch `ai-harness-docs/ah-05-harness-tables`, worktree name `ah-05`. Size: about 35k working tokens. Commit subject: `feat(docs-gen): harness-hooks and harness-agents generators (AH-05)`.

**Rules for this packet:**

- Work only in your worktree, with absolute paths. Start every PowerShell call with `Set-Location <worktree>;`, and run every check in the foreground.
- PowerShell only. No `cargo`, no lane. Never run `git stash`, `git push`, `git worktree prune` or `gh pr`.
- **Do not run `regen.py` to rewrite files, and add no `<!-- gen:` markers to any doc.** AH-13 and AH-14 add the markers (D-AH15). Never hand-edit a generated block.
- Standard library only (`ast`, `json`, `pathlib`, `re`, `dataclasses`); CI has no PyYAML.
- Files stay under 700 lines; `harness_tables.py` should come in around 150.
- `tools/docs-gen/README.md` keeps its line endings; use the Edit tool.
- Commit once at the end.

**Files:** new `tools/docs-gen/harness_tables.py`, new `tools/docs-gen/test_harness_tables.py`, `tools/docs-gen/regen.py` (the registration and the docstring's generator list), and `tools/docs-gen/README.md` (the Generators table).

**Steps:**

1. Read `regen.py` lines 1-60 and 270-300, and `tools_catalog.py` `render()` (line 350), so you match the style.
2. Write `harness_tables.py` to the C5 API. Notes:
   - `json_hooks` reads `data.get("hooks", {})`. Each event maps to a list of entries; each entry has an optional `matcher` and a `hooks` list of `{type, command}`.
   - For `runs == "agent-board"` the reason comes from `INLINE_REASONS`. Otherwise it is `script_reason(root/".claude/hooks"/runs)`.
   - A command with no recognisable script gives `runs = "(inline)"` and reason `""`.
   - `agent_hooks` sets `where = f"agent: {name}"`, using the frontmatter `name`.
3. Register both generators in `regen.py` as in C5. Add two entries to the module docstring's generator list:
   - `harness-hooks       every hook in .claude/settings.json, agent frontmatter and .codex/hooks.json, with its reason, from harness_tables.py`
   - `harness-agents      every .claude/agents/*.md: model, effort, max turns and role, from harness_tables.py`
4. Add two rows to the README.md Generators table, in the same format as the `tools-catalog` row.
5. Write `test_harness_tables.py`, loading the module by path as `test_tools_catalog.py` does (read its first 30 lines). Tests:
   - `test_render_hooks_from_fixture`: a temp root with `.claude/settings.json` holding three entries: board inline, a no-matcher sh hook running `mode_banner.sh`, and a `Bash|PowerShell` hook whose command runs `shell_guard.py` through `py.sh`. Add `.claude/hooks/shell_guard.py` with docstring `"""Guard X.\n\nmore"""`, `.claude/hooks/mode_banner.sh` with first line `# Banner Y.`, and a `.codex/hooks.json` with one hook. The output has a row with `(all)` and `mode_banner.sh` and `Banner Y.`, a row with `Bash\|PowerShell` and `shell_guard.py` and `Guard X.`, and `.codex/hooks.json` in some row. `py.sh` never appears as `runs`.
   - `test_agent_frontmatter_hooks_are_collected`: an `.claude/agents/orc.md` with a frontmatter `hooks:` block shaped like orchestrator.md lines 7-12. It gives a row containing `agent: orc` and the matcher `Edit\|Write\|Read`.
   - `test_render_agents_sorted_and_escaped`: two agent files, `b.md` (model haiku, effort low, maxTurns 9, description `"Does b. More."`) and `a.md` (no model, description containing `x|y`). Rows appear in the order a, b; a shows `inherit` and `-`; `x\|y` is escaped; b's role is `Does b.`.
   - `test_every_repo_hook_states_its_reason`: on the real repo, `collect_hooks(REPO)` is non-empty and every hook has a non-empty `reason`. Every `.claude/hooks/*.py` and `*.sh` file has a non-empty `script_reason`.
   - `test_generators_registered`: import `regen` by path, and check that `"harness-hooks"` and `"harness-agents"` are in `GENERATORS`.

**Test type:** unit test (Python unittest). Removing the registration fails `test_generators_registered`. Dropping the docstring of any hook script fails the repo test, and that test is the drift guard (D-AH14).

**Checks (in order):**

```powershell
Set-Location <worktree>; python -m unittest discover -s tools/docs-gen -p "test_*.py"
Set-Location <worktree>; python tools/docs-gen/regen.py --check --skip-crate-graph
```

The second check may report the tools catalog as stale, because `harness_tables.py` is a new tool. That is expected; do not commit a regenerated block. It must not raise `unknown generator`.

**Docs owed:** the `tools/docs-gen/README.md` generator rows (step 4).

---

## Wave 2: agents, skill and workflow alignment

All W2 packets edit agent, skill or workflow text. The rules below are common to every W2 packet; each packet repeats the ones it needs.

### AH-06: orchestrator reminder and routing; CLAUDE.md mode block

Agent: packet-coder. Branch `ai-harness-docs/ah-06-orchestrator-mode`, worktree name `ah-06`. Size: about 15k working tokens. Commit subject: `docs(agents): orchestrator hands-on reminder, escalation routing and CLAUDE.md mode block (AH-06)`.

**Rules for this packet:**

- Work only in your worktree, with absolute paths. Start each PowerShell call with `Set-Location <worktree>;`, and run every check in the foreground.
- PowerShell only. No `cargo`. Never run `git stash`, `git push`, `git worktree prune` or `gh pr`.
- Use the Edit tool. `CLAUDE.md` and `.claude/agents/*.md` are CRLF in the working tree; keep them CRLF.
- **CLAUDE.md must stay at most 4,500 bytes**: `(Get-Item <abs>\CLAUDE.md).Length`.
- After editing an agent file, run `python tools/agent-skills/sync.py`, and commit what it regenerates (`.codex/agents/orchestrator.toml`).
- Never hand-edit a `<!-- gen: -->` block.
- Commit once at the end.

**Files:** `.claude/agents/orchestrator.md` (body only; AH-03 already changed the frontmatter), `CLAUDE.md`, and anything `sync.py` regenerates.

**Steps:**

1. In orchestrator.md `## Role`, replace bullet 1 with: `- Dispatch, decide, keep the ledger. You have no shell; Edit and Write are for \`docs/analysis/**\`, \`.claude/state/**\` and \`.claude/reports/**\`, in the main checkout or a campaign worktree under \`.claude/worktrees/<name>/\`.`
2. After the bullet `- Never read source, write code or docs, or run builds yourself.`, add: `- **Hands-on reminder, not a stop.** When the user asks for direct work you can't do (run a command, edit code, try something by hand), say once, in one line, that \`claude --agent hands-on\` starts a full-tool session. Then, in the same reply, dispatch the agent that fits the task and carry on. Never refuse a task by pointing at hands-on.`
3. In `## Routing table`, after the `packet-review-fixer` line, add the coder-loop and review-passes bullets from C6, worded as instructions to the orchestrator. After the `architect` line, add: `- \`hands-on\` (Opus, all tools): only when the user launches it; never dispatch it for packet work.`
4. In `## Waiting`, after bullet 2, add: `- A review-fixer return ends with \`verdicts=<packet>:<ship|recheck|decision>,...\`. Act on \`ship\` and \`recheck\` from the return line; open the report only for \`decision\`.`
5. In `CLAUDE.md`, insert after line 1 (the H1) and its blank line, before `A server emulator...`: `> **Session mode.** Claude Code starts here as the \`orchestrator\` persona (\`.claude/settings.json\`): it dispatches agents and has no shell. For direct work run \`claude --agent hands-on\`. How the AI workflow works: [README § How AI development works here](README.md#how-ai-development-works-here).`, then a blank line. Check the size (at most 4,500), and record it in your report.

**Test type:** none. This is a docs and prompt change with no runtime behaviour. The existing WiringTest still has to pass.

**Checks (in order):**

```powershell
Set-Location <worktree>; python -m unittest discover -s tools/agent-skills -p "test_*.py"
Set-Location <worktree>; python tools/agent-skills/sync.py --check
Set-Location <worktree>; (Get-Item "<worktree>\CLAUDE.md").Length
```

**Docs owed:** doc-update-map row 18 (CLAUDE.md under the cap); list it in the report.

### AH-07: packet-coder continuation and review-fixer verdicts

Agent: packet-coder. Branch `ai-harness-docs/ah-07-agent-loop`, worktree name `ah-07`. Size: about 15k working tokens. Commit subject: `docs(agents): coder continuation sessions and review-fixer ship/recheck/decision verdicts (AH-07)`.

**Rules for this packet:** the same as AH-06, except there is no CLAUDE.md work. Edit only the two agent files plus whatever sync regenerates (`.codex/agents/packet-coder.toml` and `.codex/agents/packet-review-fixer.toml`).

**Files:** `.claude/agents/packet-coder.md` and `.claude/agents/packet-review-fixer.md`.

**Steps:**

1. In packet-coder.md, replace line 23 (`- Your context limit is 95k tokens...`) with: `- Your context limit is 95k tokens. Read only the line ranges you need (use Grep to find them). Past about 90k, stop: commit only work whose checks pass (subject suffix \`(part N)\`), write \`docs/analysis/<campaign>/worknotes/<packet>.md\` in your worktree (what is done, what is left, the next step, the last check's \`status=\` line), and return \`blocked\` with the reason \`context\`. A fresh packet-coder continues from that worknote; a packet gets at most 3 coder sessions.`
2. Add after it: `- If the brief says "continuation N of 3", read the worknote first and start at its next step; never redo committed work. Review findings never come to you: the packet-review-fixer fixes them.`
3. In line 16, after `Never call \`cargo\` directly...`, add: `If PowerShell is unavailable, return \`blocked: no PowerShell (pwsh) on this host\`.`
4. In packet-review-fixer.md, replace line 29 with: `Report once, compactly: the findings (file:line, one line each, marked fixed or not fixed with the reason), each check's final \`status=\` line, your commit hash, and a verdict per packet: \`ship\` (nothing left, or only trivial fixes), \`recheck\` (you made a non-trivial fix that a fresh reviewer must check; a packet gets at most 3 review passes), or \`decision\` (a finding needs a design change or a call the spec does not make; say which). End your one-line return with \` verdicts=<packet>:<verdict>,...\`. No praise, no summary of the diff.`
5. Add a line after line 12: `If the brief says "pass 2" or "pass 3", review the earlier fixer's commit as well as the coder's, and read the earlier report and any ruling note in \`worknotes/<packet>.md\` first.`
6. Line 25: change `At most five fix rounds per check.` to `At most five fix rounds (lane reruns) per check; a fix round is not a review pass.` Line 27: append `Mark any packet you could not finish \`recheck\`.` Line 24: append `If PowerShell is unavailable, return \`blocked: no PowerShell (pwsh) on this host\`.`

**Checks:** the first two AH-06 checks.

**Docs owed:** none (AH-09 and AH-13 cover the docs).

### AH-08: campaign-packet skill cycle and escalation; ship-pr note

Agent: packet-coder. Branch `ai-harness-docs/ah-08-skill-ladder`, worktree name `ah-08`. Size: about 15k working tokens. Commit subject: `docs(skills): campaign-packet sessions, passes and escalation ladder (AH-08)`.

**Rules for this packet:** the same as AH-07. The skill files are wrapped at about 78 columns; match that. Run `python tools/agent-skills/sync.py` afterwards; it regenerates `.agents/skills/campaign-packet/` and `.agents/skills/ship-pr/`.

**Files:** `.claude/skills/campaign-packet/SKILL.md` and `.claude/skills/ship-pr/SKILL.md`.

**Steps:**

1. In SKILL.md §3, item 4 (`On a one-line return, update the ledger row and stop...`), append: `A \`blocked\` return with the reason \`context\` starts the coder loop in § Sessions, passes and escalation.`
2. In §4, item 1: after `...packet touches their area.`, add: `Its return gives a verdict per packet (\`ship\`, \`recheck\` or \`decision\`); see below.`
3. Insert a new section after §4's list and before `## 5. State on disk...`. Its heading is `### Sessions, passes and escalation`. Its body is the C6 table plus a 4-item numbered list:
   1. Context stop → a fresh packet-coder with "continuation N of 3" and the worknote. Findings never go back to a coder.
   2. After 3 coder sessions, or 2 sessions that each end at 5 failed fix rounds → one Sonnet implementer (`rust-gameserver-dev` for code and tools, `documentation-writer` for docs) with the worknote → then `architect` re-slices. Opus never implements a packet.
   3. `recheck` → a fresh review-fixer for that packet, naming the pass number and the earlier report. At most 3 passes.
   4. After pass 3 without `ship`, or on `decision` → `architect` (or the Opus advisor for protocol and RE) writes a ruling note in `worknotes/<packet>.md` and no code; a fresh review-fixer applies it. An owner call is BlockedDecision.

   End with a `Why:` line: `Haiku 5.5 bills 5x per request above a 100k prompt, so a fresh session is cheaper than a long one; oversteps where Opus wrote code cost $107.43 in the Haiku campaigns (docs/analysis/campaign-metrics/findings.md), so escalation climbs one tier at a time and Opus only rules.`
4. In ship-pr/SKILL.md §5 (line 92), after the first sentence, add: `For a campaign packet, the wave's \`packet-review-fixer\` pass is this review: it did not write the code and reviews adversarially, so don't run a second reviewer.`

**Checks:** the first two AH-06 checks.

**Docs owed:** none.

### AH-09: development-workflow worker lifetime, plus AGENTS.md parity

Agent: packet-coder. Branch `ai-harness-docs/ah-09-workflow-agents-md`, worktree name `ah-09`. Size: about 20k working tokens. Commit subject: `docs(agents): align worker lifetime and AGENTS.md with the campaign cycle (AH-09)`.

**Rules for this packet:**

- Work only in your worktree, with absolute paths. Start each PowerShell call with `Set-Location <worktree>;`, and run every check in the foreground.
- PowerShell only. No `cargo`. Never run `git stash`, `git push`, `git worktree prune` or `gh pr`.
- Use the Edit tool. `docs/**/*.md` and `AGENTS.md` are CRLF; keep them CRLF. Afterwards, check that `[IO.File]::ReadAllText(<abs>)` contains no lone LF: `-match "(?<!\r)\n"` must be `False`.
- Never hand-edit a `<!-- gen: -->` block.
- Edit only lines 112 and 128 of development-workflow.md, plus the AGENTS.md lines named below. AH-11 owns the platform lines of development-workflow.md (182 and onward).
- Commit once at the end.

**Files:** `docs/agents/development-workflow.md` and `AGENTS.md`.

**Steps:**

1. development-workflow.md line 112: replace the sentence `Don't send the implementing worker back for round two, and never keep an agent alive as a teammate with \`SendMessage\`.` with `Review findings never go back to the implementing worker, and never keep an agent alive as a teammate with \`SendMessage\`.`
2. Insert a new bullet after line 112: `- **Sessions, passes and escalation.** A packet gets at most 3 packet-coder sessions: one that stops on context leaves a worknote, and a fresh session continues from it. Next comes one Sonnet implementer, and then the architect re-slices; Opus never implements. The review-fixer returns \`ship\`, \`recheck\` or \`decision\` per packet, and a packet gets at most 3 review passes before the architect rules. The ladder and its reasons: [campaign-packet skill § Sessions, passes and escalation](../../.claude/skills/campaign-packet/SKILL.md#sessions-passes-and-escalation).`
3. Line 128 (`- **Hand off early.** At about 200 requests...`): replace it with `- **Hand off early.** A packet-coder hands off at about 90k tokens, its context limit. Other workers hand off at about 200 requests or about 250k tokens of context: the worker writes its worknote and stops, and the orchestrator starts a fresh one. Don't wait for an automatic compaction.`
4. AGENTS.md step 4 (line 34): replace it with `4. Review each wave with one \`packet-review-fixer\` run that covers every packet in the wave. It returns \`ship\`, \`recheck\` or \`decision\` per packet; a \`recheck\` gets a fresh review-fixer for that packet, up to 3 passes. A coder that stops on context gets a fresh coder that continues from its worknote, up to 3 sessions, then one Sonnet implementer. The full ladder is in the campaign-packet skill.`
5. AGENTS.md, at the end of the `## Start here` first paragraph (line 5): append `How the AI workflow works, for every harness: [README § How AI development works here](README.md#how-ai-development-works-here) and [docs/agents/ai-harness.md](docs/agents/ai-harness.md).`
6. AGENTS.md, in the "Codex does not enforce these" list (after line 43), add two bullets:
   - `- **No mode banner.** Claude Code prints the session's persona at start (\`.claude/hooks/mode_banner.sh\`). Codex has no default persona, so there is nothing to announce.`
   - `- **Hooks need Python.** The shell guard runs through \`.claude/hooks/py.sh\` in both harnesses. With no python3, python or py on PATH it warns once per call and does nothing.`
7. AGENTS.md line 42 (`- **No path guard.**`): change ``edit only `docs/analysis/**` and `.claude/state/**` `` to ``edit only `docs/analysis/**`, `.claude/state/**` and `.claude/reports/**` (in the main checkout or a campaign worktree)``.

**Checks:**

```powershell
Set-Location <worktree>; python -m unittest discover -s tools/agent-skills -p "test_*.py"
```

Then the CRLF check above, on both files.

**Docs owed:** doc-update-map row 18 (rule changes mirrored in AGENTS.md); list it.

---

Waves 3 and 4 are in [work-packets-w3.md](work-packets-w3.md) (split to keep each file under the 700-line cap).
