//! Tech-disable darts through `apply_damage_to_target` (ammo campaign AM-11b):
//! a player's dart shot with `Dart_EMP` loaded drains the NPC's FOCUS on a
//! hit, and one with `Dart_Radioactive` loaded starts a radiation dose that
//! the pulse tick keeps delivering.
//!
//! The rows and effects are the ones `ammo_modifiers_dart_tech.sql` seeds,
//! built from the constants in `cimmeria_cell_world`'s `ammo_dart_tech`
//! (whose live-DB guard pins the seed to them). The dart weapon's
//! `ammo_types` widening is AM-11a's: the bandolier slot is stubbed here.
//!
//! Like `ammo_tests`, these turn `ammo.finite_special` on for the process.

use super::tests::{make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::cell::effects::ammo_dart_tech::{
    DART_EMP_DAMAGE_MULT, DART_EMP_EFFECT_ID, DART_EMP_FOCUS_DRAIN, DART_EMP_PENETRATION_MULT,
    DART_EMP_TOGGLE_ABILITY, DART_RADIOACTIVE_EFFECT_ID, DART_RADIOACTIVE_PULSES,
    DART_RADIOACTIVE_PULSE_DAMAGE, DART_RADIOACTIVE_PULSE_SECS, DART_RADIOACTIVE_TOGGLE_ABILITY,
};
use crate::cell::spawner::{AmmoCatalog, AmmoModifier};
use crate::test_support::NoContentEvents;
use cimmeria_entity::abilities::{EffectDef, RC_HIT};
use cimmeria_entity::ammo_type::{DART_DEFAULT, DART_EMP, DART_RADIOACTIVE};
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::FOCUS;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const ABILITY: i32 = 1086;
const EFFECT: i32 = 1237;
const SHOT_HEALTH: i32 = 10;
const NPC_HEALTH: i32 = 1_000;
const NPC_FOCUS: i32 = 500;

fn rows() -> Vec<AmmoModifier> {
    vec![
        AmmoModifier {
            ammo_type: DART_EMP,
            damage_mult: DART_EMP_DAMAGE_MULT,
            penetration_mult: DART_EMP_PENETRATION_MULT,
            damage_type: Some(i32::from(DT_PHYSICAL)),
            on_hit_effect_id: Some(DART_EMP_EFFECT_ID),
            toggle_ability_id: DART_EMP_TOGGLE_ABILITY,
            beneficial: false,
        },
        AmmoModifier {
            ammo_type: DART_RADIOACTIVE,
            damage_mult: 1.0,
            penetration_mult: 1.0,
            damage_type: None,
            on_hit_effect_id: Some(DART_RADIOACTIVE_EFFECT_ID),
            toggle_ability_id: DART_RADIOACTIVE_TOGGLE_ABILITY,
            beneficial: false,
        },
    ]
}

fn effect(id: i32, script: &str, nvp: (&str, i32), pulses: i32, secs: f32) -> EffectDef {
    EffectDef {
        effect_id: id,
        script_name: (!script.is_empty()).then(|| script.to_string()),
        pulse_count: pulses,
        pulse_duration: secs,
        params: HashMap::from([(nvp.0.to_string(), nvp.1.to_string())]),
        ..Default::default()
    }
}

/// Player 1 fires the dart pistol auto attack (10 HEALTH, one round) with
/// `ammo_type` loaded at NPC 2. Returns the manager and the result code.
async fn fire(ammo_type: i32, effect_seq: u32) -> (SpaceManager, u8) {
    cimmeria_entity::ammo_feature::set_finite_special(true);
    let mut mgr = make_mgr_player_vs_npc();
    let mut ability = make_ability(ABILITY, vec![EFFECT]);
    ability.required_ammo = 1;
    ability.is_ranged = true;
    mgr.ability_defs.insert(ABILITY, ability.clone());
    for e in [
        effect(EFFECT, "", ("HealthDamage", SHOT_HEALTH), 1, 0.0),
        effect(
            DART_EMP_EFFECT_ID,
            "RangedEnergyDamage",
            ("FocusDamage", DART_EMP_FOCUS_DRAIN),
            1,
            0.0,
        ),
        effect(
            DART_RADIOACTIVE_EFFECT_ID,
            "RadiationDamage",
            ("HealthDamage", DART_RADIOACTIVE_PULSE_DAMAGE),
            DART_RADIOACTIVE_PULSES,
            DART_RADIOACTIVE_PULSE_SECS,
        ),
    ] {
        mgr.effect_defs.insert(e.effect_id, e);
    }
    mgr.ammo_catalog = AmmoCatalog::from_rows(rows(), [(DART_EMP, 9008), (DART_RADIOACTIVE, 9009)]);
    let p = mgr.get_entity_mut(1).unwrap();
    p.active_bandolier_slot = 0;
    p.bandolier_items.insert(
        0,
        BandolierItem {
            instance_id: 1,
            item_id: 3584,
            clip_size: 10,
            default_ammo_type: DART_DEFAULT,
            current_ammo: 10,
            cur_ammo_type: ammo_type,
        },
    );
    let npc = mgr.get_entity_mut(2).unwrap();
    npc.stats
        .get_mut(HEALTH)
        .unwrap()
        .update(0, NPC_HEALTH, NPC_HEALTH);
    npc.stats
        .get_mut(FOCUS)
        .unwrap()
        .update(0, NPC_FOCUS, NPC_FOCUS);

    let seed = pseudo_random_seed(1, ABILITY, effect_seq);
    let qr = combat::calculate_qr(
        &mgr.get_entity(1).unwrap().stats,
        &mgr.get_entity(2).unwrap().stats,
        true,
    );
    let code = combat::calculate_result(qr, seed).result_code;
    let (tx, _rx) = mpsc::channel(256);
    apply_damage_to_target(
        1,
        2,
        ABILITY,
        &Some(ability),
        effect_seq,
        false,
        &tx,
        &mut mgr,
    )
    .await;
    (mgr, code)
}

/// An `effect_seq` whose roll is a plain hit.
async fn hit_seq() -> u32 {
    for seq in 1..200 {
        if fire(DART_DEFAULT, seq).await.1 == RC_HIT {
            return seq;
        }
    }
    panic!("no plain hit in 200 rolls");
}

fn npc(mgr: &SpaceManager, stat: i32) -> i32 {
    mgr.get_entity(2).unwrap().stats.get(stat).unwrap().cur
}

/// Same roll, default dart vs EMP dart: the EMP hit costs the NPC exactly
/// the seeded FOCUS drain more (the shot's own damage is HEALTH only).
#[tokio::test]
async fn emp_dart_hit_drains_the_targets_focus() {
    let seq = hit_seq().await;
    let (plain, _) = fire(DART_DEFAULT, seq).await;
    let (emp, code) = fire(DART_EMP, seq).await;
    assert_eq!(code, RC_HIT);
    assert_eq!(npc(&plain, FOCUS), NPC_FOCUS, "default dart drains nothing");
    assert_eq!(npc(&emp, FOCUS), NPC_FOCUS - DART_EMP_FOCUS_DRAIN);
    assert!(
        emp.get_entity(2).unwrap().active_effects.is_empty(),
        "the drain is one-shot"
    );
}

/// A Radioactive hit takes the first dose pulse with the shot, registers
/// the rest on the NPC, and the pulse tick delivers the next one.
#[tokio::test]
async fn radioactive_dart_hit_starts_a_dose_the_tick_keeps_delivering() {
    let seq = hit_seq().await;
    let (plain, _) = fire(DART_DEFAULT, seq).await;
    let (mut rad, code) = fire(DART_RADIOACTIVE, seq).await;
    assert_eq!(code, RC_HIT);
    let shot_only = npc(&plain, HEALTH);
    assert_eq!(npc(&rad, HEALTH), shot_only - DART_RADIOACTIVE_PULSE_DAMAGE);

    let dose = rad
        .get_entity(2)
        .unwrap()
        .active_effects
        .iter()
        .find(|i| i.effect_id == DART_RADIOACTIVE_EFFECT_ID)
        .cloned()
        .expect("the dose registers on the target");
    assert_eq!(dose.remaining_pulses, DART_RADIOACTIVE_PULSES - 1);
    assert_eq!(dose.invoker_id, 1);

    // Make the next pulse due and tick once.
    let target = rad.get_entity_mut(2).unwrap();
    target.active_effects[0].next_pulse_at = Instant::now() - Duration::from_secs(1);
    let (tx, _rx) = mpsc::channel(256);
    crate::cell::effects::effect_pulse_tick(&NoContentEvents, &tx, &mut rad).await;
    assert_eq!(
        npc(&rad, HEALTH),
        shot_only - 2 * DART_RADIOACTIVE_PULSE_DAMAGE
    );
    assert_eq!(
        rad.get_entity(2).unwrap().active_effects[0].remaining_pulses,
        DART_RADIOACTIVE_PULSES - 2
    );
}
