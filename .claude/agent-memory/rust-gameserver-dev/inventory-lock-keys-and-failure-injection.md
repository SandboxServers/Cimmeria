---
name: inventory-lock-keys-and-failure-injection
description: Inventory writers do NOT share one lock key, so anything that reads-then-sends a row must row-lock it; and how to inject DB failures for LogCapture guards (unreachable pool, lock_timeout)
metadata:
  type: project
---

Inventory writers take different locks (verified 2026-09-27, BV-01 round 4, PR #872):

- `moveItem`: advisory `(player_id, 0)`, then `(player_id, container)` for target and source, then row `FOR UPDATE`.
- grants: advisory `(player_id, container)` only; `grant_item.rs` MERGES into an existing stack with `UPDATE ... stack_size = stack_size + $1` (so "grants never touch an existing row" is false).
- removes / vendor sale: row `FOR UPDATE` or a bare `UPDATE`/`DELETE`, no advisory lock.
- trade (since 2026-09-27) and crafting item use: `take_inventory_locks` (key 0, then 1, 15), rows, then player rows. Vendor buyback still takes `(p,1)` only, after its row locks.

**Why:** a read-then-send under only the `(player, 0)` move lock can be overtaken by a stack merge; the stale packet lands after the writer's own `onUpdateItem`. Every writer row-locks the row it changes, so `SELECT 1 ... WHERE character_id=$1 AND item_id=$2 FOR UPDATE` (after the advisory lock, same order as the move path) closes it for one-row resends. `FOR UPDATE` cannot sit on the nullable side of a `LEFT JOIN`, so lock with a separate statement.

**How to apply:** any new "resend what the server has" path (refusal snap-back, vault resync) holds the row lock across the send. For negative-log guards on DB-failure seams (TESTING.md type 12): `PgPoolOptions::new().acquire_timeout(50ms).connect_lazy(<URL of a port nothing listens on>)` makes begin/query fail with no database (see `unreachable_pool()` in `move_/refusal_infra_tests.rs` for the exact form); `(*pool.connect_options()).clone().options([("lock_timeout","200ms")])` plus a lock held on another connection makes a lock step fail deterministically. Since round 5 a refusal that cannot lock sends nothing (`move_resync_skipped reason=lock_timeout`) rather than an unlocked resend. `sqlx::query` needs `&'static str`, so a helper taking SQL must declare `&'static str`. Examples: `move_/refusal_infra_tests.rs`, `move_/refusal_resync_tests.rs`. Related: [[container-capacity-and-grant-targets]].
