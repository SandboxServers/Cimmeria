# Token profiler

Measures what AI-assisted work in this repo costs: tokens, estimated list-price USD, context pressure and orchestration overhead, per session, agent, campaign and PR. It reads local Claude Code transcripts into a local SQLite store; nothing it reads leaves the machine except scrubbed aggregates.

Status: **Wave 2.** The data contract, the ingest (TP-01a), the reports (TP-01b) and the reconciliation against Claude Code's own totals (TP-05) are in place. The plan, the decisions and the cut-line log are in the ledger, [docs/analysis/token-usage/](../../docs/analysis/token-usage/README.md); the issue is [#957](https://github.com/SandboxServers/Cimmeria/issues/957).

| File | What it is |
|---|---|
| [`schema.sql`](schema.sql) | The SQLite schema the ingest writes and the reports read. Version 3. |
| [`transcript-format.md`](transcript-format.md) | The transcript shapes the profiler depends on, and the trigger classification rules. |
| [`attribution.md`](attribution.md) | How a request is charged to a PR, and the invariants every attribution keeps. |
| [`fixtures/build_fixtures.py`](fixtures/build_fixtures.py) | Builds a synthetic transcript tree, with an `expected.json` of what a correct ingest produces and hostile values a report must never show. |
| [`ingest/`](ingest/) | The ingest: transcripts into the schema, incrementally, with trigger classification, prices and PR attribution. Its tests are `ingest/test_*.py`. |
| [`report/`](report/) | The reports: raw tokens, estimated USD, context pressure, tool exposure, cost per merged PR and a cache-policy simulator, behind a privacy scrubber. See [Reports](#reports). |
| [`reconcile/`](reconcile/) | Checks the profiler against Claude Code's `cost-state` records and OTel `api_request` events, with tolerances. See [Reconciliation](#reconciliation). |
| [`cutlines/`](cutlines/) | Before-and-after measures across the ledger's cut lines. See [Cut lines](#cut-lines). |
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
| `--lane-log <file>` | The build lane's job log, for branch heads; default the lane's own `jobs.jsonl` if present. |
| `--allow-unknown` | Exit 0 although `unknown_shapes` has rows. |

Re-running is cheap: each file is read from where the last run stopped, and a run with nothing new adds nothing. Attribution is rebuilt on every run, so a PR opened later places requests made before it. The run prints a JSON summary of counts (never transcript content) and exits 1 when unknown transcript shapes are recorded, 2 when `attribution_imbalance` is not empty.

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
| Estimated USD | The same breakdowns priced per model from the stamped price table, plus requests whose model has no price. Always labelled a plan-usage proxy, not a bill (D-TP1) | USD |
| Context pressure | Context per request and peak per transcript, requests per transcript, the first request of each transcript by agent type (static context), cache writes by idle gap, and compactions | tokens |
| Tool results and exposure | Result characters and exposure (characters x later requests) by tool, by command or path fingerprint, and by MCP server | characters, never USD |
| Cost per merged PR | For PRs merged in the window, each PR's whole attributed spend, its distribution and top-10% share, the unattributed share of window spend, and the method mix. Unattributed spend is reported, never spread onto PRs | USD |
| Cache-policy simulation | Each transcript replayed under a 5m and a 1h TTL over its real idle gaps, by agent type, with a calibration ratio against the observed cache cost | USD |

Every distribution gives n, p50, p75, p90, p95, p99, max and mean. Every report opens with its stamp: window, profiler commit, report commit, price table, Claude Code versions, models, schema version, unknown-record counts, and how many values the privacy filter rejected.

`report.sections.prs.pr_records(db, scrubber, [pr, ...])` returns the database-backed fields of the `cimmeria-pr-stats/1` block, for TP-10's per-PR comments.

### Privacy

The repo is public, so a report is built to be safe to commit. [`report/scrub.py`](report/scrub.py) applies three layers, and its tests prove that each one is needed:

1. **Field validation.** Every free-text value a report shows must have the shape the contract promises. A command fingerprint is one or two plain words, a path fingerprint is repo-relative, an MCP fingerprint is the tool name. Anything else becomes `<invalid>` and is counted in the stamp, so an ingest bug shows up instead of leaking.
2. **Redaction.** Every string is scrubbed of URL credentials and query strings, non-public hosts, auth headers, secret flags and assignments, known token formats, high-entropy tokens, emails, IP addresses, absolute local paths, and the deny words: the local user and machine names, `--deny` and the comma-separated `TOKEN_PROFILE_DENY`.
3. **The gate.** The rendered Markdown and JSON are searched again with the same detectors. A hit names the detector, never the value, and nothing is written (exit code 3).

Session ids, agent ids, project directories, branch names, teammate names and message text never reach a report. Claude Code writes an in-process teammate's name as its `agentType`, so a teammate with no `.claude/agents` definition is reported as `teammate`.

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

The undercount is real and expected: the cost-state holds requests that no transcript records. By model (Haiku aside), the transcripts hold 1-10% of the cost-state's input tokens, 73-97% of its cache reads, 91-100% of its cache writes and 98-99% of its output: the missing requests read a large cached context, send fresh input and write little. The gap grows with subagent activity (correlation 0.83 with a session's subagent requests). The OTel `query_source` of those requests should name them; until it does, the profiler's USD is a floor about 10% under Claude Code's.

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
