---
name: project-live-db-marker-convention
description: Since 2026-09-27 live-DB tests need `live_db` in their nextest name and run N-way parallel, each on its own per-slot database clone; guards in cimmeria-test-support enforce the marker and the URL resolver
metadata:
  type: project
---

Live-DB tier (`tools/test-live-db.sh`, nextest profile `ci-live-db`): tests whose name
contains `live_db` go in test group `live-db` (`max-threads = N`, N=8 at landing). The
script clones the template DB into `<db>_0..<db>_<N-1>`; `test_support::database_url()`
(crates/test-support/src/live_db_slot.rs) maps `NEXTEST_TEST_GROUP_SLOT` to its clone and
writes it back to `DATABASE_URL`. Non-DB tests run fully parallel. Branch
`ci/live-db-serialise-db-only`, 2026-09-27; ~1,330 tests renamed with a `live_db_` prefix.

Guards (crates/test-support/src/live_db_group/): `every_live_db_test_is_in_the_live_db_group`
(test reaching `require_db_or_skip!`, directly or via helper, must carry the marker; it
caught two new DB tests that arrived from main during the rebase),
`database_url_is_only_resolved_by_the_gate` (no direct `DATABASE_URL` read, no hard-coded
`postgres://` server URL, no `sqlx::test`), `live_db_each_slot_talks_to_its_own_clone`
(end to end, live).

**Why:** the old `filter = "all()"` + `threads-required` ran ~5,500 tests one at a time on
one DB (305 s locally); per-slot parallel runs ~55 s.

**How to apply:** when auditing a PR with a new live-DB test, the marker and URL guards
already fail CI in the no-DB job. A test that passes alone but fails in the tier likely
depended on rows another test left behind in the old single DB, or on server-wide state
(xids, `pg_stat_activity` without a `current_database()` filter). First real bug found by
parallelism: `resources.resource_versions.snapshot varchar(100)` overflowed when a dozen
transactions were open (fixed to `text`, guarded by
`live_db_a_resource_edit_succeeds_under_many_concurrent_transactions`). Docs citing a
renamed test by its old name still substring-match it. See
[[finding-livedb-self-skip-masks-revert-verify]].
