//! SS-D3: the end paths, one at a time (audit § 6, CAT-M-14 and CAT-M-15).
//!
//! A (challenger, entity 10) and B (target, entity 20) duel; C (entity 30)
//! is a bystander with both in AoI. The arena centre is (2.5, 0, 0).

use crate::cell::duel::DuelResources;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_common::Vector3;
use cimmeria_entity::stats::HEALTH;
use cimmeria_wire::cell::client_methods::duel::{
    build_pvp_flag, TEXT_DUEL_ABORTED, TEXT_DUEL_FORFEITED, TEXT_DUEL_LOST, TEXT_DUEL_LOST_RANGE,
    TEXT_DUEL_LOST_TELEPORT, TEXT_DUEL_OUT_OF_RANGE, TEXT_DUEL_WON, TEXT_FORFEIT_NOT_ENGAGED,
};

use super::engage::{aoi_mgr, engage, to_witnesses, DUEL_CLEAR, PVP_FLAG};
use super::*;
use crate::cell::duel::limits::{COUNTDOWN, RANGE_GRACE};
use crate::cell::duel::tick::run_at;
use crate::cell::duel::DuelState;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};

const A: (u32, i32) = (A_EID, A_PID);
const B: (u32, i32) = (B_EID, B_PID);

/// Like [`drain`], but a disconnect's `LeftAoI` and other non-duel traffic
/// are dropped instead of failing the test.
pub(super) fn drain_duel(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<Sent> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => out.push(Sent {
                entity_id,
                method_index,
                args,
                witness: None,
            }),
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                args,
                ..
            } => out.push(Sent {
                entity_id,
                method_index,
                args,
                witness: Some(witness_id),
            }),
            _ => {}
        }
    }
    out
}

/// The single `duel.ended` row.
pub(super) fn ended_row(capture: &LogCaptureGuard) -> Captured {
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "duel.ended"))
        .collect();
    assert_eq!(rows.len(), 1, "exactly one duel.ended: {rows:?}");
    rows.into_iter().next().unwrap()
}

/// Assert a decided end: `loser` lost for `reason` (`defeat_reason` is the
/// client's `EDuelDefeatReason` value), the other duelist won.
fn assert_decided(row: &Captured, reason: &str, value: u8, loser: i32, winner: i32) {
    for (k, v) in [
        ("reason", reason.to_string()),
        ("outcome", "decided".to_string()),
        ("defeat_reason", value.to_string()),
        ("loser_player_id", loser.to_string()),
        ("winner_player_id", winner.to_string()),
        ("player_id", A_PID.to_string()),
        ("target_player_id", B_PID.to_string()),
        ("account_id", "500".to_string()),
        ("target_account_id", "600".to_string()),
    ] {
        assert!(row.has_field(k, &v), "duel.ended {k}={v}: {row:?}");
    }
}

