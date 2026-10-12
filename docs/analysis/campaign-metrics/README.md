# Campaign metrics

> Type: ledger. Audience: the coordinator and owner who want to know what each campaign cost, how clean it was, and whether the Haiku packet model beat the old model.
> Window: 2026-09-21 to 2026-10-12 (three weeks). All times UTC. Packets: [work-packets.md](work-packets.md). Written 2026-10-12 by the research agent; nothing is built yet.

## Purpose

Give every campaign in the window one committed, regenerable metrics record, and compare the **old model** (Opus coordinator plus Opus/Sonnet domain agents implementing) with the **Haiku packet model** (Haiku `packet-coder` implements, Sonnet `packet-reviewer` and `packet-review-fixer` review and fix, Opus coordinates and writes no code).

The work **extends** [`tools/token-profile/`](../../../tools/token-profile/README.md) (campaign #957, closed 2026-10-03). It already ingests transcripts, prices them, attributes requests to PRs and campaigns, and posts per-PR stats. This campaign adds the metrics it lacks: packet-level retries, escalation, stop context, CI pass rate, escaped defects, cycle time and role overstep. No sibling tool.

## What the research found

| Question | Answer | Evidence |
|---|---|---|
| Where are the transcripts? | `~/.claude/projects/c--Users-Steve-source-projects-Cimmeria/`: 186 session files plus 2,136 `subagents/agent-*.jsonl` (each with `.meta.json`). Worktree sessions land here too (the directory name is the main checkout's; the worktree is in `cwd`). Oldest file 2026-09-14. Other project folders (agentcraft, Rasa.NET) are not this repo's. | directory listing 2026-10-12 |
| Is there a cost tool already? | Yes. `python tools/token-profile/ingest` builds `~/token-profile.sqlite` (schema v5, 163,206 requests, 2026-09-13 to 2026-10-12 10:48Z). `report`, `reconcile`, `pr_stats`, `validate`, `scheduled` sit beside it. | [token-profile README](../../../tools/token-profile/README.md) |
| What does a transcript hold per message? | `message.model`, `message.usage` (input, output, cache creation with a 5m/1h split, cache read), `requestId`, `cwd`, `gitBranch`, `timestamp`, and `message.stop_reason`. The ingest keeps the first set but **drops `stop_reason`**, so worker stop context needs a schema change (CM-02). | one subagent transcript, 29 assistant records |
| When did the Haiku packet model start? | `packet-coder` first appears **2026-10-10T20:02:54Z**; `packet-reviewer` 20:05:06Z the same night. Memory `feedback_haiku_packets_sonnet_review` is dated 2026-10-10 and `feedback_opus_orchestrates_only` 2026-10-11 (adds Sonnet `packet-review-fixer`, first seen 2026-10-11T23:06Z). Haiku 5.5 requests: 1,574 on 10-10, 6,703 on 10-11, 4,632 on 10-12; Haiku `lab-driver` agents run from 10-10T15:25Z, so cheap lab driving predates the coder. Proposed switch: D-CM1. | `agents` table first-seen times |
| What is the model mix? | opus-5-5 103,930 requests; opus-5 17,307; sonnet-5 14,588; sonnet-5-5 10,179; haiku-5-5 12,909; fable-5-1 3,452; opus-4-8 812; haiku-4-5 29. | `requests` table |
| Is CI history reachable? | Not through `gh run list`: 1,000 runs only reach back to 2026-10-11T19:04Z. Per-PR check data comes from `gh pr checks` / the check-runs API on the PR's head commits (CM-04). | `gh run list --limit 1000` |
| Can requests be tied to campaigns? | Partly. `campaigns.json` tags 17 campaigns; in the DB **1,071 of the PRs since 09-13 have no campaign** because most branches are `feat/`, `fix/`, `docs/` (and 2026-10-10+ campaigns are missing from the file). Ledger packet tables name each PR (`Integrated (#1527)`), which is a second, better source (CM-05). | `prs.campaign`, ledger tables |
| How much spend cannot be attributed? | 25,615 of 163,206 requests (15.7% by count) have method `unattributed`; another 15,605 are `campaign` rows (packet work with no PR, still assigned to a campaign). `packet-coder` requests carry `gitBranch = main` in 346 of 393 agents, so they are placed by worktree and ancestry, not branch. The profiler's USD is also a **floor**: about 10% under Claude Code's own cost-state (requests no transcript records). | `pr_attribution`, [reconcile](../../../tools/token-profile/README.md#reconciliation) |
| Pricing source | `tools/token-profile/ingest/prices.py` (versioned table, source cited: the Anthropic pricing page; Haiku 5.5 has a 5x long-prompt card over 100K). One place already; D-CM2 keeps it. Not re-verified against the live page in this pass (see D-CM2). | file |

## Metrics

All dollars are the profiler's **list-price estimate of plan usage** (D-TP1: the project is on a Max subscription), never a bill, and a floor. Every metric is computed per campaign and per regime. A metric with missing inputs says `n/a` and why, never `0`.

| Metric | Formula | Source | Known gaps |
|---|---|---|---|
| Campaign cost | sum of `estimate_usd` over requests with `pr_attribution.campaign = C` (PR rows via `prs.campaign`, plus `campaign` rows), weights applied | token-profile DB | unattributed share reported beside it; cost-state floor |
| Cost by role | the same, split into coordinator (main session), `packet-coder`, `packet-reviewer`, `packet-review-fixer`, other named agents, other | `requests` + `agents` | teammates without an agent definition show as `teammate` |
| PRs | count of PRs in the campaign (ledger PR list union tagged PRs), and merged count | `prs`, ledger tables | PRs for packets that landed via an integration branch count under the integration PR once (D-TP7) |
| Cost per PR | campaign cost attributed to PRs / PR count (also p50, p90 per PR) | DB | |
| Cost per accepted change | campaign cost / accepted packets (ledger status Integrated, UATPending or Done) | DB + ledger | campaigns with no packet table fall back to merged PRs, flagged |
| First-pass completion | accepted packets with exactly **one** implementer session, **zero** reviewer-requested fix commits and no escalation / total accepted packets | transcripts + git + ledger | reviewer findings live in the coordinator's chat, so "fix commits" = commits on the packet branch after the first `packet-reviewer` start |
| Retries and why | implementer sessions beyond the first per packet, each classified: `context-exhausted`, `stalled-background`, `permission-denied`, `review-fix`, `ci-failure`, `merge-conflict`, `other` | transcripts (stop context) + worknote / ledger text | text classification is keyword-based; unmatched = `other` |
| CI pass rate | first-attempt = share of PRs whose **first** pushed head commit had a green `ci gate`; overall = share of PRs whose final head is green at merge. CI re-runs per PR also reported | GitHub check runs per head SHA | merge-queue runs counted separately; docs-only PRs run fewer jobs, so rates are also given for code PRs only |
| Escaped defects | PRs and issues, not in campaign C, that fix a defect traceable to C: (a) title starts `fix`/`revert` and body or title cites a merged PR of C by `#N`; (b) a closed bug issue that cites such a PR; (c) a revert commit of a C commit. Rate = escaped / merged PRs of C | `gh pr list`, `gh issue list`, git log | untraced defects (no citation) are invisible; the count is a floor |
| Escalation rate | packets whose implementer was moved up a tier (Haiku to Sonnet, `rust-gameserver-dev`, or the coordinator finishing) / packets dispatched to Haiku | ledger Implementer column + transcripts (Haiku agent then a different agent in the same worktree) + coordinator edits in the worktree | |
| Worker stop context | per worker session: why it stopped (`end_turn` final message, `tool_use` mid-call, error, no final record) and the context tokens of its last request, plus the share that stopped above 90k (Haiku 5.5's 100K cliff) | `agent_stops` (CM-02) | "why" for a mid-call stop is inferred (stalled background job if the last tool call was a backgrounded `lane`; permission denial if the last tool result was an error mentioning denial) |
| Cycle time | packet: first worker request in its worktree to PR `mergedAt`; campaign: first ledger commit to the close-out PR `mergedAt` | transcripts, git, `gh` | idle time inside waits (CI, owner) included; a `queue wait` split is reported where merge-queue timestamps exist |
| Overstep | requests by a role doing another role's job, repriced at the tier that should have done it (see CM-07) | transcripts | an estimate: same tokens at lower-tier prices, ignoring that a cheaper model may need more turns |

## Campaigns in the window

PR counts are from `gh pr list --search "created:>=2026-09-21"` (823 PRs) grouped by branch prefix, 2026-10-12; they are a starting point. CM-01 and CM-05 produce the real list from the ledger folders (47 under `docs/analysis/`). "Regime" is assigned by CM-01 from where `packet-coder` ran; the column below is the expectation.

| Campaign (ledger folder) | Branch prefix | PRs (merged) | Dates | Regime |
|---|---|---:|---|---|
| dialog-ui-redesign | `dialog-ui/` | 8 (8) | 09-21..09-25 | old |
| ability-trees | `trees/` | 9 (9) | 09-25..09-27 | old |
| npc-ai-restoration | `npcai/` | 36 (36) | 09-25..09-27 | old |
| social-systems | `social/` | 18 (18) | 09-27 | old |
| crafting | `craft/` | 18 (18) | 09-27..09-28 | old |
| bank-vault | `bank/` | 12 (12) | 09-27 | old |
| pets | `pets/` | 12 (12) | 09-27 | old |
| organizations | `org/` | 11 (11) | 09-27 | old |
| black-market | `bm/` | 10 (10) | 09-27..09-28 | old |
| ammo | `ammo/` | 16 (12) | 09-28..09-29 | old |
| castle-rebuild / castle-population / castle-cellblock-rebuild / harset-rebuild | `castle/`, `content/…` | ~16 | 09-21..10-06 | old |
| token-usage | `#957` | 22 (22) | 10-03 | old |
| ability-mechanics | `abilities/` | 13 (13) | 10-03..10-04 | old |
| debug-area | `debugarea/` | 7 (7) | 10-04..10-05 | old |
| launcher-consolidation (earlier launcher work) | `launcher/` | 5 (5) | 10-04..10-05 | old |
| class-start-v6 | `cs6/` | 7 (7) | 10-05..10-10 | old |
| dakara-e1-rebuild | `dakara/` | 4 (4) | 10-06..10-08 | old |
| legacy-command-parity, client-limits' predecessors, security fixes | various | ~20 | 09-27..10-10 | old |
| lab-parallel-clients | `lab/`, `feat/` (#1312) | ~6 | 10-10..10-11 | mixed: the first Haiku coders |
| lab-spec-vocab | `lab-spec-vocab/` | 17 (17) | 10-11 | Haiku |
| lab-fixtures | `lab-fixtures/` | 14 (14) | 10-11 | Haiku |
| lab-golden | `lab-golden/` | 11 (11) | 10-11 | Haiku |
| lab-chaos | `lab-chaos/` | 9 (9) | 10-11 | Haiku |
| lab-record | `lab-record/` | 9 (9) | 10-11 | Haiku |
| lab-watch | `lab-watch/` | 5 (5) | 10-11 | Haiku |
| client-limits | `client-limits/` | 11 (11) | 10-11 | Haiku |
| telemetry-gaps | `telemetry-gaps/` | 150 (150) | 10-11 | Haiku (24 coder agents on this prefix; the 150 PRs in one day need checking: many are likely small batch PRs) |
| wireclient-ci | `wireclient-ci/` | 21 (21) | 10-11 | Haiku |
| launcher-consolidation (desktop launcher) | `launcher-consolidation/` | 25 (25) | 10-11 | Haiku |
| lab-cli-rust | `lab-cli-rust/` | 3 (2) | 10-11..10-12 | Haiku, one PR open |
| lab-harness, haiku-lab-uat | `lab-harness/` | 1 + ledger PR | 10-12 | Haiku |
| any-install-adoption | `any-install-adoption/` | 1 (1) | 10-12 | Haiku (23 coder agents) |
| campaign-metrics (this one) | `campaign-metrics/` | this campaign | 10-12 | Haiku packets, own cost owed at close-out |

Not campaigns, reported as one line each: generic `feat/`, `fix/`, `docs/`, `ci/`, `dependabot/` PRs with no ledger link (CM-05 decides which ones a ledger claims); `worktree-agent-*` PRs.

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-CM1 | Proposed | Regime switch = **2026-10-10T20:02:54Z**, the first `packet-coder` transcript. Per packet, regime is the implementer that actually ran (a packet touched by `packet-coder` is Haiku regime), so mixed campaigns split cleanly. Old = Opus/Sonnet/`rust-gameserver-dev` implementing, anything started before the switch. | Memory dates say 10-10/10-11; the transcript is the evidence. Owner to confirm. |
| D-CM2 | Proposed | One price table: keep `tools/token-profile/ingest/prices.py` (versioned, source URL, Haiku long card). Do not add a second config. A new price list is a new key there. CM-03 re-checks the current table against the pricing page before the final run. | A second table would drift from the reconcile checks (0.03% residual today). The lead asked for "one config file"; this is that file. |
| D-CM3 | Proposed | Dollars are labelled "list-price estimate, a floor" everywhere. Where Claude Code's cost-state covers a window, the summary shows it next to the profiler total (D-TP6). | Inherited from D-TP1 and D-TP6. |
| D-CM4 | Proposed | Unattributed spend is reported per campaign and in total, never spread. Campaign-level cost-per-X is computed on attributed spend only and says so. | Inherited from attribution invariants. |
| D-CM5 | Proposed | Committed outputs are aggregate-only and pass the existing privacy gate (`report/scrub.py`): campaign names are ledger folders, no branches, session ids, prompts, absolute paths. Per-packet rows use the ledger packet ID. | The repo is public. |
| D-CM6 | Proposed | The ledger's packet table is the authority for packet identity, size, implementer and status; transcripts and git add evidence. Where they disagree the report prints both and flags it. | "A ticket is a claim, not evidence." |
| D-CM7 | Proposed | Overstep rules (CM-07): Opus main sessions editing code paths or running implementation lanes in the Haiku regime; Opus or Sonnet research in the coordinator session; Sonnet implementing a packet whose ledger implementer is `packet-coder` without a recorded escalation. Ledger/docs edits by the coordinator are allowed. Repricing target: Haiku for implementation, Sonnet for review and research. | The user's requirement. Thresholds are a starting point; owner may tune. |
| D-CM8 | Proposed | The report may be rerun later. Committed data lives in `docs/analysis/campaign-metrics/data/` and `reports/`; the DB and raw transcripts stay out of the repo. A scheduled rerun is out of scope. | Keeps this campaign finite. |
| D-CM9 | BlockedDecision | Do we post per-campaign numbers to the board (`handoffs`)? Owner call at close-out. | Outward-facing. |

## Packets

Status words: Ready / BlockedDependency / BlockedDecision / Writing / Review / Integrated / UATPending / Done. Spec: [work-packets.md](work-packets.md).

| ID | Packet | Implementer | Size | Wave | Depends on | Status |
|---|---|---|---|---|---|---|
| CM-01 | Campaign registry, regime config, missing campaign tags | packet-coder | S | 1 | none | Ready |
| CM-02 | Schema v6: `stop_reason` and `agent_stops` | packet-coder (3 sessions max, then Sonnet) | M | 1 | none | Ready |
| CM-04 | GitHub collector: PRs, CI per head commit, escaped defects | packet-coder | M | 1 | none | Ready |
| CM-05 | Ledger parser: packets, PR links, implementer, retry and escalation text | packet-coder | S | 1 | none | Ready |
| CM-03 | Cost rollup: by campaign, role, PR, accepted change | packet-coder | M | 2 | CM-01 | BlockedDependency |
| CM-06 | Packet join: retries, first-pass, escalation, stop context, cycle time | packet-coder (3 sessions max, then Sonnet) | M | 2 | CM-01, CM-02, CM-05 | BlockedDependency |
| CM-07 | Overstep detection and wasted-cost estimate | packet-coder | M | 2 | CM-01, CM-05 | BlockedDependency |
| CM-08 | Report generator: per-campaign files, summary, pre/post comparison | packet-coder | M | 3 | CM-03, CM-04, CM-06, CM-07 | BlockedDependency |
| CM-09 | Docs: tools catalog row, guide section, doc-update rows | documentation-writer | S | 3 | CM-08 interface frozen | BlockedDependency |
| CM-10 | Close-out: run, review numbers, this campaign's own cost, status, handoff | coordinator + documentation-writer | M | 4 | all | BlockedDependency |

CM-01, CM-02, CM-04 and CM-05 touch different files and can run in parallel in separate worktrees. CM-03, CM-06 and CM-07 share only the `metrics/` package `__init__.py`, which CM-01 creates.

## Resume note

Nothing is built. Next step: owner answers D-CM1 and D-CM2, then dispatch wave 1. Re-ingest `~/token-profile.sqlite` right before CM-10 so the window is complete. The campaign's own transcripts are in the coordinator session; CM-10 names it with `--self-session`.
