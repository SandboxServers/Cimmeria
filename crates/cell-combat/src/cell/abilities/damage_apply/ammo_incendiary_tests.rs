//! Incendiary rounds through `apply_damage_to_target` and the pulse tick
//! (ammo campaign AM-08): a hit ignites the target, the burn ticks on, and
//! it stacks by the effects ADR's decision 4 (same shooter refreshes, a
//! second shooter adds their own).
//!
//! The row and the burn come from
//! `cimmeria_cell_world::cell::effects::ammo_incendiary`, whose live-DB guard
//! pins them to the seed. Like `ammo_tests`, these turn
//! `ammo.finite_special` on for the process and never off.

use std::time::Instant;

use super::tests::{make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::cell::effects::effect_pulse_tick;
use crate::cell::spawner::AmmoCatalog;
use crate::test_support::NoContentEvents;
use cimmeria_cell_effect_scripts::cell::effects::ammo_incendiary::{
    incendiary_burn_effect, incendiary_modifier, INCENDIARY_BURN_EFFECT,
    INCENDIARY_BURN_FOCUS_DAMAGE, INCENDIARY_BURN_HEALTH_DAMAGE, INCENDIARY_BURN_PULSES,
};
use cimmeria_entity::abilities::{EffectDef, RC_MISS};
use cimmeria_entity::ammo_type::{BULLET_DEFAULT, BULLET_INCENDIARY};
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::FOCUS;

const ABILITY: i32 = 7;
const EFFECT: i32 = 100;
const NPC: u32 = 2;
const SECOND_SHOOTER: u32 = 3;
const FULL: i32 = 100_000;

/// Player 1 (and player 3) carrying a pistol loaded with `ammo_type`, NPC 2
/// at full pools, ability 7 a one-round shot of 1000 HealthDamage, and the
/// Incendiary row and burn loaded.
fn world(ammo_type: i32) -> SpaceManager {
    cimmeria_entity::ammo_feature::set_finite_special(true);
    let mut mgr = make_mgr_player_vs_npc();
    let mut ability = make_ability(ABILITY, vec![EFFECT]);
    ability.required_ammo = 1;
    ability.is_ranged = true;
    mgr.ability_defs.insert(ABILITY, ability);
    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), "1000".to_string());
    mgr.effect_defs.insert(
        EFFECT,
        EffectDef {
            effect_id: EFFECT,
            params,
            ..Default::default()
        },
    );
    mgr.effect_defs
        .insert(INCENDIARY_BURN_EFFECT, incendiary_burn_effect());
    mgr.ammo_catalog = AmmoCatalog::from_rows([incendiary_modifier()], [(BULLET_INCENDIARY, 9002)]);
    mgr.create_entity(SECOND_SHOOTER, "Castle", [0.0; 3], [0.0; 3])
        .unwrap();
    for (shooter, player_id) in [(1, 100), (SECOND_SHOOTER, 101)] {
        let p = mgr.get_entity_mut(shooter).unwrap();
        p.is_player = true;
        p.player_id = Some(player_id);
        p.active_bandolier_slot = 0;
        p.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: shooter as i32,
                item_id: 3241,
                clip_size: 15,
                default_ammo_type: BULLET_DEFAULT,
                current_ammo: 15,
                cur_ammo_type: ammo_type,
            },
        );
    }
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.stats.get_mut(HEALTH).unwrap().update(0, FULL, FULL);
    npc.stats.get_mut(FOCUS).unwrap().update(0, FULL, FULL);
    mgr
}

async fn shoot(mgr: &mut SpaceManager, shooter: u32, seq: u32) {
    let ability = mgr.ability_defs.get(&ABILITY).cloned();
    let (tx, _rx) = mpsc::channel(256);
    apply_damage_to_target(shooter, NPC, ABILITY, &ability, seq, false, &tx, mgr).await;
}

fn pool(mgr: &SpaceManager, stat: i32) -> i32 {
    mgr.get_entity(NPC).unwrap().stats.get(stat).unwrap().cur
}