/// **CAT-M-14, type 12.** `duelForfeit` acts only on the caller's own
/// engaged duel. With no duel, with a challenge waiting, during the
/// countdown, and from a bystander while two others duel, the caller gets
/// 880, a `duel.forfeit_refused` row with `reason = not_engaged` and the
/// stage, and nothing changes.
#[tokio::test]
async fn forfeit_rejected_when_not_engaged() {
    let capture = LogCapture::install();
    let mut mgr = aoi_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    let forfeit = crate::cell::duel::forfeit::handle;

    // No duel at all.
    forfeit(A_EID, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_FORFEIT_NOT_ENGAGED]);
    assert_eq!(sent.len(), 1, "only the 880 line: {sent:?}");

    // A challenge waiting for B's answer: neither side can forfeit it.
    let t0 = Instant::now();
    challenge(&mut mgr, &tx, A, B, t0).await;
    drain(&mut rx);
    forfeit(B_EID, &tx, &mut mgr).await;
    assert_eq!(
        lines_to(&drain(&mut rx), B_EID),
        vec![TEXT_FORFEIT_NOT_ENGAGED]
    );
    assert!(
        mgr.resources.duels().pending_for(B_PID).is_some(),
        "the challenge survives"
    );

    // The countdown.
    crate::cell::duel::response::handle_at(B_EID, &[1], &tx, &mut mgr, t0).await;
    drain(&mut rx);
    forfeit(A_EID, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_FORFEIT_NOT_ENGAGED]);
    assert!(lines_to(&sent, B_EID).is_empty(), "B hears nothing");
    assert!(matches!(
        mgr.resources.duels().duel_of(A_PID).unwrap().state,
        DuelState::StartPending { .. }
    ));

    // Engaged, but the caller is the bystander.
    run_at(&tx, &mut mgr, t0 + COUNTDOWN).await;
    drain(&mut rx);
    forfeit(C_EID, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, C_EID), vec![TEXT_FORFEIT_NOT_ENGAGED]);
    assert_eq!(sent.len(), 1);
    assert!(
        mgr.resources.duels().can_harm(A_PID, B_PID),
        "the duel is untouched"
    );

    let refused: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "duel.forfeit_refused"))
        .collect();
    let stages: Vec<&str> = ["none", "challenge", "countdown", "none"].to_vec();
    assert_eq!(refused.len(), stages.len(), "{refused:?}");
    for (row, stage) in refused.iter().zip(stages) {
        assert!(row.has_field("reason", "not_engaged"), "{row:?}");
        assert!(row.has_field("stage", stage), "stage {stage}: {row:?}");
    }
    assert!(refused[0].has_field("account_id", "500") && refused[0].has_field("player_id", "1000"));
    assert!(capture
        .all()
        .iter()
        .all(|c| !c.has_field("event", "duel.ended")));
}

/// Forfeit while engaged: the caller loses (`EDUEL_DEFEAT_Forfeit` = 7),
/// the partner hears 879 and the caller "You forfeited the duel."; both
/// flags go to 0 and the registry is empty.
#[tokio::test]
async fn forfeit_ends_the_duel_with_the_caller_as_loser() {
    let capture = LogCapture::install();
    let mut mgr = aoi_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    engage(&mut mgr, &tx, &mut rx).await;
    drain(&mut rx);

    crate::cell::duel::forfeit::handle(B_EID, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_WON]);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_FORFEITED]);
    for me in [A_EID, B_EID] {
        assert_eq!(own(&sent, me, PVP_FLAG), vec![build_pvp_flag(false)]);
        assert_eq!(own(&sent, me, DUEL_CLEAR).len(), 1);
    }
    assert!(!mgr.resources.duels().is_busy(A_PID) && !mgr.resources.duels().is_busy(B_PID));
    assert_decided(&ended_row(&capture), "forfeit", 7, B_PID, A_PID);
}

/// **CAT-M-15.** A duelist who disconnects loses at once
/// (`EDUEL_DEFEAT_Connection` = 3), inside `disconnect_entity` and before
/// the entity goes: the partner's flag goes to 0 for them and the
/// bystander, the disconnecting duelist's flag goes to 0 for everyone who
/// still sees them, both get 153, the partner leaves combat and hears 879.
/// The one leaving gets no loser line (their client is going).
#[tokio::test]
async fn disconnect_ends_duel_and_clears_both_flags() {
    let capture = LogCapture::install();
    let mut mgr = aoi_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    engage(&mut mgr, &tx, &mut rx).await;
    drain(&mut rx);

    mgr.disconnect_entity(B_EID, &tx).await;
    let sent = drain_duel(&mut rx);
    let off = build_pvp_flag(false);
    assert_eq!(own(&sent, A_EID, PVP_FLAG), vec![off.clone()]);
    assert_eq!(own(&sent, B_EID, PVP_FLAG), vec![off.clone()]);
    assert_eq!(
        to_witnesses(&sent, A_EID, PVP_FLAG),
        vec![(B_EID, off.clone()), (C_EID, off.clone())]
    );
    assert_eq!(
        to_witnesses(&sent, B_EID, PVP_FLAG),
        vec![(A_EID, off.clone()), (C_EID, off.clone())],
        "B's flag is cleared for its witnesses before it leaves"
    );
    assert_eq!(own(&sent, A_EID, DUEL_CLEAR).len(), 1);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_WON]);
    assert!(
        lines_to(&sent, B_EID).is_empty(),
        "no line to a leaving client"
    );
    assert!(mgr.get_entity(A_EID).unwrap().threatened_mobs.is_empty());
    assert!(!mgr.resources.duels().is_busy(A_PID) && !mgr.resources.duels().is_busy(B_PID));
    let row = ended_row(&capture);
    assert_decided(&row, "connection", 3, B_PID, A_PID);
    assert!(row.has_field("cleared", "true") && row.has_field("target_cleared", "true"));
}

