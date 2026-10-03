# TP-01b worknote: profiler reports and privacy scrubber

> Packet TP-01b of [#957](https://github.com/SandboxServers/Cimmeria/issues/957). Ledger: [../README.md](../README.md). Written 2026-10-03 by the implementing worker.

## Done

- `tools/token-profile/report/`: stdlib-only reports over `schema.sql` (version 1), run as `python tools/token-profile/report --db <db> --out <dir>`. Writes `token-report.md` and `token-report.json` (`cimmeria-token-report/1`).
- Separate layers: raw tokens (`sections/tokens.py`), estimated USD labelled as a plan-usage proxy (`sections/cost.py`), context pressure with static context, idle-gap cache writes and compactions (`sections/context.py`), tool results and exposure by tool, fingerprint and MCP server (`sections/tools.py`), cost per merged PR with the unattributed share and method mix (`sections/prs.py`), and the 5m-vs-1h cache-policy simulator with a calibration ratio (`sections/cache_sim.py`).
- Every distribution is n/p50/p75/p90/p95/p99/max/mean (`stats.py`). Every report carries a version stamp (`db.stamp`).
- Privacy scrubber (`scrub.py`) in three layers: field validation, redaction, and a gate over the rendered output that refuses to write. Tests prove each layer is needed: disabling any one of them fails `test_privacy.py`.
- `sections/prs.pr_records()` returns the database-backed fields of the `cimmeria-pr-stats/1` block for TP-10.
- Tests: `test_stats.py`, `test_scrub.py`, `test_reports.py`, `test_privacy.py`, 40 in all. `fixture_db.py` loads the Wave 0 fixture into `schema.sql` from its `expected.json` (not an ingest), with synthetic PRs, attribution and prices. CI's existing `token-profile contract tests` step discovers them; no workflow change was needed.
- `tools/token-profile/README.md`: a Reports section and a privacy section.

## Left

- Run against TP-01a's real database once it lands, and check the cache simulator's calibration ratio on real transcripts. On the synthetic fixture it is 0.5-1.6, because the fixture's cache numbers aren't internally consistent; it says nothing about real data yet.
- TP-05 reconciles the USD layer against OTel and `cost-state`. The reports read `cost_states` nowhere yet; a reconciliation section belongs to TP-05.

## Branch and commands

Branch `feat/token-profile-report`, worktree `.claude/worktrees/tp01b`.

```bash
python -m unittest discover -s tools/token-profile -p "test_*.py"
python tools/token-profile/report --db <profile.sqlite> --out <dir>
```

## Open questions

- **Cache-simulator floor.** On a cold request the replay still reads the smallest cache read the transcript actually got on its own cold requests (the prefix other sessions kept warm). That is a model, not a measurement; TP-06 should check it against the calibration column before acting on a replay.
- **Per-PR `context`.** The plan's block has `{peak, p50}` only, so the per-PR record follows it. The report's other distributions carry the full tail.
- **`web_search` and `web_fetch`** are counted but not priced: `price_tables` has no column for them. If they ever matter, TP-01a would add one with a schema version bump.
