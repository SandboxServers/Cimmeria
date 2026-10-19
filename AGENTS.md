# Repo Instructions

## Start here

These rules apply to every harness: Claude Code, Codex, Copilot and the rest. Read [CLAUDE.md](CLAUDE.md) first. It is the short core: repo invariants, the top project rules, the build-lane rule and the test and doc duty for every PR. It applies to every agent, not only Claude. After that, read only what your task needs. How the AI workflow works, for every harness: [README § How AI development works here](README.md#how-ai-development-works-here), its deeper reference [docs/agents/ai-sdlc.md](docs/agents/ai-sdlc.md), and [docs/agents/ai-harness.md](docs/agents/ai-harness.md).

| Before you... | Read |
|---|---|
| design any change | [docs/agents/rules-and-gotchas.md](docs/agents/rules-and-gotchas.md) (decisions already made, traps already hit), and grep `.claude/agent-memory/` for the topic |
| change a wire constant, method index or layout, or act on a ticket or draft chapter | [docs/agents/domain.md](docs/agents/domain.md). **A ticket or draft chapter is a claim, not evidence:** check it against `docs/protocol/` first |
| write or change a test | [TESTING.md](TESTING.md): pick the test type before you write the fix |
| write a script, parser, decoder, extractor or CLI | [tools/README.md](tools/README.md#tools-catalog), [crates/README.md](crates/README.md) and `crates/*/src/bin/`. Extend the closest tool rather than adding a sibling. Give a new tool a one-line summary (docstring, leading comment, `.SYNOPSIS` or `//!`) so the catalog lists it |
| build, test or run the pre-PR checks | the [`lane-build`](.claude/skills/lane-build/SKILL.md) skill and [docs/agents/pre-pr-checks.md](docs/agents/pre-pr-checks.md) |
| open a PR | the [`ship-pr`](.claude/skills/ship-pr/SKILL.md) skill and [docs/agents/doc-update-map.md](docs/agents/doc-update-map.md) |
| run agents or a campaign | [docs/agents/development-workflow.md](docs/agents/development-workflow.md) and [Running a campaign](#running-a-campaign) below |

`.claude/agent-memory/main-session/` is the shared project memory from earlier top-level sessions. Write what you learn there and commit it with your change, following the "Project memory" rules in development-workflow.md. The repo is public, so no IPs, credentials or account names. If your harness has no subagents, read an advisor's definition in `.claude/agents/` and its `.claude/agent-memory/<agent>/MEMORY.md` as briefing material.

Driving the game client (the Live Research Lab): set it up with [docs/guides/lab-contributor-setup.md](docs/guides/lab-contributor-setup.md), which has the Codex MCP recipe. Its [verified/unproven section](docs/guides/lab-contributor-setup.md#whats-verified-and-what-isnt-as-of-2026-10-11) says what has passed live and what hasn't; the Codex recipe is untested. **Ask the user, and wait for a yes, before you take the lab lease, start or restart the client, or change the server the lab points at** ([lab-uat skill](.claude/skills/lab-uat/SKILL.md)).

Ticket and triage conventions for agent skills: [docs/agents/issue-tracker.md](docs/agents/issue-tracker.md) and [docs/agents/triage-labels.md](docs/agents/triage-labels.md).

## Running a campaign

A campaign is a folder `docs/analysis/<campaign>/` that holds a ledger (`README.md`), `work-packets.md` and `RESUME.md`. The [campaign-packet skill](.claude/skills/campaign-packet/SKILL.md) has the full process; Codex reads its mirror in `.agents/skills/campaign-packet/`.

In Claude Code, `"agent": "orchestrator"` in `.claude/settings.json` makes the orchestrator the default persona. That setting belongs to Claude Code and does nothing in Codex. A Codex session takes the orchestrator role only when it runs or resumes a campaign. For any other task it works directly, under the rules in this file.

When you orchestrate a campaign, in any harness:

1. Before acting, read `docs/analysis/<campaign>/RESUME.md` and then the ledger `README.md`. Open a packet's section in `work-packets.md` only when you dispatch or review that packet.
2. Write the campaign's folder name to `.claude/state/active-campaign`.
3. Give each packet its own worktree (`pwsh tools/build-lane/mk-worktree.ps1 <campaign>/<packet>-<slug> <name>`; on Linux and WSL, `bash tools/build-lane/mk-worktree.sh` with the same arguments) and one implementing agent: the one the packet names. Codex agents are in `.codex/agents/`. The brief carries the worktree path, the path of the packet section, the report path `<worktree>\.claude\reports\<campaign>\<packet>-<role>.md` (role `coder`, `review-<N>`, `fix-<N>`, `ruling-<N>` or `ruling-check`) inside that worktree, and the one-line return `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
4. Review each packet with a fresh, read-only `packet-reviewer` as soon as its coder is done. It returns `ship`, `fix` or `decision`; on `fix`, a fresh `packet-coder` applies its numbered fix notes and a fresh reviewer checks again, up to 3 passes. The same loop runs once over each finished wave and once over the finished campaign. A coder that stops on context gets a fresh coder that continues from its worknote, up to 3 sessions, then one Sonnet implementer. The full ladder is in the campaign-packet skill.
5. Ship, merge and retire with `pwsh tools/build-lane/ship.ps1` and `pwsh tools/build-lane/rm-worktree.ps1` (on Linux and WSL, their `.sh` twins with the same arguments). Never run `git worktree prune`. Before a worktree is retired, copy anything worth keeping from its report to `docs/analysis/<campaign>/worknotes/`.
6. At every wave boundary, update the ledger rows and `RESUME.md`. Keep RESUME.md to 40 lines at most: the current wave, the packets in flight and their worktrees, the next action, and open decisions.
7. Every compiling `cargo` call goes through the build lane (`pwsh tools/build-lane/lane.ps1 cargo ...` on Windows, `bash tools/build-lane/lane.sh cargo ...` on Linux and WSL). Put that rule in every brief.

Codex does not enforce these, as of 2026-10-15, so do them yourself:

- **No resume card.** Claude Code's SessionStart hook shows RESUME.md again after a compaction; Codex does not. After a compaction or codex resume, read RESUME.md again before acting.
- **No path guard.** While you orchestrate, edit only `docs/analysis/**`, `.claude/state/**` and `.claude/reports/**` (in the main checkout or a campaign worktree), and hand everything else to an agent. In Claude Code, a hook enforces this for the orchestrator.
- **No model tiers or tool lists.** A Codex agent runs on the session's model with every tool. The `model` and `tools` lines in `.claude/agents/*.md` apply in Claude Code only.
- **No mode banner.** Claude Code prints the session's persona at start (`.claude/hooks/mode_banner.sh`). Codex has no default persona, so there is nothing to announce.
- **Hooks need Python.** The shell guard runs through `.claude/hooks/py.sh` in both harnesses. With no python3, python or py on PATH it warns once per call and does nothing.

## Frontend UAT Rule

For every new frontend feature, milestone, or meaningful behavior change:

- perform a JS REPL-style logic UAT in addition to normal tests/builds
- verify the underlying state/persistence behavior, not just compile success
- summarize what was exercised and what passed
- explicitly call out what was **not** covered by the REPL pass

This applies especially to:

- card create/update/delete flows
- sequence/thread edits
- card movement and layout state
- input/output port changes
- persistence and dirty-state transitions
- validation-state changes
- content-engine serialization/deserialization changes

## Notes

- REPL-style logic UAT supplements browser/manual UAT; it does not replace visual verification.
- If a feature cannot be meaningfully exercised in the JS REPL, state that clearly and explain why.
- In user-facing agent updates, final reports, issue comments, and PR bodies, reference files from the project root (for example, `docs/guides/...`). Do not expose absolute host filesystem paths unless the user explicitly asks for one.

## PR process — tests and docs

Every PR that changes runtime behaviour must add or update a test, and every PR that changes user-visible behaviour, public surface, file layout, build steps, or test policy must update the corresponding documentation in the same PR.

- **Tests**: read [TESTING.md](TESTING.md) before authoring. It covers the twelve test types (unit / wire-format / live-DB / smoke / concurrency / chain-replay / legacy reference / fan-out byte / Mercury session / network chaos / wire-level replay / negative-log), the picker for which type fits which bug shape, and the review gotchas mined from PRs #131 onwards. A PR that adds or removes ≥5% of the workspace test count (the threshold is in the generated totals of [docs/testing/inventory/README.md](docs/testing/inventory/README.md#workspace-totals)) updates the catalogue at [docs/testing/inventory/](docs/testing/inventory/) in the same PR; smaller drifts get folded in by periodic sweeps.
- **Docs**: see the "what changed → what to update" map in [docs/agents/doc-update-map.md](docs/agents/doc-update-map.md). For non-trivial doc work, use the **Documentation Writer** agent — Diátaxis-aware (tutorials / how-to / reference / explanation) and keeps voice consistent with the rest of `docs/`. Index entries in `docs/readme.md` and per-section README files stay in sync with the documents they list.
- **Generated blocks and status docs**: never hand-edit text between `<!-- gen:NAME -->` markers or the crate graph. `tools/docs-gen/regen.py` owns them and a workflow reruns it on `main` every six hours; commit its output only when your PR adds the marker. `docs/gap-analysis.md` (with its area files in `docs/gap-analysis/`) and `docs/project-status.md` are updated once per campaign, in its close-out or release packet; per-packet progress goes in the campaign's ledger under `docs/analysis/<campaign>/`.
