//! AM-11d: support darts (beneficial ammo) land on allies and the shooter,
//! never on a hostile target, and never harm or engage anyone.
//!
//! `duel_gate`'s fixture: player A (1) shoots, player B (2) is an ally at
//! (3,0,0), player C (3) a bystander, NPC 4 a hostile mob at (2,0,0). A's
//! weapon shot ([`SHOT`]) costs one dart and carries 5 HealthDamage; a
//! support row scales that to 0.
//!
//! `ammo.finite_special` is switched on for the process, as every ammo test
//! in this crate does.

use std::collections::HashMap;

use cimmeria_cell_world::cell::duel::DuelResources;

use crate::cell::spawner::{AmmoCatalog, AmmoModifier};
use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::ammo_type::{DART_DEFAULT, DART_STIM};
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::{FOCUS, HEALTH};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::state_field::BSF_IN_COMBAT;

use super::duel_gate::{duel_mgr, engage, A, A_PID, B, B_PID, MOB};
use super::warmup::{after_warmup, cast_ability, WARMUP_ABILITY};
use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::test_support::NoContentEvents;

/// A's instant weapon shot.
const SHOT: i32 = 60;
/// The seeded Stim on-hit effect: `HealFocus`, 10%.
const STIM_EFFECT: i32 = 9160;
const FOCUS_MAX: i32 = 1000;
const CLIP: i32 = 100;
const SUPPORT_DAMAGE_MULT: f32 = 0.0001;

/// `duel_mgr` with A's dart pistol loaded with `ammo_type`, and a Stim row
/// whose `beneficial` flag is `beneficial`. Every entity's Focus is empty.
fn support_mgr(ammo_type: i32, beneficial: bool) -> SpaceManager {
    cimmeria_entity::ammo_feature::set_finite_special(true);
    let mut mgr = duel_mgr();
    let mut shot = cast_ability(SHOT, 0.0);
    shot.required_ammo = 1;
    mgr.ability_defs.insert(SHOT, shot);
    mgr.ability_defs
        .get_mut(&WARMUP_ABILITY)
        .unwrap()
        .required_ammo = 1;
    mgr.effect_defs.insert(
        STIM_EFFECT,
        EffectDef {
            effect_id: STIM_EFFECT,
            ability_id: 992,
            script_name: Some("HealFocus".to_string()),
            params: HashMap::from([("HealPercentage".to_string(), "10".to_string())]),
            ..Default::default()
        },
    );
    mgr.ammo_catalog = AmmoCatalog::from_rows(
        [AmmoModifier {
            ammo_type: DART_STIM,
            damage_mult: SUPPORT_DAMAGE_MULT,
            penetration_mult: 1.0,
            damage_type: None,
            on_hit_effect_id: Some(STIM_EFFECT),
            toggle_ability_id: 992,
            beneficial,
        }],
        [(DART_STIM, 9010)],
    );
    for eid in [A, B, 3, MOB] {
        let f = mgr
            .get_entity_mut(eid)
            .unwrap()
            .stats
            .get_mut(FOCUS)
            .unwrap();
        f.update(0, 0, FOCUS_MAX);
        f.clear_dirty();
    }
    let a = mgr.get_entity_mut(A).unwrap();
    a.account_id = Some(901);
    a.abilities.add_ability(SHOT);
    a.active_bandolier_slot = 0;
    a.bandolier_items.insert(
        0,
        BandolierItem {
            instance_id: 1,
            item_id: 1520,
            clip_size: CLIP,
            default_ammo_type: DART_DEFAULT,
            current_ammo: CLIP,
            cur_ammo_type: ammo_type,
        },
    );
    mgr
}

fn focus(mgr: &SpaceManager, eid: u32) -> i32 {
    mgr.get_entity(eid).unwrap().stats.get(FOCUS).unwrap().cur
}

