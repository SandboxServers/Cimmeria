---
name: project-mail-handlers-unimplemented
description: Mail send/take-cash/take-item/pay-COD/return are stubbed; cell dispatcher returns true so future implementations inherit unvalidated wire surface
metadata:
  type: project
---

`crates/services/src/cell/cell_methods/mail.rs` has five stubbed arms:
`SEND_MAIL_MESSAGE` (44), `RETURN_MAIL_MESSAGE` (47), `TAKE_CASH_FROM_MAIL` (49),
`TAKE_ITEM_FROM_MAIL` (50), `PAY_COD_FOR_MAIL` (51). Each logs
`tracing::info!("UNIMPLEMENTED: …")` and returns `true` (claims to have handled
the message). The dispatcher silently consumes the bundle; the client receives
no `sendMailResult`/error path.

**Why:** Audit (CAT-G, 2026-05-31) confirmed only `requestMailHeaders`,
`requestMailBody`, `archiveMailMessage`, `deleteMailMessage` are wired to the DB.
The implemented quartet correctly scopes by `character_id = $player_id` via
`resolve_mail_player_id()` in [[reference-mail-validation-locations]].

**How to apply:** When reviewing any future PR that touches mail handlers,
re-derive the validation contract from scratch — there is no existing pattern
in the file to copy from for the mutating paths. Required validations for
each future arm are enumerated in `.scratch/audit/findings/CAT-G-mail.md` 
CAT-G-01 through CAT-G-06. Send-mail dupe scenario uses TOCTOU on
`sgw_gate_mail.cash` between take-cash and take-item — see CAT-G-04.

Wire shape per arm comes from `entities/defs/interfaces/SGWMailManager.def`
— see [[reference-mail-wire-spec]].
