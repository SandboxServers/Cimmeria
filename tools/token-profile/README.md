# Token profiler

Measures what AI-assisted work in this repo costs: tokens, estimated list-price USD, context pressure and orchestration overhead, per session, agent, campaign and PR. It reads local Claude Code transcripts into a local SQLite store; nothing it reads leaves the machine except scrubbed aggregates.

Status: **Wave 2.** The data contract, the ingest (TP-01a), the reports (TP-01b), the reconciliation against Claude Code's own totals (TP-05) and the attribution validation (TP-05b) are in place. The plan, the decisions and the cut-line log are in the ledger, [docs/analysis/token-usage/](../../docs/analysis/token-usage/README.md); the issue is [#957](https://github.com/SandboxServers/Cimmeria/issues/957).

| File | What it is |
|---|---|
| [`schema.sql`](schema.sql) | The SQLite schema the ingest writes and the reports read. Version 4. |
| [`transcript-format.md`](transcript-format.md) | The transcript shapes the profiler depends on, and the trigger classification rules. |
| [`attribution.md`](attribution.md) | How a request is charged to a PR or a campaign, and the invariants every attribution keeps. |
| [`campaigns.json`](campaigns.json) | Which branches, branch prefixes and tracking issues belong to which campaign (a `docs/analysis/<campaign>/` folder). |
| [`fixtures/build_fixtures.py`](fixtures/build_fixtures.py) | Builds a synthetic transcript tree, with an `expected.json` of what a correct ingest produces and hostile values a report must never show. |
| [`ingest/`](ingest/) | The ingest: transcripts into the schema, incrementally, with trigger classification, prices and PR attribution. Its tests are `ingest/test_*.py`. |
| [`report/`](report/) | The reports: raw tokens, estimated USD, context pressure, tool exposure, cost per merged PR and a cache-policy simulator, behind a privacy scrubber. See [Reports](#reports). |
| [`reconcile/`](reconcile/) | Checks the profiler against Claude Code's `cost-state` records and OTel `api_request` events, with tolerances. See [Reconciliation](#reconciliation). |
| [`cutlines/`](cutlines/) | Before-and-after measures across the ledger's cut lines. See [Cut lines](#cut-lines). |
| [`pr_stats/`](pr_stats/) | The per-PR stats comments: one idempotent `cimmeria-pr-stats/1` comment per PR, and the backfill. See [Per-PR stats comments](#per-pr-stats-comments). |
| [`validate/`](validate/) | Scores PR attribution against ground truth. See [Validating attribution](#validating-attribution). |
| [`scheduled/`](scheduled/) | The weekly report and the daily per-PR stats sweep, and the script that puts them in Task Scheduler. See [Scheduled jobs](#scheduled-jobs). |
| [`test_contract.py`](test_contract.py) | Checks the schema's constraints and that the fixture tells a correct ingest from a wrong one. CI runs it. |

```bash
python -m unittest discover -s tools/token-profile -p "test_*.py"
python tools/token-profile/fixtures/build_fixtures.py <out-dir>
```

Stock Python 3.11+, no dependencies.

## Rules every packet keeps

- **Dedupe by `requestId`, keeping the last record.** The first record of a request is a streaming partial.
- **`thinking_tokens` is part of `output_tokens`.** Never add it on top.
- **Price per model, from a versioned table.** One set of cache weights is wrong: Opus 5.5 reads cache at 0.05 of input and Fable 5.1 at 0.025. USD is a list-price estimate of plan usage, always labelled as one; the project is on a Max subscription (D-TP1).
- **Unknown transcript shapes fail the run.** They are counted, never dropped.
- **Context exposure is not cost.** `result_chars × later requests` ranks offenders; it is never reported as dollars.
- **Reports carry their window and versions** (Claude Code, profiler commit, price table, models seen) and contain no commands, transcript text, absolute paths or credentials.

## Ingest

```bash
python tools/token-profile/ingest --db ~/token-profile.sqlite --repo . --fetch-prs
```

| Option | Does |
|---|---|
| `--db <file>` | The SQLite store, created on first use. Keep it outside the repo and never commit it: it holds local paths and session ids. |
| `--projects <dir>` | Where the transcripts are; default `~/.claude/projects`. |
| `--project-prefix <name>` | Which project directories to read; default the main checkout's path in Claude Code's naming, which also covers sessions started in its worktrees. |
| `--repo <checkout>` | Read branch heads (ref snapshot, merge subjects) and commit ancestry from this checkout. Without it, rule A3 places nothing. |
| `--fetch-prs` or `--prs-json <file>` | Load the `prs` table from `gh pr list --state all`, live or from a saved file. Without PRs every request is unattributed. |
| `--lane-log <file>` | The build lane's job log, for branch heads and which branch each worktree had; default the lane's own `jobs.jsonl` if present. |
| `--campaigns <file>` | Campaign tags; default [`campaigns.json`](campaigns.json). |
| `--allow-unknown` | Exit 0 although `unknown_shapes` has rows. |

Re-running is cheap: each file is read from where the last run stopped, and a run with nothing new adds nothing. Attribution and campaign tags are rebuilt on every run, so a PR opened later places requests made before it. A database of an older schema version is refused: start a new one (about two minutes). The run prints a JSON summary of counts (never transcript content) and exits 1 when unknown transcript shapes are recorded, 2 when `attribution_imbalance` is not empty.

USD is computed from `price_tables` by the reports, never stored per request; `ingest/prices.py` holds the versioned table and `estimate_usd()`.

## Reports

```bash
python tools/token-profile/report --db <profile.sqlite> --out <dir> [--since 2026-09-14T00:00:00Z] [--until ...]
    [--price-table <version>] [--top 20] [--deny <word>]...
```

It reads the database read-only and writes `token-report.md` and `token-report.json` (schema `cimmeria-token-report/1`) into `--out`. `--since` is inclusive and `--until` exclusive, both compared with request timestamps. The price table defaults to the one the last successful ingest used.

The layers stay separate:

| Section | What it reports | Unit |
|---|---|---|
| Raw tokens | Each category on its own (input, output, thinking as part of output, cache read, 5m and 1h cache writes, context), by model, main or subagent, agent type and trigger kind, with per-request distributions | tokens |
| Estimated USD | The same breakdowns priced per model from the stamped price table, plus requests whose model has no price, and Claude Code's own `cost-state` total for the window next to the profiler's over the same process windows (D-TP6). Always labelled a plan-usage proxy, not a bill (D-TP1) | USD |
| Context pressure | Context per request and peak per transcript, requests per transcript, the first request of each transcript by agent type (static context), cache writes by idle gap, and compactions | tokens |
| Tool results and exposure | Result characters and exposure (characters x later requests) by tool, by command or path fingerprint, and by MCP server | characters, never USD |
| Cost per merged PR | For PRs merged in the window, each PR's whole attributed spend, its distribution and top-10% share, the unattributed share of window spend, and the method mix. Unattributed spend is reported, never spread onto PRs | USD |
| Cost per campaign | Each campaign with spend in the window: its PRs' spend, the packet work that reached it without a PR, and the total (D-TP7). Needs a schema version 4 database | USD |
| Cache-policy simulation | Each transcript replayed under a 5m and a 1h TTL over its real idle gaps, by agent type, with a calibration ratio against the observed cache cost and the 1h-minus-5m range over the model's least certain input, the cache read on a cold request | USD |

Every distribution gives n, p50, p75, p90, p95, p99, max and mean. Every report opens with its stamp: window, profiler commit, report commit, price table, Claude Code versions, models, schema version, unknown-record counts, and how many values the privacy filter rejected.

`report.sections.prs.pr_records(db, scrubber, [pr, ...])` returns the database-backed fields of the `cimmeria-pr-stats/1` block, for TP-10's per-PR comments.

### Privacy

The repo is public, so a report is built to be safe to commit. [`report/scrub.py`](report/scrub.py) applies three layers, and its tests prove that each one is needed:

1. **Field validation.** Every free-text value a report shows must have the shape the contract promises. A command fingerprint is one or two plain words, a path fingerprint is repo-relative, an MCP fingerprint is the tool name. Anything else becomes `<invalid>` and is counted in the stamp, so an ingest bug shows up instead of leaking.
2. **Redaction.** Every string is scrubbed of URL credentials and query strings, non-public hosts, auth headers, secret flags and assignments, known token formats, high-entropy tokens, emails, IP addresses, absolute local paths, and the deny words: the local user and machine names, `--deny` and the comma-separated `TOKEN_PROFILE_DENY`.
3. **The gate.** The rendered Markdown and JSON are searched again with the same detectors. A hit names the detector, never the value, and nothing is written (exit code 3).

Session ids, agent ids, project directories, branch names, teammate names and message text never reach a report; a campaign is named after its ledger folder or its integration PR (`pr-<N>`), never a branch. Claude Code writes an in-process teammate's name as its `agentType`, so a teammate with no `.claude/agents` definition is reported as `teammate`.

## Reconciliation

```bash
python tools/token-profile/reconcile --db <profile.sqlite> [--out <dir>] [--since ISO] [--until ISO] [--otel <events.json>]
```

Compares the profiler with Claude Code's own records and exits 4 when a check is out of tolerance (0 when all pass, 3 if the privacy gate refuses, 2 on an input error). With `--out` it writes `reconcile.md` and `reconcile.json`, aggregates only. `--since` and `--until` select sessions by the start of the process that wrote their cost-state.

**Against `cost-state`.** A cost-state record covers one Claude Code process, subagents included, from its start (`cost_states.process_start`, schema version 3) on. Each is compared with the same session's requests from that time on; spend outside every such window (sessions with no record, a resumed session's requests from before the resume) is reported as uncovered, not compared.

| Check | Fails when | Limit | 2026-10-03 |
|---|---|---:|---:|
| `price_residual` | the cost-state's own tokens, repriced with the profiler's table (its 5m/1h split, web search at $0.01), miss the cost-state's USD | 1% | 0.03% |
| `output_gap` | profiler output tokens differ from the cost-state's; a dedupe bug shows here first | 3% | 1.1% |
| `undercount` | the profiler is below the cost-state by more than this | 15% | 10.2% |
| `overcount` | the profiler is above the cost-state at all (a double count) | 1% | 0% |
| `session_overcount` | any session with 20 or more requests is more than 2% above its cost-state | 0 sessions | 0 of 69 |

The undercount is real and expected: the cost-state holds requests that no transcript records. So the report's USD section shows the cost-state total next to the profiler's wherever a cost-state covers the window (D-TP6). By model (Haiku aside), the transcripts hold 1-10% of the cost-state's input tokens, 73-97% of its cache reads, 91-100% of its cache writes and 98-99% of its output: the missing requests read a large cached context, send fresh input and write little. The gap grows with subagent activity (correlation 0.83 with a session's subagent requests). The OTel `query_source` of those requests should name them; until it does, the profiler's USD is a floor about 10% under Claude Code's.

**Against OTel.** `--otel` takes `claude_code.api_request` events from a file: an OTLP JSON log export, a SigNoz log search result saved as JSON, or JSON Lines of flat attributes. Events join to requests on `request_id`.

| Check | Fails when | Limit |
|---|---|---:|
| `matched_token_mismatch` | matched requests carry different token counts (OTel's cache creation against the profiler's 5m plus 1h writes) | 0.5% of requests |
| `matched_cost_residual` | OTel `cost_usd` and the profiler's USD differ over matched requests | 1% |
| `profiler_only` | profiler spend inside the time span OTel covers for a session has no event (lost telemetry) | 2% |

OTel-only requests are not a failure: they are the requests the transcripts don't record, listed by `query_source` and model. The OTel total is also set against the cost-state for the sessions both have.

## Cut lines

```bash
python tools/token-profile/cutlines --db <profile.sqlite> [--out <dir>] [--since ISO] [--cut TP-03=ISO ...]
```

Splits three measures at the cut of the packet that targeted them (defaults from the ledger's cut-line log): the first request of each transcript by agent type (TP-03), the result size of `lane.sh` calls, foreground and background apart (TP-02), and requests and peak context per subagent transcript (TP-00). A transcript still running at ingest is cut short, so a recent "after" side is a floor. Output goes through the same privacy gate as the reports.

## Per-PR stats comments

Every merged PR gets one comment with its numbers (D-TP5): a short table for people and a `cimmeria-pr-stats/1` JSON block for the trend aggregation, under the `<!-- cimmeria-pr-stats:v1 -->` marker. The stats are built locally because the transcripts are local.

```bash
python tools/token-profile/pr_stats <PR>                    # print the comment, write nothing
python tools/token-profile/pr_stats <PR> --post             # create it, or edit it in place
python tools/token-profile/pr_stats --backfill [--since 2026-09-13] [--post] [--rate 6/min]
```

Run the ingest first, so the database has the PR's merge time and its latest requests. `--db` defaults to `~/token-profile.sqlite`; `--repo`, `--price-table` and `--deny` work as for the reports, and `--out <dir>` also writes each body to `<dir>/<PR>.md`.

| Exit code | Meaning |
|---|---|
| 0 | Printed (dry run), or posted: `status=created`, `updated` or `unchanged` on stderr |
| 2 | Error: no database, or a `gh` call failed. Nothing was posted |
| 3 | The privacy gate refused the comment. Nothing was posted |
| 4 | No request in the database is attributed to the PR. Nothing was posted |

**Idempotent.** `--post` lists the PR's comments and takes the ones whose body starts with the marker. None: it creates one. One: it edits it in place (`gh api -X PATCH`), or leaves it alone when the body is unchanged. Several, from some earlier bug: it edits the oldest and reports the rest as `duplicates=N` without touching them. Without `--post` it makes no write call.

**Where each field comes from.** The database-backed fields are `report.sections.prs.pr_records()`; the stamp adds the profiler commit of the last ingest, the newest Claude Code version among the PR's requests, and the price table. `gh` answers the rest:

| Field | Meaning |
|---|---|
| `quality.ci_rounds` | Head commits of the PR that ran any workflow |
| `quality.ci_fail_rounds` | Those where at least one run failed, timed out or failed to start (`cancelled` is not a failure) |
| `quality.review_rounds` | Commits that received at least one submitted review, so two bots reviewing one push are one round |
| `quality.followup_fix_prs` | Merged PRs titled `fix:`/`fix(...):` that cross-reference this PR after it merged |
| `quality.reverted` | A merged PR titled `Revert ...` cross-references this PR |
| `diff` | Additions, deletions and changed files from `gh pr view`, the database's copy when `gh` has none |

**Cost-state and campaign (D-TP6, D-TP7).** Claude Code's `cost-state` total is per process, and one process spans several PRs, so it cannot be split by PR and the comment never invents a per-PR number: `cost_state.usd` is always null. Instead the block says how far the profiler fell short of the cost-state totals of the sessions this PR's spend came from (`sessions_gap_share`) and how much of the PR's spend they cover (`covered_share`), and the comment reads its USD as a floor. `campaign` names the PR's campaign; an integration PR's number excludes the packets merged into it, which the report's campaign section adds up. Both keys come after the plan's.

The table's wall clock runs from the PR's first attributed request to its merge. Fields are only ever added; a breaking change bumps the schema version. `pr_stats.block.parse(body)` reads a block back for the aggregation.

**Privacy.** The block goes through the same scrubber as the reports (field validation and redaction), and the rendered comment through its gate before anything is printed or posted. A gate hit names the detector, exits 3 and posts nothing.

**Backfill.** `--backfill` walks the database's PRs merged since `--since` (default 2026-09-13), oldest first. It is a dry run unless `--post` is given, and prints one `pr=N status=...` line per PR. A PR with no data costs no `gh` call; every other PR waits its turn under `--rate` (default 6 a minute, `N/h` also works). Each outcome goes into a state file next to the database (`--state` to move it), per mode, so a stopped run resumes where it left off: successes and no-data PRs are skipped, errors and gate refusals are retried. `--restart` forgets the mode's outcomes, `--limit N` stops after N PRs, and three `gh` errors in a row stop the run. The backfill is posted only after attribution is validated; TP-05b did that on 2026-10-03, see [Validating attribution](#validating-attribution).

## Validating attribution

```bash
python tools/token-profile/validate --db <profile.sqlite> [--labels <labels.csv>] [--truth-db <older.sqlite>] [--out <file.json>]
    [--max-wrong-share 0.10]
```

Scores the database's PR attribution against ground truth, by estimated USD and by request count: precision, recall, and the wrong, campaign and unattributed shares, with the wrong share by method. The ground-truth sets (a local labels CSV of worker name, day and PR; workers that created exactly one PR) and the rule that a method misplacing more than 10% of labelled spend is fixed first are in [attribution.md § Validating the rules](attribution.md#validating-the-rules). `--truth-db` takes the ground truth from another database, to score an older database on the same requests. The output holds shares and totals only, never names or ids. A labelled request's weight that its attribution rows don't account for counts as unattributed. `--max-wrong-share` makes the run exit 4 when one method misplaces more than that share of a set's USD; the weekly job passes 0.10. Exit code 2 is an input error, including an `--out` it can't write.

## Scheduled jobs

Two jobs keep the profile current without anyone running it by hand. Both run the commands above as child processes, from the main checkout, and write only to a local folder outside the repo, the job home: `TOKEN_PROFILE_HOME`, else `%LOCALAPPDATA%\cimmeria-token-profile`.

| Script | When | Does |
|---|---|---|
| [`scheduled/weekly.ps1`](scheduled/weekly.ps1) | Mondays 08:00 local | Incremental ingest with `--fetch-prs`; then `reconcile` and `report` over the 7 whole UTC days before today, and `validate --max-wrong-share 0.10` over the whole database. Output in `<home>\reports\<YYYY-MM-DD>\`: the reports, each step's log, `weekly-summary.txt` |
| [`scheduled/pr-sweep.ps1`](scheduled/pr-sweep.ps1) | Daily 07:30 local | Incremental ingest with `--fetch-prs`; then `pr_stats --backfill --post --restart` over the PRs merged in the last 3 whole UTC days, at 6 a minute. Logs in `<home>\logs\pr-sweep-<YYYY-MM-DD>*` |
| [`scheduled/register-tasks.ps1`](scheduled/register-tasks.ps1) | Once, after the scripts are on `main` | Registers or updates `\Cimmeria\TokenProfile-Weekly` and `\Cimmeria\TokenProfile-PrSweep` for the current user; `-WhatIf` prints them, `-Unregister` removes them |

```powershell
pwsh tools/token-profile/scheduled/weekly.ps1 [--days 7] [--until YYYY-MM-DD] [--labels <csv>] [--skip-ingest]
pwsh tools/token-profile/scheduled/pr-sweep.ps1 [--days 3] [--rate 6/min] [--dry-run] [--skip-ingest]
pwsh tools/token-profile/scheduled/register-tasks.ps1 [-WhatIf] [-Unregister]
```

The `.ps1` files find Python (`TOKEN_PROFILE_PYTHON`, else `python`, else `py -3`) and run `python tools/token-profile/scheduled weekly|pr-sweep`, whose logic is in [`scheduled/jobs.py`](scheduled/jobs.py); every option after the script name is passed through, so `--home`, `--db` (default `TOKEN_PROFILE_DB`, else `~/token-profile.sqlite`) and `--repo` (default the main checkout) work too. The weekly job's `--labels` defaults to `TOKEN_PROFILE_LABELS`.

**Exit status.** 0 every step passed; 1 a step failed; 2 a usage error, no Python, or another job holds `<home>\job.lock` (both jobs write the database; a lock older than six hours is taken over). A failed step does not stop the others: the weekly job still writes the report when reconcile is out of tolerance. The last line, and the summary file, name each failed step and what its exit code means: ingest 1 unknown transcript shapes, 2 attribution imbalance; reconcile 4 out of tolerance; validate 4 a method over the 10% limit; any 3 the privacy gate; pr_stats 2 `gh` errors.

**The sweep is idempotent.** pr_stats finds each PR's comment by its marker: it creates a missing one, edits a stale one in place and leaves a current one alone. `--restart` makes every run look at every PR in the window again, using the sweep's own state file (`<home>\pr-sweep-state.json`), never the backfill's. So the sweep both fills in comments for newly merged PRs and refreshes a comment whose numbers moved, for example because requests made after the merge were ingested later. A comment's stamp holds the profiler commit, so after the main checkout pulls a profiler change every comment in the window is edited once.

**The tasks** run as the current user only while that user is logged on, so no password is stored, without elevation. They start late if the machine was off at the set time, and a run still going when the next is due makes it wait. They use the user's own `gh` login and environment. Register them from the main checkout's copy, never a worktree's; the script refuses if the scripts are not on the main checkout yet. Reconcile on a short window (a day or two) can fail its tolerances on few sessions; the weekly 7-day window passed on 2026-10-03.