fn health(mgr: &SpaceManager, eid: u32) -> i32 {
    mgr.get_entity(eid).unwrap().stats.get(HEALTH).unwrap().cur
}

fn ammo(mgr: &SpaceManager) -> i32 {
    mgr.get_entity(A).unwrap().active_ammo()
}

/// Whether A was sent the support refusal line.
fn got_hostile_feedback(msgs: &[CellToBaseMsg]) -> bool {
    let want = serialize_on_player_communication(
        "SYSTEM",
        0,
        CHAN_FEEDBACK,
        "Support rounds only affect allies.",
    );
    msgs.iter().any(|m| {
        matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: A, method_index, args }
            if *method_index == method_idx::ON_PLAYER_COMMUNICATION && *args == want)
    })
}

fn sent(msgs: &[CellToBaseMsg], method: u16) -> bool {
    msgs.iter().any(|m| match m {
        CellToBaseMsg::EntityMethodCall { method_index, .. } => *method_index == method,
        CellToBaseMsg::EntityMethodCallBatch { calls, .. } => {
            calls.iter().any(|(i, _)| *i == method)
        }
        _ => false,
    })
}

/// A Stim dart on an ally restores 10% of their Focus, deals no damage, and
/// costs its dart and its cooldown like any shot.
#[tokio::test]
async fn a_stim_shot_on_an_ally_heals_them() {
    let mut mgr = support_mgr(DART_STIM, true);
    let (tx, mut rx) = mpsc::channel(1024);
    let full = health(&mgr, B);

    assert!(
        handle_use_ability(A, SHOT, B as i32, &tx, &mut mgr).await,
        "a support shot at an ally commits"
    );
    assert_eq!(
        focus(&mgr, B),
        FOCUS_MAX / 10,
        "one shot restores 10% Focus"
    );
    assert_eq!(health(&mgr, B), full, "a support shot deals no damage");
    assert_eq!(ammo(&mgr), CLIP - 1, "the dart is spent");
    assert!(mgr.get_entity(A).unwrap().abilities.is_on_cooldown(SHOT));
    let msgs = drain(&mut rx);
    assert!(
        sent(&msgs, method_idx::ON_STAT_UPDATE),
        "the heal reaches the clients"
    );
    assert!(!got_hostile_feedback(&msgs));
}

/// A support shot at the shooter's own entity heals the shooter.
#[tokio::test]
async fn a_stim_shot_on_self_heals_the_shooter() {
    let mut mgr = support_mgr(DART_STIM, true);
    let (tx, _rx) = mpsc::channel(1024);

    assert!(
        handle_use_ability(A, SHOT, A as i32, &tx, &mut mgr).await,
        "a support shot at yourself commits"
    );
    assert_eq!(focus(&mgr, A), FOCUS_MAX / 10);
    assert_eq!(ammo(&mgr), CLIP - 1);
}

/// A Stim dart at a hostile NPC does nothing: refused at launch with the
/// feedback line, no heal, no damage, no ammo, no cooldown, no threat.
#[tokio::test]
async fn a_stim_shot_on_a_hostile_does_nothing_and_sends_feedback() {
    let mut mgr = support_mgr(DART_STIM, true);
    let (tx, mut rx) = mpsc::channel(1024);
    let full = health(&mgr, MOB);

    assert!(
        !handle_use_ability(A, SHOT, MOB as i32, &tx, &mut mgr).await,
        "a support shot at a hostile is refused"
    );
    assert_eq!(focus(&mgr, MOB), 0, "the hostile is not healed");
    assert_eq!(health(&mgr, MOB), full);
    assert_eq!(ammo(&mgr), CLIP, "a refused shot spends no dart");
    assert!(!mgr.get_entity(A).unwrap().abilities.is_on_cooldown(SHOT));
    assert!(mgr.get_entity(MOB).unwrap().threat_list.is_empty());
    assert!(
        got_hostile_feedback(&drain(&mut rx)),
        "the first press gets the feedback line"
    );
}

