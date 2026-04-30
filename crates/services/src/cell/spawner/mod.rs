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
    EVENT_ABILITY_BEGIN, EVENT_ABILITY_END,
    load_ability_defs, load_effect_defs, load_event_set_sequences,
};
pub use dialogs::{DialogSetMapEntry, load_dialog_set_maps};
pub use loot::{
    LootTableEntry, WeaponDef,
    load_item_containers, load_item_defs, load_loot_tables,
};
pub use missions::{
    MissionDefEntry, MissionObjectiveDef,
    load_mission_defs, load_step_objectives,
};
pub use npcs::{
    SpawnRecord, class_id_for_class, load_spawns_from_db,
    spawn_instance_npcs_from_records, spawn_npcs_from_records,
};
pub use regions::{RegionLoadData, load_regions_from_db};
pub use respawners::{RespawnerDef, load_respawners};
pub use stargates::{StargateEntry, load_stargates};
