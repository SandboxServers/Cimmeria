//! AB-N2 regression guards for GM god mode (`gmSetGodMode`, 142): every
//! damage seam leaves a god-mode target's Health and Focus where they were.
//!
//! Each test runs the same hit twice, once without the flag (the damage
//! lands, proving the fixture deals damage) and once with it (nothing
//! lands). Reverting either `GodModeGuard::restore` call site, in
//! `apply_hit` or in `fire_pulse`, fails the god-mode half.

use std::time::{Duration, Instant};

use super::tests::{drain, make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx;
use crate::test_support::{LogCapture, NoContentEvents};
use cimmeria_entity::abilities::{EffectDef, EF_DONT_USE_QR, SRC_ABSORB, SRC_MORTAL};
use cimmeria_entity::cell_entity::ActiveEffectInstance;
use cimmeria_entity::stats::FOCUS;

const PLAYER: u32 = 1;
const NPC: u32 = 2;
const ATTACK: i32 = 7401;
const ATTACK_EFFECT: i32 = 7402;

/// The NPC attacks the player (500 Health, 1000 Focus) with one effect that
/// never misses; `script` optional. `god_mode` sets the player's flag.
fn fixture(god_mode: bool, script: Option<&str>, focus: i32, health: i32) -> SpaceManager {
    let mut mgr = make_mgr_player_vs_npc();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.god_mode = god_mode;
    p.stats.get_mut(HEALTH).unwrap().update(0, 500, 500);
    p.stats.get_mut(FOCUS).unwrap().update(0, 1000, 1000);
    mgr.ability_defs
        .insert(ATTACK, make_ability(ATTACK, vec![ATTACK_EFFECT]));
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
    mgr
}

async fn hit(mgr: &mut SpaceManager) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    let ability = mgr.ability_defs.get(&ATTACK).cloned();
    apply_damage_to_target(NPC, PLAYER, ATTACK, &ability, 1, false, &tx, mgr).await;
    drain(&mut rx)
}

async fn pulse(mgr: &mut SpaceManager) {
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
    crate::cell::effects::effect_pulse_tick(&NoContentEvents, &tx, mgr).await;
}

/// `(Focus, Health)` of the player.
fn pools(mgr: &SpaceManager) -> (i32, i32) {
    let p = mgr.get_entity(PLAYER).unwrap();
    (
        p.stats.get(FOCUS).unwrap().cur,
        p.stats.get(HEALTH).unwrap().cur,
    )
}

fn absorbed_seams(capture: &crate::test_support::LogCaptureGuard) -> Vec<String> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "god_mode_absorbed"))
        .filter_map(|c| c.fields.get("seam").cloned())
        .collect()
}

/// **Guard: NVP hit.** 100 Focus / 40 Health lands on an ordinary player
/// and on a god-mode one nothing moves. The god-mode hit's
/// `onEffectResults` carries a 0 with `SRC_ABSORB`, never a loss.
#[tokio::test]
async fn an_nvp_hit_lands_nothing_on_a_god_mode_target() {
    let mut plain = fixture(false, None, 100, 40);
    hit(&mut plain).await;
    let (focus, health) = pools(&plain);
    assert!(focus < 1000 && health < 500, "the fixture deals damage");

    let mut god = fixture(true, None, 100, 40);
    let capture = LogCapture::install();
    let msgs = hit(&mut god).await;
    assert_eq!(pools(&god), (1000, 500), "god mode: nothing lands");
    assert_eq!(absorbed_seams(&capture), vec!["ability_hit".to_string()]);

    let args = msgs
        .iter()
        .find_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } if *method_index == method_idx::ON_EFFECT_RESULTS => Some(args.clone()),
            _ => None,
        })
        .expect("the hit still sends onEffectResults");
    // The trailing HEALTH entry: stat i8, delta i32, damage code i8, SRC i8.
    let entry = &args[args.len() - 7..];
    assert_eq!(entry[0], HEALTH as u8);
    assert_eq!(i32::from_le_bytes(entry[1..5].try_into().unwrap()), 0);
    assert_eq!(entry[6] as i8, SRC_ABSORB);
}

/// **Guard: a lethal hit does not kill.** 900 Health against 500: the
/// ordinary player dies, the god-mode one stays at 500 and is not marked
/// mortal.
#[tokio::test]
async fn a_lethal_hit_does_not_kill_a_god_mode_target() {
    let mut plain = fixture(false, None, 0, 900);
    hit(&mut plain).await;
    assert_eq!(pools(&plain).1, 0, "the fixture is lethal");

    let mut god = fixture(true, None, 0, 900);
    let msgs = hit(&mut god).await;
    assert_eq!(pools(&god).1, 500);
    let mortal = msgs.iter().any(|m| match m {
        CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } if *method_index == method_idx::ON_EFFECT_RESULTS => {
            args.last().is_some_and(|&b| b as i8 == SRC_MORTAL)
        }
        _ => false,
    });
    assert!(!mortal, "no SRC_MORTAL entry for a god-mode target");
}

/// **Guard: damage script.** `RangedPhysicalDamage` (150 Focus / 15
/// Health) writes the pools itself; god mode puts them back.
#[tokio::test]
async fn a_damage_script_lands_nothing_on_a_god_mode_target() {
    let mut plain = fixture(false, Some("RangedPhysicalDamage"), 150, 15);
    hit(&mut plain).await;
    assert_ne!(pools(&plain), (1000, 500), "the fixture deals damage");

    let mut god = fixture(true, Some("RangedPhysicalDamage"), 150, 15);
    hit(&mut god).await;
    assert_eq!(pools(&god), (1000, 500));
}

/// **Guard: after-hit script.** `Suppression` chips 20 Health after the
/// hit; the second restore (seam `ability_script`) puts it back.
#[tokio::test]
async fn an_after_hit_script_lands_nothing_on_a_god_mode_target() {
    let mut plain = fixture(false, Some("Suppression"), 0, 20);
    hit(&mut plain).await;
    assert!(pools(&plain).1 < 500, "the fixture deals damage");

    let mut god = fixture(true, Some("Suppression"), 0, 20);
    let capture = LogCapture::install();
    hit(&mut god).await;
    assert_eq!(pools(&god), (1000, 500));
    assert!(absorbed_seams(&capture).contains(&"ability_script".to_string()));
}

/// **Guard: NVP DoT pulse.** 50 Focus / 5 Health per pulse.
#[tokio::test]
async fn an_nvp_dot_pulse_lands_nothing_on_a_god_mode_target() {
    let mut plain = fixture(false, None, 50, 5);
    pulse(&mut plain).await;
    assert_ne!(pools(&plain), (1000, 500), "the fixture deals damage");

    let mut god = fixture(true, None, 50, 5);
    let capture = LogCapture::install();
    pulse(&mut god).await;
    assert_eq!(pools(&god), (1000, 500));
    assert_eq!(absorbed_seams(&capture), vec!["effect_pulse".to_string()]);
}

/// **Guard: scripted DoT pulse.** The same pulse through
/// `RangedPhysicalDamage`.
#[tokio::test]
async fn a_scripted_dot_pulse_lands_nothing_on_a_god_mode_target() {
    let mut plain = fixture(false, Some("RangedPhysicalDamage"), 50, 5);
    pulse(&mut plain).await;
    assert_ne!(pools(&plain), (1000, 500), "the fixture deals damage");

    let mut god = fixture(true, Some("RangedPhysicalDamage"), 50, 5);
    pulse(&mut god).await;
    assert_eq!(pools(&god), (1000, 500));
}
