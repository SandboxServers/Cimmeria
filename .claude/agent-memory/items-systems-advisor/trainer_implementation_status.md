---
name: trainer-implementation-status
description: Ability trainer onTrainerOpen is a complete, current implementation in crates/services/src/cell/interactions/trainer.rs; the crates/game stub was already retired (not just dead — deleted), superseding the 2026-05-27 memory below
metadata:
  type: project
---

Verified 2026-09-17 (Harset campaign evidence pass): the file layout described in the older note below has moved/consolidated. Current state:

- `crates/services/src/cell/interactions/trainer.rs` (483 lines) is the live `onTrainerOpen` implementation (flat method index 113). Builds the per-player trainable ability list, mirrors Python `AbilityTrainer.canTrainAbility`, sends `onTrainerOpen`. This is the file to read for trainer work now, not `cell_methods/player/trainer_interaction.rs`.
- `crates/game/src/lib.rs` doc comment explicitly states the trainer stub (along with vendor/lootable/stargate stubs) was **retired as dead code after an audit confirmed zero callers** — this is not an open TODO, it's done. `crates/game/src/npc.rs::NpcState` still legitimately holds `trainer_list_id: Option<i32>` / `vendor_list_id: Option<i32>` as data-model fields (`is_trainer()` helper), which is fine — that's state, not a handler stub.
- Grepped `vendor_list_id`/`trainer_list_id` usage outside `crates/game`: **zero hits**. Nothing currently wires an NPC's `NpcState.trainer_list_id` to the real trainer flow or the real vendor flow — the live vendor path instead resolves lists via `resources.entity_templates.{buy,sell,repair,recharge}_item_list` keyed off `template_id` (see [[vendor_trainer_seed_gap]]), not through this struct field. Worth reconciling if `NpcState` is meant to be the source of truth eventually.

**Superseded content below (kept for history — do not trust file paths in it):**

As of 2026-05-27, the ability trainer feature is largely implemented:

**DONE (as of that date):**
- `crates/services/src/cell/cell_methods/player/trainer_interaction.rs` — `try_open_trainer()` builds per-player ability list from `template_trainer_lists` + `trainer_abilities` + `archetype_ability_trees`, computes trainable flags, sends `onTrainerOpen`. 4 unit tests with byte-exact wire assertions.
- `crates/services/src/cell/cell_methods/player/vendor.rs:491-658` — `handle_train_ability()` cell-side 6-step validation. Sends `CellToBaseMsg::TrainAbility`.
- `crates/services/src/base/world_entry/methods/progression/mod.rs:400-530` — base-side atomic DB UPDATE with double-debit guard, TP debit, `BaseToCellMsg::AbilityGranted`.
- `crates/services/src/cell/service/base_messages/mod.rs:363-381` — `AbilityGranted` mirrors onto entity, sends `onKnownAbilitiesUpdate` (method 101).

**REMAINING (small, per that date):**
1. Routing split: `dispatch.rs` calling old stub vs `try_open_trainer` — likely resolved since, given current `trainer.rs` is the sole file now.
2. "Resend list on prereq unlock" Python parity gap — status unverified this pass.
3. Dead stub removal — DONE per the `crates/game/src/lib.rs` doc comment found this pass.