/// An engaged duel opponent is hostile too: no heal for them.
#[tokio::test]
async fn a_stim_shot_on_a_duel_opponent_is_refused() {
    let mut mgr = support_mgr(DART_STIM, true);
    engage(&mut mgr);
    let (tx, mut rx) = mpsc::channel(1024);

    assert!(!handle_use_ability(A, SHOT, B as i32, &tx, &mut mgr).await);
    assert_eq!(focus(&mgr, B), 0);
    assert!(got_hostile_feedback(&drain(&mut rx)));
}

/// Default darts at an ally are still refused by the #444 gate, and the
/// hostile-target feedback is not sent (it is the support refusal only).
#[tokio::test]
async fn a_default_dart_on_an_ally_is_still_refused() {
    let mut mgr = support_mgr(DART_DEFAULT, true);
    let (tx, mut rx) = mpsc::channel(1024);
    let full = health(&mgr, B);

    assert!(!handle_use_ability(A, SHOT, B as i32, &tx, &mut mgr).await);
    assert!(!handle_use_ability(A, SHOT, A as i32, &tx, &mut mgr).await);
    assert_eq!(health(&mgr, B), full);
    assert_eq!(ammo(&mgr), CLIP);
    assert!(!got_hostile_feedback(&drain(&mut rx)));
}

/// The ally path keys on the explicit `beneficial` column, not on the
/// on-hit effect's script: the same healing row with the flag off does not
/// widen targeting.
#[tokio::test]
async fn a_healing_row_not_marked_beneficial_cannot_target_an_ally() {
    let mut mgr = support_mgr(DART_STIM, false);
    let (tx, _rx) = mpsc::channel(1024);

    assert!(!handle_use_ability(A, SHOT, B as i32, &tx, &mut mgr).await);
    assert_eq!(focus(&mgr, B), 0);
}

/// A support shot at an ally who is fighting a mob puts nobody in combat:
/// no threat for the shooter on the mob, no in-combat bit, no duel or PvP
/// state, and no `onEffectResults` (the damage pipeline never runs).
#[tokio::test]
async fn a_support_shot_starts_no_threat_combat_or_pvp() {
    let mut mgr = support_mgr(DART_STIM, true);
    mgr.get_entity_mut(MOB)
        .unwrap()
        .threat_list
        .insert(B, 100.0);
    mgr.get_entity_mut(B).unwrap().threatened_mobs.insert(MOB);
    let (tx, mut rx) = mpsc::channel(1024);

    assert!(handle_use_ability(A, SHOT, B as i32, &tx, &mut mgr).await);
    assert_eq!(focus(&mgr, B), FOCUS_MAX / 10);

    let a = mgr.get_entity(A).unwrap();
    assert!(a.threatened_mobs.is_empty(), "the shooter threatens no mob");
    assert_eq!(
        a.state_field & BSF_IN_COMBAT,
        0,
        "the shooter is not in combat"
    );
    let mob = mgr.get_entity(MOB).unwrap();
    assert!(
        !mob.threat_list.contains_key(&A),
        "the mob gains no threat on A"
    );
    assert!(
        mgr.resources.duels().duel_of(A_PID).is_none()
            && mgr.resources.duels().duel_of(B_PID).is_none()
    );
    let msgs = drain(&mut rx);
    assert!(
        !sent(&msgs, method_idx::ON_EFFECT_RESULTS),
        "no hit is resolved"
    );
    assert!(
        !sent(&msgs, method_idx::ON_ENTITY_PROPERTY),
        "no PvP flag is sent"
    );
    assert!(!sent(&msgs, method_idx::ON_STATE_FIELD_UPDATE));
}

