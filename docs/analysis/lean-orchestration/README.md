# Lean orchestration campaign

> Type: campaign ledger. Audience: the owner, the coordinator and the agents that run packets. Opened 2026-10-13. Packets: [work-packets.md](work-packets.md).

## Purpose

Cut the non-code cost of campaigns (coordination, packet writing, reviews and agent-to-coordinator messages) by an order of magnitude, while keeping code, test and doc quality the same. After this campaign, typing `claude` in the repo starts a **Sonnet orchestrator** that dispatches agents and decides, but does not research, code or write docs.

## Evidence

Source: `~/token-profile.sqlite` (the [token-profile](../../../tools/token-profile/README.md) database), Cimmeria sessions from 2026-10-10 20:00Z (the switch to Haiku coders) to 2026-10-12 10:48Z, priced with table 2026-10-10. Total **$2,043.61**. Cross-checked against [campaign-metrics findings](../campaign-metrics/findings.md) ("Haiku regime: Opus 75%, Sonnet 22%, Haiku 2%").

| Role | Model | Requests | USD | Share | Avg context |
|---|---|---:|---:|---:|---:|
| Main session (coordinator) | Opus | 8,052 | 811.31 | 39.7% | 354,545 |
| rust-gameserver-dev | Opus (pinned) | 8,198 | 611.97 | 29.9% | 179,375 |
| packet-reviewer | Sonnet | 3,175 | 104.67 | 5.1% | 71,387 |
| general-purpose | Sonnet | 1,112 | 74.46 | 3.6% | 144,383 |
| documentation-writer | Opus | 873 | 65.96 | 3.2% | 201,492 |
| packet-review-fixer | Sonnet | 1,122 | 37.04 | 1.8% | 92,472 |
| **packet-coder** | **Haiku** | **11,201** | **14.46** | **0.7%** | 51,304 |

Findings, each with the packet that answers it:

| # | Finding | Packet |
|---|---|---|
| F1 | The main session's cost is cache reads of a huge context: $567 of $811. 63% of its spend came from requests made above 300k context. It ran on the 1M window and auto-compacted only 3 times (at about 773k). | LO-03 |
| F2 | Most main-session requests were wake-ups, not human turns: 3,543 background completions, 1,302 teammate idle notifications and 524 teammate messages, against 1,854 human prompts. Each wake-up re-reads the whole context. | LO-02, LO-06 |
| F3 | The main session did hands-on work: 3,666 PowerShell, 941 Edit, 874 Read and 609 Grep calls, against 1,020 Agent calls. | LO-01, LO-02 |
| F4 | A fresh main session starts at about 73k tokens (p50 first-request context over 41 sessions), against about 15k for a packet-coder and 80k for rust-gameserver-dev. CLAUDE.md (19 KB) and the @-imported project memory index (18 KB) load into every session and every agent that does not set `omitClaudeMd`. | LO-01, LO-05 |
| F5 | rust-gameserver-dev is pinned to `model: opus` and was 30% of all spend. Most domain advisors and documentation-writer are also pinned to Opus. | LO-04 |
| F6 | In campaign-metrics, reviewers were 31% of the spend and 17 of 20 packets needed a fix round; the 13 small packets reviewed in one wave context were the cheapest per packet. | LO-06 |

### Codex review findings (amendment, 2026-10-15)

The owner had the campaign reviewed from Codex. Each finding was checked against the repo on 2026-10-15.

