---
name: reference-orchestrator-guard-scope
description: Hooks declared in an agent's frontmatter also fire inside subagents it dispatches; orchestrator_guard.py now allows calls whose payload carries agent_id or agent_type
metadata:
  type: reference
---

Seen 2026-10-15 while writing LO-05 as a documentation-writer subagent dispatched by the orchestrator session: every `Write` was refused by `.claude/hooks/orchestrator_guard.py`. The hook is wired in the `hooks:` frontmatter of `.claude/agents/orchestrator.md`, and with `"agent": "orchestrator"` as the session persona it also fired for the subagent's tool calls.

**Fixed** in commit 6027cf892 (`fix(hooks): let subagents past the orchestrator path guard`): the guard now allows any call whose payload carries `agent_id` or `agent_type`, plus Edit and Write under `.claude/reports/**`.

**How to apply:** a frontmatter hook on a persona agent is a session-wide hook, not a hook on that agent alone. If a guard refuses a subagent's call on an older checkout, write through a script and report it; do not widen the guard from a packet.