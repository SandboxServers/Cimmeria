---
name: trainer-implementation-status
description: Ability trainer feature is substantially complete in crates/services; the todo!() stubs in crates/game are dead code never called by service code
metadata:
  type: project
---

As of 2026-05-27, the ability trainer feature is largely implemented:

**DONE:**
- `crates/services/src/cell/cell_methods/player/trainer_interaction.rs` — `try_open_trainer()` builds per-player ability list from `template_trainer_lists` + `trainer_abilities` + `archetype_ability_trees`, computes trainable flags, sends `onTrainerOpen`. 4 unit tests with byte-exact wire assertions.
- `crates/services/src/cell/cell_methods/player/vendor.rs:491-658` — `handle_train_ability()` cell-side 6-step validation (ability exists, player_id, not already known, in archetype tree, level, prereqs). Sends `CellToBaseMsg::TrainAbility`.
- `crates/services/src/base/world_entry/methods/progression/mod.rs:400-530` — base-side atomic DB UPDATE with double-debit guard (`NOT (abilities @> ARRAY[...])`), TP debit, `BaseToCellMsg::AbilityGranted`.
- `crates/services/src/cell/service/base_messages/mod.rs:363-381` — `AbilityGranted` mirrors onto entity, sends `onKnownAbilitiesUpdate` (method 101) for hotbar.

**REMAINING (small):**
1. Routing split: `dispatch.rs:145` calls old stub `send_trainer_open` (in `interactions/trainer.rs`) when `NpcInteractionType::Trainer` is matched, bypassing `try_open_trainer`. Must consolidate.
2. "Resend list on prereq unlock": after `AbilityGranted`, if the new ability was a prereq for another offered ability, re-send `onTrainerOpen`. Python parity gap.
3. Dead stub removal: `crates/game/src/interactions/trainer.rs` has `todo!()` at lines 25, 30 but is never called. Remove or tombstone.

**Why:** The triage update on issue #55 said "no other trainer code" — this was wrong. The real implementation exists in `crates/services`, not `crates/game`.
