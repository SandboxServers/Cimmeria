//! NPC spawn system + DB-backed startup caches for the CellService.
//!
//! Loads spawn data and several lookup tables from the database
//! (`resources.spawnlist` joined with `resources.entity_templates` and
//! `resources.worlds`, plus mission/dialog/stargate/region/loot caches)
//! that `SpaceManager` uses to populate world spaces with NPC entities.
//!
//! Submodule layout:
//! - `missions` — mission definitions + step objectives.
//! - `dialogs` — dialog set map cache.
//! - `npcs` — the `spawnlist` loader and its `SpawnRecord` normalisers. The
//!   record type itself is wire contract (`cimmeria_wire::cell::spawn_record`),
//!   and populating spaces from it is `space_manager::spawn_npcs_from_records`.
//! - `respawners` — defeat-window respawn locations.
//! - `eye_heights` — per-body-set eye heights for line of sight (NA31).
//! - `stargates` — gate destination cache.
//! - `worlds` — world name → `world_id` map (the DB-only half of `spaces.xml`).
//! - `navmesh_mode` — the per-world `NavmeshMode` those rows carry.
//! - `regions` — generic region (AreaSet) loading.
//! - `abilities` — ability/effect defs + event-set sequence map.
//! - `loot` — loot tables + item container map + weapon defs.
//! - `templates` — prototype `SpawnRecord` per `entity_templates` row, for
//!   the content engine's `spawn_entity` action (no `spawnlist` row exists
//!   for a mission-scoped spawn).
//!
//! Reference: `python/base/SGWSpawnSet.py`, `python/cell/SGWMob.py`,
//!            `python/cell/SGWSpawnableEntity.py`

mod abilities;
mod dialogs;
mod eye_heights;
mod loot;
mod missions;
mod navmesh_mode;
mod npcs;
mod regions;
mod respawners;
mod stargates;
mod templates;
mod worlds;

#[cfg(test)]
mod tests;

// Public re-exports — keep `super::spawner::Foo` paths stable for sibling modules.
pub use abilities::{
    archetype_item_event_set, load_ability_defs, load_effect_defs, load_event_set_sequences,
    load_item_event_set_abilities, load_template_trainer_lists, load_trainer_abilities,
    EVENT_ABILITY_BEGIN, EVENT_ABILITY_END, EVENT_ITEM_EQUIP, EVENT_ITEM_MELEE, EVENT_ITEM_RANGED,
    EVENT_ITEM_RELOAD, EVENT_ITEM_UNEQUIP, EVENT_ITEM_USE, EVENT_ITEM_USE_ABILITY,
};
pub use dialogs::{
    load_dialog_screen_text, load_dialog_set_maps, load_monologue_dialog_ids, DialogSetMapEntry,
};
pub use eye_heights::load_body_set_eye_heights;
pub use loot::{load_item_containers, load_item_defs, load_loot_tables, LootTableEntry, WeaponDef};
pub use missions::{load_mission_defs, load_step_objectives, MissionDefEntry, MissionObjectiveDef};
pub use navmesh_mode::NavmeshMode;
pub use npcs::{class_id_for_class, load_spawns_from_db, SpawnRecord};
// Internal helper reused by the base-side GM spawn handler
// (`base::gm_spawn::load_spawn_record_for_template`). Crate-visible only — not
// part of the spawner's public surface.
pub(crate) use npcs::load_patrol_points;
pub use regions::{
    is_point_in_region, load_regions_from_db, RegionLoadData, GENERIC_REGION_CHECK_THRESHOLD,
};
// Exact XZ containment, re-exported for the playtest-friction watcher and
// the `.bug` bookmark, which reach it via `playtest_friction`.
pub(crate) use regions::region_contains_xz;
pub use respawners::{load_respawners, RespawnerDef};
pub use stargates::{load_stargates, StargateEntry};
pub use templates::load_spawn_templates;
// The `entity_templates` SELECT + row mapper, shared between the cell's
// startup template cache and the base-side GM spawn handler so a schema
// change can only be missed in one place (PR #662 review, finding 3).
// `cargo fmt` sorts these re-exports, so keep this comment glued to the
// line below rather than to the group.
pub(crate) use templates::{build_prototype, entity_template_select};
pub use worlds::{load_world_rows, WorldRow};
