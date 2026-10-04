//! AB-10: absorb shields in the damage seams. A shield on the timed effect
//! ledger takes a hit's Focus and Health damage first (NVP damage, damage
//! scripts and DoT pulses), the hit settles the shield's pool, an emptied
//! shield comes off with its icon clear, and the expiry takes what is left.

use std::time::{Duration, Instant};

use super::tests::{drain, make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::cell::client_methods::being::ON_TIMER_UPDATE;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::NoContentEvents;
use cimmeria_entity::abilities::{EffectDef, EF_DONT_USE_QR};
use cimmeria_entity::cell_entity::{ActiveEffectInstance, TimedEffectSpec, TimedStacking};
use cimmeria_entity::stats::{ABSORB_PHYSICAL, FOCUS};

const PLAYER: u32 = 1;
const NPC: u32 = 2;
const SHIELD_EFFECT: i32 = 4306;
const ATTACK: i32 = 7101;
const ATTACK_EFFECT: i32 = 7102;

/// The player at full pools under a physical shield of `capacity`.
fn shielded(capacity: i32) -> SpaceManager {
    let mut mgr = make_mgr_player_vs_npc();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.stats.get_mut(HEALTH).unwrap().update(0, 500, 500);
    p.stats.get_mut(FOCUS).unwrap().update(0, 1000, 1000);
    mgr.apply_timed_effect(
        PLAYER,
        TimedEffectSpec {
            effect_id: SHIELD_EFFECT,
            ability_id: 1013,
            invoker_id: PLAYER,
            effect_flags: 342,
            moniker_ids: vec![],
            stats: vec![],
            absorb: vec![(ABSORB_PHYSICAL, capacity)],
            duration_secs: Some(30.0),
            stacking: TimedStacking::PerSource,
            invoker_identity: Default::default(),
        },
        Instant::now(),
    )
    .expect("the shield goes on");
    mgr
}

/// The NPC's attack: one effect, never misses, `script` optional.
fn attack(mgr: &mut SpaceManager, script: Option<&str>, focus: i32, health: i32) -> AbilityDef {
    let ability = make_ability(ATTACK, vec![ATTACK_EFFECT]);
    mgr.ability_defs.insert(ATTACK, ability.clone());
    mgr.effect_defs.insert(
        ATTACK_EFFECT,
        EffectDef {
            effect_id: ATTACK_EFFECT,
            ability_id: ATTACK,
            flags: EF_DONT_USE_QR,
            script_name: script.map(str::to_string),
            params: [
                ("FocusDamage".to_string(), focus.to_string()),
                ("HealthDamage".to_string(), health.to_string()),
            ]
            .into(),
            ..Default::default()
        },
    );
    ability
}

async fn hit(mgr: &mut SpaceManager, ability: AbilityDef) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    apply_damage_to_target(NPC, PLAYER, ATTACK, &Some(ability), 1, false, &tx, mgr).await;
    drain(&mut rx)
}

fn pools(mgr: &SpaceManager) -> (i32, i32, i32) {
    let p = mgr.get_entity(PLAYER).unwrap();
    let cur = |id| p.stats.get(id).unwrap().cur;
    (cur(FOCUS), cur(HEALTH), cur(ABSORB_PHYSICAL))
}

fn shield_left(mgr: &SpaceManager) -> Option<i32> {
    mgr.get_entity(PLAYER)
        .unwrap()
        .stat_buffs
        .entries
        .iter()
        .find(|b| b.effect_id == SHIELD_EFFECT)
        .map(|b| b.absorb_remaining())
}

