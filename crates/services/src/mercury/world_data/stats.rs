//! Archetype stat lookups.
//!
//! Ability trees are not here: `onAbilityTreeInfo` is built from the shared
//! `crate::ability_tree::AbilityTreeCatalog` (AT-02).

use super::ArchetypeStats;

// ── Archetype data ───────────────────────────────────────────────────────────

/// Look up archetype base stats by archetype ID.
///
/// Hardcoded from `db/resources/Archetypes/Seed/archetypes.sql`. All archetypes
/// except Commando share the same stat spread in the seed data.
pub fn archetype_stats(archetype_id: i32) -> ArchetypeStats {
    match archetype_id {
        2 => ArchetypeStats {
            // Commando (only one with different stats)
            coordination: 4,
            engagement: 4,
            fortitude: 2,
            morale: 3,
            perception: 5,
            intelligence: 3,
            health: 760,
            focus: 1570,
            health_per_level: 10,
            focus_per_level: 70,
        },
        _ => ArchetypeStats {
            // Soldier, Scientist, Archeologist, Asgard, Goa'uld, Shol'va, Jaffa
            coordination: 5,
            engagement: 4,
            fortitude: 3,
            morale: 4,
            perception: 3,
            intelligence: 2,
            health: 760,
            focus: 1570,
            health_per_level: 10,
            focus_per_level: 70,
        },
    }
}
