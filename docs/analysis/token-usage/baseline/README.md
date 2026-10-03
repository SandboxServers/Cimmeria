# Token usage baseline, 2026-09-14 to the Wave 0 cut

> Type: reference. Packet TP-05 of [#957](https://github.com/SandboxServers/Cimmeria/issues/957); ledger [../README.md](../README.md). Generated 2026-10-03 from one ingest of the local transcripts at profiler commit `820709c150f9` (schema version 3, price table `2026-10-03`). Estimated USD is list price, a plan-usage proxy on a Max subscription, not a bill (D-TP1).

The files here are generated and pass the profiler's privacy gate. Don't edit them by hand; regenerate them with the commands at the end.

| File | What it is |
|---|---|
| [token-report.md](token-report.md), [token-report.json](token-report.json) | The full report for requests from 2026-09-14T00:00:00Z up to the TP-00 cut, 2026-10-03T11:39:49Z. The stamp at the top gives the window, commits, versions and models. |
| [reconcile.md](reconcile.md), [reconcile.json](reconcile.json) | The same window checked against Claude Code's `cost-state` records. |
| [cutlines.md](cutlines.md), [cutlines.json](cutlines.json) | Before-and-after measures at the TP-00, TP-03 and TP-02 cuts, through the last request ingested (2026-10-03T13:41Z). |

## Headline numbers

| Measure | Value |
|---|---|
| Requests, transcripts | 90,466 requests (2026-09-17 to 2026-10-03 11:37Z), Claude Code 2.1.274 to 2.1.288 |
| Estimated spend, transcripts | $11,265: subagents 79%, main sessions 21% |
| Spend by trigger | teammate messages 29%, background completions 27%, subagent prompts 24%, human prompts 9%, monitor events 3% |
| Claude Code's own total over the same sessions | $11,198 (93 sessions with a `cost-state` record) |
| Per merged PR (408 PRs) | median $7.61, p90 $38.20, p99 $140, max $1,274 (#662, the Harset integration PR, by ancestry); the top 10% of PRs hold 56% of PR spend |
| Attribution | 71% to PRs merged in the window, 1% to other PRs, 28% unattributed; by method: branch 39%, ancestry 11%, `pr-link` 8%, parent session 6%, trigger 5%, split 3% |
| Static context (first request) | main sessions p50 73k, p90 83k (n=93); subagents p50 65k, p90 75k (n=813) |

## What the reconciliation says

The quick pass's 0.9% agreement ($11,301 against $11,198) was two errors of about $1.2k cancelling out:

- **The profiler holds $1,212 the cost-state doesn't.** A `cost-state` record covers only the Claude Code process that wrote it. Seven sessions were resumed, and their records leave out everything before the resume (six read $0); one session has no record.
- **The cost-state holds about $1,144 the transcripts don't.** Compared over the same process windows, the profiler is **10.2% under** Claude Code ($10,053 against $11,198). Prices are not the cause: the cost-state's own tokens, repriced with the profiler's table, give its USD to within 0.03%. Output tokens agree to 1.1%. The missing requests are mostly cache reads and fresh input with little output, and they grow with subagent activity. Something Claude Code runs outside the transcripts makes them; OTel's `query_source` should say what, once it is live.
- **No session is over its cost-state.** Of 68 sessions with 20 or more requests, the closest was 0.002% under. So the dedupe double-counts nothing.

The duplicate `requestId`s are fork subagents copying their parent's history (150 requests, 856 records, all within a session), not resumed sessions. The ingest used to give 58 of those requests to the fork when its file name sorted before its parent's; it now reads forks after their parents. Session totals were never affected.

The check is `python tools/token-profile/reconcile`, with tolerances in [tools/token-profile/README.md § Reconciliation](../../../../tools/token-profile/README.md#reconciliation). It passes on this window.

## Before and after Wave 0 and Wave 1

The window after the first cut is two hours of one coordinator's Wave 2 dispatch, so these are early signals with small n, not results.

| Cut | Measure | Before | After |
|---|---|---|---|
| TP-03 (12:04:06) | First request, subagents | p50 64.6k, p90 75.3k (n=813, 2026-09-14 on) | p50 61.4k, max 62.2k (n=17) |
| TP-03 | First request, the same coordinator's workers | Wave 1 (spawned 11:45Z): 65.9k-66.1k (n=4; a fifth with a longer prompt, 74.4k) | Wave 2 (13:25Z): 60.9k-61.4k (n=6), about 5k less with prompts of similar size |
| TP-03 | First request, main sessions | p50 73.1k, p90 83.0k (n=93) | 83.0k (n=1), but that session's checkout was 16 commits behind `origin/main` and didn't load the TP-03 trims. The next main session is the first real measurement. |
| TP-02 (12:33:56) | `lane.sh` result characters, foreground | p50 490, p90 2,683, max 15,250 (n=1,599) | no calls yet (n=0) |
| TP-00 (11:39:49) | Requests per subagent transcript | p50 61, p90 204, max 1,035; 86 of 803 over 200 | p50 33, max 88; 0 of 22 over 200, with most still running |
| TP-00 | Peak context per subagent transcript | p50 238k, p90 500k, max 960k | p50 180k, max 395k (n=22, still running) |

TP-02 was measured at merge time from lane output itself (36,765 to about 270 characters for the same nextest run); it has no transcript data yet.

## Regenerate

```bash
python tools/token-profile/ingest --db <profile.sqlite> --repo . --fetch-prs
B=docs/analysis/token-usage/baseline
python tools/token-profile/report --db <profile.sqlite> --out $B --since 2026-09-14T00:00:00Z --until 2026-10-03T11:39:49Z
python tools/token-profile/reconcile --db <profile.sqlite> --out $B --since 2026-09-14T00:00:00Z --until 2026-10-03T11:39:49Z
python tools/token-profile/cutlines --db <profile.sqlite> --out $B
```

A later ingest changes the cut-line files (more "after" data) but not the baseline window, except that requests are reattributed as PRs merge.
