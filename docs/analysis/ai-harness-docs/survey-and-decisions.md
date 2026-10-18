# ai-harness-docs: survey and owner decisions

Source: Explore survey of origin/main (2026-10-18) plus owner answers in the orchestrator session. Treat survey figures as findings to re-verify, not as ground truth.

## Goal

Contributors who use Claude Code "normally" in this repo get confused: the repo defaults to the orchestrator persona (no shell), hooks call bare `python`, and nothing at the top level explains the AI development process. Explain it for humans in the README, make a fresh Claude self-explain it, and fix the portability gaps.

## Owner decisions (binding)

1. **Mode banner.** Add a first SessionStart hook entry in `.claude/settings.json` that prints a one-line mode banner (e.g. "Mode: orchestrator (no shell). For direct work: `claude --agent hands-on`"). Also add the same short block to the top of CLAUDE.md (CLAUDE.md is capped near 4.5 KB; stay under it).
2. **Orchestrator prompt.** Add a reminder (not a stop point) that `hands-on` exists for direct interactive work. The orchestrator must still dispatch the appropriate agent for the task the user called for.
3. **Python hooks.** Make hooks work where only `python3` exists (launcher: try `python3`, `python`, `py`), but only if it adds no meaningful time per call. Measure before and after (baseline: `shell_guard.py` about 50-80 ms, bare Python startup about 44 ms). Today a missing interpreter exits 127, which does not block, so the guards silently turn off.
4. **Platform statement.** Fully supported today: Windows with PowerShell. WSL/Linux, and bash even on Windows, are "not validated, but not unsupported". nopjmp is NOT building a Linux pipeline (it was a joke); drop that from any text.
5. **rules-and-gotchas.md.** Add the rules and intentional decisions other contributors need (audit table below).
6. **README.md.** Add a top-level visible section covering the entire cycle: orchestrator -> architect -> packet writer -> packet coder (3x loop for the context limit) -> reviewer, up to 3 times, with allowed model escalation points for coder and reviewer if still not correct. Also explain every hook and why it exists, and fully detail every customization of the AI harnesses that controls how they operate here (settings, agents, skills, hooks, guards, board, memory). Use the numbers to prove why it works this way.
7. **Review bots.** Keep both Senpai and CodeRabbit.
8. **Credential extraction.** Aura's anecdote was about other project work. Out of scope.

## Findings

### Aura (aurablacklight) PRs
- #1796 merged: `.github/workflows/senpai-review.yml`, opt-in (label `senpai` or manual), `claude -p` with pinned sha512-checked CLI 2.1.302 (claude-code-action billing issue #1901), `--setting-sources user --strict-mcp-config`, restores `.claude/` and `CLAUDE.md` from base branch, Bash via allow-list. So Senpai does NOT run as the orchestrator persona; no agent pin is needed. No CodeRabbit mention in it.
- #1797 open: adds a "WSL and Linux hosts" section to `docs/agents/development-workflow.md`, a CLAUDE.md pointer, makes `.claude/agents/ops.md` return `blocked` when PowerShell is missing (and regenerates `.codex/agents/ops.toml`), and flags `docs/guides/autonomous-agent-kickoff.md` (`flock` plus shared `CARGO_TARGET_DIR` conflicts with the lane rule). It states WSL is NOT supported for interactive sessions: this CONFLICTS with decision 4. Raises three open questions: WSL support, a `.sh` path for `ops`, and scoping the orchestrator default to interactive sessions. Does not fix hooks. The campaign must reconcile with it (amend or land it first), and answer his questions.
- #1799 open: Senpai stops writing `@Name-kun`.

