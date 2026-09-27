---
name: project-bank-vault-bv08-review
description: BV-08 org treasury deposit/withdraw review (2026-09-27) - cleared lock/guard shape, no blockers, two missing regression guards (demote-while-parked, recycled entity id)
metadata:
  type: project
---

BV-08 review (branch bank/bv08-org-cash, commit 5dfab75ba on 13643442d), 2026-09-27. Verdict CONDITIONAL, no blockers.

**Cleared shape (template for any wallet <-> org balance move):**
- `CashDir::from_wire` uses `unsigned_abs`, so i32::MIN is Withdraw(2^31), not an overflow; zero is refused on the cell (`forward::zero_cash`) and backstopped by the log's `amount > 0` CHECK.
- `org_cash/persist.rs::transfer_cash`: actor row `FOR KEY SHARE` -> `lock_org` -> `member_access_locked` -> guarded wallet UPDATE (RETURNING decides, not the first read) -> guarded treasury UPDATE -> log row, one tx. The KEY SHARE taken first is what stops trade/vendor from holding the actor row `FOR UPDATE` while this tx holds the org, so there is no ABBA with them. The later plain UPDATE only waits on other NO KEY UPDATE writers, and none of those hold an org lock.
- Disband reads `org_vault_is_empty_sql` (cash = 0) under its own org lock, so a deposit either lands first (then disband refuses) or sees `no_such_org`.
- The treasury balance goes to the client only on the member-gated `InsufficientOrgCash` resync (`seen.member`) and in the success broadcast. The WARN log carries it for non-members, which is operator-only.
- Sends are keyed by player_id through `send_to_current_player` (D-BV33).

**Gaps sent back:** no guard for a permission revoke or kick landing while the transfer waits behind `lock_org` (it would fail if membership were read before the lock); no D-BV33 recycled-entity guard; `no_such_org` vs `not_a_member` lines are an org-id existence oracle (nit).

**How to apply:** for BV-09 (vault expansion debit) and any Black Market escrow, reuse this order and check that the new path also takes the actor KEY SHARE before `lock_org`. See [[project-bank-vault-bv07-review]], [[advisory-lock-namespaces]], [[reference-org-lock-authority]].
