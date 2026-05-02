//! Cell-side entity representation.
//!
//! A `CellEntity` lives on the CellApp and owns the spatial state of a game
//! entity: its position, direction, area-of-interest radius, and the set of
//! other entities that can currently "see" it (witnesses). Property changes
//! on the cell entity may be broadcast to witnesses depending on distribution
//! flags.
//!
//! This corresponds to the C++ `CellEntity` / `Entity` classes in
//! `src/server/CellApp/`.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use cimmeria_common::{EntityId, SpaceId, Vector3};

use crate::abilities::AbilityManager;
use crate::base_entity::PropertyValue;
use crate::missions::MissionManager;
use crate::stats::StatList;

/// Types of NPC interaction available when a player clicks on this entity.
///
/// Maps to the static interaction set IDs from `python/common/Constants.py`.
#[derive(Debug, Clone, PartialEq)]
pub enum NpcInteractionType {
    /// Dialog NPC — shows `onDialogDisplay` with a dialog tree.
    Dialog { dialog_id: i32 },
    /// Vendor NPC — opens `onStoreOpen` with buy/sell lists.
    Vendor,
    /// Ability trainer — opens `onTrainerOpen` with trainable abilities.
    Trainer { archetype_id: i32 },
    /// Lootable entity — opens `onLootDisplay`.
    Loot,
}

/// The cell-side half of a game entity.
///
/// Manages spatial position, orientation, area-of-interest, and the witness
/// set. Created when a base entity requests a cell presence via
/// `BaseApp::createCellEntity`.
pub struct CellEntity {
    /// Runtime entity ID (matches the corresponding `BaseEntity`).
    pub entity_id: EntityId,

    /// The space (world/zone) this entity currently inhabits.
    pub space_id: SpaceId,

    /// World-space position.
    pub position: Vector3,

    /// Facing direction (unit vector or yaw/pitch/roll encoded).
    pub direction: Vector3,

    /// Current velocity in world units per second.
    /// Sent to clients for interpolation between position updates.
    pub velocity: [f32; 3],

    /// Whether the entity is currently on the ground (affects movement mode).
    pub is_on_ground: bool,

    /// Cell-local property values (CELL_PUBLIC, CELL_PRIVATE, etc.).
    pub properties: HashMap<String, PropertyValue>,

    /// Set of entity IDs that currently have this entity in their AoI.
    ///
    /// When a property with `OTHER_CLIENTS` or `ALL_CLIENTS` distribution
    /// changes, updates are sent to all witnesses.
    pub witnesses: HashSet<EntityId>,

    /// Area-of-interest radius in world units. Other entities within this
    /// radius may become witnesses.
    pub aoi_radius: f32,

    /// Whether this entity has a client controller (i.e., is a player).
    /// Only player entities generate AoI notifications.
    pub is_player: bool,

    /// Entity class ID for CREATE_ENTITY wire format (0x02 = SGWPlayer, 0x04 = SGWMob).
    pub class_id: u8,

    /// Combat stats for this being entity.
    /// Initialized from `SGWBeing.statsTemplate` defaults, overwritten by
    /// archetype values for player entities.
    pub stats: StatList,

    /// Ability manager: known abilities, cooldowns, auto-cycle state.
    pub abilities: AbilityManager,

    /// NPC interaction type (what happens when a player interacts with this entity).
    /// None = no interaction. Only meaningful for NPC entities.
    pub interaction_type: Option<NpcInteractionType>,

    /// Display name for NPCs (sent in dialog headers, etc.).
    pub npc_name: Option<String>,

    /// Mission tracking for player entities.
    pub missions: MissionManager,

    /// Database player_id (for persistence operations). Only set for player entities.
    pub player_id: Option<i32>,

    /// Archetype ID for content engine conditions. Set from character data on connect.
    pub archetype_id: Option<i32>,

    /// Entity level (for XP calculations on kill). Default 1.
    pub level: u32,

