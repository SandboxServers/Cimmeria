# Campaign metrics findings

> Type: findings. Audience: the owner and the coordinator who asked whether the Haiku packet model beat the old model and what the campaigns cost. Written 2026-10-12 from the close-out run. Per-campaign numbers: [reports/summary.md](reports/summary.md) and the page per campaign beside it; data: [data/](data/); ledger: [README.md](README.md).

Dollars are the profiler's list-price estimate of plan usage, a floor, never a bill. Window 2026-09-21 to 2026-10-12, price table 2026-10-12, database snapshot 2026-10-12 18:50Z.

## Headline answers

| Question | Answer |
|---|---|
| Cost per merged PR | Old: mean $22.01, p50 $18.18 over 248 PRs in 28 pure campaigns ($5,727.50). Haiku: mean $2.67, p50 $0.91 over 185 PRs in 7 pure campaigns ($594.06). |
| Cost per accepted change | Old $18.65, Haiku $2.35. |
| Per 1,000 changed lines | Old $7.44, Haiku $3.03 (additions plus deletions of merged PRs, recomputed from the database): 2.5 times cheaper, not the 8 to 20 times the per-PR figures suggest. Median PR: old 1,941 changed lines, Haiku 169. Median of per-PR cost per 1k lines (PRs of 20+ lines): old $9.43, Haiku $5.64. |
| First-pass completion | 4.8% of 125 judged accepted Haiku-regime packets (the report prints 5.0%). Low because almost every packet needed a review fix. Old: not counted. |
| Retries per accepted packet | 0.60 (Haiku regime). Old: not counted. |
| CI pass rate | 100% first attempt, on 21 PRs merged since 2026-10-11 20:06Z. Old: unmeasured. |
| Escaped defects per merged PR | Old 0.04, Haiku 0.01. A floor: only defects that cite the PR they fix are seen. |
| Escalation rate | 16.9% of Haiku-regime packets (44 of 261): 14 context-exhausted, 30 unclassified. An upper bound. |
| Packet cycle time | p50 1.1 h in both regimes. Includes waits for CI, the merge queue and the owner. |
| Where the money goes | Haiku regime: Opus 75%, Sonnet 22%, Haiku 2%. Old regime: Opus 93%. The Haiku model moved implementation off Opus; coordination and review still cost most. |

### What the comparison does and does not say

- **Units differ.** Haiku packets are sliced to fit a 90k context, so PRs are about a tenth the size. Cost per PR and per packet overstates the saving. Per 1,000 changed lines is fairer and still favours Haiku 2.5 times. Changed lines include docs, tests and generated files, so they proxy work, not value.
- **Work differs.** Old campaigns include design, research, owner-UAT waits and rework. Haiku campaigns implement from a finished spec whose writing cost sits in the Opus coordinator, which is why Opus is still 75% of the Haiku regime.
- **The quality columns use a wider base than the cost columns.** Cost, PR and CI figures use pure campaigns only. First pass, retries and escalation use every packet whose implementer was Haiku, including 59 packets from nine mixed campaigns (261 packets, 223 accepted). Pure Haiku campaigns alone: first pass 6.0% (6 of 100 judged), escalation 15.3% (31 of 202), retries per accepted 0.61. The summary footnote says mixed campaigns are excluded from the cost comparison; it should say the quality rates are not.
- **PR counts are PRs with attributed spend.** wireclient-ci shows 16 where GitHub has 21 merged (spot check 1). Mean cost per PR is slightly high as a result.

## Where Opus or Sonnet overstepped in the Haiku campaigns

Rules (CM-07, D-CM7): O1 an Opus main session editing a code path or running a build lane; O2 an Opus main session researching; O3 a Sonnet main session implementing; O3b an Opus agent implementing a packet whose ledger implementer is packet-coder with no recorded escalation. Waste is actual spend minus the same tokens priced at the intended tier (Haiku to implement, Sonnet to research). Totals cover every post-switch campaign, pure and mixed.

