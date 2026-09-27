---
name: project-bank-vault-bv05-review
description: BV-05 vault expansion (Banker Expand dialog 60110) cleared shape; entity-keyed sends and ignored button_id were flagged and fixed on the branch
metadata:
  type: project
---

BV-05 (branch bank/bv05-expansion, reviewed 2026-09-27): verdict CONDITIONAL/SHIP-with-should-fixes.

Cleared shape (reuse as the template for any "buy a permanent upgrade" flow):
- Offer key is server-origin (base reads bank_slots -> cell VaultSession.expansion_offer -> base); client only supplies dialog_id/button_id.
- One UPDATE ... FROM price table, `WHERE bank_slots = from_slots AND bank_slots < ceiling AND naquadah >= price`, relative SET. READ COMMITTED EPQ re-check makes concurrent duplicates charge once; concurrent naquadah writers are safe because every naquadah writer in base-methods is relative (`naquadah = naquadah +/- $1`), none absolute.
- Grow-only bank_slots: persist_expansion is the only writer; all readers (bank_rules, vendor serializers, inventory resync) read DB fresh. Invariant documented in bank_expand/persist.rs module docs + bv-03 worknote.
- Three layers of one-shot: #479 offered-dialog take, expansion_offer.take(), DB size key.

Residuals flagged:
- bank_expand/sends.rs addresses by caller.entity_id via entity_to_addr -> the [[exploit-entity-id-recycling]] shape; feedback.rs has send_to_current_player(player_id) (PR #893) for exactly this.
- button_id ignored except -1; an evicted one-button dialog's discard reply is unverified in RE.

Resolution (same branch, commit "address expansion results to the character..."): both residuals FIXED. Sends now go through `send_to_current_player(player_id)` with the session found by `active_player_id` (guard `a_recycled_entity_id_receives_nothing`); only ButtonID 8 buys, `-1`/other ids log `expand_dismissed reason=closed|unexpected_button` (guard `only_the_expand_button_buys`). Nits adopted too: the offer carries the shown price and the UPDATE matches `x.price_naquadah = $offered` (`price_changed`), a row that changed between write and read is `row_changed` not `replay`. Nit 5 (drop the "vault is full" line at 100) was NOT kept: the packet and UAT step 11 require feedback at the ceiling. Dialog 60110 then went into `QUARANTINED_DIALOG_OVERRIDES` (#943 map-load crash), so the UI path is dormant until that lifts.

**How to apply:** future paid-upgrade packets should copy the size-keyed single UPDATE and send via send_to_current_player. See [[project-bank-vault-bv03-review]].
