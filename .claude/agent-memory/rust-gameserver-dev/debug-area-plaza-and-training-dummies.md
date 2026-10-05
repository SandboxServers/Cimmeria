---
name: debug-area-plaza-and-training-dummies
description: DA-02 (2026-10-04) facts - chain tags fire in any world, TrainingDummy mark is the one never-fights-back switch, gm_ability_bulk action, SpawnRecord field adds touch ~20 literals, hub single-placement guards scoped to Castle_CellBlock, nav_inspect dy sign.
metadata:
  type: project
---

Found building the Debug Area services plaza and dummies (DA-02, world 1300,
`docs/content/debug-area.md`).

- **`content_chains.scope_id` is a label.** `interact_tag` chains fire for that
  tag in any world, so a copy of a hub NPC in another world needs its own tag
  and chain (the plaza uses `DebugArea_*`, chains 13000-13008).
- **`TrainingDummy` (cell-world `space_manager/training_dummy.rs`) is the one
  "never fights back" mark.** `.dummy` places it beside `LabDummy`; a template
  with `entity_templates.training_dummy = true` gets it at spawn (with 1M
  Health, half for a non-hostile one). `ai_driven_npc_entity_ids` skips it. A
  test fixture that inserts only `LabDummy` no longer skips the AI.
- **A dummy has no leash**, so nothing drained its attackers' combat sets; the
  1 Hz `training_dummy_combat_tick` (cell-combat) releases one whose threat
  total stood still 10 s.
- **Heals on NPCs fall back to the caster** unless `support_shot::classify`
  says Ally; a non-attackable TrainingDummy is now Ally.
- **GM from an NPC:** content action `gm_ability_bulk {change: grant_all|reset}`
  reuses `SpaceManager::plan_tree_grant` + `reset_all_cooldowns` and sends the
  same `GmAbilityBulk` the native 153/154 send; GM gate is
  `cimmeria_cell_world::cell::dispatch::is_gm(access_level)` inside the action.
- **Adding a `SpawnRecord` field** breaks ~20 struct literals in tests across
  cell crates; the scratchpad script pattern "insert after each `vault_scope`
  line" did it in one pass. Both loaders (`npcs.rs`, `templates.rs`) select it.
- **Reusing a hub template elsewhere** trips the hub's "placed exactly once"
  guards (auctioneer, banker, org bankers, registrars, pet trainer) and
  `seeded_spawns_load_vault_scope`; they now filter `world_name ==
  "Castle_CellBlock"`.
- **`nav_inspect` prints `dy = probe_y - floor_y`.** Floor = probe minus dy.
- Shipped monikers for a dev NPC: 21666 'Train Testing Abilities', 22555 'Jay
  Test Abilities' (the original devs' test trainers).

Related: [[debug-hub-npc-authoring-traps]], [[trainer-seed-and-gm-grant-traps]].
