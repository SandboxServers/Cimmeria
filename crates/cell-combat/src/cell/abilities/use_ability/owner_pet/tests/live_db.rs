//! Live-DB guards: the seed carries what the PT-08 unit fixture (`mod.rs`)
//! models, and a cast on seeded rows acts on the pet.
//!
//! The 2009 rows shipped no `script_name` and no NVPs for these effects, and
//! 1650 shipped no effect at all. Every value here is a seed edit
//! (`db/resources/Effects/Seed/effects.sql`, `effect_nvps.sql`,
//! `db/resources/Abilities/Seed/abilities.sql`); drop one and its test fails.

use cimmeria_cell_world::cell::effects::pet_scripts::is_passive_script;
use cimmeria_entity::abilities::EF_ALWAYS_PERSIST;
use cimmeria_entity::stats::{ACCURACY, DEFENSE};
use tokio::sync::mpsc;

use super::*;
use crate::cell::abilities::use_ability::handle_use_ability;
use crate::cell::abilities::use_ability::owner_pet::is_owner_pet_ability;
use crate::cell::spawner::{load_ability_defs, load_effect_defs};
use crate::test_support::require_db_or_skip;

/// Each effect names its script and carries the NVPs the fixture models;
/// 1650 owns the new effect 350; the fixture's ability rows match the seed.
#[tokio::test]
async fn seeded_pet_effects_match_the_fixture() {
    let pool = require_db_or_skip!();
    let effects = load_effect_defs(&pool).await.expect("effects load");
    let abilities = load_ability_defs(&pool).await.expect("abilities load");
    let mut fixture = SpaceManager::new(1);
    seed_rows(&mut fixture);

    for (id, want) in &fixture.effect_defs {
        let got = effects
            .get(id)
            .unwrap_or_else(|| panic!("effect {id} seeded"));
        assert_eq!(got.ability_id, want.ability_id, "effect {id}");
        assert_eq!(got.script_name, want.script_name, "effect {id} script_name");
        assert_eq!(got.pulse_count, want.pulse_count, "effect {id}");
        assert!(
            (got.pulse_duration - want.pulse_duration).abs() < 1e-6,
            "effect {id}"
        );
        assert_eq!(
            got.target_collection_method, want.target_collection_method,
            "effect {id}"
        );
        for (name, value) in &want.params {
            assert_eq!(
                got.params.get(name).map(|v| v.parse::<f32>().unwrap()),
                Some(value.parse::<f32>().unwrap()),
                "effect {id} NVP {name}"
            );
        }
    }
    for (id, want) in &fixture.ability_defs {
        let got = abilities
            .get(id)
            .unwrap_or_else(|| panic!("ability {id} seeded"));
        assert_eq!(got.effect_ids, want.effect_ids, "ability {id} effects");
        assert_eq!(got.flags, want.flags, "ability {id} flags");
        assert!((got.cooldown - want.cooldown).abs() < 1e-6, "ability {id}");
        assert!((got.warmup - want.warmup).abs() < 1e-6, "ability {id}");
    }
    // 1207 Repair Turret: Full, 10 pulses of 10%.
    assert_eq!(effects[&3350].script_name.as_deref(), Some("HealPetHealth"));
    assert_eq!(
        effects[&3350]
            .params
            .get("HealPercentage")
            .map(String::as_str),
        Some("10.00")
    );
}

/// The redirect recognises exactly the owner-pet abilities on seeded data.
/// 1646 "Health Heal" (`HealHealth`, the universal starter, D-AT09) and the
/// passive 2852 are not redirected.
#[tokio::test]
async fn seeded_owner_pet_abilities_are_recognised() {
    let pool = require_db_or_skip!();
    let mut mgr = SpaceManager::new(1);
    mgr.effect_defs = load_effect_defs(&pool).await.expect("effects load");
    mgr.ability_defs = load_ability_defs(&pool).await.expect("abilities load");
    for id in [2824, 2839, 1650, 967, 968, 1207] {
        assert!(is_owner_pet_ability(&mgr, id), "{id} acts on the pet");
    }
    for id in [1646, 2852, 2826, 592] {
        assert!(!is_owner_pet_ability(&mgr, id), "{id} is not redirected");
    }
}

/// The `EF_AlwaysPersist` effects a login runs: Heed Our Calling's 4968
/// (`SpeedPet` = 100) and, since ability mechanics AB-08, the three stat
/// passives the generator binds to `TimedStat` (1741 Cover Penetration,
/// 2645 Warrior's Resilience, 4782 Create Density: Basic). Any other
/// passive-script row would fire on every login and grant.
#[tokio::test]
async fn the_seeded_passives_are_exactly_these() {
    let pool = require_db_or_skip!();
    let effects = load_effect_defs(&pool).await.expect("effects load");
    let mut passives: Vec<i32> = effects
        .values()
        .filter(|e| e.flags & EF_ALWAYS_PERSIST != 0)
        .filter(|e| e.script_name.as_deref().is_some_and(is_passive_script))
        .map(|e| e.effect_id)
        .collect();
    passives.sort_unstable();
    assert_eq!(passives, vec![1741, 2645, 4782, 4968]);
    assert_eq!(effects[&4968].ability_id, 2852);
    assert_eq!(effects[&4968].param_i32("SpeedPet"), 100);
}

/// End to end on seeded rows: Holy Warrior buffs the fixture pet.
#[tokio::test]
async fn holy_warrior_on_seeded_rows_buffs_the_pet() {
    let pool = require_db_or_skip!();
    let (mut mgr, pet) = world();
    mgr.effect_defs = load_effect_defs(&pool).await.expect("effects load");
    mgr.ability_defs = load_ability_defs(&pool).await.expect("abilities load");
    let (tx, _rx) = mpsc::channel(256);
    assert!(handle_use_ability(OWNER, HOLY_WARRIOR, 0, &tx, &mut mgr).await);
    assert_eq!(stat(&mgr, pet, ACCURACY), 100);
    assert_eq!(stat(&mgr, pet, DEFENSE), -100);
}