| # | Finding | Packet |
|---|---|---|
| F7 | Nothing tells a Codex session how to run a campaign. `"agent": "orchestrator"` in `.claude/settings.json` sets Claude Code's default persona only; Codex reads `AGENTS.md`, which has no campaign rule. | LO-09a |
| F8 | `AGENTS.md` "Start here" still tells every harness to read six sources before designing any change. LO-05 only swapped the memory-index read for a grep. LO-05's goal was task-specific reads. | LO-09a |
| F9 | `tools/agent-skills/sync.py` turns each agent into a `.codex/agents/<name>.toml` with name, description, `model_reasoning_effort` and instructions only. It drops `model`, `tools`, `hooks`, `omitClaudeMd` and `maxTurns`, but its docstring and `development-workflow.md` name only `model`, `tools`, `color` and `memory`. A Codex agent runs on the session's model with every tool, and no frontmatter hook (such as `orchestrator_guard.py`) runs. | LO-09b |
| F10 | `.codex/hooks.json` wires the board `SessionStart` hook and `shell_guard.py` (`Bash` matcher) only. There is no Codex resume card and no orchestrator path guard, and nobody has tested whether Codex can run either. | LO-09c, LO-09a |
| F11 | `work-packets.md` gave one fixed report path in the root checkout. A worker can only write inside its own worktree, so reports go to a path in that worktree, passed in the brief. The campaign-packet skill (LO-06) and `orchestrator.md` give the relative path `.claude/reports/<campaign>/<packet>-<role>.md` without saying it is relative to the worktree. | Ledger (done), LO-09b |
| F12 | LO-08 measures Claude sessions only. `token-profile` reads Claude Code transcripts, so it has no numbers for Codex. | LO-08 |

## Decisions