| Rule | Hits | Actual USD | Recoverable at the intended tier |
|---|---:|---:|---:|
| O1 Opus coding in the main session | 395 | $76.48 | $59.93 |
| O3b Opus implementing a Haiku packet | 309 | $27.43 | $22.51 |
| O2 Opus research in the main session | 42 | $3.52 | $1.76 |
| O3 Sonnet implementing | 0 | $0.00 | $0.00 |
| Total | 746 | $107.43 | $53.72 to $84.20 |

- Inside the Haiku-regime campaigns (the report headline): $40.07 to $63.29, 6.7% to 10.7% of their $594.06. The other $13.65 to $20.91 sits in the unattributed bucket.
- Largest campaigns by the high figure: lab-fixtures $12.58 (139 O3b hits), wireclient-ci $9.95, lab-spec-vocab $7.93, lab-chaos $7.77, telemetry-gaps $7.06, lab-record $4.04. lab-parallel-clients, the first Haiku night, shows 15 O1 hits ($2.36) in the first half hour after the first packet-coder.
- O1 and O2 are direct observations of tool calls. O3b is an upper bound.
- The lower-tier saving is small next to Opus's 75% share of Haiku-regime spend (about $446), which is coordination, spec writing and cache reads rather than rule-breaking.

## Packet sizing against the 90k Haiku cap

A hook caps Haiku workers at 90k tokens of total context. About 35 to 40k is fixed overhead (system prompt, CLAUDE.md, memory indexes), leaving roughly 40k of working budget. The packets were first written to the campaign-packet skill's "under 100k" guideline. Verified from the agent records (cm0Xx-a1 is the first attempt, -a2 the retry):

| Packet | Size | First attempt, peak context | Outcome |
|---|---|---:|---|
| CM-01 | S | 74.6k | done |
| CM-05 | S | 84.9k | done |
| CM-02 | M | 92.4k | overflow; a2 finished at 52.1k |
| CM-03 | S | 99.4k | overflow; a2 finished at 48.2k |
| CM-04 | M | 90.5k | overflow; a2 finished at 26.8k |
| CM-06 | M | 95.6k | overflow; a2 finished at 77.5k |
| CM-07 | M | 91.7k | overflow; a2 finished at 76.2k |

- 5 of 7 first attempts overflowed, CM-03 although sized S.
- **Four fix rounds also reached 90k** (cm04-fix1a, cm04-fix1b, cm05-fix1, cm06-fix1b); three were re-run as continuations (cm04-fix1a2, cm04-fix1b2, cm06-fix1b2). cm06-fix1b ended with no final record at 92.5k.
- CM-08 and CM-09 were then re-sliced into 13 packets of about 15 to 28k working tokens, contract excerpt inlined. **All 13 first attempts finished first time**, peak context 41.8k to 75.6k. None of the 36 packet-coder sessions of CM-08, CM-09 and CM-10 reached 90k; 9 of the 34 before the re-slice did.
- Rule to carry forward: size a Haiku packet to about 40k working tokens, allow no M size, inline what it needs.

## Figures checked

