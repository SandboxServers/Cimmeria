# TP-05b worknote: attribution validation

> Type: worknote. Packet TP-05b of [#957](https://github.com/SandboxServers/Cimmeria/issues/957); ledger [README.md](../README.md). Written 2026-10-03 by the TP-05b worker. Branch `feat/token-attribution-validation`. Implements D-TP6 and D-TP7.

## Done

- **Validation command**, `tools/token-profile/validate/` (`python tools/token-profile/validate --db <db> [--labels <csv>] [--truth-db <db>]`). It scores attribution against two ground-truth sets, by USD and by request count: precision, recall, and the wrong, campaign and unattributed shares, with the wrong share per method. The sets are described in [attribution.md § Validating the rules](../../../../tools/token-profile/attribution.md#validating-the-rules). The labels file stays local; the output holds shares only.
- **Five failure classes found and fixed**, each with a test that fails when its fix is reverted (13 reverts checked, all caught). See [Failure classes](#failure-classes).
- **D-TP7.** A3 no longer charges packet work to an integration PR: a packet commit that reached the PR's head through a merge (not on its first-parent chain) goes to a `campaign` row. Campaigns are tagged from [`campaigns.json`](../../../../tools/token-profile/campaigns.json): branch, branch prefix, tracking issue in the title, base branch. An untagged integration PR forms its own campaign, `pr-<N>`. The report has a new "Cost per campaign" section: PR spend plus packet work with no PR. The per-PR comment carries `campaign`.
- **D-TP6.** The report's USD section shows Claude Code's own `cost-state` total for the window next to the profiler's over the same process windows. A per-PR comment never shows a per-PR cost-state number, because the cost-state is per process and one process spans several PRs. Its `cost_state` block has `usd: null` and a `reason`. Instead it gives how far the profiler fell short of the cost-state totals of the sessions the PR's spend came from (`sessions_gap_share`) and how much of the PR's spend those totals cover (`covered_share`), and the comment says to read the USD as a floor.
- **Schema version 4** (new database required, about two minutes): `tool_calls.worktree`, `branch_seen`, `pr_verb`; `prs.title`, `base_branch`, `campaign`; a `worktree_branches` table; `pr_attribution.campaign` and the methods `worktree` and `campaign`. Reports read 1-4; the campaign section needs 4.
- Contract docs: `attribution.md` (contract version 4: work branch, the D-TP7 A3 split, A5's agent-wide case, campaigns, validation), `transcript-format.md`, `schema.sql`, README.
- 185 token-profile tests pass (156 before).

## Failure classes

Ground truth: the Wave 1/2 worker-to-PR pairs in the ledger, plus every teammate whose name is a packet code that matches exactly one PR head branch opened within four days (for example `cr06` and `craft/cr06-*`). That makes 91 workers and 84 PRs. Workers that opened two PRs (7, among them `bv07`, `bm-03` and `bv08`, which split their packets into a and b PRs) are left out, since one label can't name both.

1. **A subagent's record branch is not its branch.** This is the main cause. Claude Code writes the process checkout's `gitBranch` on every record, which the coordinator and parallel agents move. An in-process teammate carries the coordinator's branch and cwd, and an isolation-worktree subagent carries its siblings' branches although its `cwd` is its own. On requests after a subagent's first git call, the record branch matched the subagent's own git output 4% of the time for teammates and 30% for worktree subagents. This is why `tp10-rebase` and `tp05`'s first turn were unattributed (their branch was `main`), and why Wave 1 workers' first requests went to the coordinator's ledger PR #1123. **Fix:** a subagent's branch comes from its own tool calls: branches named in its git output, and worktrees named in its paths and commands, resolved through `worktree_branches`. It never comes from the record. The record-derived worktree-to-branch source (`request-cwd`) now takes main-session requests only.
2. **`split` charged a PR the turn only read.** `tp10` ran `gh pr view 1128` (TP-01a's PR, for reference) in the turn that opened #1131, so the turn was split. **Fix:** `tool_calls.pr_verb`; a turn that created a PR is charged to the PRs it created only.
3. **A worker's turns before its PR existed.** `tp05` opened #1134 in its second turn; its first 43 requests named no PR. **Fix:** A5's agent-wide case: a subagent that created exactly one PR gets every request its work branch doesn't place.
4. **`gh pr create` read a number from its title.** `--title "... 1012 Deployable ..."` gave `pr_ref` 1012. Fixed: `create` takes its number only from the `/pull/N` in its result.
5. **`git fetch` output read as a push.** `main -> origin/main` under `From <remote>` showed workers as on `main`. Fixed: ref updates count only under `To <remote>`.

Also: `gh pr diff|comment|edit|review|ready|close N` now name a PR for A5's turn case (reviewer turns). The real-data effect is small, under 0.1% of spend.

Branches deleted by `rm-worktree.sh` and `branch_heads` gaps were not the cause of any Wave 2 failure. The teammates' work branches still exist as PR head branches, so A2 places them once the work branch is right. A3 is unaffected.

## Real-data numbers (2026-10-03, 91,631 requests, $11,364 estimated)

Validation. "Before" is the TP-05 ingest's attribution (schema 3), scored on the same requests:

| Set | Labelled | Precision (USD) before → after | Recall (USD) before → after | Unattributed before → after | Worst rule after |
|---|---|---|---|---|---|
| labels (91 workers, 84 PRs) | 14,554 requests, $1,827 | 0.408 → 0.992 | 0.216 → 0.980 | 47.0% → 1.2% | `worktree`, 0.8% of the set misplaced |
| authors (created exactly one PR) | 12,545 requests, $1,265 | 0.329 → 0.996 | 0.302 → 0.996 | 8.1% → 0% | `worktree` and `ancestry`, 0.2% each |

Before, `branch` misplaced 29% of the labelled spend and `parent-session` 23% of the authors set, both over the 10% limit. Now no rule misplaces more than 1%. The authors set partly overlaps A5's agent-wide case, so read its recall as a ceiling; its precision still measures the branch rules.

Wave 1/2 workers (requests on the right PR / unattributed / on another PR):

| Worker | PR | Before | After |
|---|---|---|---|
| tp01a | #1128 | 54 / 7 / 9 | 70 / 0 / 0 |
| tp01b | #1126 | 41 / 0 / 3 | 44 / 0 / 0 |
| tp02 | #1127 | 55 / 3 / 7 | 65 / 0 / 0 |
| tp03 | #1124 | 61 / 0 / 5 | 66 / 0 / 0 |
| tp04 | #1125 | 29 / 6 / 4 | 39 / 0 / 0 |
| tp05 | #1134 | 58 / 43 / 0 | 101 / 0 / 0 |
| tp06-tp09 | #1130, #1133, #1132, #1135 | all right | all right |
| tp10 | #1131 | 29 / 1 / 26 | 56 / 0 / 0 |
| tp10-rebase | #1131 | 0 / 22 / 0 | 22 / 0 / 0 |

Coverage, all spend:

| | Before | After |
|---|---|---|
| On a PR | 72.5% | 68.1% |
| On a campaign, no PR (D-TP7) | n/a | 18.5% ($2,098) |
| Unattributed | 27.5% ($3,120) | 13.5% ($1,530) |
| Subagent spend unattributed | 29.4% | 11.9% |
| Main-session spend unattributed | 20.3% | 19.4% |
| #662 (Harset integration PR) | $1,274 | $62.88 |
| Merged PRs since 2026-09-13: median / p90 / max | $7.87 / $38.51 / $1,274 | $12.09 / $41.65 / $279 (#642) |
| Top 10% of those PRs' share | 56% | 37% |
| Per-PR unattributed share, mean / p90 (all merged PRs) | 13.8% / 63.8% | 6.4% / 22.2% |

Campaign rollup, largest: harset-rebuild $1,689 (3 PRs $111, packets with no PR $1,578), npc-ai-restoration $817 (36 PRs), castle-rebuild $705 (12), crafting $522 (18), social-systems $456 (18), pets $427 (12), bank-vault $344 (12), `pr-683` $327 (1 PR $14, packets $314), legacy-command-parity $319, black-market $297. 207 of 804 PRs are tagged by `campaigns.json`.

Cost-state next to profiler (D-TP6), whole history: Claude Code's cost-state $11,232 over 94 sessions; the profiler over the same process windows $10,087 (10.2% under); profiler spend no cost-state covers $1,278.

What stays unattributed ($1,530): teammates and subagents that never ran git or touched a PR, such as research, review and RE workers whose parent turn is itself unattributed ($1,072), and main-session turns with no PR activity ($458, of which $173 is on `main`). Main-session attribution has no independent ground truth here; it improves only through A1, which follows the workers.

### Backfill

Posted on 2026-10-03 between about 16:20 and 17:00 UTC with `pr_stats --backfill --post`, over the 426 PRs the database had merged since 2026-09-13. The first run, at the default 6 a minute, was stopped after 140 PRs; the second resumed from the state file at 30 a minute and skipped those 140.

| Outcome | PRs |
|---|---:|
| Comment created | 404 |
| Comment already current (`unchanged`, from the TP-10 live posts) | 2 |
| No attributed request (`no-data`, nothing posted) | 20 |
| `gh` error or privacy-gate refusal | 0 |

The 20 no-data PRs are #621-#637 (eight), #697-#702 (four), #758-#764 (six), #972 and #975: no request in the database is attributed to them.

## Judgment

**Good enough for the first live post and the backfill.** On independent labels, 99.2% of the spend attribution places on a PR is on the right one, and 98% of labelled spend is placed. No rule misplaces more than 1% of labelled spend, against the 10% limit. Every Wave 1/2 worker is fully on its own PR, the three Wave 2 failures included. The remaining risks are known and stated in each comment: coordinator turns with no PR activity stay unattributed and appear as the PR's `unattributed_share`; USD is a floor about 10% under Claude Code's own total; an integration PR's number excludes its packets, which the campaign rollup carries.

Before posting, rebuild the default database. `C:\Users\Steve\token-profile.sqlite` is schema 3 with the old attribution, and `pr_stats` would still read it. Delete it (or point `--db` elsewhere) and run the ingest. Post one Wave 2 PR first (#1131) and check the rendered comment before the backfill.

## Left / open

- The labels CSV is local. It is rebuilt from the agent names with the rule above; nothing in the repo holds names or ids.
- `campaigns.json` covers the campaigns whose branches carry a prefix or whose PR titles name a tracking issue. Others (castle `pkg/*` content packets, ad-hoc `feat/`/`fix/` work) have no campaign, by design: a guess would be worse than none.
- A fast-forward merge of a packet into an integration branch lands on the first-parent chain and still counts as the integration PR's. None was seen in the data.
- The ledger README's TP-05b row and the D-TP6/D-TP7 entries belong to the coordinator.

## Commands

```bash
python -m unittest discover -s tools/token-profile -p "test_*.py"
python tools/token-profile/ingest --db <new.sqlite> --repo . --fetch-prs
python tools/token-profile/validate --db <new.sqlite> --labels <labels.csv> [--truth-db <db>]
python tools/token-profile/report --db <new.sqlite> --out <dir>
python tools/token-profile/pr_stats --backfill --db <new.sqlite> --out <dir>
```
