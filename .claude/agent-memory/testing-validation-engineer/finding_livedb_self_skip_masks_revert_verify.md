---
name: finding-livedb-self-skip-masks-revert-verify
description: require_db_or_skip! self-skips on connect failure (pool timeout / DB down) and the test still reports "ok" — a live-DB revert-verify can silently run against a skipped test
metadata:
  type: finding
---

`require_db_or_skip!` (crates/services/src/test_support.rs) builds a per-test
`PgPoolOptions::new().max_connections(4).acquire_timeout(5s)`. On ANY connect
failure — DB down, port 5433 not listening, or server-side `max_connections`
saturation from rapid successive test runs — the macro **self-skips and the
test still prints `... ok`** with the line:
`skipping live-DB test (DATABASE_URL set but connect failed: pool timed out ...)`.

**Why this matters for revert-verification:** the [[workflow_revert_audit]] /
[[feedback_revert_to_verify_regression_guards]] step (revert the fix → rerun →
confirm it fails) is INVISIBLE when the DB is down. The reverted test "passes"
because it skipped, not because the guard is weak. A green revert-verify on a
live-DB test is only meaningful if you first confirm the test actually
**connected** (look for real query work / 10s+ duration, or grep stderr for the
`skipping live-DB test` substring and assert it's absent).

**How to apply:**
- Before trusting a live-DB run, `grep -i skip` the output. If you see the
  skip line, the result is meaningless — the DB isn't reachable.
- Probe connectivity cheaply first (run one known DB test like
  `base::crafting::persistence::tests::load_missing_player_returns_default`);
  if it skips, the bundled Postgres on :5433 is down (`Get-NetTCPConnection
  -State Listen` on 5433 returns nothing; no postgres process).
- The bundled :5433 Postgres can go down mid-session and rapid test cycling can
  exhaust server `max_connections` transiently. Per repo convention, don't
  start/restart it yourself — note the gap in the report and rely on a prior
  run that demonstrably connected.

**G12-class gotcha (NULL-in-NOT-NULL column tests):** to reproduce a
`try_get`+`?` graceful-drop on a column the handler decodes as non-Option
(e.g. `entity_templates.template_name`), the column is schema-NOT-NULL, so you
must `ALTER COLUMN ... DROP NOT NULL` → INSERT the NULL sentinel → run → DELETE
sentinel → `ALTER COLUMN ... SET NOT NULL` restore. Live-DB tests run
serialized (`--test-threads=1`) so the transient constraint relaxation can't
race. Always run teardown before the final assert so a failing assert still
leaves the schema clean. Note `BaseToCellMsg` is NOT `Debug` — assert on
`cell_rx.try_recv().is_ok()` (a bool), never `{reply:?}`.
