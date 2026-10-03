# Token profiler

Measures what AI-assisted work in this repo costs: tokens, estimated list-price USD, context pressure and orchestration overhead, per session, agent, campaign and PR. It reads local Claude Code transcripts into a local SQLite store; nothing it reads leaves the machine except scrubbed aggregates.

Status: **Wave 0, the data contract.** The ingest (TP-01a) and the reports (TP-01b) are not written yet. The plan, the decisions and the cut-line log are in the ledger, [docs/analysis/token-usage/](../../docs/analysis/token-usage/README.md); the issue is [#957](https://github.com/SandboxServers/Cimmeria/issues/957).

| File | What it is |
|---|---|
| [`schema.sql`](schema.sql) | The SQLite schema the ingest writes and the reports read. Version 1. |
| [`transcript-format.md`](transcript-format.md) | The transcript shapes the profiler depends on, and the trigger classification rules. |
| [`attribution.md`](attribution.md) | How a request is charged to a PR, and the invariants every attribution keeps. |
| [`fixtures/build_fixtures.py`](fixtures/build_fixtures.py) | Builds a synthetic transcript tree, with an `expected.json` of what a correct ingest produces and hostile values a report must never show. |
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