Owner interview, 2026-10-13.

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-LO1 | The default persona for everyone who launches Claude Code in the repo is the `orchestrator` agent, set by `"agent": "orchestrator"` in the committed `.claude/settings.json`. | Decided | The main thread takes the agent's system prompt, tools and model; CLAUDE.md still loads. |
| D-LO2 | The orchestrator runs on **Sonnet**. Design, packet slicing and hard calls go to an **Opus `architect` subagent**, which writes the ledger and packet specs to disk and returns a path. | Decided | Opus judgment where it pays; its context ends with each call instead of accumulating. |
| D-LO3 | The orchestrator auto-compacts at **200k** (`CLAUDE_CODE_AUTO_COMPACT_WINDOW`). After every compaction a `SessionStart` hook (matcher `compact`) injects a resume card from the active campaign's ledger. | Decided | A hook cannot run `/compact` or `/clear`, but it can re-inject state after one. 200k is about 3x below today's 354k average. |
| D-LO4 | The orchestrator has **no shell**. Worktree creation, ship, merge and retire go through a **Haiku `ops` agent** that returns one line. | Decided | F3. |
| D-LO5 | Every agent writes its full report to a file and **returns one line** (status, commit, report path). Agents are one-shot; no teammates and no SendMessage loops. | Decided | F2. Idle-notification payloads can't be shortened (Claude Code docs), so teammates are dropped. |
| D-LO6 | Tiers: rust-gameserver-dev, documentation-writer, testing-validation-engineer, server-authority-enforcer, database-persistence, network-security-auth and the combat, mission, movement, npc-ai and minigame advisors move to **Sonnet**. game-archaeology-specialist and bigworld-engine-advisor stay on or move to **Opus**. | Decided | F5. Binary and engine reasoning is where Opus pays. |
| D-LO7 | CLAUDE.md becomes a **lean core of about 4 KB**. The build-lane, pre-PR, testing and file-organization detail moves to the skills and docs that already hold it. The @-import of the project memory index is removed, and agents grep the memory folder on demand. | Decided | F4. |
| D-LO8 | Only **packet-coder** skips CLAUDE.md (`omitClaudeMd: true`). packet-reviewer and packet-review-fixer load it again, so they can check repo-rule violations. | Decided | Owner choice; the lean core keeps that cheap. Today both reviewers set `omitClaudeMd: true`, so LO-04 removes it. |
| D-LO9 | Review is **one Sonnet packet-review-fixer per wave**: one context reviews and fixes every packet in the wave. | Decided | F6. |
| D-LO10 | The change is rolled out **as a campaign**. Wave 1 is dispatched by the current (Opus) coordinator; from wave 2 on, the new orchestrator runs the campaign itself. | Decided | It dogfoods the process. |
| D-LO11 | Agent report files live under a **gitignored** `.claude/reports/<campaign>/<packet>-<role>.md`, not in the ledger. Anything worth keeping is copied to `worknotes/` by the orchestrator. | Decided | The reports are transient, and committing them would add noise to every PR. |
| D-LO12 | The orchestrator keeps Read, Grep, Glob, Edit and Write, but a PreToolUse hook limits its Edit and Write to `docs/analysis/**` and `.claude/state/**`, and its Read to files under 400 lines unless they are in a ledger. | Decided | It has to update ledger rows and the resume card itself; anything else goes to an agent. |
| D-LO13 | An escape-hatch persona, `claude --agent hands-on` (Opus, all tools, today's behaviour), for one-off direct work. | Decided | Not every session is a campaign. |
| D-LO14 | Success targets, measured on the first full campaign run under the new process: orchestrator first-request context ≤ 25k; orchestrator average context ≤ 120k; coordination (orchestrator + architect + reviewers) ≤ 40% of campaign spend, down from 97%; CI first-attempt rate and escaped defects no worse than the campaign-metrics baseline. | Decided | Lets LO-08 grade the change. |
| D-LO15 | Which Codex sessions act as the orchestrator? **(A)** Only sessions that run or resume a campaign; other Codex sessions work directly under `AGENTS.md`. **(B)** Every Codex session starts as the orchestrator, as Claude Code does. Architect recommends **A**: Codex has no committed default-persona setting and no path guard (F10), so under B every Codex session would rely on instructions alone, and one-off Codex work would have no `hands-on` escape hatch. | **BlockedDecision** (owner) | Blocks LO-09a. The packet gives the exact text for both answers. |
| D-LO16 | A worker's report goes to `<worktree>\.claude\reports\<campaign>\<packet>-<role>.md` inside its own worktree. The brief passes the absolute path. Nobody writes a report to a fixed root-checkout path. The orchestrator copies anything worth keeping to `worknotes/` before ops retires the worktree, because retiring deletes the report. | Decided (owner, from the Codex review, 2026-10-15) | Workers write only inside their worktree. This amends where D-LO11 puts reports; they stay gitignored. |
| D-LO17 | Should `sync.py` give each Codex agent a model that matches its Claude tier? Codex agents run on the session's model today (F9). | **BlockedDecision** (owner); no packet waits on it | Picking the OpenAI models is the owner's call. LO-09b documents the current limit. Revisit after the LO-08 Codex UAT. |

## Packets

| Packet | Title | Wave | Agent | Status |
|---|---|---|---|---|
| LO-01 | Orchestrator persona, settings and escape hatch | 1 | general-purpose (Sonnet) | Done (#1705) |
| LO-02 | Architect and ops agents, report protocol | 1 | general-purpose (Sonnet) | Done (#1705) |
| LO-03 | Context cap, resume-card hook and orchestrator path guard | 1 | packet-coder | Done (#1705) |
| LO-04 | Agent tiers and one-line report contract | 2 | packet-coder | Reviewed; e3842f9a8 (lo-04); ship waits on #1736, #1737 |
| LO-05 | Lean CLAUDE.md | 2 | documentation-writer | Reviewed; 0439b1c3b (lo-05); ship waits on #1736, #1737 |
| LO-06 | campaign-packet skill and development-workflow rewrite | 2 | documentation-writer | Reviewed; afab72e0e (lo-06); ship waits on #1736, #1737 |
| LO-07 | Coordination-overhead view in token-profile | 2 | packet-coder | Done (#1705) |
| LO-09a | AGENTS.md campaign entry rule and task-specific reads | 3 | documentation-writer | BlockedDecision (D-LO15); also waits for wave 2 to merge and the LO-09c report |
| LO-09b | Codex limits of sync.py, report path inside the worktree | 3 | documentation-writer | BlockedDependency (wave 2 merge; LO-09c report) |
| LO-09c | Codex hook probe and resume-card wiring | 3 | packet-coder | BlockedDependency (wave 2 merge, #1737) |
| LO-08 | Pilot run, measurement, Codex UAT and close-out | 4 | orchestrator + ops | BlockedDependency (wave 2, wave 3) |

Wave 1 is reviewed by one packet-review-fixer (D-LO9), merged, and then the owner relaunches `claude` to get the new orchestrator for wave 2.

Wave 3 is the Codex compatibility pass. It starts once wave 2 and the guard fix (#1737) are on `main`, because it edits files that LO-05 and LO-06 rewrite. Dispatch LO-09c first, on its own. When it returns, dispatch LO-09a (once D-LO15 is answered) and LO-09b together, because both repeat LO-09c's probe result. The three packets touch disjoint files, so ops can merge the branches into one PR without conflicts. One packet-review-fixer reviews all three. LO-08 (wave 4) starts after wave 3 merges.
