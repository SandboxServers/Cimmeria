---
name: client-error-codes-and-ammo-chatter
description: onCharacterCreateFailed codes are served ErrorStrings.pak ids, not error_texts (small ids are CONDITION_FEEDBACK rows); the client prints "out of ammo" itself on any AmmoSlotN stat update at 0
metadata:
  type: project
---

Two client behaviours that look like server bugs in UAT (Class Start v6 CS-08, 2026-10-10):

- **`onCharacterCreateFailed` (0x83) code = a cooked `ErrorStrings` id (category 11).** The client shows the served `Text` of `data/cache/ErrorStrings.pak`'s entry, never the `error_texts` seed (the seed has rows the PAK lacks, e.g. Giza's 20001). Ids 1-3 are `CONDITION_FEEDBACK_*` entries, so the old 1/2/3 codes printed "CONDITION_FEEDBACK_PositionCheckNotBelow" for a bad name. Many shipped entries carry the bare or quoted moniker as text, so a code also needs served text: `crates/resources/src/base/attribute_patches/` patches `Text` (10000-10003, 42) and adds missing ids (`additions.rs`, 20001). Codes live in `crates/base/src/base/character_create/fail_code.rs`. Before sending any code the client renders, check the PAK has it with real text.
- **"Your X is out of ammo." is client-generated.** `ChatWindow/ChatEvents.lua` `CHAT_onStatUpdated` prints `Chatter_OutOfAmmo` for every `onStatUpdate` of `AmmoSlotN` (stats 49-53) with current <= 0 and max > 0, naming bandolier slot N. Placing an empty gun (OD-CS13: every gun is acquired empty) prints it once by design. To attribute one, find the AmmoSlot stat push (SyncBandolierItems, grant insert, `set_slot_ammo`), not a refusal path; the INFO log has no stat sends, so use a packet tap.

**How to apply:** before calling either a server bug, check the code-to-text mapping or the stat push timeline. Related: [[cell-systems-index]].
