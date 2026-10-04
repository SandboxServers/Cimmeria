//! Ability mechanics AB-09a: a stunned or knocked-down caster cannot cast,
//! and a stun that lands mid-warmup cancels the warmup.
//!
//! `warmup`'s fixture (player 1, hostile NPC 2, warmup ability 50 with the
//! Begin / End / Interrupt sequences) plus a stun ability both can fire:
//! effect 1599 as seeded (`Stun`, 5 s) with `EF_DontUseQR` so the test's
//! hit never rolls a miss.

use std::collections::HashMap;
use std::time::Instant;

use cimmeria_entity::abilities::{
    serialize_timer_update, EffectDef, EF_DONT_USE_QR, TIMER_ABILITY_COOLDOWN, TIMER_ABILITY_WARMUP,
};
use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};
use cimmeria_wire::state_field::BSF_MOVEMENT_LOCK;

use super::super::incapacitated::{INCAPACITATED_ERROR_CODE, INCAPACITATED_TEXT};
use super::warmup::{
    after_warmup, calls, cast_ability, effect_results, sequences, warmup_mgr, INSTANT_ABILITY,
    ON_TIMER_UPDATE, SEQ_INTERRUPT, WARMUP_ABILITY,
};
use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::cell::effects::stat_buff::StatBuffRemoval;
use crate::test_support::NoContentEvents;

const STUN_ABILITY: i32 = 1355;
const STUN_EFFECT: i32 = 1599;

fn stun_mgr() -> SpaceManager {
    let mut mgr = warmup_mgr();
    crate::test_support::install_effect_scripts(&mut mgr);
    mgr.ability_defs.insert(
        STUN_ABILITY,
        AbilityDef {
            effect_ids: vec![STUN_EFFECT],
            event_set_id: None,
            ..cast_ability(STUN_ABILITY, 0.0)
        },
    );
    mgr.effect_defs.insert(
        STUN_EFFECT,
        EffectDef {
            effect_id: STUN_EFFECT,
            ability_id: STUN_ABILITY,
            script_name: Some("Stun".to_string()),
            flags: 64 | EF_DONT_USE_QR,
            pulse_count: 1,
            pulse_duration: 5.0,
            params: HashMap::from([("CcDuration".to_string(), "5".to_string())]),
            ..Default::default()
        },
    );
    for eid in [1, 2] {
        let e = mgr.get_entity_mut(eid).unwrap();
        e.abilities.add_ability(STUN_ABILITY);
        e.abilities.add_ability(WARMUP_ABILITY);
    }
    mgr
}

fn stun(mgr: &mut SpaceManager, eid: u32) {
    let spec = TimedEffectSpec {
        effect_id: STUN_EFFECT,
        ability_id: STUN_ABILITY,
        invoker_id: if eid == 1 { 2 } else { 1 },
        state_flags: BSF_MOVEMENT_LOCK,
        duration_secs: Some(5.0),
        stacking: TimedStacking::PerSource,
        ..Default::default()
    };
    mgr.apply_timed_effect(eid, spec, Instant::now()).unwrap();
}

/// **Regression guard (stunned players could cast).** A stunned player's
/// press is refused with `onErrorCode(0, ability, 15)` and the feedback
/// line, charges no cooldown and fires nothing; once the stun is off the
/// same press fires. Without the gate the cast launches (`refused` fails).
#[tokio::test]
async fn a_stunned_player_cannot_cast() {
    let mut mgr = stun_mgr();
    stun(&mut mgr, 1);
    let (tx, mut rx) = mpsc::channel(512);

    let launched = handle_use_ability(1, INSTANT_ABILITY, 2, &tx, &mut mgr).await;
    assert!(!launched, "refused");
    let msgs = drain(&mut rx);
    assert_eq!(effect_results(&msgs, 1), 0, "nothing fired: {msgs:?}");
    let mut err = vec![0u8];
    err.extend_from_slice(&INSTANT_ABILITY.to_le_bytes());
    err.extend_from_slice(&INCAPACITATED_ERROR_CODE.to_le_bytes());
    let sent = calls(&msgs);
    assert!(
        sent.iter()
            .any(|(e, m, a)| *e == 1 && *m == method_idx::ON_ERROR_CODE && *a == err),
        "onErrorCode: {sent:?}"
    );
    let line = cimmeria_wire::cell::chat::serialize_on_player_communication(
        "SYSTEM",
        0,
        cimmeria_wire::cell::chat::CHAN_FEEDBACK,
        INCAPACITATED_TEXT,
    );
    assert!(sent
        .iter()
        .any(|(e, m, a)| *e == 1 && *m == method_idx::ON_PLAYER_COMMUNICATION && *a == line));
    assert!(!mgr
        .get_entity(1)
        .unwrap()
        .abilities
        .is_on_cooldown(INSTANT_ABILITY));

    let _ = mgr.remove_timed_effects(1, StatBuffRemoval::Expired, |_| true);
    assert!(handle_use_ability(1, INSTANT_ABILITY, 2, &tx, &mut mgr).await);
    assert_eq!(effect_results(&drain(&mut rx), 1), 1, "fires once free");
}