### Harness today
- `.claude/settings.json` (41 lines): `"agent": "orchestrator"`; `CLAUDE_CODE_AUTO_COMPACT_WINDOW=200000`; SessionStart runs `~/.agent-board/board hook session-start` (external tool, always exit 0); SessionStart matcher `compact` runs `python resume_card.py` (prints first 40 lines of the active campaign RESUME.md); PreToolUse `Bash|PowerShell` runs `python shell_guard.py --harness claude` (refuses direct compiling cargo outside the build lane; Claude only: refuses pure read/search commands; `# guard-ok` bypasses; born from 2026-10-03..10 telemetry: 326 direct cargo calls, about 10,000 grep/cat calls). No permissions rules. No settings.local.json.
- `orchestrator.md` frontmatter hook runs `python orchestrator_guard.py` on `Edit|Write|Read`: refuses writes outside `docs/analysis`, `.claude/state`, `.claude/reports`; refuses reads of 400+ lines unless `limit` <= 200; subagent calls exempt. Note: the guard also blocked the orchestrator from writing into a worktree's docs/analysis under `.claude/worktrees/` (path not under the main checkout's docs/analysis). That is why this file lives in the main checkout; copy it into the worktree. Consider whether the guard should allow a worktree's `docs/analysis/**` and `.claude/state/**`.
- Banner placement: new first SessionStart entry before line 8 of settings.json.
- Local checkout was 42 commits behind origin/main; local `orchestrator.md` differed. Work from the worktree.

### Agents (`.claude/agents`, 24 files)
orchestrator (sonnet, medium; Agent, AskUserQuestion, TaskStop, Read, Glob, Edit, Write); architect (opus, high); packet-coder (haiku, medium, maxTurns 90); packet-reviewer (sonnet, high, 60, read-only); packet-review-fixer (sonnet, high, 120); ops (haiku, low, 30); lab-driver (haiku, low, 50); hands-on (opus, all tools); documentation-writer and domain advisors (sonnet); bigworld-engine-advisor and game-archaeology-specialist (opus). There is NO packet-writer agent: the architect writes `work-packets.md`.