    // ── Template-driven fields (populated from DB spawnlist + entity_templates) ──

    /// Source template ID from `entity_templates.template_id`.
    pub template_id: Option<i32>,

    /// Spawn tag from `spawnlist.tag` — used by content chains to target this entity
    /// (e.g., `"ArmYourself_FrostBody"`, `"Preparation_Terminal"`).
    pub tag: Option<String>,

    /// Localized name string ID from `entity_templates.name_id`.
    pub name_id: Option<i32>,

    /// Speaker ID for dialog from `entity_templates.speaker_id`.
    pub speaker_id: Option<i32>,

    /// Event set ID for behavior triggers from `entity_templates.event_set_id`.
    pub event_set_id: Option<i32>,

    /// Raw INT_* interaction type bitfield from `entity_templates.interaction_type`.
    /// Modified at runtime by `SetInteractionType` content actions (OR/AND-NOT).
    pub interaction_type_flags: i64,

    /// Entity flags from `entity_templates.flags`.
    pub entity_flags: u64,

    /// Faction ID from `entity_templates.faction` (0=neutral, 1=Tau'ri, 3=SGC, 10=hostile).
    pub faction: u8,

    /// Alignment ID from `entity_templates.alignment`.
    pub alignment: u8,

    /// Always-available interaction set IDs from `entity_templates.static_interaction_sets`.
    pub static_interaction_sets: Vec<i32>,

    /// Whether space scripts modify this entity's properties dynamically.
    pub has_dynamic_properties: bool,

    /// Per-player available interactions: template_id → Vec<(dialog_set_map_id, dialog_id, interaction_flags)>.
    /// Populated by `add_dialog_set` content action. Only used for player entities.
    pub available_interactions: HashMap<i32, Vec<(i32, i32, i64)>>,

    /// Static mesh path for non-humanoid entities (e.g., `"CA-Props.CA-PrisonerCorpse00"`).
    pub static_mesh: Option<String>,

    /// Body set path from `entity_templates.body_set`.
    pub body_set: Option<String>,

    /// Visual component paths from `entity_templates.components`.
    pub components: Vec<String>,

    // ── Being state ─────────────────────────────────────────────────────────
    /// State field bitfield (EStateField flags from Atrea.enums).
    /// Bit 0: BSF_Dead, Bit 1: BSF_AutoCycling, Bit 2: BSF_Crouching,
    /// Bit 3: BSF_InCombat, Bit 4: BSF_PlayingMinigame, Bit 5: BSF_InStealth,
    /// Bit 6: BSF_MovementLock, Bit 7: BSF_Walking, Bit 8: BSF_Holster.
    pub state_field: u32,

    // ── Ammo state ────────────────────────────────────────────────────────────
    //
    // Per-slot ammo lives on `BandolierItem` (`current_ammo`, `cur_ammo_type`)
    // and on the `Stat[AMMO_SLOT_1+slot]` map. Use `active_ammo()`,
    // `active_clip_size()`, `active_ammo_type()`, and `set_slot_ammo()` to
    // read and write — never re-introduce a shadow scalar.

    /// When `Some(t)`, a reload is in progress and the magazine is not yet
    /// available; fire paths must reject until `Instant::now() >= t`. The
    /// reload-completion tick (cell::service::reload_completion_tick) refills
    /// the slot pinned by `reload_slot_id` and clears both fields.
    pub reload_complete_at: Option<std::time::Instant>,
    /// The bandolier slot that initiated the in-flight reload. Pinning to
    /// this slot (rather than `active_bandolier_slot` at completion time)
    /// prevents a mid-reload weapon swap from refilling the wrong magazine.
    /// `None` whenever `reload_complete_at` is `None`.
    pub reload_slot_id: Option<i32>,

