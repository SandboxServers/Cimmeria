# Campaign metrics: work packets

> Type: reference (packets). Ledger: [README.md](README.md). Every packet extends [`tools/token-profile/`](../../../tools/token-profile/README.md); read its README rules first (dedupe by `requestId`, price per model, aggregate-only output).

## Dispatch rules

- **Integration branch:** all packets build on `campaign-metrics/ledger`. Each packet gets a worktree and branch `cm/<id>` (for example `cm/cm-01`) started from `campaign-metrics/ledger`, and after review is merged back into it locally. No per-packet PR and no push: one PR for the whole campaign at close-out (CM-10). The `Branch/worktree` lines in the packets below name older per-packet branch names; use `cm/<id>` instead.
- One worktree and one `packet-coder` per packet: `pwsh tools/build-lane/mk-worktree.ps1 cm/<id> cm-<nn>`, based on `campaign-metrics/ledger`.
- PowerShell only. No bash, no direct `cargo` (none is needed: this is Python), no `git stash`, no `git worktree prune`. Run every command from the worktree: `Set-Location <worktree>;` at the start of each call.
- Python 3.11+, standard library only (the profiler's rule). `python -I` for anything that reads untrusted data.
- Edit only the files the packet names. docs/**/*.md are CRLF.
- Tests are Python `unittest`, beside the code (`test_*.py`), run with `python -m unittest discover -s tools/token-profile -p "test_*.py"` from the worktree root. Per [TESTING.md](../../../TESTING.md): a pure function gets a unit test with a fixture; a CLI gets a smoke test that runs it on a fixture and compares JSON. Each test must fail when the change is reverted; name the assertion that does.
- Reports go through the privacy gate (`report/scrub.py`); never print a branch name, session id, prompt text or absolute path.
- A worker reports once, with results. At most three `packet-coder` sessions per packet; the fourth is a Sonnet worker. Every commit gets a `packet-reviewer`.
- Commit locally with the subject given; end the message with:
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_01HnSuKqEVrRFZW3BDBLzBHG`.
- Each new tool gets a summary line (module docstring) and a row in [tools/README.md](../../../tools/README.md#tools-catalog). Never hand-edit a generated block (`<!-- gen:NAME -->`); the catalog rows are owed by the docs packet if the generator does not pick them up.

## Contract

All packets build against this. One new package: **`tools/token-profile/metrics/`**, run as `python tools/token-profile/metrics <subcommand>`.

```
tools/token-profile/metrics/
  __init__.py  __main__.py  cli.py
  config.json        # window, regime switch, role map, overstep rules (no prices)
  registry.py        # CM-01
  schema_v6 lives in tools/token-profile/schema.sql + ingest/ (CM-02)
  github.py          # CM-04
  ledger.py          # CM-05
  cost.py            # CM-03
  packets.py         # CM-06
  overstep.py        # CM-07
  report.py          # CM-08
```

Pure functions take plain dicts and lists, so tests need no database or network. The DB (`sqlite3`, read-only, `mode=ro`) is opened only in `cli.py` and `cost.py`/`packets.py`/`overstep.py` entry functions named `load_*`.

**`config.json`** (CM-01 creates it):

```json
{
  "window": {"since": "2026-09-21T00:00:00Z", "until": "2026-10-13T00:00:00Z"},
  "regime_switch": "2026-10-10T20:02:54Z",
  "roles": {
    "packet-coder": "implementer-haiku",
    "packet-reviewer": "reviewer",
    "packet-review-fixer": "fixer",
    "rust-gameserver-dev": "implementer-domain",
    "lab-driver": "lab-driver"
  },
  "accepted_statuses": ["Integrated", "UATPending", "Done"]
}
```

Prices are not here: use `ingest/prices.py` (`estimate_usd`, `CURRENT`, `normalize_model`).

**Output schema `cimmeria-campaign-metrics/1`**, one object per campaign, written to `docs/analysis/campaign-metrics/data/<campaign>.json`, with `data/summary.json` rolling up. Every number may be `null` with a sibling `"<field>_gap": "<reason>"`.

```json
{
  "schema": "cimmeria-campaign-metrics/1",
  "campaign": "lab-record",
  "regime": "haiku|old|mixed",
  "stamp": {"generated": "...", "window": {...}, "price_table": "2026-10-10", "profiler_schema": 6, "unattributed_share": 0.157},
  "cost": {"total_usd": 0, "by_model_tier": {"opus": 0, "sonnet": 0, "haiku": 0, "fable": 0},
           "by_role": {"coordinator": 0, "implementer-haiku": 0, "reviewer": 0, "fixer": 0, "other": 0},
           "attributed_usd": 0, "unattributed_in_campaign_usd": 0},
  "prs": {"count": 0, "merged": 0, "cost_per_pr": {"mean": 0, "p50": 0, "p90": 0}, "per_pr": [{"pr": 1527, "usd": 0, "ci_first_pass": true, "ci_runs": 1}]},
  "accepted": {"packets": 0, "cost_per_accepted_usd": 0},
  "first_pass": {"rate": 0, "eligible": 0, "packets": [{"id": "LR-01", "implementer_sessions": 1, "reviewer_sessions": 1, "fix_commits": 0, "first_pass": true}]},
  "retries": {"total": 0, "by_reason": {"context-exhausted": 0, "stalled-background": 0, "permission-denied": 0, "review-fix": 0, "ci-failure": 0, "merge-conflict": 0, "other": 0}},
  "ci": {"first_attempt_rate": 0, "overall_rate": 0, "code_prs_first_attempt_rate": 0},
  "escaped_defects": {"count": 0, "rate_per_merged_pr": 0, "items": [{"kind": "pr|issue|revert", "number": 1, "cites": 1527}]},
  "escalation": {"dispatched_to_haiku": 0, "escalated": 0, "rate": 0, "items": [{"id": "LR-02", "to": "rust-gameserver-dev", "why": "context-exhausted"}]},
  "stop_context": {"workers": 0, "by_stop": {"end_turn": 0, "tool_use": 0, "error": 0, "no_record": 0}, "by_error_kind": {"rate_limit": 0, "authentication_failed": 0, "other": 0}, "context_at_stop": {"p50": 0, "p90": 0, "max": 0}, "share_above_90k": 0},
  "cycle_time": {"campaign_hours": 0, "packet_hours": {"n": 0, "p50": 0, "p90": 0}},
  "overstep": {"opus_coding_usd": 0, "opus_research_usd": 0, "sonnet_implementing_usd": 0, "recoverable_usd": {"low": 0, "high": 0}, "items": []}
}
```

`data/summary.json` has `"regimes": {"old": {...}, "haiku": {...}}` with the same fields aggregated, plus `"comparison"` (see CM-08). Markdown per campaign goes to `docs/analysis/campaign-metrics/reports/<campaign>.md`, the roll-up to `reports/summary.md`.

**`stop_context.by_stop` classification.** CM-06 classifies each worker's `agent_stops` row, first match wins: no row -> `no_record`; `ended_in_api_error = 1` -> `error`; `last_stop_reason = 'end_turn'` -> `end_turn`; `last_stop_reason = 'tool_use'` -> `tool_use`; `last_stop_reason` NULL (interrupted before a final record) -> `no_record`; any other value (`max_tokens`, `refusal`, `pause_turn`) -> `error`. Each `error` row with `ended_in_api_error = 1` also counts in `by_error_kind[api_error_kind]`. A tool call the user refused is not an API error: it stays `tool_use` with `last_result_is_error = 1`. An API-error stop maps to the retry reason `other`; the `retries.by_reason` keys do not change.

**CLI** (`cli.py`, argparse subcommands; every subcommand takes `--out <dir>`, `--config`, and prints a JSON count summary, never content):

```
python tools/token-profile/metrics registry   --repo . [--check]            # CM-01
python tools/token-profile/metrics github     --repo . --out <dir> [--from-json <f>]   # CM-04
python tools/token-profile/metrics ledger     --repo . --out <dir>          # CM-05
python tools/token-profile/metrics cost       --db <sqlite> --out <dir>     # CM-03
python tools/token-profile/metrics packets    --db <sqlite> --ledger <f> --github <f> --out <dir>   # CM-06
python tools/token-profile/metrics overstep   --db <sqlite> --ledger <f> --out <dir>   # CM-07
python tools/token-profile/metrics report     --inputs <dir> --out docs/analysis/campaign-metrics [--self-session <id>...]   # CM-08
```

Intermediate files go in a scratch `--out` dir (never committed): `github.json`, `ledger.json`, `cost.json`, `packets.json`, `overstep.json`. CM-08 reads them.

---

## CM-01 Campaign registry, regime config, missing tags

- Branch/worktree: `campaign-metrics/cm01-registry`, `cm-01`. Commit: `feat(token-profile): campaign metrics registry and regime config`.
- Why: 1,071 PRs in the DB have `campaign = NULL` because `campaigns.json` omits every campaign from 10-10 on and the generic-prefix ones.
- Files:
  - Edit `tools/token-profile/campaigns.json`: add one entry per ledger folder that lacks one and has a unique branch prefix. Prefixes confirmed from `gh pr list`: `lab-spec-vocab/`, `lab-fixtures/`, `lab-golden/`, `lab-chaos/`, `lab-record/`, `lab-watch/`, `lab-cli-rust/`, `lab-harness/`, `client-limits/`, `telemetry-gaps/`, `launcher-consolidation/`, `wireclient-ci/`, `any-install-adoption/`, `campaign-metrics/`, `cs6/` -> `class-start-v6`, `debugarea/` -> `debug-area`, `dakara/` -> `dakara-e1-rebuild`, `abilities/` -> `ability-mechanics`. Skip `launcher/` (an earlier effort than `launcher-consolidation`). Add `lab-parallel-clients` by tracking issue `1312`. Do not guess any prefix not in this list: `Get-ChildItem docs/analysis -Directory` is the check, and a prefix with no folder is a test failure.
  - Create `tools/token-profile/metrics/__init__.py` (docstring: "Campaign metrics (CM-01..): cost, packets, CI, defects, overstep per campaign."), `__main__.py` (copy `reconcile/__main__.py`'s shape), `config.json` (contract above), `registry.py`, `cli.py` with only the `registry` subcommand.
  - `registry.py`: `load_config(path) -> dict`; `regime_of(ts_iso, config) -> "old"|"haiku"`; `campaign_of_pr(pr: dict, campaigns: dict) -> str|None` is NOT reimplemented: import the existing tagging function from `ingest/campaigns.py` (read it first; call it, do not copy). `registry --check` exits 1 if a `campaigns.json` key has no `docs/analysis/<key>/` folder or a prefix collides with another campaign.
- Test (`metrics/test_registry.py`): (1) `regime_of("2026-10-10T20:02:53Z")` is `old`, `...54Z` is `haiku`; reverting the comparison to `<` makes the boundary case fail. (2) A fixture `campaigns` dict with two overlapping prefixes makes `--check` return 1. (3) Real files: every `campaigns.json` key has a ledger folder (the existing ingest test already does this; extend it, do not duplicate; if it exists just run it).
- Checks, in order: `python -m unittest discover -s tools/token-profile -p "test_*.py"`; `python tools/token-profile/metrics registry --repo . --check`.
- Docs owed: `tools/README.md` catalog row for `tools/token-profile/metrics/`; token-profile README file table row.
- Done when: both checks green and one commit.

## CM-02 Schema v6: stop_reason and agent_stops

- Branch/worktree: `campaign-metrics/cm02-stop-context`, `cm-02`. Commit: `feat(token-profile): schema v6 records stop reason and how each worker stopped`.
- Why: stop context needs the last record of each worker, and `stop_reason` is dropped today.
- Read first: `tools/token-profile/transcript-format.md`, `ingest/transcript.py` (where the final record per `requestId` is kept), `ingest/store.py` (schema version check), `fixtures/build_fixtures.py` (+ `expected.json` shape). Do not read other files.
- Changes:
  1. `schema.sql`: bump the version constant to 6. Add column `stop_reason TEXT` to `requests` (the final record's `message.stop_reason`, NULL if absent). Add table:
     ```sql
     CREATE TABLE agent_stops (
         agent_id          TEXT NOT NULL,       -- agents.agent_id, or 'main:' || session_id for a main session
         session_id        TEXT NOT NULL REFERENCES sessions (session_id),
         last_ts           TEXT NOT NULL,
         requests          INTEGER NOT NULL,
         last_stop_reason  TEXT,                -- stop_reason of the last request
         last_context_tokens INTEGER NOT NULL,  -- context_tokens of the last request
         last_tool_name    TEXT,                -- tool of the last tool_use, if the last request ended in tool_use
         last_tool_background INTEGER NOT NULL DEFAULT 0 CHECK (last_tool_background IN (0, 1)),
         last_result_is_error INTEGER,          -- result of that tool call, NULL if none seen
         ended_after_result  INTEGER NOT NULL DEFAULT 0 CHECK (ended_after_result IN (0, 1)),  -- transcript ends with a tool_result, no later assistant text
         ended_in_api_error  INTEGER NOT NULL DEFAULT 0 CHECK (ended_in_api_error IN (0, 1)),  -- the transcript's last assistant record is a synthetic API error (never a request row)
         api_error_kind    TEXT,                -- that record's `error` value (rate_limit, authentication_failed, ...), 'other' if absent or not a plain name; NULL when ended_in_api_error = 0
         PRIMARY KEY (agent_id)
     );
     ```
  2. `ingest/transcript.py`: carry `stop_reason` into the request row. `ingest/store.py`: write `agent_stops` from the last request of each transcript, recomputed per run for transcripts that grew (the ingest is incremental; delete and reinsert that agent's row). `last_tool_background` = the last tool_use input has `run_in_background: true`. The ingest also sets `ended_in_api_error` = 1 and `api_error_kind` (the record's top-level `error` value when it is lower-case letters and underscores, at most 40 characters, else `other`; never the message text) when the transcript's last assistant record is a synthetic `isApiErrorMessage` record, which is never a request row; a later real assistant record clears both.
  3. Bump the "older schema refused" message path: a v5 database must be refused with the existing message.
  4. `fixtures/build_fixtures.py`: add `stop_reason` to generated assistant records (alternate `end_turn` and `tool_use`) and extend `expected.json` with the expected `agent_stops` rows for two agents (one ends `end_turn`, one ends mid-`tool_use` with a backgrounded Bash).
- Tests: `ingest/test_ingest_fixture.py` asserts the two `agent_stops` rows exactly; `test_contract.py` asserts the new column and table exist. Reverting the transcript change leaves `stop_reason` NULL, so the fixture test fails. Add a test that a second incremental run with an appended record updates the row instead of duplicating it. A test in `ingest/test_ingest_stops.py` appends a tool_result and then a synthetic API error record to the fixture's main transcript and asserts the stop row; reverting the transcript change leaves `ended_in_api_error` 0.
- Checks: `python -m unittest discover -s tools/token-profile -p "test_*.py"`.
- Docs owed: `transcript-format.md` (the `stop_reason` field and the `agent_stops` rules), token-profile README (schema version 6), `docs/guides/token-profiling.md` one line (re-ingest a v5 DB from scratch, about two minutes).
- Risk: M. If the packet-coder runs out of context on `transcript.py`, a fresh session continues from the worknote; after three sessions, hand to a Sonnet worker.

## CM-04 GitHub collector: PRs, CI per head commit, escaped defects

- Branch/worktree: `campaign-metrics/cm04-github`, `cm-04`. Commit: `feat(token-profile): campaign metrics GitHub collector`.
- Files: create `metrics/github.py`, add the `github` subcommand to `metrics/cli.py` (CM-01 creates `cli.py`; if it is not merged yet, create a minimal `cli.py` with only this subcommand and CM-01's rebase merges them by hand), `metrics/test_github.py`, fixture `metrics/fixtures/gh_prs.json`, `gh_checks.json`, `gh_issues.json`.
- `gh` runs through one function `run_gh(args: list[str]) -> object` (subprocess, JSON out). Tests inject a fake. `--from-json <dir>` replays saved JSON.
- Functions (pure, tested on fixtures):
  - `collect_prs(run_gh, since) -> list[dict]`: `gh pr list --state all --search "created:>=<since>" --limit 1000 --json number,title,body,headRefName,baseRefName,createdAt,mergedAt,closedAt,state,additions,deletions,changedFiles,files`. Keep `files` only to derive `code_pr` (any path starting `crates/`, `tools/`, `db/`, `.github/`); drop the list afterwards. Never store `headRefName` in a committed output.
  - `head_commits(run_gh, pr) -> list[{sha, committed}]`: `gh api repos/{owner}/{repo}/pulls/<n>/commits --paginate`. Last page can be large; cap at 100.
  - `ci_result(run_gh, sha) -> {gate: "success|failure|none", failed_jobs: [...]}`: `gh api repos/{owner}/{repo}/commits/<sha>/check-runs --paginate`; the gate is the check run named `ci gate` (CLAUDE.md lists it as the blocking aggregate). `none` if absent (docs-only paths may skip it: record `none`, do not count it as a failure).
  - `ci_summary(pr, commits, results) -> {first_attempt: bool|None, overall: bool|None, runs: int}`: first attempt = result for the earliest commit that has a `ci gate`; overall = result for the last commit before merge; `runs` = number of commits with a gate.
  - `escaped_defects(prs, issues, campaign_of) -> dict[campaign, list]`: rule (a) PR not in campaign C, title matches `^(fix|revert)`, text cites `#N` where N is a merged PR of C and `mergedAt` of the fix is later; rule (b) a closed issue labelled `bug` whose body cites such an N and whose `createdAt` is after N's merge; rule (c) `Revert "..."` titles quoting the subject of a C PR. Each hit is `{kind, number, cites, days_after_merge}`.
- Rate limits: sleep 0.2 s between calls, one retry on HTTP 403/429 after 60 s; the `--limit` and `--since` flags bound the run. Expect about 1,200 PRs x 2 calls; allow `--max-prs` for a trial.
- Tests: fixture of 4 PRs (green first try; red then green; no gate; a fix PR citing PR 1) asserts `ci_summary` for each and that the fix PR is returned as an escaped defect of the campaign owning PR 1. Reverting the "earliest commit with a gate" logic to "last commit" fails the red-then-green case.
- Checks: `python -m unittest discover -s tools/token-profile -p "test_*.py"`; a live trial `python tools/token-profile/metrics github --repo . --out $env:TEMP\cm --max-prs 5` (read-only).
- Docs owed: catalog row; one paragraph in the guide: needs `gh auth`, read-only.

## CM-05 Ledger parser

- Branch/worktree: `campaign-metrics/cm05-ledger`, `cm-05`. Commit: `feat(token-profile): campaign metrics ledger parser`.
- Files: `metrics/ledger.py`, `metrics/test_ledger.py`, fixture `metrics/fixtures/ledger_readme.md`, `ledger_worknote.md`, `metrics/cli.py` subcommand `ledger`.
- Parse every `docs/analysis/*/README.md` and `work-packets*.md` packet table. Two layouts exist: `| ID | Packet | Implementer | Size | Wave | Depends on | Status |` (lab-record) and older ones; find the header row by the cell `ID`/`Packet`/`Status` and map columns by name, tolerating missing ones.
- Functions:
  - `parse_packet_table(text) -> list[dict]`: `{id, title, implementer_raw, size, wave, depends, status_raw}`.
  - `normalize(row) -> dict`: `status` = first word of `status_raw` from the allowed set (Ready, BlockedDependency, BlockedDecision, Writing, Review, Integrated, UATPending, Done; else `unknown`); `prs` = all `#NNNN` in `status_raw`; `implementer` = `packet-coder` if the cell starts with it, `rust-gameserver-dev`, `documentation-writer`, `coordinator`, `sonnet`, else `other`; `implementer_note` = text in parentheses (the reason, e.g. "Haiku ran out of context"); `escalated` = true when the cell names a non-Haiku implementer together with a phrase from the list `ran out of context`, `stalled`, `took over`, `finished by`, `escalated`, `Haiku`.
  - `retry_reason(text) -> str`: keyword map to `context-exhausted` (`ran out of context`, `context cap`, `out of context`), `stalled-background` (`background`, `waiting for the notification`, `stalled`), `permission-denied` (`denied`, `permission`), `review-fix` (`review`, `finding`), `ci-failure` (`CI`, `clippy`, `nextest failed`), `merge-conflict` (`conflict`, `rebase`), else `other`. Apply to `implementer_note`, the Status text and the packet's `worknotes/<id>.md` first 60 lines when present.
  - `campaign_pr_map(rows) -> dict[int, str]`: every PR cited in a campaign's table maps to that campaign (a PR cited by two campaigns is returned in a separate `conflicts` list).
  - `campaign_window(campaign, git_log_runner) -> {start, end}`: start = author date of the first commit that touched `docs/analysis/<campaign>/` (`git log --reverse --format=%aI -- <dir>`); end = date of the last commit whose subject has `close-out` or `closeout`, else None.
- Test: fixture with 3 rows (Integrated with one PR; `rust-gameserver-dev (Haiku ran out of context)`; `Ready`) asserts statuses, PR lists, `escalated`, and reason `context-exhausted`. Reverting the parenthetical extraction fails the escalation case. A second test runs `ledger` over the real `docs/analysis/lab-record/README.md` and asserts at least 10 packets parse and LR-02 is escalated (this guards against the table format changing).
- Checks: `python -m unittest discover -s tools/token-profile -p "test_*.py"`; `python tools/token-profile/metrics ledger --repo . --out $env:TEMP\cm` then report the packet count per campaign in the commit body (not in a file).
- Docs owed: catalog row.

## CM-03 Cost rollup

- Depends on CM-01. Branch/worktree: `campaign-metrics/cm03-cost`, `cm-03`. Commit: `feat(token-profile): campaign metrics cost rollup`.
- Files: `metrics/cost.py`, `metrics/test_cost.py`, fixture builder reuse: build the DB with `report/fixture_db.py` (read it; extend minimally if a role split needs an agent row).
- `load_requests(conn, since, until) -> iterator[dict]`: join `requests`, `agents` (role by `custom_agent_type` then `agent_type`), `pr_attribution` (weight, campaign via `prs.campaign` or `pr_attribution.campaign`), and the ledger PR map (a PR with `campaign` NULL takes `ledger_pr_map[pr]` when CM-05 output is passed with `--ledger`; otherwise it stays untagged and the stamp says so).
- `tier_of(model) -> "opus|sonnet|haiku|fable|other"` from `prices.normalize_model`.
- `rollup(requests, price_table) -> dict[campaign, dict]` as the `cost` and `prs` parts of the contract: USD = `estimate_usd(request, price_table) * weight`. `by_role`: coordinator = `agent_id IS NULL`; others by `config.roles`, else `other`. Per-PR totals feed `cost_per_pr` (mean, p50, p90 over merged PRs only). `cost_per_accepted_usd` = campaign cost / accepted packets (count passed in from the ledger; if none, use merged PRs and set `accepted_gap: "no packet table, used merged PRs"`).
- `unattributed_share(conn, since, until)`: weight of method `unattributed` / total weight, by request count and by USD, global and per `sessions.worktree` family. Put in the stamp.
- Price check step: print the table in force and its `SOURCE`; the worker adds nothing to `prices.py`. The coordinator verifies prices against the pricing page before CM-10 (D-CM2).
- Tests: a 3-campaign fixture DB (Opus coordinator, Haiku coder, Sonnet reviewer, one unattributed request, one split-weight request) asserts totals to the cent against hand-computed numbers from the price table, that a split request contributes once across its PRs, and that the unattributed request lands in no campaign but in the stamp. Reverting weight multiplication fails the split case.
- Checks: unittest; then `python tools/token-profile/metrics cost --db ~/token-profile.sqlite --out $env:TEMP\cm` (read-only) and put the top 5 campaigns by cost in the commit body.
- Docs owed: catalog row; guide paragraph on what "cost" means (list-price floor).

## CM-06 Packet join: retries, first pass, escalation, stop context, cycle time

- Depends on CM-01, CM-02, CM-05 (and consumes CM-04's `github.json`). Branch/worktree: `campaign-metrics/cm06-packets`, `cm-06`. Commit: `feat(token-profile): campaign metrics packet join`. Risk M: first try packet-coder; three sessions max, then Sonnet.
- Files: `metrics/packets.py`, `metrics/test_packets.py`.
- `packet_of_session(agent, worktree, branch_hint, ledger_rows, campaign) -> packet_id|None`: match in order (1) worktree name or branch hint containing the packet ID lowercased without dash (`lr10`, `lr-10`, `cm01`); the campaign's branch pattern is `<campaign>/<id>-<slug>` so use that; (2) the agent description/name; (3) None, counted as `unmapped`. Report the unmapped share of worker sessions.
- Per packet, from `agents` rows mapped to it:
  - `implementer_sessions` = count of agents with role `implementer-haiku`; `reviewer_sessions` = role `reviewer`; `fixer_sessions` = role `fixer`.
  - `escalated` (also from the ledger) = a Haiku implementer was followed in the same worktree by an `implementer-domain` or a `general-purpose` Sonnet agent, or the ledger row says so; `escalation_to` records which; `why` from `stop_context` of the last Haiku session (`context-exhausted` if its last context >= 90k, `stalled-background` if `last_tool_background = 1` and `ended_after_result = 0`, `permission-denied` if `last_result_is_error` and the error text class was denial, else `other`; the denial class is read from `tool_calls.result_is_error` only: do not read transcript text).
  - `retries` = max(0, implementer_sessions - 1), each with the reason of the prior session's stop.
  - `fix_commits` = commits on the PR head after the first reviewer session started: from `github.json` commit times; unknown if no PR.
  - `first_pass` = `implementer_sessions == 1 and fixer_sessions == 0 and fix_commits == 0 and not escalated` (accepted packets only; others excluded from the rate).
  - `cycle_hours` = PR `mergedAt` minus the earliest request of any mapped agent; campaign cycle from `ledger.campaign_window`.
  - `stop_context`: from `agent_stops`, per worker: `last_stop_reason`, `last_context_tokens`, `ended_in_api_error`, `api_error_kind`, classified by the Contract's `by_stop` rule and aggregated into the contract's `stop_context` (p50, p90, max, share above 90k).
- Tests: a fixture DB with three packets: A (one coder, one reviewer, no fixes), B (two coders: the first stops at 96k with `tool_use` and a backgrounded last tool, then a Sonnet agent in the same worktree), C (one coder, one fixer). Assert A first_pass true; B escalated, one retry with reason `stalled-background`, first_pass false; C first_pass false. Reverting the worktree match leaves all unmapped and fails.
- Checks: unittest; trial run on the real DB prints per-campaign first-pass rate for `lab-record` in the commit body.
- Docs owed: catalog row.

## CM-07 Overstep detection and wasted-cost estimate

- Depends on CM-01, CM-05. Branch/worktree: `campaign-metrics/cm07-overstep`, `cm-07`. Commit: `feat(token-profile): campaign metrics overstep detection`.
- Files: `metrics/overstep.py`, `metrics/test_overstep.py`; extend `metrics/config.json` with an `overstep` block:
  ```json
  "overstep": {
    "code_paths": ["crates/", "tools/", "db/", ".github/", "external/"],
    "allowed_coordinator_paths": ["docs/", ".claude/agent-memory/", "CLAUDE.md"],
    "research_tool_prefixes": ["mcp__ghidra__", "mcp__x64dbg__", "mcp__signoz__"],
    "research_read_burst": 25,
    "reprice": {"implementation": "haiku", "review": "sonnet", "research": "sonnet"}
  }
  ```
- Rules, applied to requests with `ts >= regime_switch` unless stated, using `tool_calls` (name, fingerprint path) joined to `requests`:
  - **O1 Opus coding**: a main-session (`agent_id IS NULL`) request on an Opus model whose tool calls include `Edit`, `Write`, `NotebookEdit`, or `Bash` fingerprints `lane`/`cargo` that are build, check or test runs, with a path under `code_paths` and not under `allowed_coordinator_paths`. Cost = that request's USD, plus the cache cost of the following 3 requests in the same session that were triggered by its tool results (the tool-result turn).
  - **O2 Opus research**: a main-session Opus request calling any `research_tool_prefixes` tool, or a burst of `research_read_burst` or more `Read`/`Grep` calls on `code_paths` within 10 minutes in one session. A coordinator reading a ledger or a worknote is not research.
  - **O3 Sonnet implementing**: a Sonnet-model request (any agent, including `general-purpose` and `rust-gameserver-dev`) calling `Edit`/`Write` on `code_paths` inside a worktree whose packet (via CM-06's `packet_of_session`; import it, do not copy) has ledger `implementer = packet-coder`, `size` S or M, and no recorded escalation. `packet-review-fixer` edits after a review are expected, not oversteps, unless the packet's `fix_commits` exceeds 2 (reported separately as `fixer-heavy`).
  - **O4 Opus subagent where Haiku/Sonnet exists**: an Opus-model `documentation-writer` or `general-purpose` agent working in a packet worktree in the Haiku regime (reported, not priced as waste: the ledger may have chosen it on purpose).
- `reprice(request, target_tier, price_table) -> usd`: same four token counts priced with the target tier's current model row (`haiku` -> `claude-haiku-5-5`, `sonnet` -> `claude-sonnet-5-5`), applying the Haiku long-prompt rule. `wasted = usd_actual - usd_repriced` (floor at 0). Report `recoverable_usd = {low, high}`: low = reprice at Sonnet for everything; high = reprice at Haiku for implementation. Say so in the report: an estimate that ignores extra turns a cheaper model may need.
- Output per campaign: counts and USD per rule, up to 20 `items` of `{rule, ts, session_family, model, usd, tool, path_class}` with `path_class` a top-level repo dir only (never a full path or a prompt).
- Tests: fixture requests for each rule, with one negative per rule (Opus editing `docs/analysis/x/README.md` is not O1; Sonnet fixer after review is not O3). Assert wasted USD to the cent for one O1 request (Opus 5.5 vs Haiku 5.5 rows). Reverting the allowed-paths exclusion fails the negative.
- Checks: unittest; trial run on the real DB prints per-rule totals in the commit body.
- Docs owed: catalog row; guide paragraph that overstep is an estimate and thresholds live in `config.json`.

## CM-08 Report generator, summary and pre/post comparison

- Depends on CM-03, CM-04, CM-06, CM-07. Branch/worktree: `campaign-metrics/cm08-report`, `cm-08`. Commit: `feat(token-profile): campaign metrics reports and regime comparison`.
- Files: `metrics/report.py`, `metrics/test_report.py`, `metrics/fixtures/inputs/` (the five intermediate JSON files for two campaigns), `metrics/cli.py` subcommand `report`. Reuse `report/render.py` table helpers and `report/scrub.py` (call the privacy gate on the rendered Markdown and JSON before writing; exit 3 on a hit as the profiler does).
- `assemble(inputs) -> dict[campaign, record]` merges `cost.json`, `packets.json`, `github.json`, `overstep.json`, `ledger.json` into the contract schema. `regime` per campaign: `haiku` if at least 70% of its implementer sessions are `packet-coder`, `old` if none, else `mixed`; mixed campaigns are split by packet for the regime roll-up (a packet's regime = implementer that ran).
- `compare(records) -> dict` for `summary.json`: for `old` and `haiku`: campaigns, PRs, total cost, cost per PR (p50, mean), cost per accepted change, first-pass rate, retries per accepted packet, CI first-attempt rate, escaped defects per merged PR, escalation rate, p50 packet cycle hours, share of cost by model tier, overstep recoverable range. A `delta` block gives haiku minus old, each with `n` for both sides and a `low_n` flag below 5 campaigns or 10 packets. Do not present a delta without the flag when n is small.
- Markdown per campaign (`reports/<campaign>.md`): stamp, one table per metric group, each number with its gap note. `reports/summary.md`: the comparison table first, then a campaign table sorted by cost, then caveats (unattributed share, floor, regime split, tiny samples), then **this campaign's own cost** (see CM-10).
- `--self-session <id>` (repeatable): sessions whose cost is added to the `campaign-metrics` record as `coordinator` role even if attribution left them unattributed, listed in the stamp as `self_sessions: n` (count only, never ids).
- Tests: two-campaign fixture (one old, one haiku) asserts the `compare` numbers, the `low_n` flag, that a rendered file contains no branch-like string (`campaign-metrics/cm`), no `C:\`, no 8-4-4-4-12 id, and that the privacy gate refuses a record with an injected absolute path. Reverting the gate call fails the last assertion.
- Checks: unittest; `python tools/token-profile/metrics report --inputs tools/token-profile/metrics/fixtures/inputs --out $env:TEMP\cm-out`.
- Docs owed: catalog row; token-profile README section "Campaign metrics".

## CM-09 Docs

- Implementer `documentation-writer`. Branch/worktree: `campaign-metrics/cm09-docs`, `cm-09`. Commit: `docs(campaign-metrics): guide, catalog and doc-update rows`.
- Update: `docs/guides/token-profiling.md` (a "Campaign metrics" section: what each metric means, how to rerun, the caveats list from the README), `tools/README.md` catalog rows for each `metrics/` module (the summary lines come from module docstrings), the token-profile README file table, [doc-update-map](../../agents/doc-update-map.md) row "campaign metrics tool" -> the guide, and `docs/readme.md` for the new folder. CRLF. Run `tools/lint-md.ps1`.
- No numbers in these docs; they live in `reports/`.

## CM-10 Close-out: run, check, own cost, status

- Coordinator plus `documentation-writer`; not a Haiku packet. Branch `campaign-metrics/cm10-closeout`.
- Steps, in order:
  1. Verify prices in `ingest/prices.py` against the pricing page (D-CM2); fix by adding a new price key in its own commit if needed.
  2. Re-ingest at the end of the campaign: `python tools/token-profile/ingest --db ~/token-profile.sqlite --repo . --fetch-prs` (schema v6 means a fresh DB, about two minutes; keep the v5 DB until the new one reconciles: `python tools/token-profile/reconcile --db <new>`).
  3. Run each subcommand into a scratch dir, then `report --inputs <dir> --out docs/analysis/campaign-metrics --self-session <coordinator session id> --self-session <each research/worker session family that was not attributed>`. The coordinator finds ids with the profiler's `sessions` table by `first_ts >= 2026-10-12T10:00Z` and worktree `campaign-metrics` or `cm-*`.
  4. **This campaign's own cost** goes into `reports/summary.md` under "Cost of this campaign": total USD by role (Opus coordinator, the Sonnet research agent that wrote these packets, Haiku coders, Sonnet reviewers and fixers), cost per accepted packet, retries, and the unattributed share for this campaign. State that the figure is measured at close-out and excludes the close-out session's own tail.
  5. Spot-check three campaign records against the sources by hand (PR count against `gh`, a packet's first-pass against its ledger row and worktree log, one overstep item against the transcript); record the three checks in the ledger.
  6. Commit `data/` and `reports/` (aggregate-only; run the privacy gate). Update the ledger packet table to Done, add findings, then (once) `docs/gap-analysis.md` / `docs/project-status.md` only if a gap row changes (probably none).
  7. No board post (D-CM9: results stay in the repo). Open the single campaign PR from `campaign-metrics/ledger`; retire worktrees with `pwsh tools/build-lane/rm-worktree.ps1 --merged`.
- UAT the owner still has: none (no runtime behavior). Owner reads `reports/summary.md` and confirms D-CM1 and the overstep thresholds.
