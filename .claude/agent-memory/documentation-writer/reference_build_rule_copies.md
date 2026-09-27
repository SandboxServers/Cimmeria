---
name: reference-build-rule-copies
description: Where build rules are duplicated beyond the CLAUDE.md map row; sweep all of them when the toolchain, lane or build flow changes
metadata:
  type: reference
---

The CLAUDE.md doc-update map names CLAUDE.md, `.github/copilot-instructions.md`, `docs/building.md`, getting-started and troubleshooting for build changes. The 2026-09-26 build-overhaul docs pass found stale build advice in more places than that:

- `AGENTS.md` ("Start here" item 1 summarises the build rules)
- `CONTRIBUTING.md` (reading list item 3, the rules-and-gotchas trap list, and the line after the PR walkthrough)
- `.github/instructions/rust-services.instructions.md` ("Builds" section, read by review bots)
- `.claude/agents/social-systems-engineer.md` and `game-archaeology-specialist.md`, plus their mirrors under `.opencode/agents/` (keep both copies identical)
- `docs/readme.md` (the "start here" list and the troubleshooting and agents rows describe contents)
- `tools/README.md` (the tools index)
- Comments in `.cargo/config.toml` and the root `Cargo.toml` `[profile.dev.package."*"]` block
- `crates/README.md` quick-reference block (owned by the crate-split work on that pass)

Left alone on purpose: `docs/guides/autonomous-agent-kickoff.md` describes another contributor's Linux host and its own build discipline; `docs/analysis/**` worknotes and `.claude/agent-memory/**` are historical records.

A grep that finds the stragglers: `git grep -n -E 'WSL|47 GB|pkill -f|windows-gnu|CARGO_BUILD_JOBS=2|cargo \+[0-9]'` excluding `docs/analysis`, `.claude/agent-memory`, `docs/reverse-engineering` and `deprecated`.

The canonical rationale is `docs/architecture/build-system.md`; the how-to is the "Builds, worktrees and test databases" section of `docs/agents/development-workflow.md`.