/// **CAT-M-15.** A teleport or gate travel (`duel::on_travel`, called by
/// every travel site) ends the duel with the traveller as the loser
/// (`EDUEL_DEFEAT_Teleport` = 5).
#[tokio::test]
async fn teleport_ends_duel() {
    let capture = LogCapture::install();
    let mut mgr = aoi_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    engage(&mut mgr, &tx, &mut rx).await;
    drain(&mut rx);

    crate::cell::duel::on_travel(&tx, &mut mgr, A_EID).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_LOST_TELEPORT]);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_WON]);
    for me in [A_EID, B_EID] {
        assert_eq!(own(&sent, me, PVP_FLAG), vec![build_pvp_flag(false)]);
        assert_eq!(own(&sent, me, DUEL_CLEAR).len(), 1);
    }
    assert!(!mgr.resources.duels().is_busy(A_PID));
    assert_decided(&ended_row(&capture), "teleport", 5, A_PID, B_PID);

    // A bystander's travel touches nobody's duel.
    crate::cell::duel::on_travel(&tx, &mut mgr, C_EID).await;
    assert!(drain(&mut rx).is_empty());
}

/// A space change by a path with no hook: the sweep sees the duelist in
/// the world but not in the duel's space and ends it as a teleport.
#[tokio::test]
async fn space_change_without_the_hook_ends_as_teleport() {
    let capture = LogCapture::install();
    let mut mgr = aoi_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    let engaged_at = engage(&mut mgr, &tx, &mut rx).await;
    drain(&mut rx);

    mgr.destroy_entity(A_EID);
    add_player(&mut mgr, 11, A_PID, 500, "Harset", [0.0, 0.0, 0.0]);
    run_at(&tx, &mut mgr, engaged_at + Duration::from_millis(100)).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, 11), vec![TEXT_DUEL_LOST_TELEPORT]);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_WON]);
    assert!(
        own(&sent, 11, PVP_FLAG).is_empty(),
        "the new entity was never flagged"
    );
    assert_decided(&ended_row(&capture), "teleport", 5, A_PID, B_PID);
}

/// **CAT-M-15, D-SS19.** Outside the 40-unit arena for 5 s loses
/// (`EDUEL_DEFEAT_Range` = 4). Leaving warns once; coming back stops the
/// clock, so a second trip out starts it again.
#[tokio::test]
async fn range_ends_duel() {
    let capture = LogCapture::install();
    let mut mgr = aoi_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    let t0 = engage(&mut mgr, &tx, &mut rx).await;
    drain(&mut rx);
    let place = |mgr: &mut SpaceManager, x: f32| {
        mgr.get_entity_mut(B_EID).unwrap().position = Vector3::new(x, 0.0, 0.0);
    };

    // 42 units from the centre: inside is 40.
    place(&mut mgr, 44.5);
    run_at(&tx, &mut mgr, t0).await;
    assert_eq!(
        lines_to(&drain(&mut rx), B_EID),
        vec![TEXT_DUEL_OUT_OF_RANGE]
    );
    run_at(&tx, &mut mgr, t0 + Duration::from_secs(3)).await;
    assert!(drain(&mut rx).is_empty(), "one warning, no end yet");

    // Back in at 3 s, out again at 4 s: the clock restarts at 4 s.
    place(&mut mgr, 30.0);
    run_at(&tx, &mut mgr, t0 + Duration::from_secs(3)).await;
    place(&mut mgr, 44.5);
    run_at(&tx, &mut mgr, t0 + Duration::from_secs(4)).await;
    drain(&mut rx);
    run_at(
        &tx,
        &mut mgr,
        t0 + Duration::from_secs(4) + RANGE_GRACE - Duration::from_millis(1),
    )
    .await;
    assert!(
        drain(&mut rx).is_empty(),
        "the restarted clock has not run out"
    );
    assert!(mgr.resources.duels().can_harm(A_PID, B_PID));

    run_at(&tx, &mut mgr, t0 + Duration::from_secs(4) + RANGE_GRACE).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_LOST_RANGE]);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_WON]);
    assert!(!mgr.resources.duels().is_busy(B_PID));
    assert_decided(&ended_row(&capture), "range", 4, B_PID, A_PID);
    let out = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "duel.out_of_range"))
        .count();
    let back = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "duel.back_in_range"))
        .count();
    assert_eq!((out, back), (2, 1));
}

