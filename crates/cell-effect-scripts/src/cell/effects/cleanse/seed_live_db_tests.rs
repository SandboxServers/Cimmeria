//! Live-DB guards on the rows the `shield` and `cleanse` families write
//! (ability mechanics AB-10, `tools/ability_mechanics/families/`), through
//! the cell's own loaders and the real scripts. Every value is
//! RECONSTRUCTION from the effect's own `effect_desc`.

use std::time::Instant;

use cimmeria_entity::abilities::ability_is_beneficial;
use cimmeria_entity::cell_entity::ActiveEffectInstance;
use cimmeria_entity::stats::{ABSORB_ENERGY, ABSORB_HAZMAT, ABSORB_PHYSICAL};

use super::super::test_fixtures::make_mgr_with_target;
use super::super::{dispatch_by_name, EffectContext};
use super::{EFFECT_CATEGORY_NVP, REMOVE_CATEGORIES_NVP, REMOVE_POLARITY_NVP};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{load_ability_defs, load_effect_defs};
use crate::test_support::require_db_or_skip;

/// `(effect_id, RemoveCategories)` of the bound purges, with their text.
const PURGES: [(i32, &str); 4] = [
    (2672, "Mental:5"), // Warrior's Will "Purge: Mental Effects x5"
    (2827, "Mental:5"), // Clear: Mind "Purges Mental States x5"
    (4168, "Mental:2"), // Absolution "Purge Mental Effects: X 2"
    (4169, "Health:2"), // Absolution "Purge Health Effects: X 2"
];

async fn seeded(pool: &sqlx::PgPool) -> SpaceManager {
    let mut mgr = make_mgr_with_target();
    mgr.effect_defs = load_effect_defs(pool).await.expect("load_effect_defs");
    mgr.ability_defs = load_ability_defs(pool).await.expect("load_ability_defs");
    mgr
}

fn run(mgr: &mut SpaceManager, effect_id: i32) {
    let def = mgr.effect_defs[&effect_id].clone();
    let script = def.script_name.clone().expect("seeded script");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &def,
        space_mgr: mgr,
    };
    assert!(dispatch_by_name(&script, &mut ctx), "{script} registered");
}

#[tokio::test]
async fn generated_shield_and_purge_rows_live_db() {
    let pool = require_db_or_skip!();
    let defs = load_effect_defs(&pool).await.expect("load_effect_defs");
    // Personal Shield "Absorption: 500 Physical / 500 Energy / 500 Contamination".
    let shield = &defs[&4306];
    assert_eq!(shield.script_name.as_deref(), Some("AbsorbShield"));
    assert_eq!(
        shield.params.get("ShieldAmount").map(String::as_str),
        Some("500")
    );
    assert_eq!(
        shield.params.get("ShieldType").map(String::as_str),
        Some("Physical,Energy,Hazmat")
    );
    // The held mitigation toggles: Shield: Physical, Energy, Contamination
    // "+15% ... Mitigation" and Shield: Universal "+10%", AB-08 held entries.
    for (effect_id, points) in [(4270, "15"), (4271, "15"), (4272, "15"), (3148, "10")] {
        assert_eq!(
            defs[&effect_id].script_name.as_deref(),
            Some("TimedStat"),
            "{effect_id}"
        );
        assert_eq!(
            defs[&effect_id]
                .params
                .get("Mitigation")
                .map(String::as_str),
            Some(points),
            "{effect_id}"
        );
    }
    for (effect_id, slots) in PURGES {
        let def = &defs[&effect_id];
        assert_eq!(
            def.script_name.as_deref(),
            Some("RemoveEffects"),
            "{effect_id}"
        );
        assert_eq!(
            def.params.get(REMOVE_CATEGORIES_NVP).map(String::as_str),
            Some(slots),
            "{effect_id}"
        );
        assert_eq!(
            def.params.get(REMOVE_POLARITY_NVP).map(String::as_str),
            Some("Harmful")
        );
    }
    // Tags from the co-sequenced resist rolls.
    for (effect_id, kind) in [(1474, "Mental"), (2608, "Kinetic"), (4237, "Health")] {
        assert_eq!(
            defs[&effect_id]
                .params
                .get(EFFECT_CATEGORY_NVP)
                .map(String::as_str),
            Some(kind),
            "{effect_id}"
        );
    }
    // No number, no count: unbound and reported.
    for effect_id in [4785, 4852, 2693] {
        assert_eq!(
            defs[&effect_id].script_name, None,
            "{effect_id} stays unbound"
        );
    }
}

/// The routing the families rely on: the shield and purge abilities land
/// on the caster or an ally although their rows carry no beneficial bit.
#[tokio::test]
async fn shield_and_purge_abilities_are_beneficial_live_db() {
    let pool = require_db_or_skip!();
    let abilities = load_ability_defs(&pool).await.expect("load_ability_defs");
    let effects = load_effect_defs(&pool).await.expect("load_effect_defs");
    // 1013 Personal Shield, 1016 Shield: Physical, 2027 Warrior's Will,
    // 2099 Clear: Mind, 2865 Absolution.
    for id in [1013, 1016, 2027, 2099, 2865] {
        assert!(ability_is_beneficial(&abilities[&id], &effects), "{id}");
    }
}

/// Personal Shield through the real script: three pools of 500.
#[tokio::test]
async fn personal_shield_grants_three_pools_live_db() {
    let pool = require_db_or_skip!();
    let mut mgr = seeded(&pool).await;
    run(&mut mgr, 4306);
    let e = mgr.get_entity(1).unwrap();
    for stat in [ABSORB_PHYSICAL, ABSORB_ENERGY, ABSORB_HAZMAT] {
        assert_eq!(e.stats.get(stat).unwrap().cur, 500, "stat {stat}");
    }
    assert_eq!(e.stat_buffs.entries.len(), 1);
    assert_eq!(e.stat_buffs.entries[0].duration_secs, 30.0);
}

/// Absolution's "Purge Health Effects: X 2" against three seeded Health DoTs
/// (Wounding Shot 4237, from three invokers) removes exactly two.
#[tokio::test]
async fn absolution_purges_exactly_two_health_effects_live_db() {
    let pool = require_db_or_skip!();
    let mut mgr = seeded(&pool).await;
    for invoker in [7, 8, 9] {
        mgr.get_entity_mut(1)
            .unwrap()
            .active_effects
            .push(ActiveEffectInstance {
                invoker_identity: Default::default(),
                cast_id: None,
                effect_id: 4237,
                ability_id: 716,
                invoker_id: invoker,
                remaining_pulses: 5,
                total_pulses: 8,
                next_pulse_at: Instant::now(),
                pulse_interval_secs: 1.0,
                invoker_position_at_register: None,
            });
    }
    run(&mut mgr, 4169);
    let left: Vec<u32> = mgr
        .get_entity(1)
        .unwrap()
        .active_effects
        .iter()
        .map(|i| i.invoker_id)
        .collect();
    assert_eq!(left, [9], "exactly two of the three came off, oldest first");
}
