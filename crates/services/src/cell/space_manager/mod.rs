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

pub use client_move::ClientMoveOutcome;
pub(crate) use crossing_hold_state::PendingCrossing;
pub(crate) use deferred_content_actions::PendingContentAction;
pub use entities::DespawnOutcome;
pub(crate) use gate_dial_state::PendingGateDial;
// The mode is plain data owned by the spawner's world loader; the
// containment predicate over it (`navmesh_containment`) stays here.
pub use super::spawner::NavmeshMode;
pub use queries::PlayerNameLookup;

mod aoi;
mod client_move;
mod cover_hit;
pub use cover_hit::{CoverStanding, PLAYER_COVER_MAX_DY};
mod cover_sight;
pub use cover_sight::{NpcSight, SightOrigin};
mod crossing_hold_state;
mod deferred_content_actions;
mod entities;
mod gate_dial_state;
mod lab_snapshots;
mod lifecycle;
mod movement_telemetry;
mod navmesh_containment;
mod npc_population;
pub use npc_population::{spawn_instance_npcs_from_records, spawn_npcs_from_records};
#[cfg(test)]
pub(crate) mod occluder_fixtures;
mod occlusion;
pub use occlusion::{eye_height_for, occluder_probe, DEFAULT_EYE_HEIGHT, RESIDENCY_RADIUS};
mod queries;
mod spatial;
pub use spatial::AttackLosPolicy;
mod spawn;
#[cfg(test)]
pub(crate) use spawn::resolve_use_cover;
mod xml;

pub(crate) use movement_telemetry::{
    HardReject, LogThrottle, MovementTelemetry, RecoveryReport, RejectReport, SuppressionReport,
};

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

