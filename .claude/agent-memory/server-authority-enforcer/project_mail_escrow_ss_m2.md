---
name: project-mail-escrow-ss-m2
description: SS-M2 gate-mail escrow + SS-M3 take/COD/return review (2026-09-27) — cleared shapes and the residual gaps (COD from deleted sender is a stuck sink)
metadata:
  type: project
---

SS-M2 (worktree ss-m2, reviewed 2026-09-27) escrows mailed items into `sgw_gate_mail_item` inside the send tx.
Cleared: item keyed by item_id + owner + INV_MAIN allowlist + bound check, FOR UPDATE under
`take_inventory_locks(sender, [INV_MAIN])` before `sgw_player` rows (ascending); debit is a single
guarded `UPDATE ... WHERE naquadah >= cost`; split uses `stack_size > qty` + fresh seq id; COD cost to sender is postage only;
delete guard is one conditional DELETE (cash = 0 AND no escrow row).

SS-M3 (worktree ss-m3, reviewed 2026-09-27, mail/claim.rs is the shared lock order): take cash/item, pay COD, return CLEARED
for dupe/double-credit/overflow/owner-scoping/redirection/deadlock. Mail row locked `WHERE mail_id AND character_id` after
advisory locks; every write conditional + rows_affected; COD price zeroed on pay AND on return; payment mail sender_id NULL;
return destination is stored sender_id, `returned` flag stops loops; client ContainerId/SlotId only logged.

Residual after the final pass (2026-09-27): the sender-deleted COD is now cancelled on pay (buyer gets the item free,
coordinator-approved) and `cod_paid` refuses returning a paid COD. Still open: `archiveMailMessage` (read.rs archive) ORs
MAIL_ARCHIVE onto an unpaid COD with no check; archived mail cannot be returned, never expires (D-SS04) and cannot be
deleted, so a buyer can lock the seller's item away for good. Vendor buyback locks its inventory rows and `sgw_player`
before the (player, INV_MAIN) advisory key, the reverse of claim.rs, so same-player buyback vs take-cash/pay-COD can
deadlock (Postgres aborts one; no value loss). Escrow still cascades on recipient character delete.

**Why:** SS-M4 expiry sweep reuses `return_tx`; it must skip archived mail but the archive path must first refuse unpaid COD.
**How to apply:** on SS-M4 review, check the archive-COD gate landed and that the sweep takes the claim.rs lock order.
See [[project-mail-handlers-unimplemented]].
