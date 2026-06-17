//! NPC spawn system + DB-backed startup caches for the CellService.
//!
//! Loads spawn data and several lookup tables from the database
//! (`resources.spawnlist` joined with `resources.entity_templates` and
//! `resources.worlds`, plus mission/dialog/stargate/region/loot caches)
//! and populates world spaces with NPC entities.
//!
//! Submodule layout:
//! - `missions` — mission definitions + step objectives.
//! - `dialogs` — dialog set map cache.
//! - `npcs` — `SpawnRecord`, class-id mapping, DB-driven NPC population.
//! - `respawners` — defeat-window respawn locations.
//! - `stargates` — gate destination cache.
//! - `regions` — generic region (AreaSet) loading.
//! - `abilities` — ability/effect defs + event-set sequence map.
//! - `loot` — loot tables + item container map + weapon defs.
//!
//! Reference: `python/base/SGWSpawnSet.py`, `python/cell/SGWMob.py`,
//!            `python/cell/SGWSpawnableEntity.py`

mod abilities;
mod dialogs;
mod loot;
mod missions;
mod npcs;
mod regions;
mod respawners;
mod stargates;

#[cfg(test)]
mod tests;

// Public re-exports — keep `super::spawner::Foo` paths stable for sibling modules.
pub use abilities::{
    archetype_item_event_set, load_ability_defs, load_archetype_ability_trees, load_effect_defs,
    load_event_set_sequences, load_item_event_set_abilities, load_template_trainer_lists,
    load_trainer_abilities, ArchetypeAbilityTreeEntry, EVENT_ABILITY_BEGIN, EVENT_ABILITY_END,
    EVENT_ITEM_EQUIP, EVENT_ITEM_MELEE, EVENT_ITEM_RANGED, EVENT_ITEM_RELOAD, EVENT_ITEM_UNEQUIP,
    EVENT_ITEM_USE, EVENT_ITEM_USE_ABILITY,
};
pub use dialogs::{load_dialog_set_maps, load_monologue_dialog_ids, DialogSetMapEntry};
pub use loot::{load_item_containers, load_item_defs, load_loot_tables, LootTableEntry, WeaponDef};
pub use missions::{load_mission_defs, load_step_objectives, MissionDefEntry, MissionObjectiveDef};
pub use npcs::{
    class_id_for_class, load_spawns_from_db, spawn_instance_npcs_from_records,
    spawn_npcs_from_records, SpawnRecord,
};
// Internal helper reused by the base-side GM spawn handler
// (`base::gm_spawn::load_spawn_record_for_template`). Crate-visible only — not
// part of the spawner's public surface.
pub(crate) use npcs::load_patrol_points;
pub use regions::{load_regions_from_db, RegionLoadData};
pub use respawners::{load_respawners, RespawnerDef};
pub use stargates::{load_stargates, StargateEntry};