    // ── NPC AI state ──────────────────────────────────────────────────────────
    /// AI state for NPC entities (Idle, Fighting, Dead, Leashing).
    pub ai_state: AiState,
    /// Threat list: entity_id → accumulated threat value.
    pub threat_list: HashMap<u32, f32>,
    /// Position where this NPC was spawned (for leashing).
    pub spawn_position: Option<Vector3>,
    /// Ticks until next AI action (count-down from ai tick interval).
    pub ai_cooldown_ticks: u32,
    /// Navmesh path waypoints the NPC is currently following.
    /// Empty = not moving. Each tick pops the next waypoint off the front.
    /// Stored as `VecDeque` so per-tick `pop_front` is O(1) instead of the
    /// O(n) shift `Vec::remove(0)` would do.
    pub nav_path: VecDeque<Vector3>,
    /// Movement speed in world units per tick.
    pub move_speed: f32,
    /// Pin this NPC to its spawn position. AI will attack when the target
    /// is in range + LOS but never pathfind. Loaded from `spawnlist.is_stationary`.
    pub is_stationary: bool,

    // ── Saved mission state (for re-login) ────────────────────────────────────
    /// Saved missions loaded from DB, to be populated before content engine fires.
    pub saved_missions_loaded: bool,

    // ── Loot state ────────────────────────────────────────────────────────────
    /// Loot table ID from `entity_templates.loot_table_id`.
    pub loot_table_id: Option<i32>,
    /// Generated loot items on this corpse (populated on NPC death).
    pub loot: Vec<LootItem>,
    /// Next loot index counter (matches Python `Lootable.nextLootIndex`).
    pub next_loot_index: i32,
    /// Entity ID of the NPC this player is currently looting (only for player entities).
    /// Set when the player interacts with a lootable corpse, cleared on loot window close.
    /// Reference: `python/cell/SGWPlayer.py:setLooting()`
    pub looting_entity: Option<u32>,

    /// Entity ID of the currently-open vendor (only for player entities).
    pub vendor_entity: Option<u32>,

    /// Currently-active bandolier slot (0-based index).
    pub active_bandolier_slot: i32,

    // ── Ring transporter state ──────────────────────────────────────────────

    /// Region ID of the ring pad the player is currently interacting with.
    /// Set when the player triggers a ring switch (Python `interact()`),
    /// cleared when they pick a destination (Python `selectDestination`).
    /// `None` whenever no ring UI is active for this player.
    pub ring_source_id: Option<i32>,
    /// Destination ring region ID after a successful selection — used by the
    /// destination ring's `playerLoaded` callback to route the loaded player
    /// to the correct waiting list. Cleared in `playerLoaded`.
    pub destination_ring_id: Option<i32>,

    /// Player's bandolier items (quick-access equipment slots).
    pub bandolier_items: HashMap<i32, BandolierItem>,

    /// Slot ids whose `current_ammo`/`cur_ammo_type` have changed since the
    /// last persistence flush. Stage A wires this in but doesn't drain it —
    /// stages B/C/D read/clear this on reload completion, slot swap, ammo
    /// change, and logout.
    pub bandolier_ammo_dirty: HashSet<i32>,
}

/// An item in a dead NPC's loot list, ready for display to players.
///
/// Reference: `python/cell/interactions/Lootable.py:LootableItem`
#[derive(Debug, Clone)]
pub struct LootItem {
    /// Item design ID, or None for naquadah (cash).
    pub design_id: Option<i32>,
    /// Quantity of this item/cash.
    pub quantity: i32,
    /// Unique index within this loot list (1-based, sent to client).
    pub index: i32,
}

/// An item slot in the player's bandolier (quick-access equipment bar).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandolierItem {
    /// Item design ID.
    pub item_id: i32,
    /// Magazine/clip size for weapons.
    pub clip_size: i32,
    /// Default ammo type for weapons.
    pub default_ammo_type: i32,
    /// Remaining ammo in this slot's magazine. Per-slot, persisted as
    /// `sgw_inventory.ammo`.
    pub current_ammo: i32,
    /// Currently selected ammo subtype (defaults to `default_ammo_type`).
    /// Persisted per-slot — players can pick a non-default subtype per weapon.
    pub cur_ammo_type: i32,
}

