//! Crowd-control darts through `apply_damage_to_target`, the pulse tick and
//! the stat-buff tick (ammo campaign AM-11a): a dart shot with Poison, Disease or Tranquilizer
//! loaded runs its on-hit effect on the target, and the effect keeps going
//! (or ends) on the pulse tick.
//!
//! The effect defs and rows below mirror `ammo_modifiers_dart_cc.sql`; the
//! live-DB guard `ammo_dart_cc::tests::live_db_dart_cc_seed_rows` in
//! `cimmeria-cell-world` pins the seed to these numbers.
//!
//! Like `ammo_tests`, these turn `ammo.finite_special` on for the process
//! and never off.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::tests::{make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::cell::effects::effect_pulse_tick;
use crate::cell::spawner::{AmmoCatalog, AmmoModifier};
use crate::test_support::NoContentEvents;
use cimmeria_entity::abilities::{EffectDef, RC_HIT};
use cimmeria_entity::ammo_type::{DART_DEFAULT, DART_DISEASE, DART_POISON, DART_TRANQUILIZER};
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::MOVEMENT_SPEED_MOD;

const ABILITY: i32 = 1086; // Dart Pistol Auto Attack
const SHOT_EFFECT: i32 = 1237;
const NPC: u32 = 2;
const NPC_HEALTH: i32 = 100_000;

const POISON_EFFECT: i32 = 9140;
const DISEASE_EFFECT: i32 = 9141;
const TRANQ_EFFECT: i32 = 9142;