### Packet cycle docs vs owner's intended cycle
- Documented in `campaign-packet/SKILL.md`, `docs/agents/development-workflow.md:106-135`, `AGENTS.md` "Running a campaign", `orchestrator.md`.
- Repo says: "Don't send the implementing worker back for round two" (`development-workflow.md:112`); one review-fixer per wave (`SKILL.md:111-121`); packet-coder past about 90k stops and reports (`packet-coder.md:23`); at most five fix rounds per check (`packet-coder.md:17`, `packet-review-fixer.md:25`); a packet is about 40k working tokens (`SKILL.md:60`).
- NOT in the repo (memory only): at most 3 Haiku coder sessions before escalating to Sonnet; a fresh coder continues from the worknote. Memory conflicts with the docs on whether findings go back to the same coder. Owner's cycle (decision 6) must be documented and the conflicting docs, skill and agent files aligned. The architect should decide the precise reviewer-pass and escalation rules consistent with the owner's text: coder 3x loop, reviewer up to 3, escalation points for coder and reviewer, and record where "packet writer" maps (the architect's work-packets.md, or a distinct step).

### Workflows and CodeRabbit
Only `senpai-review.yml` runs Claude. `.coderabbit.yaml` (273 lines): profile assertive, auto_review incremental, `auto_pause_after_reviewed_commits: 5`, drafts off, reads CLAUDE.md/AGENTS.md as guidelines, `chat.auto_reply: true`. Owner says CodeRabbit currently hits a rate limit; keep both bots; the campaign should document what each does, when each runs, and how to cope with the rate limit.

### Docs map
README sections: Status, Download the launcher, Tests & CI, Quick Start, Architecture, Crate Dependency Graph, Project Structure, Tech Stack, Database, Documentation (364), Contributing (396), Security, License. Best slot: new `##` between Documentation and Contributing. `hands-on` is documented only at `development-workflow.md:21-26`. docs/**/*.md are CRLF. Generated blocks (`<!-- gen:NAME -->`) are never hand-edited. Status docs change once per campaign.

### Numbers (PR #1665 campaign-metrics; tools in `tools/token-profile/`; guide `docs/guides/token-profiling.md`)
- Cost per merged PR: old model mean $22.01, p50 $18.18 (248 PRs, $5,727.50); Haiku model mean $2.67, p50 $0.91 (185 PRs, $594.06). Per 1,000 changed lines: $7.44 vs $3.03 (2.5x cheaper). Cost per accepted change: $18.65 vs $2.35.
- Escalation rate 16.9% (44 of 261 packets: 14 context-exhausted, 30 unclassified); first-pass completion 4.8%; 0.60 retries per accepted packet. Spend by tier in the Haiku regime: Opus 75%, Sonnet 22%, Haiku 2%.
- Overstep waste $107.43 (Opus coding in main session $76.48; Opus implementing a Haiku packet $27.43).
- Sizing: nine of 34 Haiku sessions hit the 90k cap before the re-slice, zero of 36 after. 15-28k working tokens finishes first time; rule is about 40k, no M size. Haiku 5.5 bills 5x per request above a 100k prompt.
- Token ledger: median main-session request 349k context (p99 927k); about $11.2k at list price over three weeks; top 10% of PRs are 54% of spend; cache reads 0.05x on Opus 5.5; parked workers' cache rewrites 13.2% of spend.
- Memory-only figure (#1312): Haiku coders $0.25, Sonnet reviewers $3.59, Opus coordinator $23.27 (77%). Not in repo docs.
- Sources: `docs/analysis/campaign-metrics/findings.md` plus `data/`, `reports/`; `docs/analysis/token-usage/`. Cite them; do not copy raw data.

### Memory-promotion audit (rule | in repo? | where)
- No bash / Git Bash on Windows, use PowerShell | Partly | `build-system.md:143` says "PowerShell or Git Bash" (needs reconciling with decision 4: bash is unvalidated, not forbidden); skill line 100 says workers use PowerShell only. Not in CLAUDE.md.
- Haiku session cap of 3 then Sonnet; reviewer re-check | No | memory only.
- Adversarial review of every PR | No | memory only.
- No `--target-dir target/desktop` (fills C:) | No | memory only.
- Workers run lanes in the foreground and `Set-Location <wt>;` per call | Partial | foreground rule not in repo.
- Run agents as defined, no model override | Implicit | `orchestrator.md` only.
- Every Haiku commit gets a Sonnet review | Partly | wave review stated; "every commit" strictness not stated.
- Model-role rationale (Haiku codes, Sonnet reviews/fixes, Opus writes no code) | Mostly | agent frontmatter, roster, orchestrator routing; rationale missing.
- Never rehost the client seed RAR | Not checked | grep before promoting.
- Already in repo: ship script, never `git worktree prune`, lane-only cargo, retire worktrees, ask before lab, no DB migrations, check docs first, fix adjacent, commit agent memory, one worktree per agent, merge queue, don't wake idle peers.
- Project memory rules in the repo must contain no IPs, credentials or account names.

### Platform statements to reconcile
`CLAUDE.md:33` "Windows-native", `rules-and-gotchas.md:74`, `lane-build/SKILL.md:57` (WSL cross-compile retired, CI builds on Linux), `build-system.md:54,141-146` ("No WSL builds" section; lane.ps1 exists so Windows never needs bash), `development-workflow.md:184` (every shell script has a pwsh twin), `pre-pr-checks.md:130-143` (.sh lint tools labelled macOS/Linux/WSL), `ship.py:358` (needs pwsh on PATH, `SHIP_PWSH`), `lane-slots.ps1:79`, `setup.ps1:165` (Docker), `docs/guides/autonomous-agent-kickoff.md:8` (describes a WSL2 Ubuntu host: contradicts Windows-native, flagged by #1797). The wording across these must agree with decision 4.

## Open items for the architect to decide or flag
- How to land alongside Aura's open #1797 and #1799 (avoid conflicting edits; credit him).
- Whether `ops` needs a `.sh`/non-PowerShell path (his question): recommend a position.
- Whether to scope the orchestrator default to interactive sessions (his question), given the banner and reminder approach decided above.
- A fresh-clone smoke test (a Claude with no memory, in a clean checkout, asked to make a small change, on Windows and ideally on WSL) as the acceptance test for the campaign.
- Mirror the explanation in AGENTS.md for Codex parity.
- A guard against drift: a doc-update-map row for harness changes, and ideally a regen block for the hook and agent tables in the README (`tools/docs-gen/regen.py` owns generated blocks).
