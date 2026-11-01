# Agent skills sync

| Entry point | What it does |
|---|---|
| `sync.py` | Mirror the repo's agent skills from `.claude/skills/` (the source) to `.agents/skills/` for Codex and Copilot, and generate a Codex agent `.codex/agents/<name>.toml` from every `.claude/agents/<name>.md` (field mapping in its docstring). Codex agents keep only the name, description, reasoning effort and instructions, so Claude model tiers, tool lists and agent hooks do not apply in Codex. `--check` exits 1 on drift or on a generated file whose source is gone |
| `codex_hook_probe.py` | Probe which `.codex/hooks.json` hooks this Codex build runs (SessionStart, context injection, PreToolUse, resume); `--deny` also tests whether a PreToolUse deny stops a write under `crates/`. Restores `.codex/hooks.json` when it finishes. Rerun it after a Codex upgrade. As of 2026-10-15 (codex-cli 0.162.0-alpha.17.2) no probe hook was seen to run, so the result is inconclusive |
| `live_guard_probe.py` | Live guard probe (C9): without `--live` it prints the plan and starts nothing. With `--live` it runs the `orchestrator-edit`, `hands-on-cargo` and `subagent-stop` legs (and `codex` with `--codex`) through `claude -p` in stream-json mode, judges each leg by its guard marker and writes `.claude/reports/harness-tuning/live-guard-<date>.json` in the worktree. Refuses the main checkout (exit 2). `fake_claude.py` is its test fixture, never a live run. Every live session is Haiku, so only line mode is checked |
| `memory_lint.py` | Lint `.claude/agent-memory/` for untyped, stale-state, unindexed and orphan memories and IP-like text. Observation only: `--strict` exits 1, `--json` prints per-folder counts, `--since` adds the state memories added since a date. |

```powershell
python tools/agent-skills/sync.py           # after editing a skill or an agent
python tools/agent-skills/sync.py --check   # what CI runs
python tools/agent-skills/memory_lint.py       # lint .claude/agent-memory (observation; --strict, --json, --since)
python -m unittest discover -s tools/agent-skills -p "test_*.py"
```

Tests (CI runs every `test_*.py` here in `agent-skills.yml`):

- `test_sync.py` tests the mirror.
- `test_codex_hook_probe.py` tests the probe offline.
- `test_live_guard_probe.py` tests the live guard probe offline, against `fake_claude.py`.
- `test_shell_guard.py` tests the `PreToolUse` shell guard in `.claude/hooks/shell_guard.py`.
- `test_report_contract.py` tests the `SubagentStop` hook `.claude/hooks/report_contract.py`.
- `test_coder_context_note.py` tests the PostToolUse hook `.claude/hooks/coder_context_note.py`.
- `test_coder_contract.py` tests the packet-coder contract (no report, one return line).
- `test_report_formats.py` tests the verdict words, the reviewer's report sections and the return-line formats.
- `test_agent_refs.py` checks that the harness docs name only agents that exist.
- `test_ledgers.py` checks that every campaign `RESUME.md` fits the 40-line resume card.
- `test_platform_rule.py` checks that every agent with a shell picks it from the `Platform:` line.
- `test_memory_lint.py` tests the memory lint `memory_lint.py` (C8: untyped, stale-state, unindexed and orphan memories).
- `test_agent_memory_prompt.py` tests the agent memory stub and the memory rule line that the agents carry.

See [development-workflow.md](../../docs/agents/development-workflow.md#agent-skills-and-the-shell-guard).
