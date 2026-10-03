# Token Usage Profiling

> Type: how-to. Audience: the Claude Code coordinator and the packet workers.
> Updated: 2026-10-03 (**campaign closed**, TP-12; open levers are follow-ups, see [Close-out](#close-out)). How-to: [docs/guides/token-profiling.md](../../guides/token-profiling.md). Tracking issue: [#957](https://github.com/SandboxServers/Cimmeria/issues/957); the plan is the [2026-10-03 comment](https://github.com/SandboxServers/Cimmeria/issues/957#issuecomment-5968451932), which supersedes the issue's phase list. Tool: [tools/token-profile/](../../../tools/token-profile/README.md). Workflow rules: [development-workflow.md § Worker lifetime and notifications](../../agents/development-workflow.md#worker-lifetime-and-notifications).

## Purpose

Find out what a successful unit of AI-assisted engineering costs here, in money, context, time and human intervention, and which parts of our orchestration add cost without improving correctness. Then cut those parts and prove the cut with before-and-after numbers.

The target is not fewer tokens for their own sake. A cheaper worker that needs more retries or more human correction is worse. Every change is judged on cost per merged PR together with its quality signals (CI rounds, review rounds, follow-up fixes).

## Decisions (@Cadacious, 2026-10-03)

| ID | Decision |
|---|---|
| D-TP1 | **Billing: Max subscription, within plan.** Reports give estimated list-price USD as a plan-usage proxy and always label it so. It is not a bill. |
| D-TP2 | **OTel sink: the colo SigNoz**, exported directly over the private network (runbook Path A). Revised after Wave 1: the SigNoz ports are reachable only over WireGuard, so no Cloudflare tunnel is needed. `OTEL_LOG_TOOL_DETAILS=1`; prompt logging off; the colo collector drops `user.email`; `OTEL_METRICS_INCLUDE_ACCOUNT_UUID=false`. No endpoint or credential is committed. |
| D-TP3 | **Quick wins ship before the profiler.** Transcripts are kept 365 days, so the "before" already exists. Each packet records its merge time in the [cut-line log](#cut-line-log). |
| D-TP4 | **Cap worker lifetime now.** The rules are in [development-workflow.md](../../agents/development-workflow.md#worker-lifetime-and-notifications). |
| D-TP5 | **Every merged PR gets a stats comment** with a `cimmeria-pr-stats/1` JSON block, kept idempotent (edited in place, never duplicated). Past PRs since 2026-09-13 are backfilled once attribution is validated (TP-05b). |
| D-TP6 | **Reports show both totals.** Claude Code's own `cost-state` total stands next to the profiler's (the profiler is about 10% under; it is a floor). (2026-10-03, after Wave 2) |
| D-TP7 | **An integration PR carries only its own spend.** A packet PR that merges into an integration branch keeps its own spend; a campaign's cost is the sum over its PRs. Ancestry no longer charges packet work to the integration PR. (2026-10-03, after Wave 2) |
| D-TP8 | **Rebase only when needed.** `rebase-pr.sh` runs when GitHub reports a PR behind or conflicting, not before every merge. (2026-10-03, after Wave 2) |
| D-TP9 | **`docs/readme.md` keeps `merge=union`**; `rebase-pr.sh` handles its generated count lines. (2026-10-03, after Wave 2) |
| D-TP10 | **Experiment G's guidance is a rule:** the Ghidra MCP over the headless probe when Ghidra's GUI is up, and SigNoz `aggregate_logs` over `search_logs` unless line bodies are needed. Written into `reverse-engineering-with-claude.md` and the SigNoz log-mining memory. (2026-10-03, after Wave 2) |

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
| TP-05 Baseline and reconciliation | 2 (after TP-01) | this folder | Merged; cost-state reconciled, OTel arm waits for data, see [Wave 2 results](#wave-2-results) | [#1134](https://github.com/SandboxServers/Cimmeria/pull/1134) |
| TP-06 Per-agent cache TTL (A) | 2 | `.claude/agents/*.md` frontmatter | Merged | [#1130](https://github.com/SandboxServers/Cimmeria/pull/1130) |
| TP-07 Read discipline (C) | 2 | rules, the most-exposed docs | Merged | [#1133](https://github.com/SandboxServers/Cimmeria/pull/1133) |
| TP-08 Experiment G (Ghidra MCP vs headless; SigNoz search vs aggregate) | 2 | this folder | Merged; [write-up](experiment-g.md) | [#1132](https://github.com/SandboxServers/Cimmeria/pull/1132) |
| TP-09 Rebase churn (F) | 2 | a rebase script under `tools/` | Merged | [#1135](https://github.com/SandboxServers/Cimmeria/pull/1135) |
| TP-10 Per-PR stats comments | 2 | `tools/token-profile/pr_stats/` | Merged; posting began after TP-05b, see [Close-out](#close-out) | [#1131](https://github.com/SandboxServers/Cimmeria/pull/1131) |
| TP-05b Attribution validation, D-TP6 both totals, D-TP7 integration PRs and campaign rollup | 2 | `tools/token-profile/ingest/`, `report/`, `pr_stats/`, `validate/` | Merged 2026-10-03 16:11:11Z; schema version 4, [worknote](worknotes/TP-05b.md) | [#1138](https://github.com/SandboxServers/Cimmeria/pull/1138) |
| TP-05b review fix: the cache simulator's range is three scenarios, not a bound | 2 | `tools/token-profile/report/` | Merged 2026-10-03 16:11:23Z (CodeRabbit finding) | [#1139](https://github.com/SandboxServers/Cimmeria/pull/1139) |
| TP-11 Retro cost study | 2 (after TP-10 backfill) | this folder | Merged 2026-10-03 16:29:54Z; [study](retro-cost-study.md) | [#1140](https://github.com/SandboxServers/Cimmeria/pull/1140) |
| TP-12 Close-out: docs and status | 3 | the [how-to guide](../../guides/token-profiling.md), rules, status docs, the `gap-analysis.md` split | Merged; [worknote](worknotes/TP-12-docs.md) | [#1142](https://github.com/SandboxServers/Cimmeria/pull/1142) |
| TP-12 Close-out: scheduled jobs | 3 | `tools/token-profile/scheduled/` | Merged; tasks registered on the coordinator workstation 2026-10-03 (first sweep 2026-10-04 07:30, first weekly report 2026-10-05 08:00, local time); [worknote](worknotes/TP-12-tools.md) | [#1143](https://github.com/SandboxServers/Cimmeria/pull/1143) |

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

### Wave 2 results

All six packets merged on 2026-10-03. Worknotes: [worknotes/](worknotes/). USD is list price, a plan-usage proxy (D-TP1).

- **TP-05 reconciliation and baseline.** The profiler is **10.2% under** Claude Code's `cost-state` totals over the same process windows ($10,053 against $11,198, 93 sessions). Wave 1's 0.9% agreement was two errors of about $1.2k cancelling: `cost-state` misses everything before a resume, and transcripts miss requests (mostly cache reads, growing with subagent use, correlation 0.83). Repricing `cost-state`'s own tokens with our table agrees to 0.03%, so prices are not the cause. Until OTel's `query_source` explains the missing requests, **profiler USD is a floor**. `python tools/token-profile/reconcile` checks five tolerances and exits 4 on a failure; it passes. The 878 duplicate `requestId`s are fork subagents that open with a copy of the parent's history, and the ingest now reads forks after their parents. Schema version 3 (`cost_states.process_start`). Teammate names no longer reach reports as agent types. The scrubbed baseline is in [baseline/](baseline/README.md): 90,466 requests, about $11,265, 79% subagents; per merged PR (408) median $7.61, p90 $38.20, max $1,274 (#662); the top 10% of PRs hold 56%; 28% is unattributed. The OTel comparison is built and tested on a synthetic export; it runs once telemetry data exists.
- **TP-06 cache TTL.** The cache simulator's model of a cold request was wrong (it used the smallest cold read, often 0; held-out prediction 0.38 of real) and now uses the mean (0.91). Only `rust-gameserver-dev` passes the rule (estimate, assumption range and bootstrap interval all favour 1h): about -$512, -9.5%, over three weeks; it now has `experimental.cacheTtl: 1h`. Cache writes after a gap over 5 minutes fall from an estimated 14.6% to 5.0% of subagent spend, about $170 a week. The main session's 1h TTL is confirmed (5m would have cost $3,357 instead of $1,958).
- **TP-07 read discipline.** Correction to the baseline: `sed -n` slices are mostly narrow (median 41 lines); the volume is their count. The waste is whole-file reads of large docs and lines too long for Grep. Experiment C (10 lookups, key written first): whole-file reads returned 757k characters, Grep then a narrow `Read` 25k, both 10/10. `observability.md`, `content-engine.md`, the abilities ADR and `services-crate-split.md` were split along their seams; a "Reading files" rule is in `development-workflow.md`. `docs/gap-analysis.md` (1,476 lines) is left for TP-12.
- **TP-08 experiment G.** Both arms of both comparisons scored full marks; cost followed the request count. Ghidra MCP beat headless (7 against 9 requests, 47 s against 88 s) and SigNoz `aggregate_logs` beat `search_logs` (4 against 9 requests, 7k against 32.5k characters). Ghidra and SigNoz are 1.3% and 2.2% of all exposure, `Read` 25%, so neither is a workflow-wide lever. The runs corrected two caller counts in the RE docs. Whether the guidance becomes a rule is open for TP-12.
- **TP-09 rebase churn.** 607 rebase episodes, at most 6.9% of spend; only 9 conflicted in generated or lock files alone. [`tools/build-lane/rebase-pr.sh`](../../../tools/build-lane/rebase-pr.sh) turns a clean rebase into one call (about $440 per 17 days, an upper bound) and stops on a semantic conflict with the branch untouched. It also fixes a `merge=union` duplicate-row bug on generated count lines in `docs/readme.md`. Its first real use, on #1131, stopped correctly on a three-file semantic conflict with TP-05, which a fresh worker resolved.
- **TP-10 per-PR stats.** `pr_stats <PR> [--post]` and `--backfill`, idempotent by marker; a dry run over 416 merged PRs built 392 comments with no gate refusals. **Nothing is posted yet.** Checking Wave 2's own workers against the PRs they produced shows attribution errors: TP-10's worker is split half onto #1128, and its rebase worker and 43 of TP-05's 101 requests are unattributed. TP-05b fixes attribution, using known packet-to-PR pairs like Wave 2's as ground truth, before the first live post and the backfill.
- **Telemetry.** Path A was set up on the coordinator workstation on 2026-10-03 at about 13:20 UTC, and the colo collector drops `user.email` (see [claude-code-telemetry.md](../../operations/claude-code-telemetry.md#optional-scrub-the-login-email-at-the-collector-operator)). It exports from the next Claude Code start; that time opens the OTel reconciliation window.
- **Static context, main session.** A main session started on 2026-10-03 at about 13:05 UTC still measured a first request of 83,006 tokens, because the main checkout was 16 commits behind `origin/main` and had not loaded TP-03. The first post-TP-03 main session gives the real number. Subagent first requests fell from a median of 64.6k to 61.4k (n=17). `disabledMcpjsonServers` in the local settings did not stop the disabled servers' tools from loading (deferred), which needs a look.

### Open questions for the owner (Wave 2)

Answered on 2026-10-03 as D-TP6 to D-TP10 above. TP-05b implements D-TP6 and D-TP7. The questions were:

1. Reconciliation: keep the 15% undercount limit, or show the `cost-state` total next to the profiler's in reports?
2. Does a campaign's integration PR (#662) carry its packets' spend in its stats comment, or only its own?
3. Run `rebase-pr.sh` before every merge, or only when GitHub says the PR is behind?
4. Keep `docs/readme.md` at `merge=union`?
5. Make experiment G's guidance (Ghidra MCP over headless, SigNoz `aggregate_logs` over `search_logs`) a rule?

## Close-out

Wave 3 closed the campaign on 2026-10-03. Worknotes: [TP-05b](worknotes/TP-05b.md), [TP-11](worknotes/TP-11.md), [TP-12-docs](worknotes/TP-12-docs.md). USD is list price, a plan-usage proxy and a floor about 10% under Claude Code's own total (D-TP1, D-TP6).

### TP-05b: attribution validated

Scored against 91 workers' known PRs (labels) and against every worker that created exactly one PR, precision by USD went from 0.41 to **0.99** and recall from 0.22 to 0.98; no rule misplaces more than 1% of labelled spend, against the 10% limit. Five failure classes were found and fixed. The main one: a subagent's transcript records the coordinator's branch, not its own, so a subagent's branch now comes from its own git output and worktree paths. Unattributed spend fell from 27.5% to 13.5%, and 18.5% now sits on a campaign with no PR (D-TP7). So the Harset integration PR #662 dropped from $1,274 to $63, and its packets moved to the `harset-rebuild` rollup. Schema version 4; a new database is needed. Details: [worknote](worknotes/TP-05b.md).

### Stats comments and the backfill

Live posting started after #1138 merged. The backfill of every PR merged since 2026-09-13 finished at about 17:00 UTC on 2026-10-03: **406 PRs posted, 20 with no data** (dependency bumps and human-only work), **0 errors**; a spot check found no duplicate comments. From here the merging agent posts each PR's comment, and a daily local sweep catches what it misses ([how-to](../../guides/token-profiling.md#let-the-scheduled-jobs-keep-it-current)).

### TP-11: what work costs, and what didn't buy quality

The [retro cost study](retro-cost-study.md) covers the 426 PRs merged from 2026-09-13 to 2026-10-03. Every Wave 0-2 cut merged after its data, so it is the **before** for all of them.

- **Per PR:** p50 $11.38, p90 $41.09, max $279 (#642). Size explains more than type: under 300 changed lines p50 $3.27, 1.5k-5k lines $23.23. Features p50 $20.62, fixes $10.10, docs $3.22.
- **Spend did not buy quality.** Within the same type and size, the dearer half cost three times as much (p50 $23.51 against $7.74, 172 PRs a side) with the same CI-failure rate (6%) and follow-up-fix rate (9% against 10%), and twice the review rounds. Its extra went on idle cache rewrites (18.3% of its spend against 7.0%), mechanical steps (13.5% against 9.8%) and rebases (6.9% against 4.6%).
- **Campaigns:** harset-rebuild $1,689 (93% of it packet work with no PR of its own), npc-ai-restoration $817, castle-rebuild $705; this campaign $107 over 17 PRs.
- **Defects that shipped** cost $496 in 31 follow-up fix PRs, 6.5% of PR spend; spending more per PR did not lower it.

### Before and after, so far

The cuts are hours old, so these "after" samples are small: early signals, not results. From `python tools/token-profile/cutlines` and direct queries on a copy of the database, ingested at 17:27 UTC on 2026-10-03. Letters are the issue's lever names.

| Lever | Measure | Before | After | Reading |
|---|---|---:|---:|---|
| H, static context (TP-03) | Main-session first request, p50 tokens | 73.1k (n=93) | 64.8k (n=5) | Down about 8k; still above the ≤55k target, because the user-level part is not cut |
| H | Subagent first request, p50 tokens | 64.6k (n=813) | 61.3k (n=22) | Down about 3k |
| D, worker lifetime (TP-00) | Requests per subagent transcript, p50 / p90 | 61 / 204 (n=803) | 38 / 73 (n=27) | Shorter workers; none over 200 requests after the cut, against 86 before |
| D | Peak context per subagent transcript, p50 / p90 | 238k / 500k | 182k / 317k | Lower. A transcript still running at ingest is cut short, so "after" is a floor |
| E, notifications (TP-00) | Share of main-session spend in turns an event started | 53.7% (of $2,344) | 47.5% (of $25) | Lower, on one coordinator's afternoon |
| B, quiet build output (TP-02) | `lane.sh` foreground result, p50 characters | 490 (n=1,599) | 526 (n=1) | No sample yet; the controlled run in [Wave 1 results](#wave-1-results) stands (36,765 to about 270) |
| A, cache TTL (TP-06) | `rust-gameserver-dev` requests that wrote a 1-hour cache | 0 of 23,571 | none yet | No `rust-gameserver-dev` request since the cut; unverified |
| A | Generic teammates: idle 5-minute rewrites, share of their spend | 10.8% | 34% (305 requests, $37) | Teammates without an agent definition still use the 5-minute TTL |

### Open levers (follow-ups)

The campaign closes with these open. Each has a number from the study, so a follow-up can measure its own cut.

1. **Idle cache rewrites, 13.2% of all spend ($1,498).** 90% of the subagent part would stay warm under a 1-hour TTL. TP-06 gave `rust-gameserver-dev` (72% of it) the 1-hour TTL, but no request has confirmed it works yet, teammates included. Teammates with no agent definition stay on 5 minutes, and whether `experimental.cacheTtl` can reach them is unchecked. Until then, the workflow rule is not to park workers ([development-workflow.md](../../agents/development-workflow.md#worker-lifetime-and-notifications)).
2. **Mechanical steps in large contexts, 11.2% ($1,272).** Plumbing calls (`git status`, `git add`, `git fetch`, `ls`, `date`) run one per request at over 200k tokens. TP-12 added the batching rule; the candidate tool is one ship script that stages, commits, pushes and opens or updates the PR with a one-line status.
3. **Static context, about 15% of PR spend.** TP-03's repo-side trim held (above). The user-level part (MCP servers, claude.ai connectors, the personal memory index, document skills) was applied locally and then reverted on 2026-10-03 when the local settings were discarded, so it is open again. `disabledMcpjsonServers` not stopping deferred tools from loading belongs to it.
4. **OTel reconciliation.** Waits for the first telemetry export, which needs a Claude Code restart on the configured workstation.
5. **Small ones.** The study's type rule files unprefixed PR titles as chore (a required prefix or label would fix that at the source), and failed CI rounds are undercounted after a force-push.

## Cut-line log

Each packet that changes behaviour adds a row when it merges. A before-and-after comparison uses requests on either side of the cut, and states which other cuts fall inside its window.

| Packet | PR | Merged (UTC) | What changed at the cut |
|---|---|---|---|
| TP-00 | [#1122](https://github.com/SandboxServers/Cimmeria/pull/1122) | 2026-10-03 11:39:49 | Worker lifetime cap and notification rules take effect; `cimmeria-rag` leaves the example MCP config |
| TP-03 | [#1124](https://github.com/SandboxServers/Cimmeria/pull/1124) | 2026-10-03 12:04:06 | `CLAUDE.md` and agent descriptions trimmed by about 13.7k tokens per main session |
| TP-02 | [#1127](https://github.com/SandboxServers/Cimmeria/pull/1127) | 2026-10-03 12:33:56 | Lane prints a summary, not the full cargo output, when stdout is not a terminal |
| TP-06 | [#1130](https://github.com/SandboxServers/Cimmeria/pull/1130) | 2026-10-03 13:43:26 | `rust-gameserver-dev` requests use the 1-hour cache TTL; check that its new transcripts show `cache_write_1h > 0`, teammates included |
| TP-07 | [#1133](https://github.com/SandboxServers/Cimmeria/pull/1133) | 2026-10-03 13:44:55 | "Reading files" rule; four oversized docs split |
| TP-09 | [#1135](https://github.com/SandboxServers/Cimmeria/pull/1135) | 2026-10-03 13:57:11 | Mechanical rebases go through `rebase-pr.sh` |
| TP-05b | [#1138](https://github.com/SandboxServers/Cimmeria/pull/1138) | 2026-10-03 16:11:11 | Attribution rebuilt (schema version 4: a subagent's branch from its own git output; packet work to the campaign, not the integration PR); per-PR stats comments go live and the backfill starts. Per-PR numbers from before and after this cut use different rules, so re-ingest before comparing a PR across it |
| TP-12 | [#1142](https://github.com/SandboxServers/Cimmeria/pull/1142) | 2026-10-03 17:34:01 | "Batch mechanical steps" and "don't park a worker" rules; worker worktree safety rules |
| TP-12 | [#1143](https://github.com/SandboxServers/Cimmeria/pull/1143) | 2026-10-03 17:41:14 | Daily PR-stats sweep and weekly report run from Windows Task Scheduler; the sweep re-edits recent comments once after each profiler change |

## Acceptance criteria

The profiler is trusted only when all of these hold (the issue's 2026-09-30 Phase 0 list plus the 2026-10-03 additions):

- [x] Raw token categories are reported independently. (TP-01b)
- [x] Thinking tokens are not double-counted (test: `test_thinking_is_a_subset_of_output`, plus an ingest test). (TP-01a)
- [x] A final-record dedupe test fails a first-record dedupe (the fixture is built for it). (TP-01a)
- [x] Model-specific USD, including the Opus 5.5 and Fable 5.1 cache-read prices. (TP-01a)
- [ ] Transcript totals reconcile with both OTel and `cost-state` for a controlled session, within a documented tolerance. (`cost-state`: done in TP-05. OTel: still open at close-out, a follow-up. Telemetry exports only after a Claude Code restart on the configured workstation, and none has happened yet.)
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
- [x] Every PR merged after TP-10 has one idempotent stats comment with the `cimmeria-pr-stats/1` block, and the backfill is posted. (TP-05b, then the backfill: 406 posted, 20 no data, 0 errors, no duplicates on a spot check, finished about 17:00 UTC 2026-10-03.)
