// ── Public types ─────────────────────────────────────────────────────────────

/// A character entry for the `onCharacterList` response.
///
/// Fields match the `CharacterInfo` FIXED_DICT from `custom_alias.xml`:
/// `playerId`, `name`, `extraName`, `alignment`, `level`, `gender`,
/// `worldLocation`, `archetype`, `title`, `playerType`, `playable`.
#[derive(Debug, Clone)]
pub struct CharacterInfo {
    pub player_id: i32,
    pub name: String,
    pub extra_name: String,
    pub alignment: u8,
    pub level: u8,
    pub gender: u8,
    pub world_location: String,
    pub archetype: u8,
    pub title: u8,
    pub player_type: i32,
    pub playable: u8,
}

/// Data for world entry (Phase 5) pulled from the database.
#[derive(Debug, Clone)]
pub struct WorldEntryInfo {
    pub player_entity_id: u32,
    pub space_id: u32,
    pub pos: [f32; 3],
    pub rot: [f32; 3],
    /// The world resource name (e.g. "Castle_CellBlock") — used by `onClientMapLoad`
    /// to tell the client which terrain to load for this space_id.
    pub world_name: String,
}

/// Archetype base stat values (from `db/resources/Archetypes/Seed/archetypes.sql`).
#[derive(Debug, Clone)]
pub struct ArchetypeStats {
    pub coordination: i32,
    pub engagement: i32,
    pub fortitude: i32,
    pub morale: i32,
    pub perception: i32,
    pub intelligence: i32,
    pub health: i32,
    pub focus: i32,
    pub health_per_level: i32,
    pub focus_per_level: i32,
}

/// Data loaded from the database for a player entering the world.
/// Used by [`super::world_data::build_map_loaded`] to build the `mapLoaded()` multi-packet sequence.
#[derive(Debug, Clone)]
pub struct PlayerLoadData {
    pub player_id: i32,
    pub level: i32,
    pub player_name: String,
    pub extra_name: String,
    pub alignment: i32,
    pub archetype: i32,
    pub gender: i32,
    pub bodyset: String,
    pub components: Vec<String>,
    pub exp: i32,
    pub naquadah: i32,
    pub known_stargates: Vec<i32>,
    pub abilities: Vec<i32>,
    pub training_points: i32,
    pub applied_science_points: i32,
    pub blueprint_ids: Vec<i32>,
    pub first_login: i32,
    pub access_level: i32,
    pub skin_color_id: i32,
    /// Ability tree data (3 branches). Loaded from `resources.archetype_ability_tree`.
    pub ability_tree: cimmeria_entity::abilities::AbilityTreeData,
    /// Inventory items loaded from `sgw_inventory`.
    pub items: Vec<cimmeria_entity::inventory::InvItem>,
}
