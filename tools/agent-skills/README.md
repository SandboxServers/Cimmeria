# Agent skills sync

| Entry point | What it does |
|---|---|
| `sync.py` | Mirror the repo's agent skills from `.claude/skills/` (the source) to `.agents/skills/` for Codex and Copilot, and generate a Codex agent `.codex/agents/<name>.toml` from every `.claude/agents/<name>.md` (field mapping in its docstring); `--check` exits 1 on drift or on a generated file whose source is gone |

```powershell
python tools/agent-skills/sync.py           # after editing a skill or an agent
python tools/agent-skills/sync.py --check   # what CI runs
python -m unittest discover -s tools/agent-skills -p "test_*.py"
```

`test_sync.py` tests the mirror and `test_shell_guard.py` tests the `PreToolUse` shell guard in `.claude/hooks/shell_guard.py`. CI runs both in `agent-skills.yml`. See [development-workflow.md](../../docs/agents/development-workflow.md#agent-skills-and-the-shell-guard).
