---
name: project-mail-escrow-ss-m2
description: SS-M2 gate-mail attachment escrow review (2026-09-27) — what was cleared, and the residual gaps the SS-M3 take/return work must close
metadata:
  type: project
---

SS-M2 (worktree ss-m2, reviewed 2026-09-27) escrows mailed items into `sgw_gate_mail_item` inside the send tx.
Cleared: item keyed by item_id + owner + INV_MAIN allowlist + bound check, FOR UPDATE under
`take_inventory_locks(sender, [INV_MAIN])` before `sgw_player` rows (ascending); debit is a single
guarded `UPDATE ... WHERE naquadah >= cost`; split uses `stack_size > qty` + fresh seq id; COD cost to sender is postage only;
delete guard is one conditional DELETE (cash = 0 AND no escrow row).

Residual: take/return/payCOD are silent UNIMPLEMENTED stubs in cell-methods mail.rs, so SS-M2 alone makes attachments an
unrecoverable sink; escrow row cascades on recipient character delete (sender's item/COD lost).

**Why:** SS-M3 inherits these invariants; the take path must re-check escrow row under lock and credit with overflow check.
**How to apply:** when SS-M3 lands, verify take is one tx (DELETE escrow row RETURNING -> insert inventory), cash take zeroes
`cash` with `WHERE cash = $seen`, COD pay debits recipient + credits sender atomically. See [[project-mail-handlers-unimplemented]].
