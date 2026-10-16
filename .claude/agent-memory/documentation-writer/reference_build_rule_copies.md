---
name: reference-build-rule-copies
description: Where build rules are duplicated beyond the CLAUDE.md map row; sweep all of them when the toolchain, lane or build flow changes
metadata:
  type: reference
---

The doc-update map (`docs/agents/doc-update-map.md`, moved out of CLAUDE.md on 2026-10-03) names CLAUDE.md, `.github/copilot-instructions.md`, `docs/building.md`, getting-started and troubleshooting for build changes. The 2026-09-26 build-overhaul docs pass found stale build advice in more places than that:

- `AGENTS.md` ("Start here" item 1 summarises the build rules)
- `CONTRIBUTING.md` (reading list item 3, the rules-and-gotchas trap list, and the line after the PR walkthrough)
- `.github/instructions/rust-services.instructions.md` ("Builds" section, read by review bots)
- `.claude/agents/social-systems-engineer.md` and `game-archaeology-specialist.md`, plus their mirrors under `.opencode/agents/` (as of 2026-10-03, 14 of the 16 `.opencode` bodies had already drifted from `.claude/agents/`, so "identical" no longer holds; check before syncing)
- `docs/readme.md` (the "start here" list and the troubleshooting and agents rows describe contents)
- `tools/README.md` (the tools index)
- Comments in `.cargo/config.toml` and the root `Cargo.toml` `[profile.dev.package."*"]` block
- `crates/README.md` quick-reference block (owned by the crate-split work on that pass)

Left alone on purpose: `docs/guides/autonomous-agent-kickoff.md` describes another contributor's Linux host and its own build discipline; `docs/analysis/**` worknotes and `.claude/agent-memory/**` are historical records.

A grep that finds the stragglers: `git grep -n -E 'WSL|47 GB|pkill -f|windows-gnu|CARGO_BUILD_JOBS=2|cargo \+[0-9]'` excluding `docs/analysis`, `.claude/agent-memory`, `docs/reverse-engineering` and `deprecated`.

Since LO-05 (2026-10-15) CLAUDE.md is a 4.5 KB core: the build commands live in the `lane-build` skill, the pre-PR checklist in `docs/agents/pre-pr-checks.md`, file organization in `docs/agents/rules-and-gotchas.md#file-organization`, and the agent board in `docs/agents/development-workflow.md#agent-board`. Referrers that pointed at CLAUDE.md sections were repointed then; `git grep -n -E 'CLAUDE\.md.{0,40}(Build rules|Pre-PR checklist|File organization)'` finds new stragglers. Rust doc comments that cite the CLAUDE.md 500/700 caps were left alone, because the caps line stayed.

The canonical rationale is `docs/architecture/build-system.md`; the how-to is the "Builds, worktrees and test databases" section of `docs/agents/development-workflow.md`.