fn effect(id: i32, script: Option<&str>, pulses: (i32, f32), nvps: &[(&str, &str)]) -> EffectDef {
    EffectDef {
        effect_id: id,
        script_name: script.map(str::to_string),
        pulse_count: pulses.0,
        pulse_duration: pulses.1,
        params: nvps
            .iter()
            .map(|&(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>(),
        ..Default::default()
    }
}

/// The three on-hit effects as `ammo_modifiers_dart_cc.sql` seeds them.
fn seeded_effects() -> Vec<EffectDef> {
    vec![
        effect(
            POISON_EFFECT,
            Some("Suppression"),
            (5, 2.0),
            &[("HealthDamage", "4"), ("EffectCategory", "Poison")],
        ),
        effect(
            DISEASE_EFFECT,
            Some("Suppression"),
            (10, 2.0),
            &[("HealthDamage", "2"), ("EffectCategory", "Disease")],
        ),
        effect(
            TRANQ_EFFECT,
            Some("MovementSlow"),
            (1, 6.0),
            &[("SpeedReduction", "40")],
        ),
    ]
}

fn seeded_rows() -> Vec<AmmoModifier> {
    [
        (DART_POISON, POISON_EFFECT, 990),
        (DART_DISEASE, DISEASE_EFFECT, 991),
        (DART_TRANQUILIZER, TRANQ_EFFECT, 998),
    ]
    .into_iter()
    .map(|(ammo_type, on_hit, toggle)| AmmoModifier {
        ammo_type,
        damage_mult: 1.0,
        penetration_mult: 1.0,
        damage_type: Some(i32::from(DT_PHYSICAL)),
        on_hit_effect_id: Some(on_hit),
        toggle_ability_id: toggle,
        beneficial: false,
    })
    .collect()
}

/// Player 1 with a CO2 Pistol Dartgun loaded with `ammo_type`, NPC 2 at
/// `NPC_HEALTH`, and the seeded dart rows and effects.
fn setup(ammo_type: i32) -> (SpaceManager, AbilityDef) {
    cimmeria_entity::ammo_feature::set_finite_special(true);
    let mut mgr = make_mgr_player_vs_npc();
    let mut ability = make_ability(ABILITY, vec![SHOT_EFFECT]);
    ability.required_ammo = 1;
    ability.is_ranged = true;
    mgr.ability_defs.insert(ABILITY, ability.clone());
    mgr.effect_defs.insert(
        SHOT_EFFECT,
        effect(SHOT_EFFECT, None, (1, 0.0), &[("HealthDamage", "10")]),
    );
    for e in seeded_effects() {
        mgr.effect_defs.insert(e.effect_id, e);
    }
    mgr.ammo_catalog = AmmoCatalog::from_rows(
        seeded_rows(),
        [
            (DART_POISON, 9005),
            (DART_DISEASE, 9006),
            (DART_TRANQUILIZER, 9007),
        ],
    );
    let p = mgr.get_entity_mut(1).unwrap();
    p.active_bandolier_slot = 0;
    p.bandolier_items.insert(
        0,
        BandolierItem {
            instance_id: 1,
            item_id: 3584,
            clip_size: 15,
            default_ammo_type: DART_DEFAULT,
            current_ammo: 15,
            cur_ammo_type: ammo_type,
        },
    );
    mgr.get_entity_mut(NPC)
        .unwrap()
        .stats
        .get_mut(HEALTH)
        .unwrap()
        .update(0, NPC_HEALTH, NPC_HEALTH);
    (mgr, ability)
}

async fn shoot(mgr: &mut SpaceManager, ability: &AbilityDef, seq: u32) {
    let (tx, _rx) = mpsc::channel(256);
    apply_damage_to_target(
        1,
        NPC,
        ABILITY,
        &Some(ability.clone()),
        seq,
        false,
        &tx,
        mgr,
    )
    .await;
}

/// Fire one shot with `ammo_type` loaded; return the manager and the HEALTH
/// the NPC lost.
async fn fire(ammo_type: i32, seq: u32) -> (SpaceManager, i32) {
    let (mut mgr, ability) = setup(ammo_type);
    shoot(&mut mgr, &ability, seq).await;
    let lost = NPC_HEALTH - npc_health(&mgr);
    (mgr, lost)
}

fn npc_health(mgr: &SpaceManager) -> i32 {
    mgr.get_entity(NPC).unwrap().stats.get(HEALTH).unwrap().cur
}

fn npc_speed(mgr: &SpaceManager) -> i32 {
    mgr.get_entity(NPC)
        .unwrap()
        .stats
        .get(MOVEMENT_SPEED_MOD)
        .unwrap()
        .cur
}

fn instance_pulses(mgr: &SpaceManager, effect_id: i32) -> Option<i32> {
    mgr.get_entity(NPC)
        .unwrap()
        .active_effects
        .iter()
        .find(|i| i.effect_id == effect_id)
        .map(|i| i.remaining_pulses)
}

/// Make every pulse due now and run one pulse tick.
async fn pulse(mgr: &mut SpaceManager) {
    let past = Instant::now() - Duration::from_secs(5);
    for inst in &mut mgr.get_entity_mut(NPC).unwrap().active_effects {
        inst.next_pulse_at = past;
    }
    let (tx, _rx) = mpsc::channel(256);
    effect_pulse_tick(&NoContentEvents, &tx, mgr).await;
}

/// An `effect_seq` whose roll is a plain, non-zero hit with default darts.
async fn hit_seq() -> u32 {
    for seq in 1..200 {
        let (mgr, _) = setup(DART_DEFAULT);
        let qr = combat::calculate_qr(
            &mgr.get_entity(1).unwrap().stats,
            &mgr.get_entity(NPC).unwrap().stats,
            true,
        );
        let seed = pseudo_random_seed(1, ABILITY, seq);
        if combat::calculate_result(qr, seed).result_code == RC_HIT {
            let (_, lost) = fire(DART_DEFAULT, seq).await;
            if lost > 0 {
                return seq;
            }
        }
    }
    panic!("no plain hit in 200 rolls");
}

/// Default darts: no on-hit effect, no instance, no slow.
#[tokio::test]
async fn default_darts_run_no_on_hit_effect() {
    let seq = hit_seq().await;
    let (mgr, _) = fire(DART_DEFAULT, seq).await;
    assert!(mgr.get_entity(NPC).unwrap().active_effects.is_empty());
    assert_eq!(npc_speed(&mgr), 100);
}

/// A Poison dart chips 4 on the hit, on top of the same shot with default
/// darts, and leaves a DoT that chips 4 more on each pulse. Fails if the
/// on-hit effect is not dispatched or not registered.
#[tokio::test]
async fn poison_dart_chips_on_the_hit_and_on_each_pulse() {
    let seq = hit_seq().await;
    let (_, base) = fire(DART_DEFAULT, seq).await;
    let (mut mgr, poisoned) = fire(DART_POISON, seq).await;
    assert_eq!(poisoned - base, 4, "the hit's chip");
    assert_eq!(instance_pulses(&mgr, POISON_EFFECT), Some(4));

    let before = npc_health(&mgr);
    pulse(&mut mgr).await;
    assert_eq!(before - npc_health(&mgr), 4, "one pulse's chip");
    assert_eq!(instance_pulses(&mgr, POISON_EFFECT), Some(3));
}

/// A Disease dart is the weaker, longer DoT: 2 on the hit, 9 pulses left.
#[tokio::test]
async fn disease_dart_is_a_weaker_longer_dot() {
    let seq = hit_seq().await;
    let (_, base) = fire(DART_DEFAULT, seq).await;
    let (mut mgr, diseased) = fire(DART_DISEASE, seq).await;
    assert_eq!(diseased - base, 2);
    assert_eq!(instance_pulses(&mgr, DISEASE_EFFECT), Some(9));
    let before = npc_health(&mgr);
    pulse(&mut mgr).await;
    assert_eq!(before - npc_health(&mgr), 2);
}

/// A Tranquilizer dart slows the target to 60% speed for 6 s as one
/// timed-ledger entry (ability mechanics AB-09b); a second hit from the
/// same shooter refreshes it without slowing further; the stat-buff tick's
/// expiry restores the speed exactly. Fails if the on-hit slow is not
/// dispatched, stacks per hit, or is never restored.
#[tokio::test]
async fn tranquilizer_dart_slows_until_the_effect_expires() {
    let seq = hit_seq().await;
    let (mut mgr, ability) = setup(DART_TRANQUILIZER);
    shoot(&mut mgr, &ability, seq).await;
    assert_eq!(npc_speed(&mgr), 60);
    assert_eq!(instance_pulses(&mgr, TRANQ_EFFECT), None, "a ledger entry");
    assert_eq!(mgr.get_entity(NPC).unwrap().stat_buffs.entries.len(), 1);

    shoot(&mut mgr, &ability, seq).await;
    assert_eq!(npc_speed(&mgr), 60, "a refresh must not slow again");

    let (tx, _rx) = mpsc::channel(256);
    let later = Instant::now() + Duration::from_secs(7);
    let expired = crate::cell::effects::stat_buff_tick_at(later, &tx, &mut mgr).await;
    assert_eq!(expired, 1, "expired");
    assert_eq!(npc_speed(&mgr), 100, "expiry restores the speed");
}
