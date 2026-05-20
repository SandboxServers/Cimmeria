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
    /// Entity class ID for CREATE_BASE_PLAYER: SGWPlayer (0x02) or SGWGmPlayer (0x03).
    /// Matches `python/base/Account.py:293-296`: access_level > 0 → SGWGmPlayer.
    pub class_id: u8,
    /// Stargate entity IDs accessible from this world.
    pub world_stargates: Vec<i32>,
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
    /// Full appearance component list: every equipment visual the player
    /// is currently wearing, including the active bandolier slot's weapon
    /// when one is equipped. This is the unfiltered shape — callers that
    /// emit `BeingAppearance` on the wire should go through
    /// [`Self::appearance_components`] to apply the holster filter, not
    /// read this field directly.
    pub components: Vec<String>,
    /// The visual component string for the active bandolier slot's weapon,
    /// or `None` if the slot is empty.
    ///
    /// **Invariant:** when `weapon_visual` is `Some(v)`, the same string
    /// also appears in `components`. Carrying it separately is how the
    /// holster filter knows which entry to drop without re-deriving the
    /// "is this a weapon" check at wire-emit time;
    /// [`Self::appearance_components`] does the entry-by-value comparison.
    pub weapon_visual: Option<String>,
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
    /// Active bandolier slot (0-based) from `sgw_player.bandolier_slot`.
    pub active_bandolier_slot: i32,
    /// Bandolier items keyed by slot id. Loaded from `sgw_inventory` (container 3)
    /// joined to `resources.items`. Stage C: this is the single source of truth
    /// for the active weapon's clip size and ammo subtype on the wire — the old
    /// shadow `active_weapon_clip_size` / `active_ammo_type` fields are gone.
    pub bandolier_items: Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)>,
    /// Ability tree data (3 branches). Loaded from `resources.archetype_ability_tree`.
    pub ability_tree: cimmeria_entity::abilities::AbilityTreeData,
    /// Inventory items loaded from `sgw_inventory`.
    pub items: Vec<cimmeria_entity::inventory::InvItem>,
}

impl PlayerLoadData {
    /// Build the `ComponentList` for `BeingAppearance` honoring the
    /// requested holster state. At world entry callers pass `true` so the
    /// player spawns weapon-down; runtime rebroadcasts driven from a
    /// live `CellEntity` pass that entity's current `weapon_holstered`
    /// so a fire/reload/draw toggle is reflected on the wire.
    ///
    /// Returns `components` unchanged when not holstered, or `components`
    /// with `weapon_visual` filtered out when holstered. Implementation
    /// shared with [`cimmeria_entity::cell_entity::CellEntity::appearance_components`]
    /// via the [`cimmeria_entity::cell_entity::filter_holstered_weapon`]
    /// free function — see its docstring for the invariant and the
    /// `debug_assert!`/`warn` semantics around drift between
    /// `weapon_visual` and `components`.
    pub fn appearance_components(&self, holstered: bool) -> Vec<String> {
        cimmeria_entity::cell_entity::filter_holstered_weapon(
            &self.components,
            self.weapon_visual.as_deref(),
            holstered,
        )
    }
}

#[cfg(test)]
mod player_load_data_tests {
    use super::*;

    fn make(components: Vec<&str>, weapon_visual: Option<&str>) -> PlayerLoadData {
        PlayerLoadData {
            player_id: 0,
            level: 1,
            player_name: String::new(),
            extra_name: String::new(),
            alignment: 0,
            archetype: 0,
            gender: 0,
            bodyset: "BS_HumanMale.BS_HumanMale".into(),
            components: components.into_iter().map(String::from).collect(),
            weapon_visual: weapon_visual.map(String::from),
            exp: 0,
            naquadah: 0,
            known_stargates: vec![],
            abilities: vec![],
            training_points: 0,
            applied_science_points: 0,
            blueprint_ids: vec![],
            first_login: 0,
            access_level: 0,
            skin_color_id: 0,
            active_bandolier_slot: 0,
            bandolier_items: vec![],
            ability_tree: Default::default(),
            items: vec![],
        }
    }

    #[test]
    fn appearance_components_holstered_filters_weapon_visual() {
        // The world-entry path passes `holstered = true` so the player
        // spawns weapon-down. The wire `ComponentList` must omit the
        // weapon visual; the SGW client's appearance compositor then
        // falls back to `WEAP_Melee = 4` for the animation key at
        // `entity+0x3D2` (`ghidra://SGW.exe@0x00ec0840`).
        let data = make(vec!["torso", "pistol", "head"], Some("pistol"));
        let wire = data.appearance_components(true);
        assert_eq!(
            wire,
            vec!["torso".to_string(), "head".to_string()],
            "holstered ComponentList must drop the weapon visual"
        );
    }

    #[test]
    fn appearance_components_not_holstered_keeps_weapon_visual() {
        // Runtime rebroadcasts after a fire / reload / draw event take this
        // branch — the weapon visual stays in the wire ComponentList so the
        // client renders the armed pose.
        let data = make(vec!["torso", "pistol", "head"], Some("pistol"));
        let wire = data.appearance_components(false);
        assert_eq!(wire, data.components);
    }

    #[test]
    fn appearance_components_no_weapon_is_components_unchanged() {
        // Empty bandolier slot: nothing to filter regardless of state.
        let data = make(vec!["torso", "head"], None);
        assert_eq!(data.appearance_components(true), data.components);
        assert_eq!(data.appearance_components(false), data.components);
    }

    /// Pin the documented invariant: `weapon_visual = Some(v)` implies
    /// `v ∈ components`. This is the contract the holster filter relies
    /// on; any DB loader / appearance assembler that produces a
    /// `PlayerLoadData` must satisfy it. The test is a regression guard
    /// for the `make` fixture itself — if a future refactor changes
    /// how `make` builds the struct (e.g., a loader bug that strips the
    /// weapon visual from `components` while leaving `weapon_visual`
    /// set), the assertion fails fast.
    #[test]
    fn weapon_visual_is_always_present_in_components_when_set() {
        let data = make(vec!["torso", "pistol", "head"], Some("pistol"));
        let weapon = data
            .weapon_visual
            .as_ref()
            .expect("test fixture must set weapon_visual");
        assert!(
            data.components.iter().any(|c| c == weapon),
            "invariant violated: weapon_visual {:?} not present in components {:?}",
            weapon,
            data.components,
        );
    }
}
