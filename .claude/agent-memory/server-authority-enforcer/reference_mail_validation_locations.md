---
name: reference-mail-validation-locations
description: File/line anchors for mail validation entry points and gaps
metadata:
  type: reference
---

**Dispatcher** (decodes mail_id / container_id / slot_id from client bytes):
`crates/services/src/cell/cell_methods/mail.rs`. Indices 43-51 (REQUEST_MAIL_HEADERS=43 … PAY_COD_FOR_MAIL=51).

**Player-id resolution** (the canonical "refuse to fall back to 0" pattern for
mail routing): `crates/services/src/cell/mail.rs:20-28` —
`resolve_mail_player_id()` reads `space_mgr.get_entity(entity_id).player_id`
and returns `None` (with `warn!`) if unset.

**DB write paths** (currently only headers/body/archive/delete are wired):
`crates/services/src/base/world_entry/methods/mail/mod.rs`
- RequestHeaders: line 82-95 — `WHERE character_id = $1`, scoped.
- RequestBody: line 155-185 — `WHERE mail_id = $1 AND character_id = $2`, scoped.
- RequestBody read_time UPDATE: line 191-200 — **NOT scoped by character_id**
  (CAT-G-07). Mitigated upstream today by the SELECT.
- Delete: line 231 — `WHERE mail_id = $1 AND character_id = $2`, scoped.
- Archive: line 275-280 — `WHERE mail_id = $1 AND character_id = $2`, scoped.

**Wire serializers**: `crates/services/src/cell/mail.rs:143-209`
(`serialize_on_mail_header_info`, `serialize_on_mail_read`,
`serialize_on_mail_header_remove`). `onMailRead.ToText` is currently
filled with the **reader's** name, not the recipient's (CAT-G-08).

**Schema**: `db/sgw/Mail/Tables/sgw_gate_mail.sql` — columns: mail_id (PK),
character_id (recipient), sender_id, subject, message, cash (bigint),
sent_time, read_time, flags, item_id, sender_name. No FK constraints.

**Stub (game crate)**: `crates/game/src/social/mail.rs` — `MailMessage` struct
with `send()` and `collect_attachments()` as `todo!()`. Not wired to the
dispatcher; safe to ignore for review purposes today.
