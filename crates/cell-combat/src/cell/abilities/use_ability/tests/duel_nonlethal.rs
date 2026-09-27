//! SS-D3, D-SS20: duel-partner damage never kills; anyone else's does.
//!
//! - A partner's lethal hit (the ability path) leaves the duelist at 1 HP,
//!   ends the duel with them as the loser, and runs nothing of the death
//!   path: no corpse, no loot, no XP, no Defeat Window.
//! - A partner's lethal DoT pulse (the effect path, which never re-checks
//!   hostility) does the same, and a second partner DoT due in the same
//!   tick does not fire once the first has ended the duel.
//! - A third party's kill is a normal death and loses the duel.

use std::time::Instant;

use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::HEALTH;
use cimmeria_wire::cell::client_methods::player::ON_BEGIN_AID_WAIT;
use cimmeria_wire::state_field::BSF_DEAD;

use super::duel_end::{dot, dueling_world, make_due, pid};
use super::duel_gate::{duel_mgr, engage, health, A, A_PID, B, B_PID, MOB};
use super::warmup::INSTANT_ABILITY;
use super::*;
use crate::cell::effects::{effect_pulse_tick, register_active_effect};
use crate::test_support::{LogCapture, NoContentEvents};

/// Set `eid`'s HEALTH to `cur` (of 100 000), clean.
fn set_health(mgr: &mut SpaceManager, eid: u32, cur: i32) {
    let hp = mgr
        .get_entity_mut(eid)
        .unwrap()
        .stats
        .get_mut(HEALTH)
        .unwrap();
    hp.update(0, cur, 100_000);
    hp.clear_dirty();
}

/// Every HEALTH value `onStatUpdate` told anyone about `eid`.
fn health_sent(msgs: &[CellToBaseMsg], eid: u32) -> Vec<i32> {
    let mut out = Vec::new();
    for (entity, method, args) in super::warmup::calls(msgs) {
        if entity != eid || method != method_idx::ON_STAT_UPDATE || args.len() < 4 {
            continue;
        }
        let n = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
        for i in 0..n {
            let at = 4 + i * 16;
            let id = i32::from_le_bytes(args[at..at + 4].try_into().unwrap());
            if id == HEALTH {
                out.push(i32::from_le_bytes(
                    args[at + 8..at + 12].try_into().unwrap(),
                ));
            }
        }
    }
    out
}

/// No part of the death path ran for `eid`: no `BSF_Dead`, no contact-list
/// Death fanout, no kill XP, no Defeat Window, no "Target killed!" row.
fn assert_no_death(
    mgr: &SpaceManager,
    msgs: &[CellToBaseMsg],
    logs: &LogCaptureGuardRef,
    eid: u32,
) {
    let e = mgr.get_entity(eid).unwrap();
    assert_eq!(e.state_field & BSF_DEAD, 0, "{eid} is not a corpse");
    assert!(
        !msgs.iter().any(|m| matches!(
            m,
            CellToBaseMsg::ContactListPresenceEvent { .. } | CellToBaseMsg::GrantXP { .. }
        )),
        "no death fanout or XP: {msgs:?}"
    );
    assert!(
        super::warmup::calls(msgs)
            .iter()
            .all(|(_, m, _)| *m != ON_BEGIN_AID_WAIT),
        "no Defeat Window"
    );
    assert!(
        logs.all()
            .iter()
            .all(|c| !c.message_contains("Target killed!")),
        "resolve_death never ran"
    );
}

type LogCaptureGuardRef = crate::test_support::LogCaptureGuard;

/// The one `duel.ended` row: decided on health, `loser` lost.
fn assert_health_end(logs: &LogCaptureGuardRef, loser: i32) {
    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "duel.ended"))
        .collect();
    assert_eq!(rows.len(), 1, "{rows:?}");
    for (k, v) in [
        ("reason", "health".to_string()),
        ("defeat_reason", "1".to_string()),
        ("loser_player_id", loser.to_string()),
    ] {
        assert!(
            rows[0].has_field(k, &v),
            "duel.ended {k}={v}: {:?}",
            rows[0]
        );
    }
}

/// **D-SS20, ability path.** A's hit would take B from 3 HP to below 0.
/// B is left at 1 (and every `onStatUpdate` says 1, never 0), the duel
/// ends with B the loser, and the DoT the same ability registered on B is
/// stripped by the end.
#[tokio::test]
async fn lethal_partner_hit_clamps_to_one_hp() {
    let logs = LogCapture::install();
    let mut mgr = duel_mgr();
    let effect = mgr.effect_defs.get_mut(&500).unwrap();
    effect.pulse_count = 3;
    effect.pulse_duration = 3.0;
    engage(&mut mgr);
    set_health(&mut mgr, B, 3);
    let (tx, mut rx) = mpsc::channel(1024);

    assert!(handle_use_ability(A, INSTANT_ABILITY, B as i32, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    assert_eq!(health(&mgr, B), 1, "the partner's hit is held at 1 HP");
    let sent = health_sent(&msgs, B);
    assert!(
        !sent.is_empty() && sent.iter().all(|&h| h == 1),
        "HEALTH sent: {sent:?}"
    );
    assert!(!mgr.duels.is_busy(A_PID) && !mgr.duels.is_busy(B_PID));
    assert!(
        mgr.get_entity(B).unwrap().active_effects.is_empty(),
        "the DoT this hit registered is stripped by the end"
    );
    assert_health_end(&logs, B_PID);
    let clamp = logs
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "duel.lethal_clamped"))
        .expect("duel.lethal_clamped row");
    for (k, v) in [
        ("source", "ability"),
        ("player_id", "101"),
        ("target_player_id", "102"),
        ("health_after", "1"),
    ] {
        assert!(clamp.has_field(k, v), "lethal_clamped {k}={v}: {clamp:?}");
    }
}

