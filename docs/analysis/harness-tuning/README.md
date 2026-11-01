# harness-tuning: ledger

> **Type:** Reference · **Audience:** The orchestrator and packet workers of this campaign · **Updated:** 2026-10-31 (re-sliced for owner rulings D-HT3 and D-HT10)
> **Companions:** [Work packets, waves 1-2](work-packets.md), [waves 3-5](packets-wave3.md), [resume card](RESUME.md)

## Purpose

The owner asked us to adopt and adapt harness ideas, but only where they **net** improve our process or make the workflow more efficient. Nothing gets added for its own sake. This campaign therefore starts with measurement. Every change has to name a number it moves, and the close-out re-measures that number with a tool kept under `tools/token-profile/`.

The four problems the data shows, in order of cost:

1. **Report-writing agents drop the report contract.** Among the Sonnet and Opus agents that are meant to write reports, 5.5% of contract dispatches wrote none and 14.6% gave no usable one-line return. Each costs the orchestrator an investigation or a re-dispatch.
2. **Haiku coders stop on context too often, and their instructions don't match the cap.** 19.4% of packet-coder dispatches stop on context. The user-level cap (`~/.claude/hooks/haiku_context_cap.py`, Haiku agents only) denies every tool past 90k, as the owner intends (D-HT3), yet `packet-coder.md` still asks a stopped coder to commit and write a worknote. The owner's rule: Haiku coders write code, not reports. The campaign aligns the coder with the cap and works on the stop rate and the cost of a continuation.
3. **The resume card names the wrong campaign.** The card reads one global pointer, which every parallel orchestrator overwrites. 14 of 32 cards since #1705 (44%) were wrong.
4. **Project memory asserts stale state.** A spot check of 10 state claims found 5 false, and no memory file carries an invalidation condition.

Two cheap correctness fixes ride along: an explicit "not checked" section in review reports, and unique decision and packet ID prefixes across campaigns. Finally, the guards lean-orchestration still owes get a live proof.

## Findings

Window: subagents whose first timestamp falls on 2026-10-03..2026-10-31, read from this machine's Claude Code transcripts (`~/.claude/projects/<repo prefix>/<session>/subagents/agent-<id>.jsonl` and `.meta.json`). One subagent transcript is one dispatch, and its first user message is the brief. Usage is the maximum of each field per `message.id`, because streaming lines carry partial usage. "Fresh" tokens are input + cache creation + output. These are tokens, not dollars, and they mix tiers.

Two regular expressions define the report contract:

- **RP**, the brief names a report path: `\.claude[\\/]reports[\\/][^\\/\s]+[\\/][^\\/\s]+\.md`.
- **LINE**, a conforming return line: `^(\S+)\s+(done|blocked|failed)\s+(\S+)\s+(\S+\.md)(\s+verdict=\w+)?(\s+verdicts=\S+)?\s*$`. From HT-01 on, the report slot also accepts `-` (a Haiku coder's line, D-HT14).

A report counts as written when the transcript holds a Write or Edit to the path, or a shell write verb (`WriteAllText`, `Set-Content`, `Out-File`, `>`) that names it. The scratch mining scripts were throwaway. HT-01..HT-05 rebuild them as `metrics` subcommands, and HT-05's baseline is the number the close-out compares against.

| ID | Finding | Number (method above unless stated) | Packet |
|---|---|---|---|
| F1 | 1,692 of 4,191 subagents had a report contract: packet-coder 863, packet-reviewer 431, review-fixer 77 (agent retired, D-AH42), architect 65, rust-gameserver-dev 60, documentation-writer 50, ops 46. Total 3.06B tokens, 187M fresh | counts per `agentType` | HT-02 |
| F2 | Strict one-line returns: 1,109 (65.5%). Line first, then prose: 157. Line buried in prose: 169. **No usable line** (prose, short or empty): **257 (15.2%)**, at 474M total / 30.1M fresh tokens. By agent: packet-coder 92 (58 of them at a context stop), packet-reviewer 65, ops 44, architect 15, documentation-writer 10, general-purpose 10, others 21. Ops' 44 are a format artifact: ops returns its own `ops <status> ...` line and has no `## Reporting`; its briefs named a comment-body file that RP matched | LINE on each line of the last assistant text; per-agent split `s16_per_agent.py` over the same rows | HT-02, HT-06 |
| F3 | **No report write: 226 (13.4%)**, at 481M / 30.4M fresh. By agent: packet-coder 148 (130 at a context stop), ops 33, architect 20 (10 returned a `docs/analysis` path instead), packet-reviewer 8, documentation-writer 6, rust-gameserver-dev 2, npc-ai-spawn-advisor 2, others 7 | report-written rule above | HT-02, HT-06 |
| F3a | **Scoped to the agents meant to write reports** (a `## Reporting` section and a non-Haiku `model:`; 14 types seen: packet-reviewer, architect, rust-gameserver-dev, documentation-writer, testing-validation-engineer and nine advisors): **691 dispatches; no usable line 101 (14.6%, 13.3M fresh); no report write 38 (5.5%, 7.1M fresh)**. Haiku packet-coder outside context stops: **34 of 696 (4.9%) with no usable line**. This, not F2/F3, is HT-06's baseline | F2/F3 rows minus packet-coder, ops, lab-driver, the retired review-fixer and agents with no `## Reporting` (general-purpose, hands-on) | HT-05, HT-06 |
| F4 | packet-coder context stops: 167/863 (19.4%), at 332M total / 22.3M fresh / 6.3M output. 130 of them wrote no report, which is expected: the cap tells the coder to stop, and under D-HT3 a coder writes none. What costs is the stop and its continuation: 87 "continuation N of 3" dispatches (123M total, 6.9M fresh, mean about 79k fresh each) and 67 other re-dispatches (80M) | `blocked` + "context" in the return; continuation phrase in the brief | HT-03, HT-16 |
| F5 | 240 of 1,209 October packet-coders hit the **user-level** cap `~/.claude/hooks/haiku_context_cap.py`. It acts on Haiku subagents only and, past 90k, denies every tool and tells the agent to stop and reply. The first denied tool was PowerShell 118, Edit 51, Grep 32, Read 27, Write 12. The cap works as the owner intends (D-HT3). The mismatch is in `packet-coder.md` line 24, which asks for a commit and a worknote "past about 90k", after the cap has already denied both | denial text in tool results; transcripts with `agentType` packet-coder | HT-07 |
| F6 | What fills a capped coder's context, as a share of tool-result characters: Read 50.9%, Grep 26.4%, shell reads 10.4%, git 6.2%, Edit/Write 2.3%, all lane output about 2.1%. Medians per capped coder: first request 11.6k, last request 92.5k, **output 37.1k** (thinking + tool inputs; a subagent run is one user turn, so its thinking likely stays in context), 34 requests, 23.6k tool-result tokens (chars/4), 3 lane calls, 12 reads, 11 edits. So trimming lane output would not pay. Reads, Grep and output dominate | tool_use → tool_result join, categorised by tool and command | HT-03, HT-16 |
| F7 | Resume card: 32 firings after #1705. **14 named the wrong campaign (44%)**, 15 matched, 2 were mixed, 1 undetermined. Examples: session `f929802f` 4 wrong cards, `8cce7543` 3. None acted on a wrong card. Only 2 of 41 compactions read RESUME.md before dispatching. Truncation at 40 lines is not a problem. This architect session also got a wrong card (`launcher-cache-presync`) at its own compaction | the card's campaign compared with the campaign most named (`docs/analysis/<x>`) in the 40 Agent prompts before the compaction | HT-04, HT-08 |
| F8 | Project memory, from a census on 2026-10-31: 771 files. 67 have no frontmatter and 68 no `type` (69 top-level `type:`, 634 nested under `metadata:`). By type: project 459, reference 210, feedback 31, user 2. 629 carry a date and **0 carry an invalidation condition**. 17 folders (25 files) belong to no repo or user agent (`adoption-ui`, `launcher-*`, `updater-*`, `wine-computer-use` and others), so nothing ever loads them. Index sizes: main-session 34.3 KB / 208 lines, rust-gameserver-dev 19.2 KB, game-archaeology 18.9 KB | PowerShell census of `.claude/agent-memory/` | HT-12a, HT-13, HT-15 |
| F9 | Of 24 STATE-style memory files, 21 have no invalidation condition. A spot check of 10 claims against the code found **5 false**: social_arm_shadow, objective-state-not-persisted, npc-follow-state-gaps, region-trigger-containment-gap, training_points_currency NotEnoughMoney. The "verify memory first" boilerplate sits in 6 agent files. `development-workflow.md` § Project memory says `CLAUDE.md` imports the main-session index; it no longer does | manual read of each claim against `crates/` | HT-12a, HT-15 |
| F10 | Reviewers shipped about 6 times with checks they never ran, and no report says so. AH-24-review-2:25 notes that no test guards the verdict words or the return-line format | ledger and review-report reading | HT-09 |
| F11 | ID collisions across campaigns: `D-CA10` is defined in castle-rebuild README:60 and castle-continuation architect-report.md:53. The `CS-` packet prefix is used by class-start-v6 and castle-continuation. Bare `D1..D8` appear in black-market, castle-continuation uat and client-tls-oidc. `metrics packets` joins sessions to packets by ID, so a shared prefix misattributes | grep of the first table cells in `docs/analysis/*/*.md` | HT-10 |
| F12 | lean-orchestration (closed 2026-10-16) still owes: a Claude UAT (a guard deny on a crate edit, and `/compact` showing the card), proof that main-thread frontmatter hooks fire, and Codex enforcement (its probe found `crates_edit_blocked=false` on codex-cli 0.162) | lean-orchestration README lines 154-179 | HT-14, HT-18 |
| F13 | Whole-file `docs/` reads over 40 KB: 183 calls, about 2.2M tokens (1.2% of Read bytes). Only 15 coders made one | Read results over 40 KB under `docs/` | none (OUT) |
| F14 | 14 `memory: project` agents carry a `# Persistent Agent Memory` section copied from Claude Code's own memory prompt, from that heading to `## When the coordinator picks this agent`. Six copies run 138 lines, about 12.95k chars (roughly 3.2k tokens): aoi-witness-broadcast, game-archaeology-specialist, items-systems-advisor, movement-teleport-advisor, server-authority-enforcer, social-systems-engineer. Those six also hard-code another machine's absolute user path and say "Your MEMORY.md is currently empty". Eight copies run 41-43 lines, about 2.2-2.4k chars (roughly 0.6k tokens). About 96k chars in all. Claude Code already adds memory instructions and the MEMORY.md head for these agents, so the copy rides on every request of every dispatch. October dispatches: rust-gameserver-dev 175, mission 38, game-archaeology 35, network-security 27, server-authority 26, npc-ai 19, items 15, others under 15 | line ranges from the heading to the agent's own sections; `agentType` counts over the window | HT-12b |

## Scope

**In** (owner list, re-sliced by the architect):

| Owner item | Becomes | Re-slice and why |
|---|---|---|
| 1 SubagentStop return contract | HT-06 | Two modes keyed on the agent file (D-HT1). **Report mode** for the agents meant to write reports (a `## Reporting` section and a non-Haiku model): block once on a missing line or a missing report (F3a). **Line mode** for Haiku agents with a `## Reporting` section (today only packet-coder): block once on a missing one-line return, never ask for a file. Neither mode blocks once the user-level cap has fired, so the hook never asks a capped agent for a denied tool. A line buried in prose still counts, because the orchestrator reads it fine. **Rejected:** the optional PostToolUse(Agent) parent note, because the orchestrator dispatches in the background, so PostToolUse fires at launch and never sees the result. |
| 2 Coder context stops | HT-03 (measure), HT-07 (align the coder with the owner rule), HT-16 (ruling), HT-17 (conditional fix) | Re-sliced by D-HT3. HT-07 drops the repo context cap and rewrites the coder's stop rule: code and commits only, then the one line; a continuation resumes from the branch. The efficiency goal stands: fewer stops and cheaper continuations (F4, F6), ruled on in HT-16. |
| 3 Resume card per session | HT-08 | The card comes from this session's own transcript. No new state files and no per-tool hook. |
| 4 Memory hygiene | HT-12a (rule), HT-12b (trim the copied memory prompt, F14), HT-13 (lint), HT-15a/b/c (one-off prune) | The lint keys on `type: project` plus state-claim phrases, because 634 files nest `type` under `metadata:`. It reports, never blocks: subagents write memory mid-task, and a merge-queue failure would cost more than it saves (D-HT7). |
| 5 Reviewer "Not checked" | HT-09 | Adds a `not_assessed=<n>` return suffix so the orchestrator sees it without opening the report. Coders write no report, so a check a coder skipped travels as `not_run=<k>,...` on its line and lands in the reviewer's `## Not checked` (D-HT8). |
| 6 Unique decision IDs | HT-10 | Packet ID prefixes too (F11, the `metrics packets` join). Existing collisions are grandfathered by name. |
| 7 Live guard proof | HT-14 (script, tested offline), HT-18 (live run, D-HT10 decided) | Closes lean-orchestration's owed UATs (F12). |
| (from D-HT3) The coder contract in the docs | HT-11a | Every place that tells a packet-coder to write a report or a worknote: the campaign-packet skill § 1, § 3, § 4 and § Sessions, `development-workflow.md`, `ai-sdlc.md`, `ai-sdlc-example.md`, `ai-harness.md` and `AGENTS.md`. The agent files are HT-07 (packet-coder, orchestrator) and HT-09 (packet-reviewer). |

**Out**, with reasons:

| Item | Reason |
|---|---|
| A repo-owned context cap, or report and worknote writes after a cap stop | Owner ruling D-HT3: Haiku coders are code writers, and the user-level cap stays as it is. D-HT2 is superseded. |
| Guard on big `docs/` reads | F13: 1.2% of Read bytes, and only 15 coders. A guard costs a hook call on every Read for every agent, and the reading rules already exist in development-workflow § Reading files. |
| Reviewer patch-id (skip re-review after a rebase) | At most 1 rebase-only re-review in the window. Nothing to save. |
| Hash stamps on UAT rows | No incident in the window. The owner runs UATs by hand and the ledgers already date them. |
| Subagent audit log | The transcripts and the token-profile DB already hold it. HT-01's reader reuses them. |
| Checks on delegation briefs | Briefs are short (about 25 lines). F2/F3 are fixed at the worker's stop, where the agent can still repair it. A brief lint would add a step to every dispatch to catch problems the stop hook already catches. |
| A human-decision format | `BlockedDecision` plus AskUserQuestion already works. The free part, a **Recommended / Evidence** column pair, is folded into the Decisions template (HT-10, SKILL § 1). |

## Decisions

| ID | Status | Decision | Recommended | Evidence |
|---|---|---|---|---|
| D-HT1 | Decided (revised 2026-10-31 for D-HT3) | A `SubagentStop` hook, `.claude/hooks/report_contract.py`, wired in `.claude/settings.json`. It acts only on agents whose file has `## Reporting`. **Report mode** (frontmatter `model:` not Haiku): when the brief names a report path and the final message has no LINE, or the named report doesn't exist, is empty or predates the agent's start, it blocks once with the exact fix. **Line mode** (Haiku): when the final message has no LINE, it blocks once asking for the line only, with no file and no tool. Neither mode blocks when the transcript holds the cap's `Context cap reached` denial; `stop_hook_active` ends the loop. No PostToolUse(Agent) note. | (decided) | F3a; D-HT3; background dispatch makes PostToolUse fire at launch |
| D-HT2 | **Superseded** by D-HT3 (2026-10-31) | Was: a repo-owned `context_cap.py` with a 75k note and a 90k deny that allowed report and worknote writes. Dropped: the owner keeps the user-level cap as it is, and Haiku coders write no report or worknote. | n/a | D-HT3 |
| D-HT3 | **Decided** (owner, 2026-10-31) | Owner, verbatim: "Haiku coders do not need to write a summary they are literally just code writers. The hook also only applies to haiku agents and not sonnet or opus." So the user-level `~/.claude/hooks/haiku_context_cap.py` stays as it is (past 90k it denies every tool for Haiku agents only and tells the agent to reply). The campaign does not replace, retire or work around it. A Haiku coder, including on a fix pass, writes code, commits passing work and returns one line: no report file, no worknote, no summary. The reviewer's report remains the packet's record. | (owner) | F4, F5 |
| D-HT4 | Decided | The resume card is per session. Tier 1: this session's own last Write/Edit of `.claude/state/active-campaign`. Tier 2: the campaign named by at least 2/3 (and at least 3) of this session's last 40 Agent prompts, labelled "inferred". Otherwise no card. The global pointer file stays, because orchestrators keep writing it and tier 1 keys on that write, but nothing reads its contents any more. | (decided) | F7: 44% wrong from the shared pointer |
| D-HT5 | Decided | Coder context: no fix is pre-committed. HT-03/HT-05 measure, the architect rules in HT-16, and HT-17 runs only if the ruling names a fix with a target. The candidate levers, in F6 order: packet-coder `effort`, packets inlining code excerpts so coders read less, Grep/Read habits, packet size, early `(part N)` commits, and a 75k note that only says "commit the parts whose checks passed" (never a deny and never a report request, D-HT3). | (decided) | F4, F6 |
| D-HT6 | Decided | Memory rule: methods over states; a state claim carries `INVALIDATED WHEN:` and a check date; memory is a lead, not evidence; the index stays under 200 lines. One rule line goes in the 16 `memory: project` agents plus architect and hands-on. The copied generic memory prompt in 14 agent bodies becomes a short stub (F14). A one-off prune of lint-flagged files follows. The 18 orphan folders (non-agent names, 2026-10-31) are folded into `main-session/` or deleted. | (decided) | F8, F9 |
| D-HT7 | Decided (revisit at HT-19) | The memory lint runs in CI in observation mode: it prints counts and exits 0. `--strict` exists for local use. HT-19 decides on a ratchet from the post-merge numbers. | (decided) | Memory is written mid-task by subagents. A queue failure costs more than a late fix. |
| D-HT8 | Decided (revised 2026-10-31 for D-HT3) | Reviewer reports get `## Not checked` (`- NOT ASSESSED — <what> — <reason>` or `- none`), and reviewer return lines a `not_assessed=<n>` suffix. Coders write no report: a coder that skipped a packet check adds `not_run=<k>,...` (the packet's check numbers) to its one line, and absence means every check ran. The orchestrator passes the coder's last line into the reviewer's brief; the reviewer lists each `not_run` check under `## Not checked` unless it ran it. On `ship` with n>0, the orchestrator copies the items to the ledger row. | (decided) | F10. The reviewer never builds, so a check the coder skipped is otherwise invisible until CI, or forever for a non-CI check. The suffix costs no tool call. |
| D-HT9 | Decided | A decision prefix (`D-XX`) and a packet prefix (`XX-`) each belong to one campaign. `test_ledgers.py` enforces this, grandfathering today's collisions by name. Decisions tables may add **Recommended** and **Evidence** columns (optional). | (decided) | F11 |
| D-HT10 | **Decided** (owner, 2026-10-31) | Owner: "Yes, as specified". `live_guard_probe.py --live` runs headless Haiku Claude sessions plus one Codex probe, at most 6 sessions per run, from a worktree, never the main checkout. It runs once at HT-18 and again on each change to a hook. | (owner) | F12 |
| D-HT11 | Decided | The Codex wiring of `report_contract.py` waits until HT-18 shows Codex hooks fire. If they don't, Codex sessions stay unguarded, as documented today. | (decided) | lean-orchestration D-LO probe result |
| D-HT12 | Decided | Measurement lives in `tools/token-profile/metrics` as three subcommands (`returns`, `coder-context`, `resume-cards`) over one raw-transcript reader. Output is aggregates only and passes the `report/scrub.py` Scrubber. No message text, paths or session ids are written. | (decided) | Brief: extend token-profile, no new tool; D-CM5 aggregates only |
| D-HT13 | Decided | The OUT list above stands. | (decided) | F13 and the Scope table |
| D-HT14 | Decided (architect, under D-HT3) | The Haiku coder contract (work-packets.md C4). The line is `<packet> <done\|blocked\|failed> <commit-sha\|-> -`, with optional `reason=<word>` (`context`, `checks`, `spec`, `host`, `platform`, `tool`) and `not_run=<k>,...`. LINE accepts `-` in the report slot. A coder may commit a finished part early as `(part N)` once its checks have passed; it never commits a failing state. A deviation from the spec or a rejected fix note is one line in the commit body (`Deviation: ...`, `Rejected N<k>: ...`): part of the commit, not a report. On `Context cap reached` the coder calls no tool and replies with its line. A continuation brief carries the base commit, and the next coder resumes from `git log <base>..HEAD`, `git status` and `git diff --stat` against the spec. | (decided) | F4, F5; the reviewer needs the reason for a rejected note, or the loop runs to a ruling |

**Open owner decisions:** none. D-HT3 and D-HT10 were decided by the owner on 2026-10-31.

## Targets and re-measurement

The baseline is HT-05's run of the new subcommands over 2026-10-03..2026-10-31. It should land within about 10% of F1-F7 and F3a; HT-05 explains any gap. The re-measure window opens when wave 2 merges. It runs until it holds at least 100 report-writer contract dispatches, 20 packet-coder context stops and 15 compact cards (HT-19 checks the counts first).

| Metric | Baseline | Target | Command (from the repo root) | Packet |
|---|---|---|---|---|
| Report writers: contract dispatches with no report write | 38/691 (5.5%) | < 2% | `python tools/token-profile/metrics returns --since <wave-2 merge> --out <dir>`, group `report_writers`, `no_report_write` | HT-06 |
| Report writers: no usable return line | 101/691 (14.6%) | < 3% | same, group `report_writers`, `no_usable_line` | HT-06 |
| packet-coder returns with no usable line, cap not fired | 34/696 (4.9%) | < 1% | same, `coder_lines.no_usable_line_uncapped` | HT-06, HT-07 |
| Wrong-campaign resume cards | 14/32 (44%) | ≤ 5% (tier 1 and tier 2 reported apart) | `python tools/token-profile/metrics resume-cards --since <wave-2 merge> --out <dir>` | HT-08 |
| packet-coder context-stop rate | 19.4% | set by the HT-16 ruling | `python tools/token-profile/metrics coder-context --since <date> --out <dir>`, `stop_rate` | HT-16, HT-17 |
| Fresh tokens per continuation session | 87 sessions, 6.9M fresh (mean about 79k); HT-05 gives the median | set by the HT-16 ruling | same, `continuation_tokens_fresh` / `continuations` | HT-16, HT-17 |
| State memories (type project, state phrase) without `INVALIDATED WHEN:` | all (0 have one) | prune flags drop ≥ 80%; ≥ 80% of state memories added after HT-12a carry one | `python tools/agent-skills/memory_lint.py --json` (and `--since <wave-3 merge>`) | HT-12a, HT-13, HT-15 |
| Copied memory-prompt chars in agent bodies | 14 agents, about 96k chars | 0 generic copies; a stub of at most 6 lines per agent | `python -m unittest discover -s tools/agent-skills -p "test_agent_memory_prompt.py"` | HT-12b |
| False claims in a 10-memory spot check | 5/10 | ≤ 1/10 | HT-19 re-checks 10 random lint-clean state memories by hand | HT-15 |
| Unregistered cross-campaign ID collisions | 2 known (`D-CA`, `CS-`); HT-10's scan lists the rest and grandfathers them | 0 new, enforced in CI | `python -m unittest discover -s tools/agent-skills -p "test_ledgers.py"` (CI: `agent-skills.yml`) | HT-10 |
| Reviewer reports with a `## Not checked` section | 0 | 100% of reviews after wave 2 | `metrics returns` fields `not_assessed_suffix` / `reviewer_lines` | HT-09 |
| Owed live guard proofs | 0/4 | 4/4 run, results recorded (Codex may be "does not fire") | `python tools/agent-skills/live_guard_probe.py --live ...` | HT-14, HT-18 |

## Packets

Ship modes and merge orders are in [work-packets.md § Waves](work-packets.md#waves); waves 3-5 are in [packets-wave3.md](packets-wave3.md). Wave 1 and wave 2 share no files and may run at the same time.

| Packet | Wave | Agent | Status | Depends on | Moves |
|---|---|---|---|---|---|
| HT-01 transcript reader, CLI wiring | 1 | packet-coder | Ready | none | enabler |
| HT-02 `metrics returns` | 1 | packet-coder | BlockedDependency | HT-01 | F2, F3, F3a baseline |
| HT-03 `metrics coder-context` | 1 | packet-coder | BlockedDependency | HT-01 | F4, F6 baseline |
| HT-04 `metrics resume-cards` | 1 | packet-coder | BlockedDependency | HT-01 | F7 baseline |
| HT-05 baseline run and token-profile docs | 1 | packet-coder | BlockedDependency | HT-02, 03, 04 | the baseline |
| HT-06 report-contract SubagentStop hook | 2 | packet-coder | Ready | none | report writers: no report 5.5% → < 2%, no line 14.6% → < 3%; coder no line 4.9% → < 1% |
| HT-07 the Haiku coder contract in the agent files | 2 | packet-coder | Ready | none | D-HT3; coder lines; continuation from the branch |
| HT-08 per-session resume card | 2 | packet-coder | Ready | none | wrong cards 44% → ≤ 5% |
| HT-09 Not checked, `not_run=`, and the format test | 2 | packet-coder | BlockedDependency | HT-06, HT-07, HT-08 | F10 |
| HT-10 unique ID prefixes test | 2 | packet-coder | Ready | none | collisions → 0 |
| HT-11a the coder contract in the skill and docs | 2 | documentation-writer | BlockedDependency | HT-07 | D-HT3 docs |
| HT-11b wave-2 harness docs and regen | 2 | documentation-writer | BlockedDependency | HT-06..HT-10, HT-11a | docs |
| HT-12a memory rule and the line in 4 agents | 3 | packet-coder | BlockedDependency | wave 2 merged | F9 |
| HT-12b trim the copied memory prompt in 14 agents | 3 | packet-coder | BlockedDependency | HT-12a head | F14: about 0.6-3.2k prompt tokens per request |
| HT-13 memory lint and CI step | 3 | packet-coder | BlockedDependency | wave 2 merged | F8 |
| HT-15a memory prune: rust-gameserver-dev | 3 | rust-gameserver-dev | BlockedDependency | HT-12b, HT-13 in `ht-w3` | F8, F9 |
| HT-15b memory prune: main-session and orphan folders | 3 | documentation-writer | BlockedDependency | HT-12b, HT-13 in `ht-w3` | F8 |
| HT-15c memory prune: the other 16 agent folders | 3 | rust-gameserver-dev | BlockedDependency | HT-12b, HT-13 in `ht-w3` | F8, F9 |
| HT-16 coder-context ruling | after wave 1 | architect | BlockedDependency | HT-05 merged | F4, F6 |
| HT-14 live guard probe (offline-tested) | 4 | packet-coder | BlockedDependency | wave 3 merged | F12 |
| HT-17 coder-context fix (conditional) | 4 | per ruling | BlockedDependency | HT-16 | stop rate, continuation cost |
| HT-18 live guard run | 4 | packet-coder | BlockedDependency | HT-14 head (D-HT10 decided) | F12 |
| HT-19 re-measure and close-out | 5 | documentation-writer | BlockedDependency | campaign review, re-measure window | all targets |

23 packets. Owner UATs (listed at close-out in `docs/guides/unified-uat.md`):

- **HT-U1**: in a real orchestrator session, write the pointer, run `/compact`, and see this session's card.
- **HT-U2**: watch one real packet-coder hit the user-level cap. It returns `<packet> blocked <sha|-> - reason=context` and writes no file, the SubagentStop hook doesn't block it, and a continuation coder resumes from the branch.

## Overlap with other campaigns

- **lean-orchestration** (closed 2026-10-16): HT-14/HT-18 deliver its owed guard UATs (F12). HT-18 adds one dated line to its owed list that points here. Nothing else in its ledger changes.
- **ai-harness-docs**: AH-15 (owner smoke test, D-AH33) and the AH-16 close-out are still open. AH-16 edits the status docs and empties `.claude/state/active-campaign`. This campaign never touches either. HT-09 closes the AH-24-review-2:25 note (no format test); HT-11b cites it.
- **campaign-metrics** (closed 2026-10-12; D-CM3..8 still Proposed): HT-01..HT-04 extend its `metrics` CLI and reuse its privacy gate (D-CM5). They resolve none of its decisions.
- **token-usage** (closed): its reading rules (TP-07) stay. F13 is why the big-read guard is OUT.
- **launcher-cache-presync** and others that use `.claude/state/active-campaign`: HT-08 keeps the file and its format, so their RESUME cards keep working. They just stop leaking into other sessions.
- **lab-autopilot** (ledger #2048, merged 2026-10-31; no packet merged yet): no file is in both ledgers. Shared touch points: (1) AX wave 6 adds `.claude/agents/frame-describer.md` (Haiku) and a `Verdicts:` block in `lab-driver.md`; neither has `## Reporting`, so HT-06 lets both stop unchecked, and HT-07's coder contract does not apply to them. (2) AX wave 6 adds a roster row to `docs/agents/development-workflow.md`, which HT-11a, HT-11b and HT-12a also edit in other sections: run `git merge-tree` before each merge. (3) HT-15c prunes `game-archaeology-specialist/` and `lab-driver/` memory; it keeps `ue3-reflection-trace-anchors.md` (added by #2048, maybe updated by AX-64) as type (a) or (b), and leaves `lab-driver/` until AX-61 merges. (4) Both orchestrators follow the `.claude/state/active-campaign` rule above.
- **Every running campaign**: from HT-07's merge on, packet-coder briefs name no report path, and the coder loop resumes from the branch, not from a worknote. A peer orchestrator mid-wave keeps working: a stray report path in a coder brief is ignored, and the line still parses.