| Figure | Verdict |
|---|---|
| CI first attempt: old n/a, Haiku 100.0% | Real but weak, not wrong. GitHub run history only reaches back to 2026-10-11T19:04Z (1,000-run limit), so 21 of 823 PRs have run data, all merged since 2026-10-11 20:06Z and all green first time (telemetry-gaps 7, lab-chaos 5, lab-cli-rust 3, lab-record 3, three others one each; none old-regime). The old column is unmeasured, not failing. The merge queue (live 2026-10-11) only lets green PRs land, so 100% on n=21 merged PRs shows no CI trouble; it is not a pass rate. |
| Escalation 17% | Reproduced: 44 of 261 Haiku-regime packets (16.9%), including mixed-campaign packets. Pure Haiku: 31 of 202 (15.3%), of which 18 are lab-harness, where 18 of 19 packets ran under general-purpose agents although the ledger planned Haiku (the ledger does not say whether that was a deliberate tier change). Reasons: 14 context-exhausted, 30 other. |
| Retries 0.60 per accepted packet | Reproduced (0.601 all Haiku-regime, 0.606 pure). The "other" class (77 of 114 retries in pure Haiku) is inflated because helper agents running in a packet's worktree count as implementer sessions. In this campaign, 9 of the 11 "other" retries are CM-10 helper agents and 2 are merge helpers counted under CM-04. By hand this campaign had 10 retries (5 first attempts that overflowed and were re-run, 5 fix-worker continuations), not the 16 reported. |
| First pass 5.0% | Reproduced (6 of 125 judged = 4.8%; 6 of 100 pure). A packet is a first pass only with one implementer session, no escalation and no commit after the first reviewer starts; 17 of this campaign's 20 packets needed a fix round. 88 of 188 accepted pure-Haiku packets are not judged (no PR commit data or no reviewer), so this is a rate over reviewed packets. |
| Per-unit costs | Recomputed from the database: $7.44 and $3.03 per 1,000 changed lines, matching the earlier review. |
| PR counts | Count PRs with attributed spend; wireclient-ci 16 against 21 merged. |
| data/campaign-metrics.json | The generated record shows 0 accepted packets with gap "no packet table": the campaign has no PR-attributed requests until its PR merges, so the cost rollup emits no accepted count for it. The hand count is authoritative: 20 packets accepted (CM-01 to CM-09b), plus CM-10 at merge. |

## This campaign's own cost

The report's "Cost of this campaign" prints $44.32 under "coordinator". Checked against the database, that is the whole coordinator session including every worker, reviewer and research agent: all 2,030 requests. The campaign has no merged PR yet, so nothing was attributed to it elsewhere and nothing was excluded. Only the role label is wrong. The split below leaves out the copilot-research agent ($0.35, 6 requests), which was a separate question, not part of this campaign.

| Role | Model | Agents | Requests | USD | Share |
|---|---|---:|---:|---:|---:|
| Coordinator, main thread | Opus | 1 | 334 | $26.05 | 59.2% |
| Reviewers (packet-reviewer) | Sonnet | 8 | 346 | $13.58 | 30.9% |
| Research, slicer, price check | Sonnet | 3 | 47 | $2.71 | 6.2% |
| Packet-coders, first attempts (a1, a2) | Haiku | 25 | 511 | $0.69 | 1.6% |
| Packet-coders, fix rounds | Haiku | 34 | 559 | $0.72 | 1.6% |
| Packet-coders, CM-10 runs and fixes | Haiku | 11 | 192 | $0.19 | 0.4% |
| Packet-coders, merge helpers | Haiku | 4 | 35 | $0.03 | 0.1% |
| Total | | 86 | 2,024 | $43.97 | |

By tier: Opus $26.05, Sonnet $16.29, Haiku $1.63.

- **Packets accepted:** 20 (CM-01 to CM-07, CM-08a to CM-08k, CM-09a, CM-09b) (CM-10 completes at merge). Cost per accepted packet: $2.20 all in, including research, slicing, the close-out runs and the coordinator.
- **Retries:** 10 (5 first attempts that overflowed 90k and were re-run; 5 fix-worker continuations). No packet was escalated and none needed a third implementer attempt.
- **Haiku sessions at or above 90k:** 9 of 34 before the re-slice (5 first attempts, 4 fix rounds); 0 of 36 after.
- **Review rounds:** 8 reviewer sessions (one each for CM-01 to CM-07; one for the 13 CM-08 and CM-09 packets together). Six were resumed for a second round after fixes, CM-07's more than once; this is inferred from gaps in their requests. 17 of 20 packets needed at least one fix round (CM-08c, CM-08d and CM-08e did not).
- **Escaped defect:** one. The first real run crashed in the report on the real unattributed-share shape because the test fixtures did not match real profiler output. Fixed in the same campaign ("report flattens the real unattributed-share shape; fixtures match real data"). The real-data run also caught before merge a wrong Sonnet 5.5 cache-read price, an overstep roll-up that skipped mixed campaigns, mismatched comparison denominators and a first-pass rule that counted merges and rebased commits; these count as caught in close-out, not escaped.
- **Span:** first worker started 16:18Z; last record in the snapshot 18:49Z.
- **Measured at close-out. It excludes the remaining close-out tail** (this write-up, its review and the merge), so the final figure is a little higher.