/// NPC AI state machine.
///
/// Reference: `python/Atrea/enums.py:228-239`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiState {
    Idle,
    Fighting,
    Dead,
    Leashing,
}

impl CellEntity {
    /// Create a new cell entity at the given position in the given space.
    pub fn new(entity_id: EntityId, space_id: SpaceId, position: Vector3) -> Self {
        Self {
            entity_id,
            space_id,
            position,
            direction: Vector3::zero(),
            velocity: [0.0; 3],
            is_on_ground: true,
            properties: HashMap::new(),
            witnesses: HashSet::new(),
            aoi_radius: 100.0, // Default AoI radius (matches grid_vision_distance)
            is_player: false,
            class_id: 0x02, // SGWPlayer by default
            stats: StatList::new(),
            abilities: AbilityManager::new(),
            interaction_type: None,
            npc_name: None,
            missions: MissionManager::new(),
            player_id: None,
            archetype_id: None,
            level: 1,
            template_id: None,
            tag: None,
            name_id: None,
            speaker_id: None,
            event_set_id: None,
            interaction_type_flags: 0,
            entity_flags: 0,
            faction: 0,
            alignment: 0,
            static_interaction_sets: Vec::new(),
            has_dynamic_properties: false,
            available_interactions: HashMap::new(),
            static_mesh: None,
            body_set: None,
            components: Vec::new(),
            state_field: 0,
            reload_complete_at: None,
            reload_slot_id: None,
            ai_state: AiState::Idle,
            threat_list: HashMap::new(),
            spawn_position: None,
            ai_cooldown_ticks: 0,
            nav_path: VecDeque::new(),
            move_speed: 0.6, // ~0.6 world units per 100ms tick = 6 units/sec
            is_stationary: false,
            saved_missions_loaded: false,
            loot_table_id: None,
            loot: Vec::new(),
            next_loot_index: 1,
            looting_entity: None,
            vendor_entity: None,
            active_bandolier_slot: 0,
            bandolier_items: HashMap::new(),
            bandolier_ammo_dirty: HashSet::new(),
            ring_source_id: None,
            destination_ring_id: None,
        }
    }

    /// Update the entity's world-space position.
    pub fn set_position(&mut self, position: Vector3) {
        self.position = position;
    }

    /// Get the entity's current world-space position.
    pub fn get_position(&self) -> &Vector3 {
        &self.position
    }

    /// Add an entity to the witness set (it can now see this entity).
    pub fn add_witness(&mut self, entity_id: EntityId) {
        self.witnesses.insert(entity_id);
    }

    /// Remove an entity from the witness set (it can no longer see this entity).
    pub fn remove_witness(&mut self, entity_id: EntityId) {
        self.witnesses.remove(&entity_id);
    }

    /// Get the current set of witness entity IDs.
    pub fn get_witnesses(&self) -> &HashSet<EntityId> {
        &self.witnesses
    }

    /// Returns `true` if the given position is within this entity's AoI radius.
    ///
    /// Uses squared distance comparison to avoid a square root.
    pub fn is_in_aoi(&self, other_pos: &Vector3) -> bool {
        self.position.distance_squared_to(other_pos) <= self.aoi_radius * self.aoi_radius
    }

    // ── Bandolier ammo helpers ───────────────────────────────────────────────
    //
    // Per-slot ammo lives on `BandolierItem.current_ammo` and is mirrored to
    // the `Stat[AMMO_SLOT_1+slot]` map. These helpers are the read/write path
    // for fire, reload, slot swap, and ammo-change; the shadow scalars that
    // used to live on `CellEntity` were removed in Stage C.

