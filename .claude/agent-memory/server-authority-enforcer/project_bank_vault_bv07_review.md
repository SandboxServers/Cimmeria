---
name: project-bank-vault-bv07-review
description: BV-07 Team/Command org vault review (2026-09-27) - cleared authz/TOCTOU shape, cross-org snap-back leak, KEY SHARE-before-advisory deadlock with vendor/trade
metadata:
  type: project
---

BV-07 review (branch bank/bv07b-org-vault-moves, commits 082c3a2dc + 3d2d8afb4 on dfb0b0f8d), 2026-09-27.

**Cleared shape (reuse as the template for org-scoped item stores):**
- `move_/org/mod.rs::route` reads `sgw_organization_vault_items` by item_id with no lock. That is safe because every later read happens again under locks: `read_source` filters the vault by the SESSION org_id, and a Carried/Carried result is refused.
- Membership + rank bits are re-read under `lock_org` on every move (`org_vault/access.rs::lock_actor`); the cell session's org_id only names the vault.
- A swap across the vault boundary needs both bits: a deposit-swap needs WithdrawBank, a withdraw-swap needs DepositBank and the occupant passes `entering_vault`.
- DB backstops: CHECK NOT bound, a composite FK that pins org_type to its container, UNIQUE slot DEFERRABLE, ON DELETE RESTRICT (D-BV18).

**Findings sent back:**
1. Cross-org disclosure. When the step-1 verdict fails, `refuse(r, item_org, None)` runs with the ROUTED org, not the session org. `refusal.rs::lock_and_find` then locks that org and sends its vault row to the client. A forged item_id with no session reads any org's vault row (item ids are sequential) and takes `lock_org` on any org. Fix: refuse with the session org, or None.
2. Deadlock. The org move holds KEY SHARE on the mover's sgw_player row while it waits for (P,0) or (P,container). Vendor purchase holds (P,0) and trade holds (P,1) for both players, and each then wants `sgw_player FOR UPDATE`, which conflicts with KEY SHARE. That is an ABBA cycle; the deadlock detector aborts one side. The api.rs lock-order doc claims no cycle. Fix: take the per-player advisory locks before the KEY SHARE / `lock_org`.
3. The infra `Ok(None)` paths (lock or apply failures, including the deadlock abort) roll back with no line and no snap-back.

**Resolution (worker, same day):** all three fixed in the BV-07b review-fix commit. (1) The verdict refusal passes the session's org (or none), and `lock_and_find` resends a vault row only for an org the player is a member of; guard: `the_verdict_must_open_this_orgs_vault` (NO_SESSION and wrong-scope drags of another org's item). (2) `move_/org/rows.rs::move_locks` takes (P,0) and the carried container's lock before `lock_actor`, and re-checks the carried row's container once locked; the api.rs doc was corrected. (3) Every infra abort is `org_move_rejected reason=move_failed` with a line and the snap-back. The S3 test gaps are closed in `org_vault/tests/move_bits.rs`.

**How to apply:** for any future shared store (BV-09 growth, org cash, Black Market escrow), check that a refusal's resync reads only an org the session authorizes. Also check that no row lock is held while waiting on a per-player advisory lock. See [[advisory-lock-namespaces]], [[project-bank-vault-bv03-review]], [[reference-org-lock-authority]].
