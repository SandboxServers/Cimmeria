---
name: client-error-codes-and-ammo-chatter
description: onCharacterCreateFailed codes are error_texts ids (small ids are CONDITION_FEEDBACK rows); the client prints "out of ammo" itself on any AmmoSlotN stat update at 0
metadata:
  type: project
---

Two client behaviours that look like server bugs in UAT (Class Start v6 CS-08, 2026-10-10):

- **`onCharacterCreateFailed` (0x83) code = an `error_texts` id.** The client shows that row's text. Ids 1-3 are `CONDITION_FEEDBACK_*` rows, so the old 1/2/3 codes printed "CONDITION_FEEDBACK_PositionCheckNotBelow" for a bad name. Use 10000-10003 (`ERROR_CharacterCreation*`) and 20001 (`ERROR_InvalidCharacterName`), as python did; constants live in `crates/base/src/base/character_create/fail_code.rs`. Any other "send a code the client renders" message likely follows the same rule.
- **"Your X is out of ammo." is client-generated.** `ChatWindow/ChatEvents.lua` `CHAT_onStatUpdated` prints `Chatter_OutOfAmmo` for every `onStatUpdate` of `AmmoSlotN` (stats 49-53) with current <= 0 and max > 0, naming bandolier slot N. Placing an empty gun (OD-CS13: every gun is acquired empty) prints it once by design. To attribute one, find the AmmoSlot stat push (SyncBandolierItems, grant insert, `set_slot_ammo`), not a refusal path; the INFO log has no stat sends, so use a packet tap.

**How to apply:** before calling either a server bug, check the code-to-text mapping or the stat push timeline. Related: [[cell-systems-index]].