/// A stunned NPC's cast is refused too, silently (its AI holds anyway).
/// Without the gate its stun lands on the player (`silent` fails).
#[tokio::test]
async fn a_stunned_npc_cannot_cast() {
    let mut mgr = stun_mgr();
    stun(&mut mgr, 2);
    let (tx, mut rx) = mpsc::channel(512);
    assert!(!handle_use_ability(2, STUN_ABILITY, 1, &tx, &mut mgr).await);
    assert!(drain(&mut rx).is_empty(), "silent, and nothing fired");
    assert!(!mgr.get_entity(1).unwrap().has_state_flag(BSF_MOVEMENT_LOCK));
    // Free again, the same cast lands.
    let _ = mgr.remove_timed_effects(2, StatBuffRemoval::Expired, |_| true);
    assert!(handle_use_ability(2, STUN_ABILITY, 1, &tx, &mut mgr).await);
    assert!(mgr.get_entity(1).unwrap().has_state_flag(BSF_MOVEMENT_LOCK));
}

/// **Regression guard (a warmup outlived the stun).** The player stuns an
/// NPC in its warmup: the NPC's `Ability_Interrupt` goes out in the same
/// burst and the warmup never fires. Without the incapacitating interrupt
/// the NPC finished the cast (`the NPC's interrupt sequence` fails).
#[tokio::test]
async fn a_stun_cancels_an_npc_warmup() {
    let mut mgr = stun_mgr();
    let (tx, mut rx) = mpsc::channel(512);
    assert!(handle_use_ability(2, WARMUP_ABILITY, 1, &tx, &mut mgr).await);
    let _ = drain(&mut rx);

    assert!(handle_use_ability(1, STUN_ABILITY, 2, &tx, &mut mgr).await);
    let burst = drain(&mut rx);
    assert_eq!(
        sequences(&burst, 2),
        vec![SEQ_INTERRUPT],
        "the NPC's interrupt sequence: {burst:?}"
    );
    assert!(!crate::cell::abilities::is_casting(&mgr, 2));
    resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    assert_eq!(effect_results(&drain(&mut rx), 2), 0, "never fires");
}

/// **Regression guard, the player half.** An NPC stuns the player in its
/// warmup: the player gets the zeroed warmup and cooldown timers and the
/// interrupt sequence, and the cast never fires.
#[tokio::test]
async fn a_stun_cancels_a_player_warmup() {
    let mut mgr = stun_mgr();
    let (tx, mut rx) = mpsc::channel(512);
    assert!(handle_use_ability(1, WARMUP_ABILITY, 2, &tx, &mut mgr).await);
    let _ = drain(&mut rx);

    assert!(handle_use_ability(2, STUN_ABILITY, 1, &tx, &mut mgr).await);
    let burst = drain(&mut rx);
    assert!(!crate::cell::abilities::is_casting(&mgr, 1), "{burst:?}");
    assert_eq!(sequences(&burst, 1), vec![SEQ_INTERRUPT]);
    for timer in [TIMER_ABILITY_WARMUP, TIMER_ABILITY_COOLDOWN] {
        let zeroed = serialize_timer_update(WARMUP_ABILITY, timer, 1, 0, 0.0, 0.0);
        assert!(
            calls(&burst)
                .iter()
                .any(|(e, m, a)| *e == 1 && *m == ON_TIMER_UPDATE && *a == zeroed),
            "zeroed timer type {timer}: {burst:?}"
        );
    }
    resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    assert_eq!(effect_results(&drain(&mut rx), 1), 0, "never fires");
}