### Did the coordinator or Sonnet overstep in this campaign?

No code was written above the Haiku tier. From the tool calls in the database:

- **Opus main thread:** 0 Edit calls, 8 Write calls (all to paths outside the repository), 45 PowerShell calls (worktree creation with the lane scripts, git status and merge steps, a cache clean-up, one issue read), no cargo or lane build, no edit of a code path. The overstep detector has no hit for this session and no campaign-metrics entry (its latest hit predates the session), so this comes from the tool calls, not a report row. The Opus cost is coordination: 86 Agent and 50 SendMessage calls.
- **Sonnet:** research, packet slicing, the price-table check and eight reviews. Its writes were documentation (the ledger and packet spec, slicing notes) and two review-note files. No Sonnet agent edited a crate.
- **Packet-coders** made all code edits (517 Edit, 77 Write).
- Opus also made a few git merges and one data-file copy; ledger and integration work, allowed by D-CM7.

## Spot checks

1. **Merged-PR count, wireclient-ci.** GitHub lists 21 merged PRs on the wireclient-ci/ branch prefix; the profiler's PR table has the same 21, all tagged to the campaign. The report counts 16. The five missing (#1363, #1398, #1412, #1473, #1636: four docs ledger updates and one CI config fix) have no priced request, so the report drops them. Mismatch found and explained. For telemetry-gaps, GitHub and the report agree at 150.
2. **A Haiku packet's first pass and retries, WC-02.** Ledger: Done, packet-coder, size M, PR #1359. Database: one packet-coder session, one packet-reviewer session, no fix agent, so implementer 1, reviewer 1, no retries. The PR has two commits: 01:37:22Z (before the reviewer started at 01:37:42Z) and 01:40:44Z (after). One fix commit, first pass false, matching the report row.
3. **An overstep item, lab-parallel-clients.** The first O1 item (2026-10-10 20:08:25.865Z, Opus, Edit on a crates path, $0.1459) is one main-session Opus request that made three Edit calls to files under crates/lab. The request itself prices at $0.0468 with the report's price table; the remaining $0.0991 is the follow-up requests' cache cost, which the rule adds to a hit by design. The first packet-coder in the database is 20:02:54Z, so the item falls five minutes into the Haiku regime. Matches.

## Known limits

- **38% of sessions unmapped** to a packet (486 of 1,275), so per-packet figures cover the mapped 62%.
- **12.2% of spend unattributed**, and **26.5% of spend sits on PRs with no campaign** (280 untagged PRs). Neither is spread over campaigns, so every campaign cost is a floor.
- **List-price floor.** The profiler's dollars run about 10% under Claude Code's own cost totals (requests no transcript records) and are list price, not plan cost.
- **155 transcript records** had a shape the ingest does not know (first full run); they carry no tokens and are in the unknown-shapes table.
- **Small samples:** CI (21 PRs), first pass (reviewed packets only), escalation (upper bound), escaped defects (only those citing their PR). Every n/a in the old column is a gap in what was measured, not a zero.
- **Mid-call stop reasons are inferred.** Of this campaign's 81 workers (74 packet-coder, 7 others with transcripts mapped), 79 ended on end_turn; one was still running at the snapshot (the close-out run) and one has no final record (cm06-fix1b, 92.5k).
