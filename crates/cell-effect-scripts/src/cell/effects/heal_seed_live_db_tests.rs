//! Live-DB guards on the heal rows the ability-mechanics generator writes
//! (AB-02, `tools/ability_mechanics/effect_nvps_from_desc.py`, block
//! `ability-mechanics generated heal` in `effect_nvps.sql`).
//!
//! They load the real seed through the cell's own loader and run each
//! effect's seeded `script_name` through the registry, so a dropped
//! `script_name`, a dropped NVP row or a wrong per-pulse share fails here.
//! Every value is RECONSTRUCTION from the effect's own `effect_desc`.

use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::{FOCUS, HEALTH};

use super::test_fixtures::make_mgr_with_target;
use super::{dispatch_by_name, EffectContext};
use crate::cell::spawner::load_effect_defs;
use crate::test_support::require_db_or_skip;

/// `(effect_id, script_name, NVP name, NVP value)` for every generated heal,
/// with the designer text it came from.
const GENERATED: [(i32, &str, &str, &str); 12] = [
    (788, "HealHealth", "HealPercentage", "10.00"), // Field Medic I "+10% Health"
    (789, "HealHealth", "HealPercentage", "20.00"), // Field Medic II "+20% Health"
    (834, "HealFocus", "HealPercentage", "10.00"),  // Restore: Concentration "+10% Focus"
    (835, "HealFocus", "HealPercentage", "20.00"), // Restore: Will "Heals 20% of target's Focus pool"
    (939, "HealFocus", "HealPercentage", "35.00"), // Morale Boost (user) "35% Focus Heal"
    (1040, "HealFocus", "HealPercentage", "35.00"), // Stim Pack "Target +35% Focus"
    (1044, "HealHealth", "HealPercentage", "10.00"), // Battlefield Heal "Target +10% Health"
    (1877, "HealFocus", "HealPercentage", "35.00"), // Synaptic Clarity "Heals 35% of players Focus pool"
    (1878, "HealFocus", "HealPercentage", "35.00"), // Synaptic Connnection, same text
    (2009, "HealFocus", "HealPercentage", "35.00"), // Focus Regeneration, same text
    (2014, "HealFocus", "HealPercentage", "35.00"), // Focus Heal: Target "Heals 35% of target's Focus pool"
    (4085, "HealHealth", "HealPercentage", "10.00"), // Lord's Vitae, 10% per 1 s pulse, 20 pulses
];

#[tokio::test]
async fn generated_heal_effects_carry_their_script_and_nvp_live_db() {
    let pool = require_db_or_skip!();
    let defs = load_effect_defs(&pool).await.expect("load_effect_defs");
    for (effect_id, script, name, value) in GENERATED {
        let def = defs
            .get(&effect_id)
            .unwrap_or_else(|| panic!("effect {effect_id} is seeded"));
        assert_eq!(
            def.script_name.as_deref(),
            Some(script),
            "effect {effect_id} script_name"
        );
        assert_eq!(
            def.params.get(name).map(String::as_str),
            Some(value),
            "effect {effect_id} {name}"
        );
    }
}

/// `(cur, max)` of `stat` on the fixture's entity 1 (HEALTH 50/100, FOCUS
/// 200/1000) after running `def`'s seeded script on it `pulses` times.
fn after_pulses(def: &EffectDef, stat: i32, pulses: i32) -> (i32, i32) {
    let mut mgr = make_mgr_with_target();
    // Start empty-ish so a multi-pulse total is not hidden by the clamp.
    let s = mgr.get_entity_mut(1).unwrap().stats.get_mut(stat).unwrap();
    let max = s.max;
    s.update(0, 0, max);
    let script = def.script_name.clone().expect("seeded script");
    for _ in 0..pulses {
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: def,
            space_mgr: &mut mgr,
        };
        assert!(dispatch_by_name(&script, &mut ctx), "{script} registered");
    }
    let s = mgr.get_entity(1).unwrap().stats.get(stat).unwrap();
    (s.cur, s.max)
}

/// The seeded rows produce the tooltip's total through the real scripts:
/// one pulse for the single-shot heals, `pulse_count` pulses for the
/// over-time ones (the pulsing layer fires exactly `pulse_count`).
#[tokio::test]
async fn generated_heals_restore_the_amount_their_text_states_live_db() {
    let pool = require_db_or_skip!();
    let defs = load_effect_defs(&pool).await.expect("load_effect_defs");

    // 788 "+10% Health": 10 of 100.
    assert_eq!(after_pulses(&defs[&788], HEALTH, 1), (10, 100));
    // 2014 "Heals 35% of target's Focus pool": 350 of 1000.
    assert_eq!(after_pulses(&defs[&2014], FOCUS, 1), (350, 1000));
    // 939 "35% Focus Heal" (Morale Boost's user half).
    assert_eq!(after_pulses(&defs[&939], FOCUS, 1), (350, 1000));
    // 4085 Lord's Vitae, 10% per second for its 20 pulses: full from empty
    // after 10, so check the per-pulse step and the pulse count.
    let vitae = &defs[&4085];
    assert_eq!((vitae.pulse_count, vitae.pulse_duration), (20, 1.0));
    assert_eq!(after_pulses(vitae, HEALTH, 3), (30, 100));
    // The hand-authored 1383 Recuperation the generator's grammar
    // reproduces: 3% x 25 pulses = "75% ... over 25 seconds".
    let recup = &defs[&1383];
    assert_eq!(recup.pulse_count, 25);
    assert_eq!(after_pulses(recup, HEALTH, recup.pulse_count), (75, 100));
}

/// Heals the generator reports instead of binding, because the current
/// pipeline would land them on the wrong entity: group halves (D-AB12),
/// deployable pulses, the self-revive (D-AB11), a damaging ability's heal
/// half, and bare "+N% Focus" on Buff abilities (stance / dart toggle).
/// Morale Boost's radius heal 1215 left this list in AB-07, which fans it
/// out to the caster's allies (`cell-combat`'s `effect_routing_live_db`).
#[tokio::test]
async fn heals_the_pipeline_would_misroute_stay_unbound_live_db() {
    let pool = require_db_or_skip!();
    let defs = load_effect_defs(&pool).await.expect("load_effect_defs");
    for effect_id in [1357, 1358, 2134, 3371, 3372, 3373, 3374, 4140, 4781, 5008] {
        let def = defs
            .get(&effect_id)
            .unwrap_or_else(|| panic!("effect {effect_id} is seeded"));
        assert_eq!(def.script_name, None, "effect {effect_id} stays unbound");
        for name in ["HealPercentage", "HealAmount"] {
            assert!(
                !def.params.contains_key(name),
                "effect {effect_id} has no {name}"
            );
        }
    }
}
