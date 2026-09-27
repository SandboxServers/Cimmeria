---
name: live-db-lock-race-tests
description: How to write a live-DB concurrency test that cannot pass vacuously, the cascade-trigger lock-order traps it caught in ORG-02, and the SHARE-lock trick for log-before-stamp
metadata:
  type: project
---

Two lessons from ORG-02 (2026-09-27, organization member-delete trigger):

1. **A lock-race test must wait until the second transaction is really blocked** before the holder commits, or it passes without exercising the race. Poll `pg_stat_activity` for `wait_event_type = 'Lock'` (tag the statement with a `/* probe */` comment, or filter `pid <> pg_backend_pid()` in the test's own database) up to ~5 s, assert it was seen, then commit the holder. The test pool has 4 connections: holder + spawned task + poller fits. `tokio::test`'s single-threaded runtime still interleaves because the poller awaits.

2. **A trigger fired by an FK cascade inverts the lock order.** `DELETE FROM sgw_player` locks the cascaded child row first; a trigger that then locks a parent (the org row) deadlocks (40P01) against any transaction holding the parent and wanting that child row. A Rust pre-lock only covers the Rust path; the fix that holds for psql and cascades too is a BEFORE DELETE row trigger on the deleted row (`org_player_before_delete`), which runs after Postgres locks that row and before its cascade.

3. **Per-row BEFORE triggers break id order across rows.** One statement that deletes several rows (an `account` delete cascading to its characters) runs each row's BEFORE trigger in turn, so character A's orgs are locked before character B's row and orgs: descending `org_id` order is possible, and it deadlocked against a single-character delete (reproduced by `account_delete_locks_all_its_characters_orgs_in_order`, round 3 of PR #881). Lock at the grandparent: a BEFORE DELETE trigger on `account` locks all child rows, then all their orgs, in id order.

4. **To prove "log before the durable write", hold `LOCK TABLE t IN SHARE MODE`.** It admits `SELECT ... FOR UPDATE` (ROW SHARE) and blocks `UPDATE` (ROW EXCLUSIVE), so an exporter can be frozen between its read and its stamp and the test can assert the log line already exists (`export_logs_before_the_stamp_can_commit`). Poll the export future and `wait_until_blocked` in one `tokio::select!`, so `LogCapture` stays on the test's thread.

**Why:** both were found only by a `database-persistence` review plus a revert proof; the happy-path tests were green.
**How to apply:** any new trigger or cascade that locks a parent row, and any concurrency test ([[vacuous-guard-and-sentinel-collision-review]] is the same "vacuous pass" family).

Also: sqlx 0.9 savepoint inside a caller's transaction is `sqlx::Acquire::begin(&mut **tx)`; roll it back on a typed refusal so the caller's transaction stays usable and unchanged.
