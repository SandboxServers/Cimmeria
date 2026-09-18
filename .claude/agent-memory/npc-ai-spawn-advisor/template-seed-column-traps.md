---
name: template-seed-column-traps
description: entity_templates seed traps — ability_sets only has ids 1/2/3 (FK), static_mesh NULL on a prop = invisible, body_set NOT NULL, class 'being'/'spawnable' skips the AI tick, NPCs have infinite ammo.
metadata:
  type: project
---

# `entity_templates` authoring traps (measured 2026-09-17)

## Only ability sets 1, 2, 3 exist

`db/resources/Abilities/Seed/ability_sets.sql` has exactly three rows:
1 = NID guard (pistol) → ability 579; 2 = Prisoner retrieval unit → 221;
3 = NID guard (SMG) → 559. Each set holds **one** ability.

`entity_templates_ability_set_id_fkey` → `ability_sets(ability_set_id)`.
Referencing a set 4/5/6 without seeding `ability_sets` **and**
`ability_set_abilities` first fails the FK. A "staff" set is new content,
not an existing id.

Empty ability bucket → `NPC_DEFAULT_ABILITY = 592` (Pistol Shot) at
`space_manager/spawn.rs:175-182`.

## Appearance: static_mesh is mandatory for props

`mercury/aoi/create.rs::append_appearance` picks exactly one branch:
1. `body_set` non-empty **AND** `components` non-empty → `BeingAppearance`.
2. else `static_mesh` non-empty → `onStaticMeshNameUpdate(mesh, body_set)`.
3. else → `warn!(target: "aoi.cascade_appearance_missing")` and the entity
   is **invisible to every witness** (and the idempotent AoI tick will not
   re-introduce it).

A prop with `body_set = 'GLB_Components.WorldObject_Small'`, NULL
`components` and NULL `static_mesh` lands in branch 3. Props need a mesh.

## Column nullability the Rust loader depends on

`SpawnRecord` (`cell/spawner/npcs.rs:14-63`) types:
- `body_set: String` — table is `NOT NULL`, keep it that way.
- `static_interaction_sets: Vec<i32>` — table has `DEFAULT ARRAY[]::integer[]
  NOT NULL`; omitting the column from the INSERT is safe, writing NULL is not.
- `flags`, `interaction_type`, `has_dynamic_properties` — `NOT NULL` with
  defaults.
- `static_mesh`, `level`, `alignment`, `faction`, `name_id`, `speaker_id`,
  `loot_table_id`, `event_set_id`, `components` — all `Option`, NULL-safe.
- `name_id` FK → `texts(moniker_id)`. `speaker_id` has **no** FK.

## class → class_id → who gets ticked

`class_id_for_class` (`spawner/npcs.rs:108`): `spawnable`→0x00,
`being`→0x01, `mob`→0x04, unknown→0x04.
`all_npc_entity_ids()` (`space_manager/queries.rs:207`) admits **only
class_id == 0x04**. So `being` / `spawnable` props never AI-tick and never
respawn — `level`/`faction` on them is inert apart from `onLevelUpdate`
(class != 0x00 only).

## NPCs have infinite ammo

`use_ability/handle.rs:366` — the ammo check is players-only, and
`auto_reload.rs:15` confirms NPCs never consume. The python three-bucket
`needs_ammo` arm has no Rust equivalent, so an ammo-costing ability on a mob
template is safe today.

## respawn precedence

`COALESCE(s.respawn_secs, t.respawn_secs)` in the spawnlist loader
(`spawner/npcs.rs:149`), then `normalize_respawn_secs` drops <= 0 to `None`.
DB CHECK floors both columns at 3. GM `.spawn` (`base/gm_spawn.rs:252`)
hardcodes `respawn_secs: None` — GM-placed NPCs are always one-shot
regardless of the template value.

Related: [[faction-10-gates-everything]], [[level-is-hp-and-xp]],
[[harset-zone-evidence]]
