---
title: Profile token usage
type: how-to
audience: the Claude Code coordinator and the person who runs the workstation; anyone reading a PR's stats comment
last_updated: 2026-10-03
companion_docs:
  - ../../tools/token-profile/README.md
  - ../analysis/token-usage/README.md
  - ../analysis/token-usage/retro-cost-study.md
  - ../operations/claude-code-telemetry.md
  - ../agents/development-workflow.md
---

# Profile token usage

This guide shows you how to measure what AI-assisted work in this repo costs, post a merged PR's stats comment, keep the numbers current with the scheduled jobs, and read the comment once it's there.

The profiler lives in [`tools/token-profile/`](../../tools/token-profile/README.md). That README is the reference for every option, exit code and section; this guide is the order you run things in. Why the profiler exists, the owner decisions behind it (D-TP1 to D-TP10) and what it found are in the campaign ledger, [docs/analysis/token-usage/](../analysis/token-usage/README.md), and the [retro cost study](../analysis/token-usage/retro-cost-study.md).

## Before you start

- **Python 3.11 or later.** The profiler uses the standard library only; there is nothing to install.
- **`gh`, logged in** to an account that can comment on `SandboxServers/Cimmeria`. Only posting and the backfill need it.
- **Claude Code transcripts on this machine**, under `~/.claude/projects`. The profiler reads local transcripts, so the stats for a PR can only be built on the workstation where its agents ran.
- **A database path outside the repo.** The store holds local paths, session ids and branch names. The tools default to `~/token-profile.sqlite`; never put it in the checkout and never commit it.

Every command below runs from the repo root.

## Ingest the transcripts

```bash
python tools/token-profile/ingest --db ~/token-profile.sqlite --repo . --fetch-prs
```

- `--repo .` lets the ingest read branch heads and commit ancestry, which attribution needs. `--fetch-prs` loads the PR list from GitHub; without it, nothing is attributed to a PR.
- The first run takes a minute or two. Later runs read only what is new and take seconds, and they rebuild attribution, so a PR opened today places requests made before it.
- The run exits 1 when it meets a transcript shape it doesn't know, and 2 when a request's attribution weights don't sum to 1. Both mean the profiler needs a fix, not a retry.
- **A database of an older schema version is refused.** When a profiler change bumps the schema (it is at version 6), point `--db` at a new file, or delete the old one, and ingest again (a version 5 database takes about two minutes to rebuild).

## Run a report

```bash
python tools/token-profile/report --db ~/token-profile.sqlite --out <dir> --since 2026-09-14T00:00:00Z
```

