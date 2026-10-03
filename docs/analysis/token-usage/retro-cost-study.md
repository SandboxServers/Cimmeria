# Retro Cost Study

> Type: explanation. Audience: the token-usage coordinator, for Wave 3 and the TP-12 close-out.
> Packet TP-11 of [#957](https://github.com/SandboxServers/Cimmeria/issues/957); ledger [README.md](README.md); handoff [worknotes/TP-11.md](worknotes/TP-11.md). Written 2026-10-03.

## What this is

Every PR merged from 2026-09-13 to 2026-10-03 (426), with its spend from the profiler's attribution, tagged by type, campaign, size and quality signals. It answers what each kind of work costs here, what pushes a PR over that, whether the extra spend bought quality, and which levers are left after Waves 0-2.

**Read every dollar figure as a floor.** USD is list price, a plan-usage proxy, not a bill (D-TP1), and the profiler is about 10% under Claude Code's own total (D-TP6). An integration PR carries only its own spend; a campaign's cost is its PRs plus the packet work that has no PR (D-TP7).

## Data and method

- **Spend.** Profiler database schema 4, ingested 2026-10-03 at about 16:20 UTC: 91,717 requests, $11,379 in all. Attribution is TP-05b's (precision 0.99 by USD on independent labels). The 426 PRs carry $7,641 of it (67%); the rest is campaign packet work with no PR, other PRs and the unattributed 13.5%. A PR's spend is the weighted sum of its `pr_attribution` rows over its whole history, the same number its stats comment shows.
- **Quality.** From GitHub, cached once (one GraphQL pass over the merged PRs, about 30 calls), with the same definitions as `pr_stats`, checked equal on 6 PRs: CI rounds are head commits that ran a workflow, failed rounds those with a failed, timed-out or unstarted run; review rounds are commits that received a submitted review (bots included); a follow-up fix is a merged `fix:` PR that cross-references the PR after it merged; a revert is a merged `Revert` PR that does.
- **Size.** GitHub's additions plus deletions, and changed files.
- **Window caveats.** Transcripts start on 2026-09-14, so PRs merged in the first days are undercounted. 20 PRs have no attributed spend (dependency bumps and human-only work); the comparisons drop the 32 under $0.50, leaving 394. All Wave 0-2 cuts merged on 2026-10-03 between 11:39 and 13:57 UTC, so these numbers are the **before** for every cut.

### Type rule

From the title's conventional-commit prefix and scope, then the changed files (share of changed lines), first match wins:

| Order | Type | Rule |
|---|---|---|
| 1 | fix | Title starts `Revert`, or prefix `fix` / `hotfix` |
| 2 | test | Prefix `test` |
| 3 | content | Prefix `content` or `data`, or a scope starting `content` |
| 4 | RE | Scope `re`, a title naming RE or Ghidra, or ≥50% of lines under `docs/reverse-engineering/` |
| 5 | content | ≥50% of lines under `db/resources/`, `data/` or `entities/` |
| 6 | tooling | Prefix `build` / `ci`, a tools scope (`tools`, `build-lane`, `token-profile`, `ci`, `lint`, `docs-gen`), or ≥50% of lines under `tools/`, `.github/`, `bootstrap/` or agent and settings config |
| 7 | test | Prefix `chore` / `refactor` or none, and ≥50% of lines in test files |
| 8 | docs | Prefix `docs`, or ≥90% of lines in Markdown |
| 9 | feature | Prefix `feat` / `perf` |
| 10 | chore | Everything else (`chore`, `refactor`, unprefixed titles, dependency bumps) |

A feature that ships its own tests stays a feature. Rules 3 and 7 took their present form after a spot check of 32 PRs; the rule still files some unprefixed titles as chore (for example a navmesh feature).

## Cost by type

Per PR, all 426 (zero-spend PRs included, which pulls chore's median down). Cost per changed line counts PRs with at least 10 changed lines.

| Type | PRs | p50 | p75 | p90 | Max | Sum | $ per 1k lines, p50 / p90 | Lines p50 | CI failed / follow-up fix |
|---|---:|---:|---:|---:|---:|---:|---|---:|---|
| feature | 150 | $20.62 | $33.87 | $52.50 | $111 | $3,854 | 9 / 19 | 2,733 | 7% / 9% |
| fix | 91 | $10.10 | $16.86 | $35.29 | $149 | $1,372 | 14 / 35 | 664 | 4% / 18% |
| content | 24 | $20.52 | $51.24 | $77.26 | $121 | $837 | 21 / 46 | 1,558 | 17% / 8% |
| chore | 39 | $1.05 | $21.09 | $31.17 | $279 | $654 | 4 / 33 | 786 | 15% / 3% |
| docs | 68 | $3.22 | $7.50 | $12.33 | $29 | $367 | 19 / 55 | 166 | 1% / 3% |
| RE | 23 | $6.83 | $11.04 | $19.98 | $45 | $220 | 28 / 59 | 293 | 0% / 0% |
| test | 10 | $17.68 | $26.17 | $31.66 | $72 | $203 | 22 / 112 | 418 | 40% / 10% |
| tooling | 21 | $4.96 | $10.08 | $11.01 | $27 | $134 | 7 / 18 | 596 | 5% / 10% |
| **all** | **426** | **$11.38** | **$23.83** | **$41.09** | **$279** | **$7,641** | **10 / 40** | | |

Size explains more than type. By changed lines (394 PRs with spend): under 300 lines p50 $3.27 (n=96); 300-1.5k $10.56 (n=125); 1.5k-5k $23.23 (n=142); over 5k $34.44 (n=31). Cost grows with the context a PR's work runs at, too: the median cost per request is $0.062 for PRs that peaked under 200k tokens (n=18) and $0.118 for those that peaked over 600k (n=130).

Quality signals are rare overall: 34 failed CI rounds and 301 review rounds over 706 CI rounds; 37 PRs drew a follow-up fix, none was reverted. The 31 follow-up fix PRs inside the window cost $496 (p50 $11.69), 6.5% of PR spend: the price of the defects that shipped.

## Cost by campaign

PR spend plus packet work with no PR (D-TP7), largest first. 230 PRs, $2,623, belong to no campaign.

| Campaign | Total | PRs (n) | Packets, no PR |
|---|---:|---:|---:|
| harset-rebuild | $1,689 | $111 (3) | $1,578 |
| npc-ai-restoration | $817 | $817 (36) | $0 |
| castle-rebuild | $705 | $705 (12) | $0 |
| crafting | $522 | $522 (18) | $0 |
| social-systems | $456 | $456 (18) | $0 |
| pets | $427 | $427 (12) | $0 |
| bank-vault | $344 | $344 (12) | $0 |
| `pr-683` (untagged integration PR) | $327 | $14 (1) | $314 |
| legacy-command-parity | $319 | $306 (7) | $13 |
| black-market | $297 | $297 (10) | $0 |
| organizations | $275 | $275 (11) | $0 |
| dialog-ui-redesign | $202 | $202 (8) | $0 |
| `pr-825` (untagged integration PR) | $185 | $7 (1) | $178 |
| ability-trees | $160 | $160 (9) | $0 |
| token-usage | $107 | $107 (17) | $0 |
| ammo | $106 | $106 (14) | $0 |

Harset is the outlier: its packets merged into an integration branch, so 93% of its cost has no PR of its own.

## Teardowns

### How each share is computed

Per request, weighted by the request's attribution weight, then summed per PR and divided by the PR's spend. **The shares overlap** (an idle rewrite in an event-triggered turn counts in both), so they don't add to 100%.

| Share | Computation |
|---|---|
| Main | Spend on main-session (coordinator) requests |
| Static | Each transcript's first-request context, read again on every later request at the cache-read price (capped at the request's cost); for the first request, its write |
| Idle | Cache writes on requests after a gap longer than the TTL the write used (5m writes after more than 5 minutes, 1h writes after more than an hour), priced as writes |
| Chatter | Main-session requests in turns an event started (background completion, monitor event, idle notification, teammate or cross-session message, scheduled task) |
| Rebase | TP-09's episode heuristic from fingerprints: from a `git rebase` / `merge` / `pull` or `rebase-pr.sh` call through the next `git push`, at most 25 requests (an upper bound) |
| Mech | Requests at over 200k context whose every tool call is mechanical (git plumbing, `gh`, `ls`, `date`, `echo`, `SendMessage`, task tools): the spend above what the same request costs at a fresh worker's 70k |
| Read | Results of `Read`, `Grep` and shell `sed -n` / `cat` / `head` / `tail`: characters / 4, written once and read on every later request |
| Reread | The same, for a `Read` of a repo path the transcript had read before |
| Build | The same, for Bash results of cargo, the build lane and the test scripts |

### The 20 most expensive PRs

Quality is failed CI rounds / review rounds / follow-up fix PRs.

| PR | Type | Campaign | USD | Requests | Lines | Peak | Main | Static | Idle | Chatter | Rebase | Mech | Read | Quality |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| [#642](https://github.com/SandboxServers/Cimmeria/pull/642) | chore | legacy-command-parity | 279 | 1903 | 5,402 | 966k | 54% | 17% | 4% | 43% | 4% | 14% | 10% | 0 / 1 / 0 |
| [#652](https://github.com/SandboxServers/Cimmeria/pull/652) | fix | castle-rebuild | 149 | 472 | 3,804 | 636k | 8% | 10% | 43% | 3% | 6% | 9% | 6% | 0 / 2 / 0 |
| [#660](https://github.com/SandboxServers/Cimmeria/pull/660) | content | castle-rebuild | 121 | 508 | 4,676 | 703k | 5% | 13% | 18% | 5% | 9% | 20% | 11% | 0 / 2 / 0 |
| [#797](https://github.com/SandboxServers/Cimmeria/pull/797) | feature | npc-ai-restoration | 111 | 709 | 7,848 | 835k | 4% | 9% | 18% | 4% | 0% | 12% | 7% | 0 / 1 / 0 |
| [#700](https://github.com/SandboxServers/Cimmeria/pull/700) | feature | - | 103 | 460 | 3,754 | 644k | 10% | 14% | 0% | 6% | 0% | 10% | 19% | 0 / 2 / 1 |
| [#896](https://github.com/SandboxServers/Cimmeria/pull/896) | feature | pets | 81 | 498 | 3,733 | 801k | 10% | 9% | 31% | 10% | 16% | 35% | 5% | 0 / 7 / 0 |
| [#909](https://github.com/SandboxServers/Cimmeria/pull/909) | content | crafting | 79 | 430 | 3,049 | 654k | 10% | 8% | 43% | 10% | 36% | 32% | 6% | 0 / 3 / 0 |
| [#668](https://github.com/SandboxServers/Cimmeria/pull/668) | content | castle-rebuild | 78 | 382 | 5,045 | 611k | 1% | 15% | 11% | 1% | 7% | 11% | 6% | 0 / 3 / 0 |
| [#984](https://github.com/SandboxServers/Cimmeria/pull/984) | feature | black-market | 76 | 382 | 5,203 | 849k | 8% | 22% | 20% | 6% | 2% | 28% | 5% | 0 / 0 / 1 |
| [#659](https://github.com/SandboxServers/Cimmeria/pull/659) | content | castle-rebuild | 76 | 309 | 3,775 | 625k | 6% | 13% | 26% | 6% | 4% | 14% | 16% | 0 / 3 / 0 |
| [#667](https://github.com/SandboxServers/Cimmeria/pull/667) | content | castle-rebuild | 74 | 440 | 1,512 | 513k | 2% | 14% | 13% | 2% | 6% | 9% | 7% | 0 / 2 / 0 |
| [#816](https://github.com/SandboxServers/Cimmeria/pull/816) | test | npc-ai-restoration | 72 | 586 | 2,960 | 959k | 8% | 11% | 0% | 8% | 7% | 6% | 16% | 0 / 1 / 0 |
| [#646](https://github.com/SandboxServers/Cimmeria/pull/646) | content | castle-cellblock-rebuild | 72 | 981 | 6,954 | 776k | 22% | 20% | 1% | 10% | 7% | 13% | 16% | 0 / 0 / 0 |
| [#663](https://github.com/SandboxServers/Cimmeria/pull/663) | feature | castle-rebuild | 72 | 345 | 4,249 | 627k | 7% | 15% | 25% | 6% | 5% | 15% | 11% | 0 / 2 / 0 |
| [#644](https://github.com/SandboxServers/Cimmeria/pull/644) | fix | - | 71 | 668 | 4,483 | 498k | 38% | 19% | 0% | 15% | 2% | 10% | 20% | 0 / 1 / 0 |
| [#651](https://github.com/SandboxServers/Cimmeria/pull/651) | fix | castle-rebuild | 70 | 263 | 1,222 | 558k | 10% | 12% | 43% | 8% | 10% | 18% | 5% | 1 / 2 / 0 |
| [#893](https://github.com/SandboxServers/Cimmeria/pull/893) | feature | social-systems | 66 | 430 | 5,099 | 768k | 8% | 9% | 27% | 8% | 2% | 26% | 7% | 0 / 8 / 0 |
| [#662](https://github.com/SandboxServers/Cimmeria/pull/662) | feature | harset-rebuild | 63 | 356 | 21,434 | 564k | 52% | 15% | 2% | 39% | 1% | 11% | 15% | 0 / 3 / 0 |
| [#1046](https://github.com/SandboxServers/Cimmeria/pull/1046) | feature | - | 61 | 314 | 2,534 | 638k | 5% | 7% | 39% | 2% | 2% | 19% | 8% | 0 / 0 / 0 |
| [#872](https://github.com/SandboxServers/Cimmeria/pull/872) | feature | bank-vault | 61 | 525 | 3,072 | 655k | 19% | 14% | 29% | 17% | 3% | 23% | 7% | 0 / 8 / 0 |

Shares of spend, top 20 ($1,834) against all 426 PRs ($7,641):

| | Main | Static | Idle | Chatter | Rebase | Mech | Read | Reread | Build |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Top 20 | 17.6% | 13.5% | 18.5% | 13.1% | 6.3% | 16.0% | 10.1% | 0.8% | 0.4% |
| All PRs | 23.4% | 15.3% | 15.0% | 13.1% | 6.2% | 12.5% | 8.6% | 0.3% | 0.3% |

The top 20 ran a median 450 requests over 38 turns with 3 agents; the other PRs ran 119 requests over 8 turns with 1. Two shapes stand out:

- **Coordinator-heavy** (#642, #662, #644): 38-54% of the spend is the main session, and most of that is chatter, turns started by worker completions and messages at 500k-966k of context.
- **Parked workers** (#652, #651, #909, #1046, #896, #872): 29-43% is idle rewrites. A worker that waits more than five minutes for the coordinator, a review or a background job writes its whole 400k context again when it wakes.

### Where the idle rewrites come from

All spend, not only PRs: $1,498, 13.2% of the $11,379. By who wrote them:

| Agent and wake-up | USD | Rewrites | Mean context |
|---|---:|---:|---:|
| `rust-gameserver-dev` teammate, woken by a teammate message | $493 | 265 | 390k |
| `rust-gameserver-dev` teammate, woken by a background completion | $235 | 122 | 413k |
| main session, after a human prompt | $122 | 32 | 356k |
| `rust-gameserver-dev` background subagent, woken by a completion | $116 | 60 | 404k |
| `rust-gameserver-dev`, resumed by a new prompt | $106 | 61 | 364k |
| generic teammate, woken by a background completion | $37 | 20 | 381k |

Of the subagents' $1,315, gaps of 5-10 minutes cost $623 and 10-30 minutes $555: $1,178, 90%, would have stayed warm under a 1-hour TTL. In PR terms, 10.0 of the 15.0 points are writes in a turn an event started, 2.2 follow a build or test call, 1.1 a human prompt.

### Mechanical work at heavy contexts

All spend: 8,728 requests at over 200k context did only mechanical calls, costing $1,547, of which $1,272 (11.2% of all spend) is above what a 70k-token context would have paid. Subagents hold $905 of it, the main session $367. Each of these steps was its own request: `git status` (420 subagent requests, $119 over), `date` (759, $112), `git fetch` (254, $108), `git add` (769, $94), `ls` (699, $70), and for the coordinator `gh pr` (559, $60), `git fetch` (316, $58) and `SendMessage` (435, $54).

## Did the spend buy quality?

Within each type and size cell (under 300, 300-1.5k, 1.5k-5k and over 5k lines) with at least six PRs, the PRs were split at the cell's median spend. Pooled over the 15 cells (172 PRs each side; median size 1,107 and 1,168 lines):

| | Spend p50 | Requests p50 | Turns p50 | Agents p50 | CI failed | CI rounds | Review rounds | Follow-up fix |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Cheaper half | $7.74 | 90 | 6 | 1 | 6% | 1.53 | 0.48 | 10% |
| Dearer half | $23.51 | 196 | 12 | 2 | 6% | 1.79 | 1.07 | 9% |

**No.** At the same type and size, three times the spend came with the same CI failure and follow-up rates and twice the review rounds. The dearer half's extra spend is in idle rewrites (18.3% of its spend against 7.0%), mechanical steps (13.5% against 9.8%) and rebases (6.9% against 4.6%), while its static share (14.2% against 17.9%) and coordinator share (19.0% against 26.8%) are lower. More review rounds probably explain part of the extra turns: each round wakes the worker or starts a new one.

The largest cell says the same. Features of 1.5k-5k lines (48 a side): $15.70 against $35.39; 154 against 295 requests; idle 7.9% against 20.1%, rebase 3.9% against 9.2%; follow-up fixes 10% against 8%; review rounds 0.48 against 1.71.

Campaign PRs against ad hoc ones of the same type point the other way on CI, at a higher price: features p50 $23.36 against $18.65 with failed CI on 3% against 16% (n=107 and 43); content $58.18 against $12.29, 0% against 33% (n=12 each). Follow-up rates are the same for features (9% against 7%) and lower only for the small content sample.

**Confounders.** The cells are small, so read the pooled row and the feature cell, not single cells. Failed CI rounds are undercounted: a commit that was force-pushed away, for example by a rebase, is no longer among the PR's commits. Follow-up fixes count only `fix:` PRs that link back. Review rounds include the bots, and the Copilot review stopped on 2026-09-27. A PR is dear partly because it was hard; difficulty is not measured.

## What work should cost, and the levers

Typical costs are the cheaper half's medians where a cell is large enough, else the type's median. Lever status: **pulled** means a Wave 0-2 packet changed it (see the [cut-line log](README.md#cut-line-log)); every cut postdates this data, so none is measured yet.

| Work | Should cost about | Over that, because | Lever | Status |
|---|---|---|---|---|
| Feature, 1.5k-5k lines | $16 (n=48; dearer half $35) | Idle rewrites 20% against 8%; rebases 9% against 4%; twice the turns and agents | 1-hour TTL for workers that wait; no parked workers | Pulled for `rust-gameserver-dev` (TP-06), unmeasured; open for generic teammates |
| Feature, 300-1.5k lines | $6 (n=14; dearer half $18) | Same pattern | Same | Same |
| Fix, 300-1.5k lines | $6.40 (n=24; dearer half $13.80) | Fix type over p75 ($16.90): idle 22% against 2%, 231 against 57 requests | Same | Same |
| Content (seed and chain) | $12 ad hoc (n=12); $22 for 1.5k-5k lines (n=5) | Top quartile: 4 agents and 435 requests at p50; idle 19%, mech 17%, rebase 11% | Fewer review re-dispatches; script the PR mechanics | Open |
| Docs and plans | $3 (n=68) | Coordinator work at 450k context: main 46%, chatter 25% in the top quartile | Coordinator compaction at wave boundaries | Pulled (D-TP4), unmeasured |
| RE write-up | $7 (n=23) | Top quartile 216 against 60 requests, 647k against 296k peak | Ghidra MCP and `aggregate_logs` (D-TP10) | Pulled, small (experiment G) |
| Tooling | $5 (n=21) | Idle 26% in the top quartile | 1-hour TTL | Open (generic teammates) |

Across all work, from the largest open share down:

1. **Idle rewrites: 13.2% of all spend ($1,498), 90% of the subagent part avoidable with a 1-hour TTL.** TP-06 set it for `rust-gameserver-dev`, which wrote $950 of the subagents' $1,315 (72%). The database has no `rust-gameserver-dev` request after the cut, so the change is not yet seen working; the first campaign after it should show `cache_write_1h > 0` on those workers. Generic teammates are still on 5 minutes (none of 221 requests after the cut wrote a 1-hour cache), and 36% of their spend after the cut was idle rewrites (221 requests, $28.80; a small sample). **Wave 3:** verify TP-06, and either give teammates the 1-hour TTL or stop parking workers: D-TP4's one phase per worker is the rule; all of this data predates it.
2. **Mechanical steps at heavy contexts: 11.2% ($1,272), not pulled.** Mostly workers running one git command per request at 400k context. **Wave 3:** one script that stages, commits, pushes and opens or updates the PR, with its status as one line; batch read-only git checks into one call. `date` is the most frequent single step (759 requests) and worth a look at what workers use it for.
3. **Static context: 15.3% of PR spend, pulled in part.** TP-03 cut about 14k of an 83k main-session first request and 6k of a 64k subagent one, about 2 points. The user-level part (MCP servers, connectors, the personal memory index, skills) is still open.
4. **Coordinator chatter: 13.1% of PR spend, pulled.** TP-00's notification rules (report once, results not progress, batched cross-session messages) took effect at its cut; the half of #642 and #662 spent this way is the case to recheck.
5. **Reads: 8.6% of PR spend, pulled (TP-07).** Rereads of a path already read are only 0.3%, so what is left is the first read's size, which TP-07's rules target.
6. **Rebases: 6.2% of PR spend, pulled (TP-09, D-TP8).** It is 9% in the dearer features, so recheck there.
7. **Build and test output: 0.3% of PR spend, pulled (TP-02).** Small before the cut; nothing left to take.
8. **Defects that shipped: $496 in 31 follow-up fix PRs (6.5% of PR spend).** No spend lever; spending more per PR did not lower it.

## Reproducing

The analysis scripts were scratch and are not committed: they read the profiler database and a GitHub cache that hold titles and branch names. To redo it, open the database with `report.db.open_db` and `scope`, take each PR's rows from `report.sections.prs.pr_records` (which leaves `temp.report_pa`, one row per attributed request), join `tool_calls` and `triggers` per request, and apply the computations above; quality comes from one GraphQL pass over merged PRs with the fields `pr_stats` reads. The handoff note has the details.
