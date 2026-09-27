---
name: project-bank-vault-bv05-review
description: BV-05 vault expansion (Banker Expand dialog 60110) cleared shape; entity-keyed feedback sends and ignored button_id are the residuals
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

**How to apply:** future paid-upgrade packets should copy the size-keyed single UPDATE and send via send_to_current_player. See [[project-bank-vault-bv03-review]].