You get `token-report.md` and `token-report.json` in `<dir>`: raw tokens, estimated USD with Claude Code's own `cost-state` total beside the profiler's, context pressure, tool exposure, cost per merged PR, cost per campaign and the cache-policy simulation. The [section table](../../tools/token-profile/README.md#reports) says what each one measures.

A report is safe to commit once it is written, because it passes the privacy gate first (see [Privacy](#privacy)). If the gate finds something, it names the detector, writes nothing and exits 3.

## Reconcile against Claude Code's own totals

```bash
python tools/token-profile/reconcile --db ~/token-profile.sqlite --out <dir>
```

This checks the profiler against Claude Code's `cost-state` records and exits 4 when a check is out of tolerance. Run it after a profiler change, or when a number looks wrong. On 2026-10-03 it passed with the profiler 10.2% under `cost-state`, which is why every USD figure is a floor.

To compare against OpenTelemetry as well, add `--otel <events.json>` with an export of `claude_code.api_request` events from SigNoz. That arm has no data yet: see [Telemetry](#telemetry).

## Validate attribution

```bash
python tools/token-profile/validate --db ~/token-profile.sqlite --labels <labels.csv>
```

Run this after any change to the attribution rules in `ingest/`. It scores PR attribution against ground truth (a local CSV of worker name, day and PR, plus the workers that created exactly one PR) and prints precision, recall and the misplaced share per rule. A rule that misplaces more than 10% of labelled spend gets fixed before anything is posted. The labels file stays on your machine; how to build it is in [attribution.md § Validating the rules](../../tools/token-profile/attribution.md#validating-the-rules).

## Post a PR's stats after it merges

The merging agent does this as the last step of a merge ([definition of done](../agents/development-workflow.md#definition-of-done)):

```bash
python tools/token-profile/ingest --db ~/token-profile.sqlite --repo . --fetch-prs
python tools/token-profile/pr_stats <PR>          # dry run: print the comment
python tools/token-profile/pr_stats <PR> --post   # create it, or edit it in place
```

1. Ingest first, so the database has the PR's merge time and its latest requests.
2. Look at the dry run once if the PR is unusual (an integration PR, a PR with human-only commits).
3. Post. The command finds the PR's comment by its `<!-- cimmeria-pr-stats:v1 -->` marker and edits it, so running it twice never makes a second comment. It prints `status=created`, `updated` or `unchanged`.

Exit code 4 means no request is attributed to the PR: a dependency bump, or work done without Claude Code. Nothing is posted, and that's correct.

## Backfill missed PRs

```bash
python tools/token-profile/pr_stats --backfill --since 2026-09-13            # dry run
python tools/token-profile/pr_stats --backfill --since 2026-09-13 --post     # post
```

The backfill walks every merged PR in the database since `--since`, oldest first, at 6 `gh` calls a minute by default (`--rate`). It records each outcome in a state file next to the database, so a stopped run picks up where it left off. Three `gh` errors in a row stop it.

The first full backfill ran on 2026-10-03 and finished at about 17:00 UTC: 406 PRs posted, 20 with no data, no errors, and no duplicate comments on a spot check. You only need it again after a profiler change that alters past numbers; in that case add `--restart` so it revisits PRs it already posted.

## Let the scheduled jobs keep it current

Two local Windows Task Scheduler jobs run the routine work, so a missed merge step is caught the next morning:

| Script | When | What it does |
|---|---|---|
| `tools/token-profile/scheduled/pr-sweep.ps1` | Daily, 07:30 | Ingests, then posts or updates the stats comment of every merged PR that lacks a current one |
| `tools/token-profile/scheduled/weekly.ps1` | Mondays, 08:00 | Ingests, then writes the weekly report and the reconciliation |

Register both once per workstation:

```powershell
pwsh tools/token-profile/scheduled/register-tasks.ps1
```

Results and logs are written outside the repo, next to the database. A weekly report is safe to commit, but nothing commits it for you; copy one into the ledger's `baseline/` only when you're recording a measurement. Check the scripts' own help for their options and where they write.

## Read a stats comment

Each merged PR carries one comment that opens with **Token profile**. Its table reads:

| Column | Meaning |
|---|---|
| est. USD (list) | The PR's attributed spend at list price |
| requests | API requests attributed to the PR |
| human prompts | Prompts a person typed, not ones an event started |
| agents | How many agents worked on it, summed over agent types |
| peak ctx | The largest context any of its requests carried, in tokens |
| wall clock | From its first attributed request to the merge |
| CI rounds | `N failed of M`: of the M head commits that ran a workflow, N had a failed, timed-out or unstarted run |

Keep four things in mind when you compare the numbers:

- **USD is list price, a plan-usage proxy, not a bill.** The project is on a Max subscription (D-TP1). Use the figure to compare work, not to budget money.
- **USD is a floor.** The profiler is about 10% under Claude Code's own `cost-state` total, because some requests never reach a transcript (D-TP6). The comment's cost-state sentence says how far under it was for the sessions this PR's spend came from. It never shows a per-PR cost-state number: `cost-state` is per process, and one process spans several PRs.
- **An integration PR carries only its own spend.** Packet PRs merged into an integration branch keep theirs (D-TP7). The campaign's cost is the sum over its PRs plus the packet work that had no PR, which the report's "Cost per campaign" section adds up. The comment names the campaign when it knows it.
- **The attribution line says how sure it is.** It gives the main method, a confidence and the share of the same sessions' spend that stayed unattributed. A high unattributed share usually means coordinator turns with no PR activity, and it isn't charged to the PR.

The JSON block under the table is the `cimmeria-pr-stats/1` record that the trend aggregation reads (`pr_stats.block.parse(body)`). Fields are only ever added; a breaking change bumps the schema name. The [retro cost study](../analysis/token-usage/retro-cost-study.md) shows what typical work costs, so you can tell whether a PR's number is high for its type and size.

## Campaign metrics

The campaign metrics read the profiler database and add what a campaign produced: packets, PRs, CI, escaped defects and overstep. The method and definitions are in the [campaign-metrics README](../analysis/campaign-metrics/README.md).

Each metric, one line:

- Campaign cost: list-price cost of a campaign's requests, by role and PR.
- Cost per PR: attributed campaign cost divided by the PR count (p50 and p90 per PR too).
- Cost per accepted change: campaign cost divided by accepted packets.
- First-pass completion: accepted packets with one implementer session, no review-fix commit and no escalation.
- Retries and why: implementer sessions beyond the first, each classified by its stop context.
- CI first-attempt rate: share of PRs whose first CI run passed.
- Escaped defects: fix or revert PRs and bug issues that cite a merged PR of the campaign.
- Escalation rate: packets whose implementer moved up a tier, over packets sent to Haiku.
- Worker stop context: why each worker session stopped, and its context size at the stop.
- Cycle time: first worker request in a packet's worktree to the PR's merge.
- Overstep and recoverable cost: work a role did that a cheaper tier should have done, and what it would have cost at that tier.

**Regime.** A packet's regime is the implementer that ran: `haiku` if a Haiku implementer session ran, else `old`. The switch to Haiku workers was `2026-10-10T20:02:54Z`. A campaign is `haiku` at 70% or more Haiku packets, `old` at none, and `mixed` in between; cost compares pure campaigns only, and packet metrics split a mixed campaign by packet.

**Before you start.** `packets` needs a schema v6 database, so re-ingest first. A full `github` run makes roughly 3,500 to 5,500 `gh` API calls (projected from a sample of tagged PRs): pass `--cache`, and expect two sittings.

**Rerun in order.** Each step except `registry --check` writes to a scratch directory outside the repository. Copy each command from `python tools/token-profile/metrics <sub> --help`, and check the flags there before you rely on them.

1. `python tools/token-profile/metrics registry --repo . --check`
2. `python tools/token-profile/metrics ledger --repo . --out <scratch>`
3. `python tools/token-profile/metrics cost --db <profile.sqlite> --out <scratch> --ledger <scratch>/ledger.json`
4. `python tools/token-profile/metrics github --repo . --out <scratch> --ledger <scratch>/ledger.json --cache <scratch>/gh-cache`
5. `python tools/token-profile/metrics packets --db <profile.sqlite> --ledger <scratch>/ledger.json --github <scratch>/github.json --out <scratch>`
6. `python tools/token-profile/metrics overstep --db <profile.sqlite> --ledger <scratch>/ledger.json --out <scratch>`
7. `python tools/token-profile/metrics report --inputs <scratch> --out docs/analysis/campaign-metrics --db <profile.sqlite> --self-session <id>`

**Scratch rule.** Intermediate JSON (`ledger.json`, `cost.json`, `github.json`, `packets.json`, `overstep.json`) and the `gh` cache are never committed. Only the aggregate outputs under `docs/analysis/campaign-metrics/` are committed.

**Exit codes.** `0` ok. `1` `registry --check` found a campaign rule problem: a campaign has no folder, or two prefixes overlap. `2` an input or output error. `3` the privacy gate refused the output, and nothing was written.

**Caveats.**

- Dollars are a list-price floor, not a bill.
- The unattributed and unmapped share is reported with every total. Requests and PRs that attribution could not place are not in any campaign.
- The escaped-defect count is a floor. A defect with no citation is invisible.
- CI first attempt is optimistic after a force-push.
- Overstep is an estimate. It reprices the same tokens at the intended tier: implementation at Haiku, research at Sonnet. The low end of the range reprices everything at Sonnet. It ignores the extra turns a cheaper model may need. Rule O3b (Opus implementing) is an upper bound, because a ledger may not record a legitimate escalation.
- A comparison side with fewer than 5 campaigns or 10 packets carries a `low_n` flag. Read flagged figures as indicative only.

**Outputs.** Aggregate data goes to `docs/analysis/campaign-metrics/data/` and the Markdown reports to `docs/analysis/campaign-metrics/reports/`.

## Telemetry

Claude Code can also export OpenTelemetry events to the colo SigNoz. The setup, the privacy settings and the kill switch are in [claude-code-telemetry.md](../operations/claude-code-telemetry.md). The profiler doesn't need it, but the OTel reconciliation does. As of 2026-10-03 the workstation is configured and nothing has arrived yet: Claude Code reads the exporter settings only when it starts, so the window opens at the first restart.

## Privacy

The repo is public. These rules keep transcript data out of it:

- **Commit only what the gate wrote.** Reports, reconciliations, validation output and stats comments go through the [three-layer scrubber](../../tools/token-profile/README.md#privacy). Never paste raw database rows, transcript text, commands, local paths, session ids, branch names or teammate names into a doc, issue or PR.
- **Keep the database, the labels CSV and the scheduled jobs' output off the repo.** They hold exactly what the gate removes.
- **Name your machine's identifiers as deny words.** The scrubber already denies the local user and machine names; add any other private word with `--deny <word>` or the comma-separated `TOKEN_PROFILE_DENY` variable.
- **Don't copy from SigNoz into the repo.** Telemetry carries tool parameters, local paths and the login email unless the collector drops it; the scrubber covers committed reports, not SigNoz.

## When something fails

| Symptom | Cause | Do this |
|---|---|---|
| Ingest exits 1 | A transcript shape the profiler doesn't know | Read the `unknown_shapes` count in its summary; fix the parser, or run with `--allow-unknown` for a one-off look |
| Ingest exits 2 | An attribution weight sum isn't 1 | A profiler bug. Don't post comments until it's fixed |
| "older schema version" | The schema moved on | Ingest into a new database file |
| Any command exits 3 | The privacy gate refused the output | Find the value the named detector matched (often a new deny word), fix the cause, rerun |
| `pr_stats` exits 4 | Nothing attributed to that PR | Expected for dependency bumps and human-only PRs |
| `reconcile` exits 4 | A tolerance failed | Compare with the 2026-10-03 column in the [reconciliation table](../../tools/token-profile/README.md#reconciliation) before trusting new numbers |

## Related

- [tools/token-profile/README.md](../../tools/token-profile/README.md): every command, option, section and exit code.
- [docs/analysis/token-usage/README.md](../analysis/token-usage/README.md): the campaign ledger, its decisions and the cut-line log.
- [retro-cost-study.md](../analysis/token-usage/retro-cost-study.md): what work costs here, and the levers still open.
- [development-workflow.md § Worker lifetime and notifications](../agents/development-workflow.md#worker-lifetime-and-notifications): the working rules the profiler's numbers produced.
- [claude-code-telemetry.md](../operations/claude-code-telemetry.md): the OpenTelemetry export.
