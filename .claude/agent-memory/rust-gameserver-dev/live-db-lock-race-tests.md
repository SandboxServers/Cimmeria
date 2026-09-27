---
name: live-db-lock-race-tests
description: How to write a live-DB concurrency test that cannot pass vacuously, and the cascade-trigger lock-order trap it caught in ORG-02
metadata:
  type: project
---

Two lessons from ORG-02 (2026-09-27, organization member-delete trigger):

1. **A lock-race test must wait until the second transaction is really blocked** before the holder commits, or it passes without exercising the race. Poll `pg_stat_activity` for `wait_event_type = 'Lock'` (tag the statement with a `/* probe */` comment, or filter `pid <> pg_backend_pid()` in the test's own database) up to ~5 s, assert it was seen, then commit the holder. The test pool has 4 connections: holder + spawned task + poller fits. `tokio::test`'s single-threaded runtime still interleaves because the poller awaits.

2. **A trigger fired by an FK cascade inverts the lock order.** `DELETE FROM sgw_player` locks the cascaded child row first; a trigger that then locks a parent (the org row) deadlocks (40P01) against any transaction holding the parent and wanting that child row. Fix: pre-lock the parents in id order before the delete (`organization::character_delete::delete_character`). Reproduced deterministically by `kick_during_character_delete_does_not_deadlock`.

**Why:** both were found only by a `database-persistence` review plus a revert proof; the happy-path tests were green.
**How to apply:** any new trigger or cascade that locks a parent row, and any concurrency test ([[vacuous-guard-and-sentinel-collision-review]] is the same "vacuous pass" family).

Also: sqlx 0.9 savepoint inside a caller's transaction is `sqlx::Acquire::begin(&mut **tx)`; roll it back on a typed refusal so the caller's transaction stays usable and unchanged.
