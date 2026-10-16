# Lean orchestration campaign

> Type: campaign ledger. Audience: the owner, the coordinator and the agents that run packets. Opened 2026-10-13. **Closed 2026-10-16 with owed items** ([Close-out](#close-out)). Packets: [work-packets.md](work-packets.md).

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
| D-LO14 | Success targets, measured on the first full campaign run under the new process: orchestrator first-request context ≤ 25k (**relaxed to about 35k by the owner, 2026-10-16**); orchestrator average context ≤ 120k; coordination (orchestrator + architect + reviewers) ≤ 40% of campaign spend, down from 97%; CI first-attempt rate and escaped defects no worse than the campaign-metrics baseline. | Decided; first-request target relaxed 2026-10-16 | Lets LO-08 grade the change. |
| D-LO15 | Which Codex sessions act as the orchestrator? **(A)** Only sessions that run or resume a campaign; other Codex sessions work directly under `AGENTS.md`. **(B)** Every Codex session starts as the orchestrator, as Claude Code does. Architect recommends **A**: Codex has no committed default-persona setting and no path guard (F10), so under B every Codex session would rely on instructions alone, and one-off Codex work would have no `hands-on` escape hatch. | **Decided 2026-10-15: A** (campaign sessions only) | Applied by LO-09a (#1742). The packet gave the exact text for both answers. |
| D-LO16 | A worker's report goes to `<worktree>\.claude\reports\<campaign>\<packet>-<role>.md` inside its own worktree. The brief passes the absolute path. Nobody writes a report to a fixed root-checkout path. The orchestrator copies anything worth keeping to `worknotes/` before ops retires the worktree, because retiring deletes the report. | Decided (owner, from the Codex review, 2026-10-15) | Workers write only inside their worktree. This amends where D-LO11 puts reports; they stay gitignored. |
| D-LO17 | Should `sync.py` give each Codex agent a model that matches its Claude tier? Codex agents run on the session's model today (F9). | **Open** (owner, 2026-10-16: stays open); no packet waits on it | Picking the OpenAI models is the owner's call. LO-09b documents the current limit. Revisit after the LO-08 Codex UAT. |
| D-LO18 | Reset the main checkout's `main` to `origin/main`, after committing or stashing its uncommitted ledger and memory edits, then re-measure the first-request context (LO-10e). The 2 local-only commits are duplicate cherry-picks of the #1737 guard fix. | Decided (owner, 2026-10-16); the owner runs the reset, no packet touches git for it | The LO-08 pilot (48,349) launched 52 minutes before LO-05 merged, so it loaded the old CLAUDE.md and the memory-index import. The reset alone saves about 15.5k tokens (LO-10 architect estimate). |
| D-LO19 | Which sessions keep direct MCP access: move the lab, RE and SigNoz servers out of the main thread into inline `mcpServers` on the subagents that use them, and turn off the claude.ai Docs connector for this project (LO-10 rank 6). | **Deferred and still open** (owner, 2026-10-16); no wave 5 packet does this | Out of scope for now. About 2.4k tokens, but `hands-on`, the lab-uat grading step and ad hoc RE sessions call these servers from the main thread. Wave 5 reaches 25k without it. |

## Packets

| Packet | Title | Wave | Agent | Status |
|---|---|---|---|---|
| LO-01 | Orchestrator persona, settings and escape hatch | 1 | general-purpose (Sonnet) | Done (#1705) |
| LO-02 | Architect and ops agents, report protocol | 1 | general-purpose (Sonnet) | Done (#1705) |
| LO-03 | Context cap, resume-card hook and orchestrator path guard | 1 | packet-coder | Done (#1705) |
| LO-04 | Agent tiers and one-line report contract | 2 | packet-coder | Done (#1739) |
| LO-05 | Lean CLAUDE.md | 2 | documentation-writer | Done (#1739) |
| LO-06 | campaign-packet skill and development-workflow rewrite | 2 | documentation-writer | Done (#1739) |
| LO-07 | Coordination-overhead view in token-profile | 2 | packet-coder | Done (#1705) |
| LO-09a | AGENTS.md campaign entry rule and task-specific reads | 3 | documentation-writer | Done (#1742); D-LO15 = A |
| LO-09b | Codex limits of sync.py, report path inside the worktree | 3 | documentation-writer | Done (#1742) |
| LO-09c | Codex hook probe and resume-card wiring | 3 | packet-coder | Done (#1742); outcome inconclusive |
| LO-08 | Pilot run, measurement, Codex UAT and close-out | 4 | orchestrator + ops | Closed with owed items (Claude UAT, Codex UAT C0-C5 and a second pilot not run) |
| LO-10a | Probe: skills listing, agent listing, lab-driver line | 5 | packet-coder | Done (no PR) |
| LO-10b/c/d | Orchestrator tool set, agent descriptions, memory index dedupe | 5 | mixed | Done (#1753) |
| LO-10e | Re-measure and grade D-LO14 | 5 | documentation-writer | Done (#1754); owner-read number, not tool-measured |

Wave 1 is reviewed by one packet-review-fixer (D-LO9), merged, and then the owner relaunches `claude` to get the new orchestrator for wave 2.

Wave 3 is the Codex compatibility pass. It starts once wave 2 and the guard fix (#1737) are on `main`, because it edits files that LO-05 and LO-06 rewrite. Dispatch LO-09c first, on its own. When it returns, dispatch LO-09a (once D-LO15 is answered) and LO-09b together, because both repeat LO-09c's probe result. The three packets touch disjoint files, so ops can merge the branches into one PR without conflicts. One packet-review-fixer reviews all three. LO-08 (wave 4) starts after wave 3 merges.

## LO-08 pilot results

Wave 2 is the first pilot (D-LO14). Measured 2026-10-16 with `python tools/token-profile/report coordination --db ~/token-profile-v6.sqlite --session ad5b8b9e-f562-4f8c-8286-1a6b8f544f2a --since 2026-10-16T02:44:00Z --until 2026-10-16T03:36:14Z`, priced with table 2026-10-12. The window runs from the orchestrator session's start to its first wave 3 dispatch (worktree `lo-09c`), so it covers LO-04, LO-05, LO-06, the guard fix (#1737), the wave 2 PR (#1739) and the retires. It also holds the architect's Codex amendment (02:55Z), which was unplanned design work. Claude sessions only (F12); Codex numbers go in their own table.

| D-LO14 target | Pilot (wave 2) | Result |
|---|---|---|
| Orchestrator first-request context ≤ 25k | 48,349 | **Fail** |
| Orchestrator average context ≤ 120k | 75,159 (58 requests, Sonnet) | Pass |
| Coordination ≤ 40% of spend (was 97%) | 46.6% ($3.99 of $8.56) | **Fail**; 28.8% without the architect's Codex amendment ($2.14) |
| CI first-attempt rate no worse than baseline | not measured by `token-profile` | Unverified |
| Escaped defects no worse than baseline | not measured by `token-profile` | Unverified |

Spend by role in the window: documentation-writer (Opus) $4.51, 52.7%; architect (Opus) $2.14, 25.0%; main thread (Sonnet) $1.06, 12.4%; packet-review-fixer (Sonnet) $0.79, 9.3%; packet-coder (Haiku) $0.05; ops (Haiku) $0.01. Total 255 requests. The largest cost is not coordination but the Opus documentation-writer, which D-LO14 does not count.

For comparison, the same session after the window (wave 3, 03:36:14Z to its end) cost $3.75; coordination there was main thread 18.9% plus packet-review-fixer 24.9%, and the main thread's average context rose to 122,503, over the 120k target. The pre-relaunch orchestrator session `144dabfa` (Opus, 15 human-prompt requests, no dispatches) had a first-request context of 74,143.

The second pilot, the next real campaign the owner picks, is still owed.

### Wave 5 re-measure

Wave 5 (LO-10b/c/d, PR #1753) cut the orchestrator's static context, and the owner reset the main checkout (D-LO18). LO-10e was to re-measure the first request of a fresh orchestrator session started from the main checkout (session `b424e6ee-45f1-49bf-8f24-e9b5a88b75f2`, first prompt "Resume work on lean orchestration campaign"). **The tool measurement was impossible.** No transcript named `b424e6ee-45f1-49bf-8f24-e9b5a88b75f2.jsonl` exists under any `~/.claude/projects/*/` directory (only a `subagents/` folder carries that id), and no transcript in the Cimmeria project slug contains the first prompt. `token-profile report coordination` also needs an ingested `--db`, so it had nothing to read. Instead the owner read the Claude window on a fresh session and reports a first-request context of **about 32-35k tokens** (owner-read from the Claude window, not tool-measured).

| Run | First-request tokens | Source |
|---|---|---|
| Pilot (wave 2 orchestrator, old CLAUDE.md and memory import) | 48,349 | `token-profile`, LO-08 above |
| LO-10a P0 (`orchestrator` persona before wave 5, `claude -p`, worktree) | 30,395 | first assistant `usage`: 2 + 30,393 cache-creation + 0 cache-read |
| LO-10a P3 (`lo10-candidate` persona, the wave 5 shape) | 21,387 | first assistant `usage`: 2 + 21,385 cache-creation + 0 cache-read |
| Wave 5 fresh session from the main checkout | ~32-35k | owner-read from the Claude window, not tool-measured |

The P0 and P3 figures were recovered from the six LO-10a probe transcripts in `~/.claude/projects/C--Users-Steve-source-projects-Cimmeria--claude-worktrees-lo-10a/`, because the LO-10a report went away with its worktree. They map to the run table by file time and prompt (P1 22,585; P1b 22,601; P2 26,273; P2b 26,318). Treat them as a reconstruction. Per LO-10a's own rule, compare probe runs only with each other, never with the pilot: the probes run in a worktree without the user's auto-memory index and with a one-word prompt.

Grade: **pass against the relaxed target** (originally: target missed, improved). On 2026-10-16 the owner accepted ~35k first-request context as good enough, so the D-LO14 first-request target of 25k is relaxed to ~35k. The original 25k target is kept here for the record. The owner-read ~32-35k is above that original 25k, but it meets the relaxed ~35k target. It is roughly 13-16k below the wave 2 baseline of 48,349 (48,349 - 35,000 = 13,349; 48,349 - 32,000 = 16,349), about a 28-34% cut. The number is an owner read of the Claude window, not a `usage` field, so treat it as approximate (the window figure is rounded and may count items a transcript `usage` would split differently). It also includes the user's auto-memory index and the real first prompt, which the LO-10a probes lacked; that is why it sits well above the P3 probe's 21,387 and why probes are never compared with it. Method and why: the transcript for the session was never found, so no tool could measure it, and a window read is the only observation of the real session available. A tool-measured rerun would still be the better number: the owner launches a fresh session from the main checkout, sends one short prompt and gives the session id (check that its `.jsonl` lands under `~/.claude/projects/c--Users-Steve-source-projects-Cimmeria/`). The remaining architect ranks are 3 (`disableBundledSkills`, owner decision, now unblocked by this number), 6 (deferred as D-LO19, owner decision) and 7 (the user's personal auto-memory, user decision); none is started.

## Close-out

Closed 2026-10-16 by owner decision. Waves 1 to 5 are merged (#1705, #1739, #1742, #1753, #1754). Several checks were never run. This section says plainly which.

### Owner decisions, 2026-10-16

| Topic | Decision |
|---|---|
| First-request context | About 32-35k (owner-read from the Claude window, see [Wave 5 re-measure](#wave-5-re-measure)) is accepted. D-LO14's first-request target is relaxed from 25k to about 35k. |
| Coordination share | The pilot's 46.6% against the 40% target is accepted for now. It is likely inflated because the owner pasted a large Codex text block into the session instead of having an agent read it from a file. Revisit if it becomes a problem. |
| Claude UAT and Codex UAT C0-C5 | Not run: the owner is out of tokens. They are owed, not failed. |
| D-LO17, D-LO19 | Stay open. |
| Stale worktrees | The owner still owes `pwsh tools\build-lane\rm-worktree.ps1 <name> --force` for the stale no-PR worktrees (the sweep skipped 120; lo-10a, lo-10c and lo-10d, among others, were NO_PR). |

### D-LO14 final grade

| Target | Result | Status |
|---|---|---|
| Orchestrator first-request context | 48,349 (pilot), then about 32-35k (owner-read) against the relaxed ~35k | Pass against the relaxed target; the original 25k was missed |
| Orchestrator average context ≤ 120k | 75,159 (pilot) | Pass |
| Coordination ≤ 40% of spend | 46.6% | Missed, accepted for now |
| CI first-attempt rate, escaped defects | not measured by `token-profile` | Unverified |

### Owed and unverified

Nothing in this table has been tested. Do not treat any of it as working.

| Item | State |
|---|---|
| Claude UAT (a fresh `claude` launch shows the `orchestrator` persona, the guard denies a crate edit, `/compact` shows the resume card) | **Not run** |
| Frontmatter `hooks:` fire for the main-thread agent | **Unverified.** The guard is proven for subagents only (the LO-06 rerun); nobody has tried an Edit on a crate path from the main thread |
| Codex UAT C0-C5 ([work-packets.md](work-packets.md#codex-uat-fresh-session-recorded-separately)) | **Not run.** There is no Codex UAT table because there are no results |
| Codex hook enforcement | **Unverified.** LO-09c was inconclusive: no SessionStart or PreToolUse hook fired under Codex 0.162.0. The docs claim no Codex resume card and no Codex path guard |
| Tool-measured first-request number | Missing; the ~32-35k is a window read |
| Second pilot (the next real campaign) | Not run |
| D-LO17 (Codex models matching tiers), D-LO19 (MCP servers out of the main thread) | Open decisions |
| `rm-worktree.ps1 <name> --force` for stale no-PR worktrees | Owner action |
| Architect ranks 3 (`disableBundledSkills`) and 7 | Not started; owner decisions |

`docs/project-status.md` and the gap-analysis files are unchanged: this campaign alters agent tooling, not a game system, so no row there moves.
