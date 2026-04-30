//! Space management for the CellService.
//!
//! Loads world definitions from `entities/spaces.xml` and creates startup
//! spaces from `entities/cell_spaces.xml`. Manages the lifecycle of space
//! instances and the cell entities within them.

use std::collections::{HashMap, HashSet};

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::navigation::NavMesh;
use cimmeria_entity::space::Space;

mod aoi;
mod entities;
mod lifecycle;
mod queries;
mod spatial;
mod spawn;
mod xml;

#[cfg(test)]
mod tests;

/// Grid cell size for spatial hashing (world units).
pub(crate) const GRID_CELL_SIZE: f32 = 50.0;

/// Flag indicating this region should be sent to the client for client-side
/// hit testing. Matches `Atrea.enums.REGION_FLAG_ClientHinted`.
pub const REGION_FLAG_CLIENT_HINTED: i32 = 1;

/// A registered generic region from the database.
///
/// Loaded from `resources.point_sets` (type='AreaSet') + `resources.point_set_points`.
/// Each region is assigned a server-side runtime ID (auto-incrementing from 1)
/// that the client sends back in `triggerClientHintedGenericRegion` calls.
///
/// Reference: `python/cell/GenericRegion.py`
#[derive(Debug, Clone)]
pub struct RegionData {
    pub runtime_id: u32,
    pub db_set_id: i32,
    /// Region tag from `point_sets.name` — used as the content engine event key
    /// (e.g., "Castle_Cellblock.Region2"). This IS the key the content engine
    /// matches on, NOT a constructed `{world}.Region{id}` string.
    pub tag: String,
    pub world_name: String,
    pub height: f32,
    pub radius: f32,
    pub flags: i32,
    /// Polygon vertices from `point_set_points`. After the cylinder→bbox workaround,
    /// all regions should have exactly 4 points.
    pub points: Vec<[f32; 3]>,
}

/// World definition parsed from `entities/spaces.xml`.
#[derive(Debug, Clone)]
pub struct WorldDef {
    pub world_name: String,
    pub instanced: bool,
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

/// A live space instance with its entity population.
pub struct SpaceInstance {
    pub space_id: u32,
    pub world_name: String,
    pub space: Space,
    pub entities: HashMap<u32, CellEntity>,
    /// Entity IDs that have a client controller (players).
    pub players: HashSet<u32>,
    /// Navigation mesh for this space (if loaded).
    pub navmesh: Option<NavMesh>,
}

/// Manages spaces and cell entities for one CellApp.
pub struct SpaceManager {
    /// This cell's ID (used in space ID scheme: `(cell_id << 16) | local_index`).
    pub(crate) cell_id: u16,
    /// World definitions keyed by WorldName (from spaces.xml).
    pub(crate) worlds: HashMap<String, WorldDef>,
    /// Active space instances keyed by space_id.
    pub(crate) spaces: HashMap<u32, SpaceInstance>,
    /// Non-instanced world name → space_id (one instance per world).
    pub(crate) world_spaces: HashMap<String, u32>,
    /// Entity ID → space_id lookup for quick entity → space resolution.
    pub(crate) entity_space: HashMap<u32, u32>,
    /// Next local index for space ID allocation.
    pub(crate) next_local_id: u32,
    /// Next NPC entity ID (starts at 100_000 to avoid player ID collision).
    pub(crate) next_npc_id: u32,
    /// Cached dialog_set_maps: dialog_set_map_id → (dialog_id, interaction_flags).
    /// Populated at startup from `resources.dialog_set_maps`.
    pub dialog_set_maps: HashMap<i32, super::spawner::DialogSetMapEntry>,
    /// Cached mission definitions: mission_id → (first step_id, objectives).
    /// Populated at startup from `resources.mission_steps` + `resources.mission_objectives`.
    pub mission_defs: HashMap<i32, super::spawner::MissionDefEntry>,
    /// Cached stargate destinations: stargate_id → (world_name, position, yaw).
    /// Populated at startup from `resources.stargates` + `resources.worlds`.
    pub stargates: HashMap<i32, super::spawner::StargateEntry>,
    /// Cached step objectives: step_id → objectives for that step.
    /// Populated at startup from `resources.mission_objectives`.
    /// Used by `advance_step` to load new objectives when advancing a mission step.
    pub step_objectives: HashMap<i32, Vec<super::spawner::MissionObjectiveDef>>,
    /// Registered generic regions keyed by runtime_id (auto-incrementing from 1).
    /// Loaded from `resources.point_sets` (type='AreaSet') at startup.
    pub regions: HashMap<u32, RegionData>,
    /// Next runtime region ID (auto-incrementing, starts at 1).
    /// Matches Python `GenericRegionManager.lastRegionId`.
    pub next_region_id: u32,
    /// Ability definitions: ability_id → AbilityDef.
    /// Loaded from `resources.abilities` at startup.
    pub ability_defs: HashMap<i32, cimmeria_entity::abilities::AbilityDef>,
    /// Effect definitions: effect_id → EffectDef.
    /// Loaded from `resources.effects` + `resources.effect_nvps` at startup.
    pub effect_defs: HashMap<i32, cimmeria_entity::abilities::EffectDef>,
    /// Event set sequence lookup: (event_set_id, event_id) → sequence_id.
    /// Used to resolve the correct KismetEventSetSeqID for onSequence calls.
    /// Loaded from `resources.event_sets_sequences` + `resources.sequences` at startup.
    pub sequence_map: HashMap<(i32, i32), i32>,
    /// Item → preferred container mapping from `resources.items.container_sets`.
    /// Loaded at startup so runtime item grants go into the correct inventory bag
    /// (e.g. mission items into INV_Mission, weapons into bandolier).
    pub item_containers: HashMap<i32, i32>,
    /// Weapon defs (clip_size + default_ammo_type) keyed by item_id.
    /// Loaded from `resources.items` at startup. Used by the content engine's
    /// GrantItem path to seed bandolier slots when a weapon is granted at
    /// runtime, so the client renders the correct empty magazine.
    pub item_defs: HashMap<i32, super::spawner::WeaponDef>,
    /// Loot tables: loot_table_id → entries.
    /// Loaded from `resources.loot` at startup for NPC death loot generation.
    pub loot_tables: HashMap<i32, Vec<super::spawner::LootTableEntry>>,
    /// Respawner definitions loaded from `resources.respawners`.
    /// Used to populate the Defeat Window and look up respawn positions.
    pub respawners: Vec<super::spawner::RespawnerDef>,
}

impl SpaceManager {
    /// Create a new SpaceManager with the given cell ID.
    pub fn new(cell_id: u16) -> Self {
        Self {
            cell_id,
            worlds: HashMap::new(),
            spaces: HashMap::new(),
            world_spaces: HashMap::new(),
            entity_space: HashMap::new(),
            next_local_id: 0,
            next_npc_id: 100_000,
            dialog_set_maps: HashMap::new(),
            mission_defs: HashMap::new(),
            stargates: HashMap::new(),
            step_objectives: HashMap::new(),
            regions: HashMap::new(),
            next_region_id: 1,
            ability_defs: HashMap::new(),
            effect_defs: HashMap::new(),
            sequence_map: HashMap::new(),
            item_containers: HashMap::new(),
            item_defs: HashMap::new(),
            loot_tables: HashMap::new(),
            respawners: Vec::new(),
        }
    }
}
