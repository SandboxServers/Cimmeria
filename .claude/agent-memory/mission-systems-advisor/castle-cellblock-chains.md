---
name: castle-cellblock-chains
description: Castle_CellBlock mission chain shapes (622/638/639/640/641) — interactability gating, re-loot guards, dialog-set→template binding, key item/step/dialog IDs.
metadata:
  type: project
---

# Castle_CellBlock content chains — confirmed facts

Seed: `db/resources/Content/Seed/castle_cellblock_chains.sql`.
Chain-id allocation: 622→1001-1010, 638→1011-1030, 639→1031-1040,
640→1041-1050, 641→1051-1070. (As of writing 1003/1004 used in the 622 range.)

## Interactability gating (how a 0-base-interaction NPC becomes clickable)

An NPC/corpse with `interaction_type = 0` is made right-clickable purely by
binding a dialog set to its `template_id` via
`add_dialog_set(dsm_id, {slot: <template_id>})`. The `add_dialog_set` handler
(`crates/services/src/cell/content/executor/dialog.rs:119`) stores
`(dsm_id, dialog_id, interaction_flags)` into the player's
`available_interactions[template_id]` and pushes an InteractionType update
merging the entity base flags with the dialog_set_map's `interaction_flags`.

The interact path (`cell/cell_methods/player/interaction.rs` INTERACT arm):
click → `fire_interact_tag(tag)` → if no tag chain → `handle_interact` returns
the bound dialog_id → `fire_dialog_open(dialog_id)`. So the dialog set is BOTH
what makes the body clickable AND what supplies the `dialog_open` event id.
**No `set_interaction_type` "set the bit first" is needed** — the dialog_set_map's
own `interaction_flags` (0x40000000 = INT_MissionWorldObject) does it.

## Mission 622 "Arm Yourself" — key IDs

- Frost corpse: tag `ArmYourself_FrostBody`, spawn 19, template 14, dialog 3995,
  dialog_set_map 5229 (dialog_set_id 803, flags 0x40000000, topic "Search Cpl.
  Frost's Corpse"). Dialog 3995 screen 96108, speaker_id 0 (monologue).
- Guard corpse: tag `ArmYourself_GuardBody`, spawn 15, template 21. Originally
  INERT (no chain/dialog).
- Items: pistol = 55, letter = 3730. Steps: 2113 = "search corpses" (active at
  start), 80622 = "equip the pistol" (PAK-override step, already shipped).
- Python ref: `deprecated/python/cell/missions/Castle_CellBlock/ArmYourself.py`.
  dialogCb on `dialog.open::3995` does: `setInteractionType(it & ~4194304)` on
  Frost body, grant 55, grant 3730, complete 622. `subscribe(..., once=False)`.
  Note 4194304 = 0x400000 is a DIFFERENT bit from the dialog-set 0x40000000;
  clearing it removes the body's "search" cursor so the client won't re-open
  the dialog (the python re-loot guard, since `once` was False).

## Re-loot / one-shot guard pattern (the canonical one)

Gate the chain on `step_status <mission> <step> = active`, and have the chain
`advance_step` past that step. The advance flips the gate false, so a second
event won't re-fire. This is the ONLY reliable guard — `once` is dead (see
[[content-engine-once-semantics]]). For a grant-only chain that does NOT advance,
either (a) make it advance a step, or (b) accept that the only guard is the
client-side dialog bit-clear (`set_interaction_type ~mask`), which stops the
`dialog_open` event from re-firing because the body stops being clickable.

## set_interaction_type op semantics

params `{"op": "|"|"~", "mask": N}`. `|` sets bits, `~` clears bits (AND NOT).
Mirrors python `entity.interactionType | N` / `& ~N`.
