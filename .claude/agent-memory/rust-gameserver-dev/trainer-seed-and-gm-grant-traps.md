---
name: trainer-seed-and-gm-grant-traps
description: Adding a trainer list or a GM ability grant - trainer_abilities.sql is generated, trainer gates block UAT buys, pet template sub-ranges, and how a grant must persist
metadata:
  type: project
---

Found 2026-09-27 building the pets PT-07 UAT tooling (branch `pets/pt-07-uat-tooling`).

- **`trainer_abilities.sql` is generated** by `tools/ability_trees/generate_seed.py`: list 1 is rebuilt from the workbook, every other list's rows are carried over verbatim, comments are dropped. Append new lists at the end in the generator's exact row format and put the explanation in `trainer_ability_lists.sql` or `entity_templates.sql`.
- **A trainer cannot sell outside the archetype tree.** `evaluate_train` checks tree membership, level, prerequisites and branch spend; rows keyed to other archetypes WARN `trainer_offered_unbound` on every open. Capstones (2826 Summon Straegis = Goa'uld L50) are unreachable for UAT via a trainer: use the GM `.giveability`.
- **A GM ability grant must go through the base to persist:** append to `sgw_player.abilities` only (never `trained_abilities`, or a respec refunds it) with a `NOT (abilities @> ...)` guard, then mirror on the cell with a player_id check (`respec.rs` shape). No base session cache holds abilities.
- **Pet id block 350-369 is split:** 350-359 pet templates (class `pet`, never in spawnlist, guarded by `live_db_pet_summons.rs`), 360-369 placed pet-campaign NPCs (360 = debug-hub pet trainer, spawn 450).
- **Adding a hub NPC with a `DebugHub_` tag** changes the hub count pinned in `cell-methods` `debug_hub_dispatch_tests.rs`; the full live-DB tier catches it, a filtered run does not.
- **The `.`-console now refuses a non-GM line naming a registered command** (feedback, no broadcast). Do not log it via `playtest_friction::console_rejected`: its streak WARN is player-triggerable.

Related: [[debug-hub-npc-authoring-traps]], [[legacy-command-parity-scoping-judgment]].
