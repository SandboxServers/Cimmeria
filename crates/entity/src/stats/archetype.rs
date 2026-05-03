//! Archetype-specific base stat values used to seed a fresh player.

/// Number of values in the `resources."EArchetype"` Postgres enum.
///
/// This constant is the Rust-side anchor for a manual-coupling drift check —
/// the live-DB test
/// `archetype_count_matches_earchetype_enum_cardinality` (in
/// `crates/services/src/base/world_entry/methods/player_load/meta.rs`)
/// asserts this matches
/// `cardinality(enum_range(NULL::resources."EArchetype"))` so that adding to
/// the SQL enum without reviewing downstream consumers fails CI loudly
/// instead of silently shipping an empty ability tree.
///
/// **Adding a new archetype requires updates in (verify all):**
/// 1. `db/resources/Archetypes/Types/EArchetype.sql` — enum entry
/// 2. `db/sgw/Players/Tables/sgw_player.sql` — `CHECK (archetype <= N)` bound
/// 3. This constant
/// 4. `db/resources/Archetypes/Seed/archetype_ability_tree.sql` — ability
///    tree rows for the new archetype (or accept empty)
/// 5. `crates/services/src/base/chardef.rs` — CharDefId entries that
///    reference the new archetype (or accept that no character can be
///    created with it)
/// 6. `crates/services/src/mercury/world_data/stats.rs::archetype_ability_tree`
///    — DB-down fallback (only if non-empty fallback is desired)
pub const ARCHETYPE_COUNT: usize = 9;

/// Archetype-specific base stat values passed to [`super::StatList::apply_archetype`].
///
/// Same fields as `ArchetypeStats` in mercury_ext.rs but decoupled from
/// wire format concerns.
#[derive(Debug, Clone)]
pub struct ArchetypeStatValues {
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
