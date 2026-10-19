## Summary

Retires `packet-review-fixer`. Campaign review is now a loop: a fresh, read-only `packet-reviewer` writes numbered fix notes, a fresh `packet-coder` applies them, and a fresh reviewer checks again, with at most 3 passes. The loop runs once per packet, once per wave and once per campaign (ai-harness-docs D-AH42 to D-AH47, packets AH-17 to AH-22).

> **Owner callout: this PR edits the default persona** (`.claude/agents/orchestrator.md`, the one `.claude/settings.json` starts every session as). It now dispatches the review loops in place of the deleted agent. **A running Claude Code session keeps the old persona until it is relaunched.** Relaunch every session after this merges; one still running will try to dispatch an agent that no longer exists.

## What changed

- **AH-17:** `packet-reviewer` reviews one packet, a wave or the campaign, and writes `## Verdict`, `## Fix notes` and `## Not fixed`. Its return ends with `verdict=<ship|fix|decision>`. `packet-coder` gains the "review fix, pass N of 3" mode. `architect` sizes waves for one reviewer. `packet-review-fixer.md` and its Codex toml are **deleted, not deprecated**: a deprecated agent stays in `/agents` and in every dispatch menu, and a cheap model picks it.
- **AH-18:** campaign-packet skill § 4 is the three loops, with the pass, session and fix-round limits. The ship-pr note on campaign reviews points to the loop.
- **AH-19:** the orchestrator persona and AGENTS.md step 4 dispatch the loops.
- **AH-20:** development-workflow.md (roster, worker lifetime, the guard's `.claude/reports` path) and rules-and-gotchas.md.
- **AH-21:** ai-harness.md § Cycle and escalation, and the README summary lines.
- **AH-22:** `tools/agent-skills/test_agent_refs.py` fails when a harness doc names a `packet-*` agent with no agent file, links to a missing one, or mentions the retired agent outside a generated block. The `agent-skills` workflow now also runs on changes to README.md, AGENTS.md, CLAUDE.md and docs/agents/. The generated agent table in ai-harness.md is regenerated.

The old names stay in history files only: `tools/token-profile` (its config, tests and the coordination report) still knows `packet-review-fixer`, so past campaigns keep their cost split (D-AH42).

## Why

Measured in campaign-metrics' own 20 packets (`docs/analysis/campaign-metrics/findings.md`): read-only Sonnet reviews cost $13.58 and the Haiku fix rounds $0.72, and 17 of the 20 packets needed at least one fix round. The fixer did both jobs on Sonnet, so every fix ran at the Sonnet price, and a recheck reviewed fixes the same agent type had written. Reviewing only at the wave boundary also found a packet's problems after the whole wave had finished. Per-packet loops find a packet's problems while its worktree is fresh; the wave and campaign loops only look for what spans packets.

## Tests

- New: `test_agent_refs.py` (4 tests). Revert proof run in the worktree: with `.claude/agents/architect.md` restored to its `origin/main` text, which names the retired agent, the test fails; with this PR's text, it passes.
- `python -m unittest discover -s tools/agent-skills -p "test_*.py"`: OK.
- `python tools/agent-skills/sync.py --check`: clean.
- `python tools/docs-gen/regen.py --check --skip-crate-graph`: README.md and ai-harness.md current.

**Not tested:** no campaign has run the new loop end to end yet; the first one is this campaign's W4b and W5 (AH-23, AH-24). The fix-pass cost per packet is not measured. Codex has not been run with the new tomls.

## Docs (doc-update-map rows)

- **AI-harness configuration:** the agent workflow and roster (development-workflow.md, rules-and-gotchas.md).
- **AI-harness wiring:** an agent's frontmatter and the campaign cycle (ai-harness.md prose and generated agent table, campaign-packet skill, development-workflow.md, AGENTS.md); the row itself now names `test_agent_refs.py`.

## After merge

1. Relaunch every Claude Code session (the persona changed).
2. Owner: retire the packet worktrees, which `--merged` skips because they merged into the integration branch, not into `main`: `pwsh tools/build-lane/rm-worktree.ps1 ah-17 ah-18 ah-19 ah-20 ah-21 --force`.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01WCmH1WPLJdizxaciRv4aJM
