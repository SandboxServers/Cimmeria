//! Space management for the CellService.
//!
//! Loads world definitions from `entities/spaces.xml` and creates startup
//! spaces from `entities/cell_spaces.xml`. Manages the lifecycle of space
//! instances and the cell entities within them.

use std::collections::{HashMap, HashSet};

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::movement_validation::MovementValidator;
use cimmeria_entity::navigation::NavMesh;
use cimmeria_entity::space::Space;

pub use entities::ClientMoveOutcome;

mod aoi;
mod entities;
mod lifecycle;
mod queries;
mod spatial;
mod spawn;
mod xml;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod aoi_churn_smoke;

#[cfg(test)]
mod aoi_differential_test;

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
    /// Set of dialog ids whose every screen has `speaker_id = 0`
    /// (player-monologue / inner-thought dialogs). Populated at startup
    /// from `resources.dialog_screens`. Used by the `DisplayDialog`
    /// executor: when no NPC entity can be resolved from chain context
    /// AND the dialog is in this set, bind the player as the wire
    /// `EntityId` of `onDialogDisplay` (correct render for monologue
    /// — speaker name falls back to player, portrait shows player's
    /// character). Dialogs NOT in this set continue to bail-and-warn
    /// when no NPC resolves, since binding the player there would
    /// blank an NPC portrait and substitute the player's name for
    /// every NPC line.
    pub monologue_dialog_ids: std::collections::HashSet<i32>,
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
    /// Per-item ability bindings: `(item_id, event_id) → ability_id`.
    /// Loaded from `resources.items_event_sets` at startup. Resolves the
    /// correct ability for a weapon's ranged/melee/use action — e.g., the
    /// pistol (item 55) fires ability 579 on `EVENT_ITEM_RANGED`, while
    /// the P90 (item 21) fires ability 559 for the same event. Previously
    /// the server hardcoded `592` (Pistol Shot) regardless of equipped
    /// weapon. Use [`crate::cell::abilities::ability_for_item`] for the
    /// lookup; it returns `Option<i32>` and callers decide their fallback
    /// (e.g., the right-click handler falls back to `592` so a fresh
    /// checkout without seeded bindings stays responsive).
    pub item_event_set_abilities: HashMap<(i32, i32), i32>,
    /// Per-archetype ability training tree: `archetype_id → Vec<entry>`.
    /// Loaded from `resources.archetype_ability_tree` at startup. Used by
    /// the `train_ability` cell method to validate that a requested
    /// ability is in the player's archetype tree, level requirement is
    /// met, and all prerequisite abilities are known. See
    /// Phase 5b.
    pub archetype_ability_trees: HashMap<i32, Vec<super::spawner::ArchetypeAbilityTreeEntry>>,
    /// Trainer NPC ability lists: `(list_id, archetype_id) → Vec<ability_id>`.
    /// Loaded from `resources.trainer_abilities` at startup. Used by the
    /// trainer NPC interaction flow (`onInteract` → `sendAbilityList`) to
    /// enumerate which abilities a specific trainer offers to a player
    /// of a given archetype.
    pub trainer_abilities: HashMap<(i32, i32), Vec<i32>>,
    /// Trainer template → ability-list mapping: `template_id → list_id`.
    /// Loaded from `resources.entity_templates` rows with non-NULL
    /// `trainer_ability_list_id`. Used to identify a trainer NPC at
    /// interaction time (fast HashMap miss for the 95%+ of templates that
    /// aren't trainers).
    pub template_trainer_lists: HashMap<i32, i32>,
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
    /// Ring transporter region definitions keyed by `region_id` (cross-world unique).
    /// Loaded once at startup from `resources.ring_transport_regions`.
    pub ring_regions: HashMap<i32, super::ring_transport::RingRegion>,
    /// Reverse index: `point_set_id` → `region_id` for O(1) ring trigger
    /// lookup. Populated alongside `ring_regions`.
    pub ring_point_set_to_region: HashMap<i32, i32>,
    /// Live ring transporter state machines keyed by `region_id`.
    /// Built from `ring_regions` at startup; one entry per ring pad.
    pub ring_transporters: super::ring_transport::RingTransporterManager,
    /// NPCs with a pending `ai_retry_at` deadline. Updated whenever
    /// `npc_ai_fight` schedules a launch-failure retry and whenever
    /// `npc_ai_retry_sweep` consumes one. The retry sweep iterates
    /// THIS set instead of `all_npc_entity_ids()`, so the per-AoI-tick
    /// (100ms) sweep cost is `O(pending)` instead of `O(total NPCs)`.
    /// Stale entries (NPC destroyed, state-transitioned away from
    /// Fighting, deadline not yet reached) are skipped by the sweep's
    /// double-check filter — the set is a "candidate" pointer set, not
    /// the source of truth.
    pub pending_ai_retries: std::collections::HashSet<u32>,
    /// Server-authoritative movement validator (issue #478). Consulted by
    /// `apply_client_position_update` on every inbound client position:
    /// bounds + navmesh + teleport hard-reject, speed warn-only. Holds a
    /// per-entity server-clock sample for the speed/teleport layer; that
    /// state is released in `destroy_entity` via `forget`.
    pub movement_validator: MovementValidator,
    /// Cover-system service handle. Loaded from `resources.cover_sets` +
    /// `resources.cover_nodes` at startup; carries the spatial index,
    /// reservation table, and per-set metadata. See
    /// `crates/services/src/cell/cover/` for details and
    /// `docs/architecture/cover-system.md` for the design.
    pub cover: super::cover::Cover,
    /// Per-player cover-detection state. Updated by the
    /// `cover_detection_tick` to track which cover sets each player is
    /// currently inside (drives `onEnterCoverSet` / `onLeaveCoverSet`
    /// fanout + content-engine `OnPlayerEnteredCover` triggers).
    pub cover_detection: super::cover::CoverDetectionTable,
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
            monologue_dialog_ids: std::collections::HashSet::new(),
            mission_defs: HashMap::new(),
            stargates: HashMap::new(),
            step_objectives: HashMap::new(),
            regions: HashMap::new(),
            next_region_id: 1,
            ability_defs: HashMap::new(),
            effect_defs: HashMap::new(),
            sequence_map: HashMap::new(),
            item_containers: HashMap::new(),
            item_event_set_abilities: HashMap::new(),
            archetype_ability_trees: HashMap::new(),
            trainer_abilities: HashMap::new(),
            template_trainer_lists: HashMap::new(),
            item_defs: HashMap::new(),
            loot_tables: HashMap::new(),
            respawners: Vec::new(),
            ring_regions: HashMap::new(),
            ring_point_set_to_region: HashMap::new(),
            ring_transporters: super::ring_transport::RingTransporterManager::new(),
            pending_ai_retries: std::collections::HashSet::new(),
            movement_validator: MovementValidator::new(),
            cover: super::cover::Cover::empty(),
            cover_detection: super::cover::CoverDetectionTable::new(),
        }
    }
}
