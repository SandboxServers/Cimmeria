---
name: org-cash-balance-and-overflow-traps
description: Wallet writes under a KEY SHARE lock see other plain writers, so take the balance from RETURNING; an int4 column plus an int8 bind overflows as SQLSTATE 22003 (query_failed), not a clean refusal, unless the WHERE guards it
metadata:
  type: project
---

Learned on BV-08 (org treasury transfers, `crates/base-session/src/base/org_cash/persist.rs`, 2026-09-27).

- **`FOR KEY SHARE` on `sgw_player` does not freeze `naquadah`.** It blocks `FOR UPDATE` (vendor, trade, character delete) but not a plain `UPDATE` (mail claim, rewards, org creation's debit), which takes `NO KEY UPDATE`. So a balance read at lock time can be stale by the write. Decide sufficiency with a guarded `UPDATE ... WHERE naquadah >= $2 RETURNING naquadah`, derive "before" as `returned +/- amount`, and re-read only to tell the player what they have.
- **`sgw_player.naquadah` is `integer` with no CHECK.** `SET naquadah = naquadah + $2` with an `i64` bind computes int8 and casts back on assignment: past `i32::MAX` Postgres raises 22003 "integer out of range", which aborts the transaction and surfaces as `query_failed`, not a refusal. Guard in the WHERE (`naquadah::bigint + $2 <= 2147483647`) so it is a zero-row result. Same for a bigint treasury: `cash <= 9223372036854775807 - $2`.
- **Revert proofs for "exactly one of two withdrawals" need a stale-read revert.** With `lock_org` taken by `member_access_locked` anyway, removing a guard only changes the loser's reason (the `CHECK (cash >= 0)` still rolls it back). The revert that shows money creation is an absolute `SET cash = $stale - $2` from a pre-lock read. Assert the loser's `reason`, not only the balances.

**How to apply:** any new cash writer on the base (BV-09's treasury debit, a future GM grant). Related: [[forced-db-race-share-lock]], [[training-points-cache-absolute-write]].
