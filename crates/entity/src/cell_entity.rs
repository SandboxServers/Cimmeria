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

use std::collections::{HashMap, HashSet};

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
}

impl CellEntity {
    /// Create a new cell entity at the given position in the given space.
    pub fn new(entity_id: EntityId, space_id: SpaceId, position: Vector3) -> Self {
        Self {
            entity_id,
            space_id,
            position,
            direction: Vector3::zero(),
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
}