/// Death from anyone but the partner (`duel::on_death`, called by the death
/// resolver) is a normal death and a lost duel (`EDUEL_DEFEAT_Health` = 1);
/// a duelist at 0 HP the death path never reported (an NPC DoT kills no
/// player) is caught by the sweep.
#[tokio::test]
async fn third_party_death_loses_the_duel() {
    for via_sweep in [false, true] {
        let capture = LogCapture::install();
        let mut mgr = aoi_mgr();
        let (tx, mut rx) = mpsc::channel(256);
        let t0 = engage(&mut mgr, &tx, &mut rx).await;
        drain(&mut rx);

        let hp = mgr
            .get_entity_mut(B_EID)
            .unwrap()
            .stats
            .get_mut(HEALTH)
            .unwrap();
        hp.update(hp.min, 0, hp.max);
        if via_sweep {
            run_at(&tx, &mut mgr, t0).await;
        } else {
            crate::cell::duel::on_death(&tx, &mut mgr, B_EID, C_EID).await;
        }
        let sent = drain(&mut rx);
        assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_LOST], "{via_sweep}");
        assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_WON], "{via_sweep}");
        assert_decided(&ended_row(&capture), "health", 1, B_PID, A_PID);
        let hp = mgr
            .get_entity(B_EID)
            .unwrap()
            .stats
            .get(HEALTH)
            .unwrap()
            .cur;
        assert_eq!(hp, 0, "a third-party death is not clamped");
    }
}

/// The response window and the countdown end on the same leave paths: a
/// challenge whose target disconnects, and a countdown whose challenger
/// travels, are withdrawn at once and the other side hears 878. No
/// cooldown starts (nobody declined).
#[tokio::test]
async fn leaving_withdraws_a_challenge_and_a_countdown() {
    let capture = LogCapture::install();
    let mut mgr = aoi_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    let t0 = Instant::now();

    challenge(&mut mgr, &tx, A, B, t0).await;
    drain(&mut rx);
    mgr.disconnect_entity(B_EID, &tx).await;
    let sent = drain_duel(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_ABORTED]);
    assert!(lines_to(&sent, B_EID).is_empty());
    assert!(!mgr.resources.duels().is_busy(A_PID) && !mgr.resources.duels().is_busy(B_PID));
    assert_eq!(mgr.resources.duels().cooldown_count(), 0);

    add_player(&mut mgr, B_EID, B_PID, 600, "Agnos", [5.0, 0.0, 0.0]);
    challenge(&mut mgr, &tx, A, B, t0).await;
    crate::cell::duel::response::handle_at(B_EID, &[1], &tx, &mut mgr, t0).await;
    drain(&mut rx);
    crate::cell::duel::on_travel(&tx, &mut mgr, A_EID).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_ABORTED]);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_ABORTED]);
    assert!(
        mgr.resources.duels().is_idle(),
        "no duel, no challenge, no cooldown"
    );
    run_at(&tx, &mut mgr, t0 + COUNTDOWN).await;
    assert!(drain(&mut rx).is_empty(), "the countdown never engages");

    let stages: Vec<String> = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "duel.withdrawn"))
        .map(|c| {
            let stage = ["challenge", "countdown"]
                .into_iter()
                .find(|s| c.has_field("stage", s))
                .unwrap_or("?");
            let reason = ["connection", "teleport"]
                .into_iter()
                .find(|s| c.has_field("reason", s))
                .unwrap_or("?");
            format!("{stage}/{reason}")
        })
        .collect();
    assert_eq!(stages, vec!["challenge/connection", "countdown/teleport"]);
}
