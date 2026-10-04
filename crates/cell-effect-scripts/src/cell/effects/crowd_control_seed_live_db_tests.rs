//! Live-DB guards on the crowd-control rows the ability-mechanics generator
//! writes (AB-09, family `cc`, block `ability-mechanics generated cc` in
//! `effect_nvps.sql`).
//!
//! They load the real seed through the cell's own loader and run each
//! effect's seeded `script_name` through the registry: a dropped
//! `script_name`, a dropped NVP or a wrong duration fails here. Every value
//! is RECONSTRUCTION from the effect's own `effect_desc`; the snare amount
//! is the family's DESIGN default.

use cimmeria_entity::stats::MOVEMENT_SPEED_MOD;
use cimmeria_wire::state_field::BSF_MOVEMENT_LOCK;

use super::crowd_control::{CC_DURATION_NVP, INTERRUPT_CHANCE_NVP};
use super::test_fixtures::make_mgr_with_target;
use super::{dispatch_by_name, EffectContext};
use crate::cell::spawner::load_effect_defs;
use crate::test_support::require_db_or_skip;

/// `(effect_id, script_name, NVP name, NVP value)` for the named abilities
/// the packet makes work.
const GENERATED: [(i32, &str, &str, &str); 6] = [
    (723, "Interrupt", INTERRUPT_CHANCE_NVP, "100"), // Interrupting Shot "Interrupts target"
    (1462, "TimedStat", "MovementSpeedMod", "-30"),  // Snare Shot "Snare: 15 Seconds"
    (1599, "Stun", CC_DURATION_NVP, "5"),            // Lethal Strike "Stun: 5 seconds"
    (1466, "Stun", CC_DURATION_NVP, "5"),            // Flashbang Grenade "Stun: 5 Seconds"
    (2608, "Knockdown", CC_DURATION_NVP, "5"),       // Takedown "Knockdown: 5 seconds"
    (3459, "Knockdown", CC_DURATION_NVP, "10"),      // Launch Grenade: Shockwave
];

#[tokio::test]
async fn generated_cc_effects_carry_their_script_and_nvp_live_db() {
    let pool = require_db_or_skip!();
    let defs = load_effect_defs(&pool).await.expect("load_effect_defs");
    for (effect_id, script, name, value) in GENERATED {
        let def = defs
            .get(&effect_id)
            .unwrap_or_else(|| panic!("effect {effect_id} is seeded"));
        assert_eq!(def.script_name.as_deref(), Some(script), "{effect_id}");
        assert_eq!(
            def.params.get(name).map(String::as_str),
            Some(value),
            "effect {effect_id} {name}"
        );
    }

    // Takedown's seeded row, run through the registry: a 5 s lock.
    let takedown = &defs[&2608];
    let mut mgr = make_mgr_with_target();
    let mut ctx = EffectContext {
        source_id: 7,
        target_id: 1,
        effect: takedown,
        space_mgr: &mut mgr,
    };
    assert!(dispatch_by_name("Knockdown", &mut ctx));
    let e = mgr.get_entity(1).unwrap();
    assert!(e.has_state_flag(BSF_MOVEMENT_LOCK));
    assert_eq!(e.stat_buffs.entries[0].duration_secs, 5.0);

    // Snare Shot's: 70 % speed for 15 s.
    let snare = &defs[&1462];
    let mut ctx = EffectContext {
        source_id: 7,
        target_id: 1,
        effect: snare,
        space_mgr: &mut mgr,
    };
    assert!(dispatch_by_name("TimedStat", &mut ctx));
    let e = mgr.get_entity(1).unwrap();
    assert_eq!(e.stats.get(MOVEMENT_SPEED_MOD).unwrap().cur, 70);
    assert!(e
        .stat_buffs
        .entries
        .iter()
        .any(|t| t.effect_id == 1462 && t.duration_secs == 15.0));
}

/// The generator's `cc` family writes the NVP names the scripts read.
#[test]
fn cc_nvp_names_match_the_generator() {
    let src = include_str!("../../../../../tools/ability_mechanics/families/cc.py");
    assert!(src.contains(&format!("CC_DURATION = \"{CC_DURATION_NVP}\"")));
    assert!(src.contains(&format!("INTERRUPT_CHANCE = \"{INTERRUPT_CHANCE_NVP}\"")));
    assert!(src.contains("SPEED = \"MovementSpeedMod\""));
}
