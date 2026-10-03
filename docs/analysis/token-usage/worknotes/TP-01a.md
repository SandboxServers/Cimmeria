# TP-01a worknote: profiler ingest

> Type: worknote. Packet TP-01a of [#957](https://github.com/SandboxServers/Cimmeria/issues/957); ledger [README.md](../README.md). Written 2026-10-03 by the TP-01a worker at the end of its one phase.

## Done

- `tools/token-profile/ingest/`, stdlib only: incremental transcript reader, final-record dedupe, trigger classification R1-R15, fingerprints, versioned price table (`prices.py`, version `2026-10-03`), PR attribution A1-A6 with the `attribution_imbalance` check, `branch_heads` from all three sources. Run: `python tools/token-profile/ingest --db <file> --repo . --fetch-prs`.
- 48 ingest tests in `ingest/test_*.py`, run by the existing CI step (`python3 -m unittest discover -s tools/token-profile -p "test_*.py"`, now named "token-profile contract and ingest tests"). The dedupe and the A3 squash guard were checked by reverting them: the tests fail.
- Schema version 2, all additive: `ingest_files.parse_state`, `tool_calls.task_id`, `tool_calls.pr_ref`, `prs.closed_at`. The contract docs (`transcript-format.md`, `attribution.md`, `README.md`) are updated to match.

## Smoke run on the local transcripts (2026-10-03, aggregates only)

| Measure | Value |
|---|---|
| Files / records / requests | 915 / 443,915 / 90,825 |
| Unknown shapes / attribution imbalance | 0 / 0 |
| Estimated list USD vs Claude Code's `cost-state` total | $11,301 vs $11,198 (0.9%) |
| Event-triggered share of coordinator main-session spend | about 57% (ledger quick pass: about 53%) |
| Spend placed by branch / ancestry / pr-link + split / parent-session / trigger | 38.8 / 11.4 / 10.9 / 6.4 / 4.9% |
| Unattributed spend | 27.6% |
| First full run / re-run | 77 s / about 5 s |

## Contract changes (say so in the ledger)

1. **Schema 2.** The resume state of an appended file needs storing (`parse_state`); A1 and A4 need the agent or background-task id a call started (`task_id`); A1 and A5 need the PR a call names (`pr_ref`), which the scrubbed fingerprint drops; the branch window needs `closed_at`.
2. **A3 tests the PR's head commit, not its merge commit.** `main` takes squash merges, so a packet commit is never an ancestor of the squash commit. A commit that `main` already had before the merge (merge commit's first parent) is skipped.
3. **`ref-snapshot` branch heads are dated by committer date**, not run time, so a backfill can place old requests.
4. **An `auxiliary` turn start doesn't replace a pending turn start of another kind.** Claude Code writes `isMeta` records right after human prompts; read literally, most human turns became `auxiliary` (727 folds on the local data).
5. R12 also matches `<command-message>`; a transcript opening on a request gets a `<request_id>:orphan` trigger (`unknown`, R15); workflow agents under `subagents/workflows/` are read as subagents.
6. A coordinator also includes a session that received a background completion of its own shell command, so A1's shell-command case can apply.

## Left / open questions

- **Campaign attribution** has no column in the schema. Reports can derive it from the PR or branch; if the coordinator wants it stored, it's a schema 3 change.
- **TP-01b** must expect `schema_version` `2` and may use `tool_calls.task_id` / `pr_ref`. Reports compute USD from `price_tables` (or `ingest.prices.estimate_usd`); nothing per-request is stored in dollars.
- `README.md`'s status line changed here; TP-01b will likely touch the same line.
- 878 assistant records repeated a `requestId` from another file (copied history); the first file owns the request. TP-05 may want to check that against `cost-state`.
- Only 1 of 91 API errors was followed by a retry in the same turn; the rest end their turn. Worth a look in TP-05 if retries matter.

## Branch and commands

- Branch `feat/token-profile-ingest`, worktree `.claude/worktrees/tp01a`.
- Tests: `python -m unittest discover -s tools/token-profile -p "test_*.py"` (about 45 s on Windows, mostly the every-line incremental test).