/// A warmed-up support shot at an ally survives the fire-time re-check.
#[tokio::test]
async fn a_warmed_up_support_shot_lands_on_the_ally() {
    let mut mgr = support_mgr(DART_STIM, true);
    mgr.get_entity_mut(A)
        .unwrap()
        .abilities
        .add_ability(WARMUP_ABILITY);
    let (tx, _rx) = mpsc::channel(1024);

    assert!(handle_use_ability(A, WARMUP_ABILITY, B as i32, &tx, &mut mgr).await);
    assert_eq!(focus(&mgr, B), 0, "nothing lands before the warmup ends");
    resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    assert_eq!(
        focus(&mgr, B),
        FOCUS_MAX / 10,
        "the warmup fire heals the ally"
    );
}

/// Loading support darts during a warmup aimed at a hostile never heals it:
/// the fire refuses the shot with the feedback line.
#[tokio::test]
async fn a_support_shot_that_turns_hostile_at_fire_is_refused() {
    let mut mgr = support_mgr(DART_DEFAULT, true);
    mgr.get_entity_mut(A)
        .unwrap()
        .abilities
        .add_ability(WARMUP_ABILITY);
    let (tx, mut rx) = mpsc::channel(1024);

    assert!(handle_use_ability(A, WARMUP_ABILITY, MOB as i32, &tx, &mut mgr).await);
    mgr.get_entity_mut(A)
        .unwrap()
        .bandolier_items
        .get_mut(&0)
        .unwrap()
        .cur_ammo_type = DART_STIM;
    drain(&mut rx);
    resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    assert_eq!(focus(&mgr, MOB), 0, "the hostile is never healed");
    assert!(got_hostile_feedback(&drain(&mut rx)));
}

/// Telemetry (TESTING.md type 12): the applied and refused rows on target
/// `ammo` carry the shooter's identity, the target, the ammo type and a
/// `decision_outcome`; the refusal names its `reason`.
#[tokio::test]
async fn support_shots_log_applied_and_refused_rows() {
    use crate::test_support::LogCapture;

    let mut mgr = support_mgr(DART_STIM, true);
    let (tx, _rx) = mpsc::channel(1024);
    let logs = LogCapture::install();
    assert!(handle_use_ability(A, SHOT, B as i32, &tx, &mut mgr).await);
    mgr.get_entity_mut(A)
        .unwrap()
        .abilities
        .clear_all_cooldowns();
    assert!(!handle_use_ability(A, SHOT, MOB as i32, &tx, &mut mgr).await);
    let all = logs.all();

    let find = |event: &str| {
        all.iter()
            .find(|c| c.target == "ammo" && c.has_field("event", event))
            .cloned()
            .unwrap_or_else(|| panic!("no ammo event={event}: {all:#?}"))
    };
    let applied = find("ammo_support_applied");
    let refused = find("ammo_support_refused");
    for (row, target, outcome) in [(&applied, B, "applied"), (&refused, MOB, "refused")] {
        assert_eq!(row.level, tracing::Level::DEBUG, "{row:?}");
        assert!(row.has_field("account_id", "901"), "{row:?}");
        assert!(row.has_field("player_id", &A_PID.to_string()), "{row:?}");
        assert!(row.has_field("entity_id", &A.to_string()), "{row:?}");
        assert!(
            row.has_field("target_entity_id", &target.to_string()),
            "{row:?}"
        );
        assert!(
            row.has_field("ammo_type", &DART_STIM.to_string()),
            "{row:?}"
        );
        assert!(row.has_field("item_id", "9010"), "{row:?}");
        assert!(row.has_field("decision_outcome", outcome), "{row:?}");
    }
    assert!(applied.has_field("target_player_id", &B_PID.to_string()));
    assert!(applied.has_field("target_focus_before", "0"), "{applied:?}");
    assert!(
        applied.has_field("target_focus_after", &(FOCUS_MAX / 10).to_string()),
        "{applied:?}"
    );
    assert!(refused.has_field("reason", "hostile_target"), "{refused:?}");
    assert!(refused.has_field("stage", "launch"), "{refused:?}");
}