/// Flag marking a region as the walk-through volume in front of a
/// stargate. Matches `Atrea.enums.REGION_FLAG_Stargate`
/// (`entities/defs/enumerations.xml:1653`). Entering one dispatches
/// `stargatePassed` rather than a generic region event — see
/// `GenericRegion.py:174-176` and `cell::gate_travel`.
pub const REGION_FLAG_STARGATE: i32 = 2;

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
    /// Numeric `resources.worlds.world_id` — the id space `spawnlist`,
    /// `stargates` and `ring_transport_regions` reference, and the one a
    /// content-engine `world` condition row is authored against.
    ///
    /// `None` until [`SpaceManager::stamp_world_ids`] runs at startup, and
    /// permanently `None` for a world that exists in `spaces.xml` but has
    /// no `resources.worlds` row. `spaces.xml` itself carries names only,
    /// which is why this cannot be filled at parse time.
    pub world_id: Option<i32>,
    /// Whether this world's navmesh may gate player movement, from
    /// `resources.worlds.navmesh_mode`.
    ///
    /// [`NavmeshMode::Enforce`] until [`SpaceManager::stamp_world_rows`]
    /// runs, and permanently so for a world with no `resources.worlds` row
    /// — a DB-down startup must not silently drop a containment gate. Read
    /// through [`SpaceManager::enforces_navmesh_containment`], never
    /// directly, so every gate agrees about what the mode means.
    pub navmesh_mode: NavmeshMode,
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
    /// Collision-geometry occluder (NA27), shared by every instance of the
    /// world. When present it is the line-of-sight source instead of the
    /// navmesh ray; see `space_manager::occlusion`.
    pub occluder: Option<std::sync::Arc<cimmeria_occluder::PagedOccluder>>,
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
    /// Line text for every dialog screen: `screen_id → text`. Populated
    /// at startup from `resources.dialog_screens`. Read only by the
    /// `npc_bark` content action, which speaks an original 2009 line
    /// into the triggering player's chat window rather than opening a
    /// dialog window (the client has no non-modal dialog path). Keeping
    /// the text here is what lets a bark seed row name a `screen_id`
    /// instead of retyping the line and drifting from the catalogue.
    pub dialog_screen_text: HashMap<i32, String>,
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
    /// Every archetype's ability tree, loaded from
    /// `resources.archetype_ability_tree` at startup. Read by
    /// `ability_tree::evaluate_train` for both the `trainAbility` purchase
    /// gate and the trainer window's `trainable` byte.
    pub ability_tree_catalog: crate::ability_tree::AbilityTreeCatalog,
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
    /// Prototype `SpawnRecord` per `resources.entity_templates` row, keyed
    /// by `template_id`. Loaded at startup by
    /// [`super::spawner::load_spawn_templates`].
    ///
    /// Separate from the `spawn_records` the cell loop threads around:
    /// those are `spawnlist` rows (a template *placed* somewhere), and a
    /// mission-scoped `spawn_entity` deliberately has no `spawnlist` row.
    /// This cache is what lets the content executor spawn a template
    /// synchronously instead of round-tripping through the base — see
    /// `cell/spawner/templates.rs` for why the round-trip is wrong for a
    /// chain's ordered action list.
    pub spawn_templates: HashMap<i32, super::spawner::SpawnRecord>,
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
    /// Server-authoritative movement validator. Consulted by
    /// `apply_client_position_update` on every inbound client position:
    /// bounds + navmesh + teleport hard-reject, speed warn-only. Holds a
    /// per-entity server-clock sample for the speed/teleport layer; that
    /// state is released in `destroy_entity` via `forget`.
    pub movement_validator: MovementValidator,
    /// Per-entity observability state for the movement path: the
    /// reject-log and NPC-path-failure throttles, and the last accepted
    /// position sample. Purely a reporting aid — nothing here changes
    /// what is accepted. Released in `destroy_entity` alongside
    /// `movement_validator.forget`, so it cannot outlive the entity
    /// population. See
    /// [`movement_telemetry`] for why each piece exists.
    pub(crate) movement_telemetry: MovementTelemetry,
    /// Gates the NPC AI's "0 HEALTH with no `BSF_DEAD`" invariant warning,
    /// one slot per NPC — see `cell::service::npc_ai::dispatch`. Released in
    /// `destroy_entity`.
    pub(crate) zero_health_npc_log: LogThrottle,
    /// NPC AI detector state (NA02): stuck / stale / floating / leash-loop
    /// trackers and their WARN throttles. Reporting only; released in
    /// `destroy_entity` and `destroy_space`. See
    /// `cell::service::npc_ai::detectors`.
    pub(in crate::cell) npc_detectors: super::service::npc_ai::detectors::NpcDetectors,
    /// Loaded occluders by world file key (`castle_cellblock`), misses
    /// included; see `SpaceManager::occluder_for_world`.
    pub(crate) occluders: HashMap<String, Option<std::sync::Arc<cimmeria_occluder::PagedOccluder>>>,
    /// The residency gauges last reported per world key; see
    /// `SpaceManager::refresh_occluder_residency`.
    pub(crate) occluder_residency: HashMap<String, occlusion::ResidencyGauge>,
    /// Eye height per body set (`resources.body_sets.eye_height`, NA31),
    /// keyed by the full body-set name (`BS_HumanMale.BS_HumanMale`).
    /// Loaded at startup; read through [`SpaceManager::eye_height_of`].
    pub body_set_eye_heights: HashMap<String, f32>,
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
    /// Per-GM `.`-console authoring buffer: `entity_id → [(seed_file, sql)]`.
    /// Spawn/patrol authoring commands push their generated seed SQL here;
    /// `.seedconfirm` groups it per file and emits it, `.seedcancel` discards
    /// it. Server-side, ephemeral (never persisted) — the durable artifact is
    /// the per-session authoring log file and the committed seed. See
    /// `crate::cell::console::seed`.
    pub authoring_changes: HashMap<u32, Vec<(String, String)>>,
    /// GMs (`entity_id`) who toggled `.autosavespawn` on. A session preference;
    /// informational hook for spawn-authoring. Never persisted.
    pub autosave_spawns: HashSet<u32>,
    /// Characters (`player_id`, the character DB id) whose GM switched
    /// proximity aggro off with `.aggro off` (NA13, D-NA02). Keyed by
    /// character rather than entity so the toggle survives zone changes and
    /// relogs; lost on a server restart, never persisted. Honoured only
    /// while the entity still has GM access. Damage and content threat still
    /// engage a GM; only the Idle auto-aggro scan skips them.
    pub gm_aggro_off: HashSet<i32>,
    /// In-memory patrol-path authoring buffer: `path_id → [waypoint]`, for
    /// `.path_add`/`.path_show`/`.path_assign`. Holds the waypoints a GM is
    /// authoring this session so `.path_assign` can apply them to an NPC's
    /// `patrol_path` immediately; the durable copy is the recorded
    /// `point_set_points` seed SQL. Path id == `point_sets.set_id`.
    pub patrol_authoring: HashMap<i32, Vec<cimmeria_common::Vector3>>,
    /// Content-engine actions deferred by `content_actions.delay_ms > 0`,
    /// keyed by the entity that triggered the chain. Drained by
    /// `content::executor::deferred_content_action_tick` on the existing
    /// 100ms cell tick — see `deferred_content_actions` for the scheduling
    /// API and `SpaceManager::destroy_entity` for the disconnect/leave-space
    /// cleanup (same choke point as `authoring_changes`/`autosave_spawns`).
    pub(crate) pending_content_actions: HashMap<u32, Vec<PendingContentAction>>,
    /// Pre-hit health percentages sampled at the damage-application seams,
    /// awaiting the content-layer `entity_health_below` drain. Filled by
    /// [`crate::cell::combat::note_pre_damage_health`], emptied by
    /// `content::fire_pending_health_below`. See
    /// [`crate::cell::combat::damage_credit`] for why the sample cannot
    /// live at the ability caller.
    pub(crate) pending_health_below: Vec<super::combat::HealthBelowSample>,
    /// In-flight stargate dials, keyed by the dialing player. Armed by
    /// `cell::gate_travel::handle_dial_gate`, opened (and marked passable)
    /// by `cell::gate_travel::gate_dial_tick` on the 100ms cell tick, and
    /// consumed by the stargate-region crossing. Scrubbed by
    /// `destroy_entity` / `disconnect_entity` so a dialer who leaves the
    /// space never gets a late `Stargate_MakeGate`. See
    /// `gate_dial_state` for the state machine.
    pub(crate) pending_gate_dials: HashMap<u32, PendingGateDial>,
    /// In-flight post-crossing holds, keyed by the crossing player. Armed by
    /// `cell::gate_travel::on_stargate_passage` right after
    /// `Stargate_CrossGate`/`onStargatePassage` are sent, drained (and the
    /// deferred `perform_gate_travel` run) by
    /// `cell::gate_travel::tick::crossing_tick` on the 100ms cell tick.
    /// Scrubbed by `destroy_entity` / `disconnect_entity` so a crossing
    /// player who leaves mid-hold never gets a deferred travel run against
    /// a dead session. See `crossing_hold_state` for the state machine
    /// (NA35).
    pub(crate) pending_crossings: HashMap<u32, PendingCrossing>,
    /// Re-entrancy bound for the H52 step-activation region replay. A
    /// replayed `enter_region` chain can advance another step, which replays
    /// again; this caps the depth and remembers which `(entity, mission,
    /// step)` triples the current outermost activation has already served.
    /// Owned here because the recursion runs through
    /// `content::executor::execute_actions`, which cannot thread a depth
    /// parameter back to the dispatcher — the `&mut SpaceManager` every frame
    /// already holds is the exclusive token. See
    /// `content::event_dispatch::step_activation`.
    pub(crate) step_region_replay: super::content::StepRegionReplayGuard,
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
            dialog_screen_text: HashMap::new(),
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
            ability_tree_catalog: crate::ability_tree::AbilityTreeCatalog::default(),
            trainer_abilities: HashMap::new(),
            template_trainer_lists: HashMap::new(),
            item_defs: HashMap::new(),
            loot_tables: HashMap::new(),
            respawners: Vec::new(),
            spawn_templates: HashMap::new(),
            ring_regions: HashMap::new(),
            ring_point_set_to_region: HashMap::new(),
            ring_transporters: super::ring_transport::RingTransporterManager::new(),
            pending_ai_retries: std::collections::HashSet::new(),
            movement_validator: MovementValidator::new(),
            movement_telemetry: MovementTelemetry::default(),
            zero_health_npc_log: LogThrottle::default(),
            npc_detectors: Default::default(),
            occluders: HashMap::new(),
            occluder_residency: HashMap::new(),
            body_set_eye_heights: HashMap::new(),
            cover: super::cover::Cover::empty(),
            cover_detection: super::cover::CoverDetectionTable::new(),
            authoring_changes: HashMap::new(),
            autosave_spawns: HashSet::new(),
            gm_aggro_off: HashSet::new(),
            patrol_authoring: HashMap::new(),
            pending_content_actions: HashMap::new(),
            pending_health_below: Vec::new(),
            step_region_replay: super::content::StepRegionReplayGuard::default(),
            pending_gate_dials: HashMap::new(),
            pending_crossings: HashMap::new(),
        }
    }
}