/// **D-SS20, D-SS22.** Nothing of the death path follows a clamped end
/// (no corpse, loot, XP, death fanout or Defeat Window), and the duel is
/// over: the pair can no longer harm each other and B left combat with A.
#[tokio::test]
async fn no_loot_xp_or_corpse_after_a_clamped_end() {
    let logs = LogCapture::install();
    let mut mgr = duel_mgr();
    engage(&mut mgr);
    set_health(&mut mgr, B, 3);
    let (tx, mut rx) = mpsc::channel(1024);

    assert!(handle_use_ability(A, INSTANT_ABILITY, B as i32, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    assert_no_death(&mgr, &msgs, &logs, B);
    assert_eq!(health(&mgr, B), 1);
    assert!(
        !mgr.duels.can_harm(A_PID, B_PID),
        "the ended duel no longer lets A hit B"
    );
    let b = mgr.get_entity(B).unwrap();
    assert!(
        !b.threatened_mobs.contains(&A),
        "B left combat with A at the end"
    );
}

/// **D-SS20, effect path.** Two of A's DoTs are due on B in the same tick
/// and the first would take B below 0. It is held at 1 HP and ends the
/// duel; the end strips both DoTs, so the second (still in the tick's
/// snapshot) does not fire. A bystander's DoT on B is not A's and stays.
#[tokio::test]
async fn lethal_partner_bleed_clamps_to_one_hp() {
    let logs = LogCapture::install();
    let mut mgr = dueling_world();
    let (first, second, other): (EffectDef, EffectDef, EffectDef) =
        (dot(7101), dot(7102), dot(7103));
    for d in [&first, &second, &other] {
        mgr.effect_defs.insert(d.effect_id, d.clone());
    }
    let (tx, mut rx) = mpsc::channel(4096);
    let now = Instant::now();
    register_active_effect(&mut mgr, B, A, &first, now, &tx).await;
    register_active_effect(&mut mgr, B, A, &second, now, &tx).await;
    set_health(&mut mgr, B, 5);
    make_due(&mut mgr, B);
    drain(&mut rx);

    effect_pulse_tick(&NoContentEvents, &tx, &mut mgr).await;
    let msgs = drain(&mut rx);
    assert_eq!(health(&mgr, B), 1, "the partner's pulse is held at 1 HP");
    assert!(health_sent(&msgs, B).iter().all(|&h| h == 1));
    assert!(mgr.get_entity(B).unwrap().active_effects.is_empty());
    assert_no_death(&mgr, &msgs, &logs, B);
    assert_health_end(&logs, pid(&mgr, B));
    let pulses = logs
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "effect_pulse_fired"))
        .count();
    assert_eq!(
        pulses, 1,
        "the second partner DoT did not fire after the end"
    );

    // A bystander's DoT registered after the end still lands: only the
    // partner's harm is stripped, and nothing is clamped any more.
    register_active_effect(&mut mgr, B, 3, &other, now, &tx).await;
    make_due(&mut mgr, B);
    effect_pulse_tick(&NoContentEvents, &tx, &mut mgr).await;
    assert!(health(&mgr, B) <= 0, "no clamp outside a duel");
}

/// **D-SS20.** A third party's kill is a normal death: the corpse, the
/// death fanout and the Defeat Window all happen, and the duel is lost on
/// health with the partner the winner.
#[tokio::test]
async fn third_party_kill_is_normal_death() {
    let logs = LogCapture::install();
    let mut mgr = duel_mgr();
    engage(&mut mgr);
    set_health(&mut mgr, B, 0);
    let (tx, mut rx) = mpsc::channel(1024);

    assert!(crate::cell::abilities::resolve_death_for_test(B, MOB, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    assert_ne!(mgr.get_entity(B).unwrap().state_field & BSF_DEAD, 0);
    assert!(msgs
        .iter()
        .any(|m| matches!(m, CellToBaseMsg::ContactListPresenceEvent { .. })));
    assert!(!mgr.duels.is_busy(A_PID) && !mgr.duels.is_busy(B_PID));
    assert_health_end(&logs, B_PID);
    assert_eq!(health(&mgr, B), 0, "not clamped");
}
