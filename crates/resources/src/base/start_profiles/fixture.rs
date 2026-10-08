//! The seeded start profiles as a value, for tests that cannot read the
//! database (the GM-only redirect, the console entry point, respawn).
//!
//! It is a copy of `db/resources/Archetypes/Seed/char_creation*.sql`, so the
//! live-DB test `fixture_is_the_seed_live_db` compares the two and fails when
//! either changes alone. Behind the `test-support` feature: no production
//! code reads it.

use super::super::chardef::chardef_lookup;
use super::{DebugKit, KitAbility, KitItem, KitSource, StartProfile, StartProfiles, StartState};

/// Castle_CellBlock stasis room: the Praxis start.
pub const CELLBLOCK_START: [f32; 3] = [-334.231, 73.472, -228.026];
/// SGC_W1: the SGU human start.
pub const SGC_W1_START: [f32; 3] = [201.5, 1.31, 49.724];
/// Dakara_E1, south of the DHD facing the gate: the Free Jaffa start.
pub const DAKARA_E1_START: [f32; 3] = [100.0, -17.4, 230.0];

/// The legacy universal kit of the holding states (and the debug kit).
pub const LEGACY_KIT: [i32; 5] = [592, 594, 597, 1218, 1646];
/// SI 3 9mm Pistol.
pub const LEGACY_PISTOL: i32 = 55;

/// The seed's 23 profiles and its debug kit.
pub fn seeded() -> StartProfiles {
    let profiles = (1..=23)
        .map(|id| {
            let c = chardef_lookup(id).expect("1-23 are client char_defs");
            let praxis = c.alignment == 1;
            let (profile_id, world, position, state) = match c.archetype {
                5 => ("SGU_ASGARD".to_string(), "SGC_W1", SGC_W1_START, legacy()),
                6 => (
                    "PRA_GOAULD".to_string(),
                    "Castle_CellBlock",
                    CELLBLOCK_START,
                    legacy(),
                ),
                7 => (
                    "SGU_FREE_JAFFA".to_string(),
                    "Dakara_E1",
                    DAKARA_E1_START,
                    canon(),
                ),
                8 => (
                    "PRA_LOYALIST_JAFFA".to_string(),
                    "Castle_CellBlock",
                    CELLBLOCK_START,
                    canon(),
                ),
                a => {
                    let class = match a {
                        1 => "SOLDIER",
                        2 => "COMMANDO",
                        3 => "SCIENTIST",
                        _ => "ARCHAEOLOGIST",
                    };
                    if praxis {
                        (
                            format!("PRA_OPCORE_{class}"),
                            "Castle_CellBlock",
                            CELLBLOCK_START,
                            canon(),
                        )
                    } else {
                        (
                            format!("SGU_HUMAN_{class}"),
                            "SGC_W1",
                            SGC_W1_START,
                            canon(),
                        )
                    }
                }
            };
            let (abilities, items) = match c.archetype {
                5 | 6 => (
                    LEGACY_KIT
                        .iter()
                        .map(|&ability_id| KitAbility {
                            ability_id,
                            source: KitSource::LegacyKit,
                        })
                        .collect(),
                    vec![KitItem {
                        item_id: LEGACY_PISTOL,
                        stack_size: 1,
                    }],
                ),
                7 => (
                    vec![
                        KitAbility {
                            ability_id: 597,
                            source: KitSource::RacialCore,
                        },
                        KitAbility {
                            ability_id: 1218,
                            source: KitSource::RacialCore,
                        },
                        KitAbility {
                            ability_id: 1984,
                            source: KitSource::Signature,
                        },
                    ],
                    vec![
                        KitItem {
                            item_id: 2797,
                            stack_size: 1,
                        },
                        KitItem {
                            item_id: 4342,
                            stack_size: 1,
                        },
                    ],
                ),
                _ => (Vec::new(), Vec::new()),
            };
            StartProfile {
                char_def_id: id,
                profile_id,
                alignment: c.alignment,
                archetype: c.archetype,
                world: world.to_string(),
                position,
                start_level: 1,
                debug_kit: false,
                start_state: state,
                abilities,
                items,
            }
        })
        .collect();
    StartProfiles::new(
        profiles,
        DebugKit {
            abilities: LEGACY_KIT.to_vec(),
            items: vec![KitItem {
                item_id: LEGACY_PISTOL,
                stack_size: 1,
            }],
        },
    )
}

fn canon() -> StartState {
    StartState::Canonical
}

fn legacy() -> StartState {
    StartState::NonCanonicalBlockedLegacy
}
