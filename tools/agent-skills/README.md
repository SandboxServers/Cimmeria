# Agent skills sync

| Entry point | What it does |
|---|---|
| `sync.py` | Mirror the repo's agent skills from `.claude/skills/` (the source) to `.agents/skills/` for Codex and Copilot, and generate a Codex agent `.codex/agents/<name>.toml` from every `.claude/agents/<name>.md` (field mapping in its docstring). Codex agents keep only the name, description, reasoning effort and instructions, so Claude model tiers, tool lists and agent hooks do not apply in Codex. `--check` exits 1 on drift or on a generated file whose source is gone |
| `codex_hook_probe.py` | Probe which `.codex/hooks.json` hooks this Codex build runs (SessionStart, context injection, PreToolUse, resume); `--deny` also tests whether a PreToolUse deny stops a write under `crates/`. Restores `.codex/hooks.json` when it finishes. Rerun it after a Codex upgrade. As of 2026-10-15 (codex-cli 0.162.0-alpha.17.2) no probe hook was seen to run, so the result is inconclusive |

```powershell
python tools/agent-skills/sync.py           # after editing a skill or an agent
python tools/agent-skills/sync.py --check   # what CI runs
python -m unittest discover -s tools/agent-skills -p "test_*.py"
```

`test_sync.py` tests the mirror, `test_codex_hook_probe.py` tests the probe offline, `test_shell_guard.py` tests the `PreToolUse` shell guard in `.claude/hooks/shell_guard.py`, `test_agent_refs.py` checks that the harness docs name only agents that exist, and `test_ledgers.py` checks that every campaign `RESUME.md` fits the 40-line resume card, and `test_platform_rule.py` checks that every agent with a shell picks it from the `Platform:` line. CI runs every `test_*.py` here in `agent-skills.yml`. See [development-workflow.md](../../docs/agents/development-workflow.md#agent-skills-and-the-shell-guard).
