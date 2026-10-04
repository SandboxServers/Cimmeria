//! Live-DB guards on the stat rows the ability-mechanics generator writes
//! (AB-04, `tools/ability_mechanics/families/stat.py`, block
//! `ability-mechanics generated stat` in `effect_nvps.sql`).
//!
//! They load the real seed through the cell's own loaders, run each
//! effect's seeded `script_name` through the registry, and check the
//! routing the generator relies on (the buffs' abilities stay beneficial).
//! Every value is RECONSTRUCTION from the effect's own `effect_desc`, with
//! D-AB09's units.

use cimmeria_entity::abilities::ability_is_beneficial;
use cimmeria_entity::stats::{ACCURACY, COVER_DEFENSE, DEFENSE, MOVEMENT_SPEED_MOD};

use super::super::test_fixtures::make_mgr_with_target;
use super::super::{dispatch_by_name, EffectContext};
use crate::cell::spawner::{load_ability_defs, load_effect_defs};
use crate::test_support::require_db_or_skip;

/// `(effect_id, NVP name, value, pulse_duration)`, with the designer text.
const PACKET: [(i32, &str, &str, f32); 4] = [
    (700, "Accuracy", "200", 15.0),      // Aim "+200 Accuracy: 15 Seconds"
    (903, "Defense", "-100", 15.0),      // Call Target "-100 Defense: 15 Seconds"
    (1747, "CoverDefense", "100", 15.0), // Hunker Down "+100 Cover Defense: 15 seconds"
    (1962, "MovementSpeedMod", "50", 10.0), // Combat Sprint "User +50% Run Speed" (D-AB09: percent)
];

#[tokio::test]
async fn generated_stat_effects_carry_their_script_and_nvp_live_db() {
    let pool = require_db_or_skip!();
    let defs = load_effect_defs(&pool).await.expect("load_effect_defs");
    for (effect_id, name, value, secs) in PACKET {
        let def = defs
            .get(&effect_id)
            .unwrap_or_else(|| panic!("effect {effect_id} is seeded"));
        assert_eq!(
            def.script_name.as_deref(),
            Some("TimedStat"),
            "effect {effect_id} script_name"
        );
        assert_eq!(
            def.params.get(name).map(String::as_str),
            Some(value),
            "effect {effect_id} {name}"
        );
        assert_eq!(
            (def.pulse_count, def.pulse_duration),
            (1, secs),
            "{effect_id}"
        );
    }
    // The deferred halves stay unbound: Leadership's regen (AB-05) and
    // Hunker Down's secondary half. Combat Sprint's penalty 2002 is bound
    // since AB-07 lands it on the user (`cell-combat`'s
    // `effect_routing_live_db`).
    for effect_id in [1211, 1746] {
        assert_eq!(
            defs[&effect_id].script_name, None,
            "{effect_id} stays unbound"
        );
    }
}

/// The seeded rows move the stat their text states, through the real script.
#[tokio::test]
async fn generated_stat_effects_move_their_stat_live_db() {
    let pool = require_db_or_skip!();
    let defs = load_effect_defs(&pool).await.expect("load_effect_defs");
    for (effect_id, stat, want) in [
        (700, ACCURACY, 200),
        (903, DEFENSE, -100),
        (1747, COVER_DEFENSE, 100),
        (1962, MOVEMENT_SPEED_MOD, 150),
    ] {
        let mut mgr = make_mgr_with_target();
        let def = &defs[&effect_id];
        let script = def.script_name.clone().expect("seeded script");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: def,
            space_mgr: &mut mgr,
        };
        assert!(dispatch_by_name(&script, &mut ctx), "{script} registered");
        let cur = mgr.get_entity(1).unwrap().stats.get(stat).unwrap().cur;
        assert_eq!(cur, want, "effect {effect_id}");
        let entries = &mgr.get_entity(1).unwrap().stat_buffs.entries;
        assert_eq!(entries.len(), 1, "one ledger entry for {effect_id}");
    }
}

/// The routing the generator's scope rules promise: the buffs' abilities
/// are beneficial (so they land on the caster or an ally), Call Target is
/// not (so it lands on the hostile through `damage_apply`). Combat Sprint
/// stopped being beneficial when AB-07 bound its penalty half: both halves
/// land on its user through the per-effect routing instead.
#[tokio::test]
async fn generated_stat_abilities_route_as_the_generator_assumes_live_db() {
    let pool = require_db_or_skip!();
    let abilities = load_ability_defs(&pool).await.expect("load_ability_defs");
    let effects = load_effect_defs(&pool).await.expect("load_effect_defs");
    // 637 Aim, 1454 Hunker Down.
    for id in [637, 1454] {
        assert!(
            ability_is_beneficial(&abilities[&id], &effects),
            "{id} must be beneficial"
        );
    }
    // 847 Call Target, and 1619 Combat Sprint (routed per effect).
    assert!(!ability_is_beneficial(&abilities[&847], &effects));
    assert!(!ability_is_beneficial(&abilities[&1619], &effects));
}