/// **Regression guard: a shield drains, then expires to 0.** A 1000-point
/// shield takes the whole NVP hit (Focus and Health stay full), its ledger
/// pool goes down by exactly what the stat lost, and the expiry takes the
/// rest of the stat back to 0. Fails on revert of: Focus passing the shield
/// (Focus drops), the hit's settle (the pool stays at 1000 while the stat
/// fell), or the ledger's release on expiry (the stat keeps the remainder).
#[tokio::test]
async fn a_shield_drains_then_expires_to_zero() {
    let mut mgr = shielded(1000);
    let ability = attack(&mut mgr, None, 100, 10);
    hit(&mut mgr, ability).await;

    let (focus, health, stat) = pools(&mgr);
    assert_eq!(
        (focus, health),
        (1000, 500),
        "the shield took the whole hit"
    );
    assert!(stat < 1000, "the hit drained the absorb stat ({stat})");
    assert_eq!(
        shield_left(&mgr),
        Some(stat),
        "the ledger pool was charged exactly what the stat lost"
    );

    let (tx, _rx) = mpsc::channel(256);
    let later = Instant::now() + Duration::from_secs(31);
    let expired = crate::cell::effects::stat_buffs::stat_buff_tick_at(later, &tx, &mut mgr).await;
    assert_eq!(expired, 1);
    assert_eq!(
        pools(&mgr).2,
        0,
        "the expiry takes the unspent capacity back"
    );
    assert_eq!(shield_left(&mgr), None);
}

/// **Regression guard: a drained shield comes off.** A 20-point shield is
/// emptied by the hit: the ledger entry comes off in the same resolution
/// and the player's client is sent the zero `onTimerUpdate` for its icon.
/// On revert of the settle the entry stays (with 20 "left" that no longer
/// exists) and no clear is sent.
#[tokio::test]
async fn a_drained_shield_comes_off_with_its_icon_clear() {
    let mut mgr = shielded(20);
    let ability = attack(&mut mgr, None, 100, 10);
    let msgs = hit(&mut mgr, ability).await;

    assert_eq!(
        shield_left(&mgr),
        None,
        "the empty shield is off the ledger"
    );
    assert_eq!(pools(&mgr).2, 0);
    let clear = msgs.iter().any(|m| {
        matches!(m, CellToBaseMsg::EntityMethodCall { entity_id, method_index, args }
            if *entity_id == PLAYER
                && *method_index == ON_TIMER_UPDATE
                && args[..4] == SHIELD_EFFECT.to_le_bytes()
                && args[13..21] == [0; 8])
    });
    assert!(clear, "the shield's icon is cleared: {msgs:?}");
    assert!(pools(&mgr).0 < 1000, "the overflow reached Focus");
}

/// **Regression guard: a damage script passes the shield.** Pistol Shot's
/// shape (`RangedPhysicalDamage`, 150 Focus / 15 Health) against a 1000
/// shield: Focus and Health stay full and the pool pays 165. On revert of
/// the script absorb the script writes Focus directly.
#[tokio::test]
async fn a_damage_script_drains_the_shield_first() {
    let mut mgr = shielded(1000);
    let ability = attack(&mut mgr, Some("RangedPhysicalDamage"), 150, 15);
    hit(&mut mgr, ability).await;
    assert_eq!(pools(&mgr), (1000, 500, 835));
    assert_eq!(shield_left(&mgr), Some(835));
}

/// **Regression guard: a DoT pulse passes the shield.** A scripted DoT
/// pulse (`RangedPhysicalDamage`, 50 Focus / 5 Health) lands on the shield
/// and the pulse settles it. On revert of the pulse's absorb Focus drops;
/// on revert of its settle the pool stays at 1000.
#[tokio::test]
async fn a_dot_pulse_drains_the_shield_first() {
    let mut mgr = shielded(1000);
    attack(&mut mgr, Some("RangedPhysicalDamage"), 50, 5);
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .active_effects
        .push(ActiveEffectInstance {
            effect_id: ATTACK_EFFECT,
            ability_id: ATTACK,
            invoker_id: NPC,
            remaining_pulses: 3,
            total_pulses: 4,
            next_pulse_at: Instant::now() - Duration::from_secs(1),
            pulse_interval_secs: 1.0,
            invoker_position_at_register: None,
        });
    let (tx, _rx) = mpsc::channel(256);
    crate::cell::effects::effect_pulse_tick(&NoContentEvents, &tx, &mut mgr).await;
    assert_eq!(pools(&mgr), (1000, 500, 945));
    assert_eq!(shield_left(&mgr), Some(945));
}
