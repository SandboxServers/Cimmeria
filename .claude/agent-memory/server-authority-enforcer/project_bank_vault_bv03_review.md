---
name: project-bank-vault-bv03-review
description: BV-03 personal vault deposit/withdraw review (2026-09-27) - cleared verdict/TOCTOU shape; its three findings were fixed in the same PR; the bank_slots grow-only invariant BV-05 must keep
metadata:
  type: project
---

BV-03 (branch bank/bv03-bank-moves) reviewed 2026-09-27: CONDITIONAL, no hard blocker.

Cleared shape: cell takes `vault_access()` per forwarded request (cell-world space_manager/vault_access.rs) and attaches
`VaultAccess` to Move/Use/Remove/RemoveByType; base checks target allowlist pre-tx, source allowlist after the FOR UPDATE
source read, bank_slots + mission rule inside the tx. Base cell-message loop is one serial consumer (base/service.rs), so
a stale verdict is bounded by queue latency; acceptable because the vault is own-items-only, no currency.

Findings, all fixed on the branch before the PR (commit "fix(bank): address the BV-03 server-authority review"):
- New move merge path (move_/apply.rs merge, finish.rs choose_shape) ignores `bound`/charges/durability; grant merge
  requires `bound = false`. D-BV08 says bound items ARE bankable, so a bound stack merged into an unbound one launders
  the bind (trade/mail/vendor all key on bound=false). FIXED: merge needs equal bound/durability/charges, else swap.
- `VaultAccess` carried no scope. FIXED: `Open { scope, .. }`; only Personal opens 17 (`vault_scope_mismatch`).
- Vault-move failures after the allowlist (item_allows_container, split-onto-occupied, missing player row, slot>=100)
  returned silently. FIXED for vault moves (item_not_allowed_in_container, split_onto_occupied_slot, slot>=100 refused
  in-tx as target_slot_beyond_bank_slots); the missing player row is still a plain error log.
- OPEN: bank_slots read unlocked, justified only by "only grows"; no writer exists yet. BV-05 must keep that true.
- Also adopted: content RemoveItem by type never searches 17 (NO_SESSION); by instance takes the live verdict.

Mission items in seed are exactly container_sets {2}; none list 17, so item_allows_container already blocks them and
D-BV08 is defense-in-depth.

**Why:** BV-04/BV-05 and the org vault waves build on this path.
**How to apply:** when reviewing BV-05 (expansion) or org vaults, check these gaps first. See [[advisory-lock-namespaces]],
[[exploit_buyback_moveitem_source]].