/// `(invoker, remaining pulses)` of every burn on the NPC.
fn burns(mgr: &SpaceManager) -> Vec<(u32, i32)> {
    let mut v: Vec<_> = mgr
        .get_entity(NPC)
        .unwrap()
        .active_effects
        .iter()
        .filter(|i| i.effect_id == INCENDIARY_BURN_EFFECT)
        .map(|i| (i.invoker_id, i.remaining_pulses))
        .collect();
    v.sort_unstable();
    v
}

/// An `effect_seq` whose roll from `shooter` is not a miss.
fn hit_seq(mgr: &SpaceManager, shooter: u32) -> u32 {
    let qr = combat::calculate_qr(
        &mgr.get_entity(shooter).unwrap().stats,
        &mgr.get_entity(NPC).unwrap().stats,
        true,
    );
    (1..500)
        .find(|&seq| {
            let seed = pseudo_random_seed(shooter, ABILITY, seq);
            combat::calculate_result(qr, seed).result_code != RC_MISS
        })
        .expect("a hit in 500 rolls")
}

/// A hit burns once at once (the shot itself deals no Focus damage, so the
/// Focus lost is exactly one burn pulse), registers the other three pulses,
/// and the pulse tick burns both pools again. Fails if the row loses its
/// on-hit effect or the burn stops being a pulsing effect.
#[tokio::test]
async fn an_incendiary_hit_ignites_and_the_burn_ticks() {
    let mut mgr = world(BULLET_INCENDIARY);
    let seq = hit_seq(&mgr, 1);
    shoot(&mut mgr, 1, seq).await;
    assert_eq!(pool(&mgr, FOCUS), FULL - INCENDIARY_BURN_FOCUS_DAMAGE);
    assert_eq!(burns(&mgr), vec![(1, INCENDIARY_BURN_PULSES - 1)]);

    let health = pool(&mgr, HEALTH);
    for inst in &mut mgr.get_entity_mut(NPC).unwrap().active_effects {
        inst.next_pulse_at = Instant::now();
    }
    let (tx, _rx) = mpsc::channel(256);
    effect_pulse_tick(&NoContentEvents, &tx, &mut mgr).await;
    assert_eq!(pool(&mgr, FOCUS), FULL - 2 * INCENDIARY_BURN_FOCUS_DAMAGE);
    assert_eq!(pool(&mgr, HEALTH), health - INCENDIARY_BURN_HEALTH_DAMAGE);
    assert_eq!(burns(&mgr), vec![(1, INCENDIARY_BURN_PULSES - 2)]);
}

/// Decision 4: the same shooter's second hit refreshes the burn (one
/// instance, back to three pulses left), a second shooter's hit adds a burn
/// of their own.
#[tokio::test]
async fn one_shooter_refreshes_the_burn_and_a_second_shooter_stacks() {
    let mut mgr = world(BULLET_INCENDIARY);
    let seq = hit_seq(&mgr, 1);
    shoot(&mut mgr, 1, seq).await;
    mgr.get_entity_mut(NPC).unwrap().active_effects[0].remaining_pulses = 1;
    shoot(&mut mgr, 1, seq).await;
    assert_eq!(burns(&mgr), vec![(1, INCENDIARY_BURN_PULSES - 1)]);

    let seq3 = hit_seq(&mgr, SECOND_SHOOTER);
    shoot(&mut mgr, SECOND_SHOOTER, seq3).await;
    assert_eq!(
        burns(&mgr),
        vec![
            (1, INCENDIARY_BURN_PULSES - 1),
            (SECOND_SHOOTER, INCENDIARY_BURN_PULSES - 1)
        ]
    );
}

/// Default ammo in the same world ignites nothing.
#[tokio::test]
async fn default_ammo_does_not_ignite() {
    let mut mgr = world(BULLET_DEFAULT);
    let seq = hit_seq(&mgr, 1);
    shoot(&mut mgr, 1, seq).await;
    assert!(burns(&mgr).is_empty());
    assert_eq!(pool(&mgr, FOCUS), FULL);
}
