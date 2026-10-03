# Token profiler

Measures what AI-assisted work in this repo costs: tokens, estimated list-price USD, context pressure and orchestration overhead, per session, agent, campaign and PR. It reads local Claude Code transcripts into a local SQLite store; nothing it reads leaves the machine except scrubbed aggregates.

Status: **Wave 1.** The data contract is in place and the reports (TP-01b) read it; the ingest (TP-01a) is in progress. The plan, the decisions and the cut-line log are in the ledger, [docs/analysis/token-usage/](../../docs/analysis/token-usage/README.md); the issue is [#957](https://github.com/SandboxServers/Cimmeria/issues/957).

| File | What it is |
|---|---|
| [`schema.sql`](schema.sql) | The SQLite schema the ingest writes and the reports read. Version 1. |
| [`transcript-format.md`](transcript-format.md) | The transcript shapes the profiler depends on, and the trigger classification rules. |
| [`attribution.md`](attribution.md) | How a request is charged to a PR, and the invariants every attribution keeps. |
| [`fixtures/build_fixtures.py`](fixtures/build_fixtures.py) | Builds a synthetic transcript tree, with an `expected.json` of what a correct ingest produces and hostile values a report must never show. |
| [`report/`](report/) | The reports: raw tokens, estimated USD, context pressure, tool exposure, cost per merged PR and a cache-policy simulator, behind a privacy scrubber. See [Reports](#reports). |
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

Session ids, agent ids, project directories, branch names, teammate names and message text never reach a report.