    /// Read the active slot's current ammo, or 0 if no item equipped.
    pub fn active_ammo(&self) -> i32 {
        self.bandolier_items
            .get(&self.active_bandolier_slot)
            .map_or(0, |i| i.current_ammo)
    }

    /// Read the active slot's clip size, or 0 if no item equipped.
    pub fn active_clip_size(&self) -> i32 {
        self.bandolier_items
            .get(&self.active_bandolier_slot)
            .map_or(0, |i| i.clip_size)
    }

    /// Read the active slot's selected ammo type, or 0 if no item equipped.
    pub fn active_ammo_type(&self) -> i32 {
        self.bandolier_items
            .get(&self.active_bandolier_slot)
            .map_or(0, |i| i.cur_ammo_type)
    }

    /// Set ammo for a slot, mirroring to the AmmoSlot{N} stat. Returns the
    /// clamped value, or `None` if the slot is unequipped.
    ///
    /// Marks the slot dirty in `bandolier_ammo_dirty` for batched persistence.
    pub fn set_slot_ammo(&mut self, slot_id: i32, current: i32) -> Option<i32> {
        let item = self.bandolier_items.get_mut(&slot_id)?;
        item.current_ammo = current.clamp(0, item.clip_size);
        let clamped = item.current_ammo;
        let stat_id = crate::stats::AMMO_SLOT_1 + slot_id;
        if let Some(stat) = self.stats.get_mut(stat_id) {
            stat.set_current(clamped);
        }
        self.bandolier_ammo_dirty.insert(slot_id);
        Some(clamped)
    }

    /// Refill the active slot's magazine to its `clip_size`. Returns the new
    /// ammo value, or `None` if no slot is equipped.
    pub fn refill_active_slot(&mut self) -> Option<i32> {
        let slot = self.active_bandolier_slot;
        let max = self.bandolier_items.get(&slot).map(|i| i.clip_size)?;
        self.set_slot_ammo(slot, max)
    }
}

