## Summary

Rewrites the README section "How AI development works here" so that it explains this repo's AI-driven development cycle to someone with no prior knowledge of it, and adds [How we build with AI](docs/agents/ai-sdlc.md) as the deeper reference behind it (ai-harness-docs AH-23a and AH-23b, D-AH50).

The owner decided on 2026-10-18 that the README may be as long as it needs, so the explanation lives where a newcomer looks first. ai-sdlc.md holds the detail. The README and ai-sdlc.md both state some headline figures (the review and fix costs, the guard evidence, the cost tables), so a figure has two homes and can drift; whether to dedupe is the owner's call, and the AH-24 campaign review checks for drift.

The README section covers:

- the short version, then campaigns, packets and waves, and why the work is cut that way;
- who does what: orchestrator, architect, ops, packet-coder, packet-reviewer and the domain agents, with their models;
- the three review loops (packet, wave, campaign), with a Mermaid diagram, and what happens after the third pass: the architect decides only what to fix and how, and writes no code or fix;
- the guards and what they save (the generated hook table stays in place, unchanged);
- shipping, when a human decides, and what it costs.

ai-sdlc.md covers the campaign files, what a packet contains, agents, tools and models, worktrees and databases, reports and returns, context limits, the review loop in detail, the guards in detail, shipping in detail, lab and UAT, where things are written down, the honest numbers with their dates, and the platform status.

Also:

- links to ai-sdlc.md from docs/readme.md, AGENTS.md, development-workflow.md, ai-harness.md and the doc-update map;
- a rules-and-gotchas bullet: no hook refuses raw `git`, and why it is still kept safe.

## Numbers and their sources

Every number in the new text comes from one of these sources:

- `docs/analysis/ai-harness-docs/hook-timing.md` (hook latency);
- `docs/analysis/campaign-metrics/findings.md` (PR #1665: campaign costs, the review and fix split, the Opus oversteps);
- the token-profile ledgers (the coordinator's context, refused-call counts, output sizes).

A saving that nobody has measured is written as "not measured". The guard savings per refused call are an example.

## Tests

- `python -m unittest discover -s tools/agent-skills -p "test_*.py"`: OK. `test_agent_refs.py` now scans the new doc and the new README text.
- Links: every relative link in the README section and in ai-sdlc.md resolves, checked in the worktree.
- `regen.py --check --skip-crate-graph`: README.md and docs/readme.md are current. The `harness-hooks` block is unchanged.

**Not tested:**

- Whether a newcomer understands it. That is AH-15 step W8, the owner's cold read.
- The Mermaid rendering. GitHub renders it; nothing checks it locally. AH-15 step W8 looks at it.

## Docs (doc-update-map rows)

- **AI-harness configuration:** a new doc in docs/agents with its row in docs/readme.md, and a project rule in rules-and-gotchas.md.
- **AI-harness wiring:** the README section and the map row's own list of files to update together.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01WCmH1WPLJdizxaciRv4aJM
