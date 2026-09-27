---
name: forced-db-race-share-lock
description: How to make a type-5 live-DB race test deterministic without a code hook - hold LOCK TABLE ... IN SHARE MODE and release once pg_stat_activity shows N lock waiters
metadata:
  type: reference
---

A naive `tokio::join!` of two sends on a current-thread runtime does NOT reproduce a check-then-insert race: with the `FOR UPDATE` removed, `concurrent_sends_respect_mailbox_cap` still passed (SS-M1, 2026-09-27), so it was not a guard.

Deterministic recipe (`crates/base-methods/src/base/world_entry/methods/mail/tests/send_race.rs`):

1. Test opens its own transaction and runs `LOCK TABLE <insert_target> IN SHARE MODE`. SHARE lets SELECT/COUNT through and blocks every INSERT (ROW EXCLUSIVE).
2. Record the gate's `pg_backend_pid()`, then `tokio::join!(sender_a, sender_b, release)`, where `release` polls on the pool until the sessions held by the gate number >= 2, then commits the gate transaction. Count only those sessions, never every lock waiter in the database (an unrelated waiter would open the gate early): `WITH held AS (SELECT pid FROM pg_stat_activity WHERE $gate = ANY(pg_blocking_pids(pid))) SELECT COUNT(*) FROM pg_stat_activity a WHERE a.pid IN (SELECT pid FROM held) OR EXISTS (SELECT 1 FROM held h WHERE h.pid = ANY(pg_blocking_pids(a.pid)))`. The second arm is needed because, with the row lock, the second sender is blocked by the first, not by the gate.
3. Without the row lock both senders are parked at INSERT having counted N-1 (box ends at N+1, test fails). With it, the second is parked at `FOR UPDATE` behind the first (box ends at N).

**Why:** the test pool (`live_db_gate`) has 4 connections: gate + A + B + the poll query uses exactly 4. Poll on the pool, not inside the gate transaction (stats views snapshot per transaction).

**How to apply:** any "two writers, one slot" invariant (mailbox cap, escrow take-once, stack merge). Pick the lock mode that blocks the write but not the read the invariant depends on. Related: [[ai-state-private-and-revert-proof-mtime]] for running the revert proof itself.