impl std::fmt::Debug for CellEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CellEntity")
            .field("entity_id", &self.entity_id)
            .field("space_id", &self.space_id)
            .field("position", &self.position)
            .field("is_on_ground", &self.is_on_ground)
            .field("witness_count", &self.witnesses.len())
            .field("aoi_radius", &self.aoi_radius)
            .field("property_count", &self.properties.len())
            .field("stats", &self.stats)
            .field("known_abilities", &self.abilities.known_count())
            .field("template_id", &self.template_id)
            .field("tag", &self.tag)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity() -> CellEntity {
        CellEntity::new(EntityId(1), SpaceId(100), Vector3::new(10.0, 0.0, 20.0))
    }

    #[test]
    fn new_entity_defaults() {
        let entity = make_entity();
        assert_eq!(entity.entity_id, EntityId(1));
        assert_eq!(entity.space_id, SpaceId(100));
        assert_eq!(entity.position, Vector3::new(10.0, 0.0, 20.0));
        assert_eq!(entity.direction, Vector3::zero());
        assert!(entity.is_on_ground);
        assert!(entity.witnesses.is_empty());
        assert_eq!(entity.aoi_radius, 100.0);
    }

    #[test]
    fn set_and_get_position() {
        let mut entity = make_entity();
        let new_pos = Vector3::new(50.0, 5.0, 60.0);
        entity.set_position(new_pos);
        assert_eq!(*entity.get_position(), new_pos);
    }

    #[test]
    fn add_and_remove_witness() {
        let mut entity = make_entity();
        entity.add_witness(EntityId(2));
        entity.add_witness(EntityId(3));
        assert_eq!(entity.get_witnesses().len(), 2);
        assert!(entity.get_witnesses().contains(&EntityId(2)));

        entity.remove_witness(EntityId(2));
        assert_eq!(entity.get_witnesses().len(), 1);
        assert!(!entity.get_witnesses().contains(&EntityId(2)));
    }

    #[test]
    fn duplicate_witness_is_idempotent() {
        let mut entity = make_entity();
        entity.add_witness(EntityId(2));
        entity.add_witness(EntityId(2));
        assert_eq!(entity.get_witnesses().len(), 1);
    }

    #[test]
    fn remove_absent_witness_is_noop() {
        let mut entity = make_entity();
        entity.remove_witness(EntityId(99)); // no panic
        assert!(entity.get_witnesses().is_empty());
    }

    #[test]
    fn is_in_aoi_within_radius() {
        let entity = make_entity(); // pos = (10, 0, 20), radius = 100
        let nearby = Vector3::new(20.0, 0.0, 25.0);
        assert!(entity.is_in_aoi(&nearby));
    }

    #[test]
    fn is_in_aoi_outside_radius() {
        let entity = make_entity(); // pos = (10, 0, 20), radius = 100
        let far_away = Vector3::new(500.0, 0.0, 500.0);
        assert!(!entity.is_in_aoi(&far_away));
    }

    #[test]
    fn is_in_aoi_at_exact_boundary() {
        let mut entity = make_entity();
        entity.aoi_radius = 10.0;
        // Point exactly 10 units away on the X axis
        let boundary = Vector3::new(20.0, 0.0, 20.0);
        assert!(entity.is_in_aoi(&boundary));
    }

    // ── New field defaults ─────────────────────────────────────────────────

    #[test]
    fn new_entity_ammo_defaults_empty() {
        let entity = make_entity();
        // Stage C: shadow scalars are gone — assert the helper-derived view
        // (no items in bandolier => zero everything) and that no slot is dirty.
        assert_eq!(entity.active_ammo(), 0);
        assert_eq!(entity.active_clip_size(), 0);
        assert_eq!(entity.active_ammo_type(), 0);
        assert!(entity.bandolier_items.is_empty());
        assert!(entity.bandolier_ammo_dirty.is_empty());
    }

    #[test]
    fn new_entity_ai_state_defaults_idle() {
        let entity = make_entity();
        assert_eq!(entity.ai_state, AiState::Idle);
        assert!(entity.threat_list.is_empty());
        assert!(entity.spawn_position.is_none());
        assert_eq!(entity.ai_cooldown_ticks, 0);
    }

    #[test]
    fn new_entity_saved_missions_loaded_false() {
        let entity = make_entity();
        assert!(!entity.saved_missions_loaded);
    }

    #[test]
    fn ai_state_equality() {
        assert_eq!(AiState::Idle, AiState::Idle);
        assert_eq!(AiState::Fighting, AiState::Fighting);
        assert_eq!(AiState::Dead, AiState::Dead);
        assert_eq!(AiState::Leashing, AiState::Leashing);
        assert_ne!(AiState::Idle, AiState::Fighting);
        assert_ne!(AiState::Dead, AiState::Leashing);
    }

    #[test]
    fn threat_list_operations() {
        let mut entity = make_entity();
        entity.threat_list.insert(10, 50.0);
        entity.threat_list.insert(20, 100.0);
        assert_eq!(entity.threat_list.len(), 2);
        assert_eq!(entity.threat_list[&20], 100.0);

        // Accumulate threat
        *entity.threat_list.entry(10).or_insert(0.0) += 25.0;
        assert_eq!(entity.threat_list[&10], 75.0);

        // Top threat target
        let top = entity.threat_list.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(&id, _)| id);
        assert_eq!(top, Some(20));

        entity.threat_list.clear();
        assert!(entity.threat_list.is_empty());
    }

    #[test]
    fn spawn_position_stores_and_retrieves() {
        let mut entity = make_entity();
        assert!(entity.spawn_position.is_none());

        let spawn = Vector3::new(100.0, 5.0, 200.0);
        entity.spawn_position = Some(spawn);
        assert_eq!(entity.spawn_position.unwrap(), spawn);
    }

    // ── Bandolier ammo helpers ──────────────────────────────────────────────

    fn make_bandolier_item(item_id: i32, clip: i32) -> BandolierItem {
        BandolierItem {
            item_id,
            clip_size: clip,
            default_ammo_type: 1,
            current_ammo: clip,
            cur_ammo_type: 1,
        }
    }

    #[test]
    fn active_ammo_helpers_with_no_item_return_zero() {
        let entity = make_entity();
        assert_eq!(entity.active_ammo(), 0);
        assert_eq!(entity.active_clip_size(), 0);
        assert_eq!(entity.active_ammo_type(), 0);
    }

    #[test]
    fn active_ammo_helpers_read_from_active_slot() {
        let mut entity = make_entity();
        entity.active_bandolier_slot = 1;
        entity.bandolier_items.insert(0, make_bandolier_item(100, 30));
        entity.bandolier_items.insert(1, BandolierItem {
            item_id: 200, clip_size: 12, default_ammo_type: 2,
            current_ammo: 8, cur_ammo_type: 2,
        });

        assert_eq!(entity.active_ammo(), 8);
        assert_eq!(entity.active_clip_size(), 12);
        assert_eq!(entity.active_ammo_type(), 2);
    }

    /// Seed an AmmoSlot stat so the `set_current()` clamp doesn't pin to 0.
    /// Stage B's world-entry init does this for real; tests have to mirror it
    /// because Stage A leaves the helper callable but unseeded by default.
    fn seed_ammo_stat(entity: &mut CellEntity, slot_id: i32, clip: i32) {
        let stat_id = crate::stats::AMMO_SLOT_1 + slot_id;
        if let Some(stat) = entity.stats.get_mut(stat_id) {
            stat.update(0, clip, clip);
            stat.clear_dirty();
        }
    }

    #[test]
    fn set_slot_ammo_clamps_and_marks_dirty() {
        let mut entity = make_entity();
        entity.bandolier_items.insert(0, make_bandolier_item(100, 30));
        seed_ammo_stat(&mut entity, 0, 30);

        // Clamp above clip_size.
        let result = entity.set_slot_ammo(0, 999);
        assert_eq!(result, Some(30));
        assert!(entity.bandolier_ammo_dirty.contains(&0));
        assert_eq!(entity.stats.get(crate::stats::AMMO_SLOT_1).unwrap().cur, 30);

        // Clamp below zero.
        entity.bandolier_ammo_dirty.clear();
        let result = entity.set_slot_ammo(0, -5);
        assert_eq!(result, Some(0));
        assert!(entity.bandolier_ammo_dirty.contains(&0));
        assert_eq!(entity.stats.get(crate::stats::AMMO_SLOT_1).unwrap().cur, 0);
    }

    #[test]
    fn set_slot_ammo_unequipped_returns_none() {
        let mut entity = make_entity();
        let result = entity.set_slot_ammo(2, 5);
        assert_eq!(result, None);
        assert!(entity.bandolier_ammo_dirty.is_empty());
    }

    #[test]
    fn refill_active_slot_fills_to_clip_size() {
        let mut entity = make_entity();
        entity.bandolier_items.insert(0, BandolierItem {
            item_id: 100, clip_size: 30, default_ammo_type: 1,
            current_ammo: 5, cur_ammo_type: 1,
        });
        seed_ammo_stat(&mut entity, 0, 30);

        let result = entity.refill_active_slot();
        assert_eq!(result, Some(30));
        assert_eq!(entity.bandolier_items[&0].current_ammo, 30);
        let stat = entity.stats.get(crate::stats::AMMO_SLOT_1).unwrap();
        assert_eq!(stat.cur, 30);
    }

    #[test]
    fn refill_active_slot_unequipped_returns_none() {
        let mut entity = make_entity();
        let result = entity.refill_active_slot();
        assert_eq!(result, None);
    }
}
