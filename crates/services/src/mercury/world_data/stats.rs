//! Archetype stats and ability tree data lookups.

use super::ArchetypeStats;

// ── Archetype data ───────────────────────────────────────────────────────────

/// Look up archetype base stats by archetype ID.
///
/// Hardcoded from `db/resources/Archetypes/Seed/archetypes.sql`. All archetypes
/// except Commando share the same stat spread in the seed data.
pub fn archetype_stats(archetype_id: i32) -> ArchetypeStats {
    match archetype_id {
        2 => ArchetypeStats { // Commando (only one with different stats)
            coordination: 4, engagement: 4, fortitude: 2, morale: 3,
            perception: 5, intelligence: 3, health: 760, focus: 1570,
            health_per_level: 10, focus_per_level: 70,
        },
        _ => ArchetypeStats { // Soldier, Scientist, Archeologist, Asgard, Goa'uld, Shol'va, Jaffa
            coordination: 5, engagement: 4, fortitude: 3, morale: 4,
            perception: 3, intelligence: 2, health: 760, focus: 1570,
            health_per_level: 10, focus_per_level: 70,
        },
    }
}

/// Ability tree data per archetype (from `db/resources/Archetypes/Seed/archetype_ability_tree.sql`).
///
/// Only Soldier (1) and Commando (2) have tree data in seed; others get empty trees.
pub fn archetype_ability_tree(archetype_id: i32) -> cimmeria_entity::abilities::AbilityTreeData {
    use cimmeria_entity::abilities::AbilityTreeData;
    match archetype_id {
        1 => AbilityTreeData { // Soldier
            trees: [
                // Tree 0 (29 abilities)
                vec![597,603,604,610,611,616,617,621,622,623,625,627,641,
                     643,645,648,650,652,661,662,663,666,667,668,672,675,
                     677,679,680],
                // Tree 1 (28 abilities)
                vec![598,599,605,606,612,613,618,619,624,626,628,629,642,
                     644,646,647,649,651,653,654,664,665,669,670,673,674,
                     676,678],
                // Tree 2 (27 abilities)
                vec![600,601,602,607,608,609,614,615,620,630,631,655,656,
                     657,658,659,660,671,681,682,683,684,685,686,687,688,689],
            ],
        },
        2 => AbilityTreeData { // Commando
            trees: [
                // Tree 0 (28 abilities)
                vec![700,706,707,713,714,720,721,726,727,731,732,733,735,
                     736,750,751,753,755,757,765,766,770,771,774,775,778,
                     780,781],
                // Tree 1 (30 abilities)
                vec![701,702,708,709,715,716,722,723,728,729,734,737,738,
                     739,752,754,756,758,759,767,768,769,772,773,776,777,
                     779,782,783,784],
                // Tree 2 (27 abilities)
                vec![703,704,705,710,711,712,717,718,719,724,725,730,740,
                     741,760,761,762,763,764,785,786,787,788,789,790,791,792],
            ],
        },
        _ => AbilityTreeData::default(), // Other archetypes: empty trees
    }
}

/// Total XP required to reach each level (from `python/common/Constants.py`).
const LEVEL_EXP: [i32; 21] = [
    0,
    // Level 1-10
    100, 200, 300, 600, 1000, 1600, 2500, 4000, 6000, 9000,
    // Level 11-20
    14000, 18000, 25000, 40000, 60000, 90000, 120000, 180000, 250000, 400000,
];

/// Get the XP required for a given level (clamped to table bounds).
pub(in crate::mercury) fn level_exp(level: i32) -> i32 {
    let idx = (level as usize).min(LEVEL_EXP.len() - 1);
    LEVEL_EXP[idx]
}
