---
name: project-bank-vault-bv09-review
description: BV-09 Team vault expansion from the treasury (GM .orgvaultexpand) review 2026-09-27 - no blockers; leader check under lock_org cleared; outcome - leader-leaves and recycled-eid guards added, other-members onBagInfo and stale bank_slots still open
metadata:
  type: project
---

BV-09 review (branch bank/bv09-team-expansion, commit a98833503 on acb43359c), 2026-09-27. Verdict CONDITIONAL, no blockers.

**Cleared shape:**
- Only trust inputs off the wire are `scope` and `from_slots` (console args); `player_id`/`account_id`/`entity_id` come from the cell's `space_mgr`, and the `.`-console is gated on server-side `access_level` in `chat/mod.rs`. The org is found from the caller's own membership, so a GM cannot name another Team.
- `expand.rs` reuses `lock_actor` (KEY SHARE -> lock_org FOR UPDATE -> rank read), then `rank == LEADER`, then one UPDATE keyed on `vault_slots = from`, `< 100`, `org_type = 1`, `x.price_naquadah = $price`, `cash >= price`. It takes no advisory or item locks and never writes the wallet, so it adds no lock edge beyond BV-07/BV-08.
- Backstops: `sgw_organizations` CHECK 40..100 %10 and cash >= 0; `cash_log` CHECK amount > 0 plus vault_expansion arithmetic (after = before + 10). A zero price is refused in Rust, and the log CHECK would abort it anyway.
- Sends go through `org_cash::sends::Actor` (addressed by active_player_id, D-BV33). The broadcast is `broadcast_to_org`.

**Gaps sent back, and their outcome (merged as #966, 6d7d43149):**
1. Demote or leader transfer parked behind `lock_org`: not reachable. The `org_member_before_update` trigger refuses any rank change to or from 8 except the member-delete promotion, so a leader cannot be demoted or replaced in place. The reachable race, the leader leaving while the purchase waits, is `not_a_member`, guarded by `refusals::a_leader_who_leaves_while_the_purchase_waits_buys_nothing`.
2. Recycled entity id: FIXED, `refusals::a_recycled_entity_id_receives_nothing`.
3. `price_missing` untested: on purpose. The seed is shared by every live-DB test; the cash log CHECK `amount > 0` is the backstop against a free step.
4. The Command-scope `not_in_org` line said "Team": FIXED, the line names the scope (asserted in `no_team_and_database_failures_are_refused`).
5. Other members get no onBagInfo for the new size: OPEN. Their open Team window keeps the old size until they reopen it (bank ledger known gap).
6. The buyer's onBagInfo `bank_slots` can be stale against a concurrent personal `.bankexpand`: OPEN, cosmetic.

**How to apply:** "leader only" does not bound GM power, because a GM can set leadership via `organization/handlers/gm.rs`. The audit trail (`expand` gm_override + cash_log row) is the control. See [[project-bank-vault-bv08-review]], [[project-bank-vault-bv05-review]], [[reference-org-lock-authority]].
