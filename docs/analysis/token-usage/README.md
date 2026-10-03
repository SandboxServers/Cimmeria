# Token Usage Profiling

> Type: how-to. Audience: the Claude Code coordinator and the packet workers.
> Updated: 2026-10-03 (Wave 1 merged). Tracking issue: [#957](https://github.com/SandboxServers/Cimmeria/issues/957); the plan is the [2026-10-03 comment](https://github.com/SandboxServers/Cimmeria/issues/957#issuecomment-5968451932), which supersedes the issue's phase list. Tool: [tools/token-profile/](../../../tools/token-profile/README.md). Workflow rules: [development-workflow.md § Worker lifetime and notifications](../../agents/development-workflow.md#worker-lifetime-and-notifications).

## Purpose

Find out what a successful unit of AI-assisted engineering costs here, in money, context, time and human intervention, and which parts of our orchestration add cost without improving correctness. Then cut those parts and prove the cut with before-and-after numbers.

The target is not fewer tokens for their own sake. A cheaper worker that needs more retries or more human correction is worse. Every change is judged on cost per merged PR together with its quality signals (CI rounds, review rounds, follow-up fixes).

## Decisions (@Cadacious, 2026-10-03)

| ID | Decision |
|---|---|
| D-TP1 | **Billing: Max subscription, within plan.** Reports give estimated list-price USD as a plan-usage proxy and always label it so. It is not a bill. |
| D-TP2 | **OTel sink: the colo SigNoz**, behind Cloudflare Access. `OTEL_LOG_TOOL_DETAILS=1`; prompt logging off. No endpoint or credential is committed. |
| D-TP3 | **Quick wins ship before the profiler.** Transcripts are kept 365 days, so the "before" already exists. Each packet records its merge time in the [cut-line log](#cut-line-log). |
| D-TP4 | **Cap worker lifetime now.** The rules are in [development-workflow.md](../../agents/development-workflow.md#worker-lifetime-and-notifications). |
| D-TP5 | **Every merged PR gets a stats comment** with a `cimmeria-pr-stats/1` JSON block, kept idempotent (edited in place, never duplicated). Past PRs since 2026-09-13 are backfilled once TP-05 has validated attribution. |

## Corrections to the issue

Checked against the Claude Code and Anthropic docs and re-measured over 899 local transcripts (89,745 deduplicated requests, 2026-09-14 to 2026-10-03).

| Issue claim | Verdict |
|---|---|
| One cost-equivalent weight set (cache read = 0.1) | Wrong for most traffic. Cache reads are 0.05x input on Opus 5.5 (59% of requests) and 0.025x on Fable 5.1, with no long-context premium. The profiler prices per model from a versioned table. |
| Usage is repeated across a request's records | Wrong. The first record is a streaming partial; dedupe must keep the final record. Output is 70.8M tokens, 9.6% of cost-equivalent, not 18M and 3%. |
| `thinking_tokens` | A subset of `output_tokens`, never added on top (0 exceptions in 110k records). |
| OTel carries request id, cost, agent and tool-result size | Confirmed. `cost_usd` is an estimate; `agent.name` and MCP names need `OTEL_LOG_TOOL_DETAILS=1`; cache writes are not split 5m/1h, so transcripts are still needed. |
| Subagents use a 5m cache TTL, main sessions 1h | Confirmed, 100% both ways. Settable per agent with `experimental.cacheTtl` in the agent's frontmatter. |
| ~59% of coordinator spend is event-triggered | Now ~53%: background completions 27%, idle notifications 8%, teammate messages 7%, cross-session messages 7%, monitor events 4%. Hooks cannot suppress teammate idle notifications; the fix is in orchestration. |
| nextest `--hide-progress-bar` | Deprecated; use `--show-progress=none`. |
| Experiment C's Cimmeria RAG arm | `cimmeria-rag` is dead. Dropped, and removed from `.mcp.json.example` in Wave 0. |

## Baseline before any change

From the 2026-10-03 quick pass (throwaway scripts, not the profiler; TP-05 replaces these with reconciled numbers):

- **Spend:** about $11.2k at list price over three weeks. Per merged PR: median about $8, p90 $44, max $388. The top 10% of PRs are 54% of attributable spend.
- **Attribution coverage:** branch matching places 49% of spend; coordinator time on `main` (about 18%) can be split through `pr-link` records.
- **Static context:** every agent's first request is already 58k-76k tokens (`Explore`: 21k), about 20-25% of every request. A recheck of the 11 most recent main sessions put it at **77k-88k**; see [TP-03 scope](#tp-03-scope-revised-2026-10-03).
- **Context size:** median 349k tokens per main-session request (p99 927k), 253k per subagent request (max 960k). 6 compactions in 899 transcripts; workers ran up to 1,036 requests.
- **Subagent cache writes by idle gap:** 40 / 49 / 4 / 8% (the issue's buckets).
- **Largest Bash cost:** `sed -n` file slicing, 38.5M result chars, about a quarter of all Bash output and more than half as much as `Read`.

## Packets

| Packet | Wave | Owns | Status | PR |
|---|---|---|---|---|
| TP-00 Ledger, data contract, attribution contract, worker and notification rules, RAG cleanup | 0 | this folder, `tools/token-profile/{schema.sql,*.md,fixtures/,test_contract.py}`, `development-workflow.md`, `.mcp.json.example` | Merged | [#1122](https://github.com/SandboxServers/Cimmeria/pull/1122) |
| TP-01a Profiler ingest | 1 | `tools/token-profile/ingest/` | Merged; schema version 2, see [Wave 1 results](#wave-1-results) | [#1128](https://github.com/SandboxServers/Cimmeria/pull/1128) |
| TP-01b Profiler reports and privacy scrubber | 1 | `tools/token-profile/report/` | Merged | [#1126](https://github.com/SandboxServers/Cimmeria/pull/1126) |
| TP-02 Quiet build and test output (B) | 1 | `tools/build-lane/`, `tools/test-live-db.*` | Merged | [#1127](https://github.com/SandboxServers/Cimmeria/pull/1127) |
| TP-03 Static context trim (H), target first request ≤55k for main sessions | 1 | `CLAUDE.md`, `.claude/agents/*.md`, memory indexes, MCP and skill config | Merged (repo side); the user-level part is open, see [Wave 1 results](#wave-1-results) | [#1124](https://github.com/SandboxServers/Cimmeria/pull/1124) |
| TP-04 OTel to the colo SigNoz | 1 | local Claude Code settings, `docs/operations/` | Merged (runbook); telemetry not live yet, see [Wave 1 results](#wave-1-results) | [#1125](https://github.com/SandboxServers/Cimmeria/pull/1125) |
| TP-05 Baseline and reconciliation | 2 (after TP-01) | this folder | Not started | |
| TP-06 Per-agent cache TTL (A) | 2 | `.claude/agents/*.md` frontmatter | Not started | |
| TP-07 Read discipline (C) | 2 | rules, the most-exposed docs | Not started | |
| TP-08 Experiment G (Ghidra MCP vs headless; SigNoz search vs aggregate) | 2 | this folder | Not started | |
| TP-09 Rebase churn (F) | 2 | a rebase script under `tools/` | Not started | |
| TP-10 Per-PR stats comments | 2 | `tools/token-profile/pr_stats.py` | Not started | |
| TP-11 Retro cost study | 2 (after TP-10 backfill) | this folder | Not started | |
| TP-12 Close-out | 3 | guide, rules, status docs | Not started | |

Wave 1 packets run in parallel, one worktree each; their files are disjoint. TP-01a and TP-01b build against [`schema.sql`](../../../tools/token-profile/schema.sql) and the fixture, so neither waits for the other. Packet scopes are in the [plan comment](https://github.com/SandboxServers/Cimmeria/issues/957#issuecomment-5968451932).

### TP-00 review round 1

CodeRabbit and Copilot reviewed #1122 on 2026-10-03. Every finding but one was fixed in the PR:

- **Triggers.** Every request must have one (`requests.trigger_id` is NOT NULL). A retry has no record of its own, so its trigger id is `<request_id>:retry`. `retry` and `mixed` were unreachable under first-match ordering, so they are now overrides R1 and R2, checked first, and the per-record rules are renumbered R3-R15. A `mixed` delivery is defined as consecutive `queued_command` attachments with no request between them.
- **API errors.** A failed call is an assistant record with model `<synthetic>` (91 observed). It is not a request; it only marks the next request as a retry.
- **Fixture coverage.** The fixture now covers all 15 rules, and a test pins it to the doc table.
- **Fingerprints.** They keep the executable alone unless the executable and its first argument are an allowlisted pair, because any argument can hold a secret. The fixture adds hostile second words (`curl` with URL credentials, `echo` of a token, `python` with a local path).
- **Attribution.**
  - A new `attribution_imbalance` view lists requests whose weights don't sum to 1, and a test covers it. An ingest run fails if the view is not empty.
  - `unattributed_share` is now bounded to 0-1.
  - A3 ancestry no longer resolves branch names, which `rm-worktree.sh` deletes. It reads commits from a new `branch_heads` table, filled from ingest ref snapshots, the build lane's job log and merge-commit subjects.
- **CI.** Changes under `tools/token-profile/` now count as code, so a Markdown-only change to the contract still runs its tests.
- **Small fixes.** Issue numbers are removed from source comments, and the RE guide now says "at least these two groups".

Not changed: CodeRabbit asked for a confidence tag on the new `docs/readme.md` row. No other row in that table (the campaign ledgers) carries one, so the row stays as it is.

### TP-03 scope (revised 2026-10-03)

The first request of the 11 most recent main sessions was **77k-88k tokens**, up from the 58k-76k first measured. About 29k-33k of it is a cache read shared by every session (the harness's system prompt and tool definitions), which we can't change. About 50k-55k is written fresh in each session, at 2x the input price because main sessions use the 1-hour cache. The parts we control, estimated from file size at about 4 characters per token:

| Source | ~Tokens | Lever |
|---|---:|---|
| `CLAUDE.md` | 10k | Move the doc-update map and the CI-failure notes into linked docs; target 3k-4k |
| Agent descriptions (all 16 load in every main session) | 8k | Drop the `<example>` blocks (3k) and cut each description to a line or two |
| Personal `MEMORY.md` index | 5k | Remove duplicate lines and archive finished campaigns |
| Tool names and server instructions from MCP servers and claude.ai connectors (Ghidra ~245 tools, x64dbg, lab, SigNoz, Gmail/Calendar/Drive) | 8k-12k | Turn off servers a session doesn't use |
| User-level skills that don't apply here (3D, React Flow, Tailwind, ...) | 3k-5k | Disable for this project |

The **target is now a first request of ≤55k for main sessions**, replacing "≤45k, from ~68k". The fixed part is roughly a fifth of each request's cost, so this cut is worth about 7-8% of spend, around $800 at list price per three weeks on current numbers. That is smaller than the history that builds up in long sessions, which the worker-lifetime cap and TP-02 address, but it is permanent and applies to every session. TP-03 measures each part with `/context` before and after, and moves content into linked docs rather than deleting it, then checks that agents still follow the moved rules.

### Wave 1 results

All five packets merged on 2026-10-03. Worknotes: [worknotes/](worknotes/).

- **TP-01a ingest.** Over the local transcripts (915 files, 90,825 requests, no unknown shapes, no attribution imbalance) the estimate is $11,301 at list price against $11,198 in Claude Code's own `cost-state` records, 0.9% apart; TP-05 makes that a controlled reconciliation. Attribution places 72.4% of spend: branch 38.8%, ancestry 11.4%, `pr-link` and split 10.9%, parent session 6.4%, trigger 4.9%; 27.6% stays unattributed. A first run takes 77 s, a re-run about 5 s.
  - Contract changes, written into the contract files: schema version 2 (additive: `ingest_files.parse_state`, `tool_calls.task_id`, `tool_calls.pr_ref`, `prs.closed_at`); A3 tests the PR's head commit, because a squash merge never has the packet's commits as ancestors; `ref-snapshot` heads are dated by commit date; a harness `isMeta` record no longer replaces the human prompt before it.
  - For TP-05: 878 records repeat a `requestId` already stored from another file; the first file keeps the request and the copies are counted. Campaign attribution has no column and would need schema version 3.
- **TP-01b reports.** Raw, USD and context layers, distributions, cost per merged PR, the cache-policy simulator and the three-layer privacy scrubber, 40 tests. For TP-06: the simulator's model of how much cache stays warm after a long gap is calibrated only on the synthetic fixture. Web search and fetch are counted but not priced. Nothing reads `cost_states` yet (TP-05).
- **Integration.** TP-01a's schema version 2 broke TP-01b's positional fixture inserts and its exact version check once both were on `main`; the coordinator fixed both in #1128 (named columns; reports read versions 1 and 2). All 105 token-profile tests pass, and an ingest of the fixture feeds a clean report.
- **TP-02 quiet build output.** For the same command through the lane, agent-facing stdout fell from 36,765 to about 270 characters (`cargo nextest run -p cimmeria-wire`, 296 tests) and from 2,524 to 267 (`cargo test -p cimmeria-commands`, warm); a failing test still shows its panic in about 700. CI keeps full output. One unexplained failure in 8 local runs of the build-lane tests; watch for a flaky test.
- **TP-03 context trim.** `CLAUDE.md` went from about 9.9k to 3.6k tokens and the 16 agent descriptions from 8.1k to 0.9k, about 13.7k in all; subagents lose about 6k each as well. The baseline first request (11 recent main sessions) was 83.0k at the median, projected about 69k after the cut, so the repo side alone misses the ≤55k target. The rest is user-level and awaits the user: MCP servers off by default (about 3k), claude.ai connectors (about 1k), the personal memory index (2.5k-3k), document skills (about 1.5k).
- **TP-04 telemetry.** The runbook, settings templates, a Key Vault header helper and a dashboard are in `docs/operations/`. The colo has no Cloudflare Tunnel, so D-TP2's route needs operator work first; the open questions (an interim private-network export, scrubbing the login email, whether the SigNoz ports are reachable from outside) are with the user. TP-05 needs the time telemetry goes live.

## Cut-line log

Each packet that changes behaviour adds a row when it merges. A before-and-after comparison uses requests on either side of the cut, and states which other cuts fall inside its window.

| Packet | PR | Merged (UTC) | What changed at the cut |
|---|---|---|---|
| TP-00 | [#1122](https://github.com/SandboxServers/Cimmeria/pull/1122) | 2026-10-03 11:39:49 | Worker lifetime cap and notification rules take effect; `cimmeria-rag` leaves the example MCP config |
| TP-03 | [#1124](https://github.com/SandboxServers/Cimmeria/pull/1124) | 2026-10-03 12:04:06 | `CLAUDE.md` and agent descriptions trimmed by about 13.7k tokens per main session |
| TP-02 | [#1127](https://github.com/SandboxServers/Cimmeria/pull/1127) | 2026-10-03 12:33:56 | Lane prints a summary, not the full cargo output, when stdout is not a terminal |

## Acceptance criteria

The profiler is trusted only when all of these hold (the issue's 2026-09-30 Phase 0 list plus the 2026-10-03 additions):

- [x] Raw token categories are reported independently. (TP-01b)
- [x] Thinking tokens are not double-counted (test: `test_thinking_is_a_subset_of_output`, plus an ingest test). (TP-01a)
- [x] A final-record dedupe test fails a first-record dedupe (the fixture is built for it). (TP-01a)
- [x] Model-specific USD, including the Opus 5.5 and Fable 5.1 cache-read prices. (TP-01a)
- [ ] Transcript totals reconcile with both OTel and `cost-state` for a controlled session, within a documented tolerance.
- [x] Unknown transcript shapes fail visibly or land in a counted bucket. (TP-01a)
- [x] Main, subagent and agent-type attribution is tested. (TP-01a)
- [x] Trigger attribution includes `unknown` and `mixed`. (TP-01a)
- [x] Compactions are modelled. (TP-01a, TP-01b)
- [x] Context exposure is kept apart from monetary cost. (TP-01b)
- [x] Reports give p50/p75/p90/p95/p99/max, not only means. (TP-01b)
- [x] At least one outcome-normalized metric (cost per merged PR). (TP-01b)
- [x] Synthetic privacy fixtures prove secrets and private machine data cannot reach a committed report. (TP-01b)
- [x] Every report states its window and version metadata. (TP-01b)
- [x] Ingest is incremental, so raw transcripts need not be kept forever. (TP-01a)
- [ ] Every PR merged after TP-10 has one idempotent stats comment with the `cimmeria-pr-stats/1` block, and the backfill is posted.
