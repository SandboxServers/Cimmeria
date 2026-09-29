//! `sendDuelResponse` (CM 102, audit CAT-M-13) and the accept.

use crate::cell::duel::DuelResources;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_common::Vector3;
use cimmeria_wire::cell::client_methods::duel::{
    TEXT_DUEL_ABORTED, TEXT_DUEL_ACCEPTED, TEXT_NO_PENDING_CHALLENGE,
};

use super::*;
use crate::cell::duel::limits::{CHALLENGE_TIMEOUT, COUNTDOWN};
use crate::cell::duel::response::handle_at as respond;
use crate::cell::duel::DuelState;
use crate::test_support::{LogCapture, LogCaptureGuard};

const A: (u32, i32) = (A_EID, A_PID);
const B: (u32, i32) = (B_EID, B_PID);

fn response_refused(capture: &LogCaptureGuard, level: Level, reason: &str) -> bool {
    capture.all().iter().any(|c| {
        c.level == level
            && c.target == "duel"
            && c.has_field("event", "duel.response_refused")
            && c.has_field("reason", reason)
    })
}

/// A challenge A -> B at `t0`, with the channel drained.
async fn pending_a_to_b(
    mgr: &mut SpaceManager,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    t0: Instant,
) {
    challenge(mgr, tx, A, B, t0).await;
    drain(rx);
    assert!(mgr.resources.duels().pending_for(B_PID).is_some());
}

/// No challenge addressed to the caller: refused with a line, nothing
/// changes. Covers the challenger answering their own challenge and a
/// bystander answering someone else's.
#[tokio::test]
async fn response_without_challenge_rejected() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let t0 = Instant::now();

    respond(B_EID, &[1], &tx, &mut mgr, t0).await;
    let sent = drain(&mut rx);
    assert_eq!(
        lines_to(&sent, B_EID),
        vec![TEXT_NO_PENDING_CHALLENGE.to_string()]
    );
    assert_eq!(sent.len(), 1);
    assert!(response_refused(
        &capture,
        Level::DEBUG,
        "no_pending_challenge"
    ));

    // A challenge to B cannot be accepted by A (the challenger) or C.
    pending_a_to_b(&mut mgr, &tx, &mut rx, t0).await;
    respond(A_EID, &[1], &tx, &mut mgr, t0).await;
    respond(C_EID, &[1], &tx, &mut mgr, t0).await;
    let sent = drain(&mut rx);
    assert_eq!(
        lines_to(&sent, A_EID),
        vec![TEXT_NO_PENDING_CHALLENGE.to_string()]
    );
    assert_eq!(
        lines_to(&sent, C_EID),
        vec![TEXT_NO_PENDING_CHALLENGE.to_string()]
    );
    assert!(
        mgr.resources.duels().pending_for(B_PID).is_some(),
        "someone else's answer must not consume B's challenge"
    );
    assert!(mgr.resources.duels().duel_of(A_PID).is_none());
}

/// The first answer consumes the challenge; a replayed accept finds
/// nothing and changes nothing.
#[tokio::test]
async fn response_replay_rejected() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let t0 = Instant::now();
    pending_a_to_b(&mut mgr, &tx, &mut rx, t0).await;

    respond(B_EID, &[0], &tx, &mut mgr, t0).await;
    drain(&mut rx);
    respond(B_EID, &[1], &tx, &mut mgr, t0).await;
    let sent = drain(&mut rx);
    assert_eq!(
        lines_to(&sent, B_EID),
        vec![TEXT_NO_PENDING_CHALLENGE.to_string()]
    );
    assert!(sent.iter().all(|s| s.entity_id == B_EID));
    assert!(
        mgr.resources.duels().duel_of(B_PID).is_none(),
        "a replayed accept must not start a duel"
    );
    assert!(response_refused(
        &capture,
        Level::DEBUG,
        "no_pending_challenge"
    ));
}

/// D-SS18: an accept at 30 s is too late even if the sweep has not run.
/// Both sides get 878, no duel starts, and the pair cooldown begins.
#[tokio::test]
async fn response_after_expiry_rejected() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let t0 = Instant::now();
    pending_a_to_b(&mut mgr, &tx, &mut rx, t0).await;

    respond(
        B_EID,
        &[1],
        &tx,
        &mut mgr,
        t0 + CHALLENGE_TIMEOUT - Duration::from_millis(1),
    )
    .await;
    assert!(
        mgr.resources.duels().duel_of(B_PID).is_some(),
        "just inside the window is accepted"
    );

    let mut mgr = make_mgr();
    pending_a_to_b(&mut mgr, &tx, &mut rx, t0).await;
    respond(B_EID, &[1], &tx, &mut mgr, t0 + CHALLENGE_TIMEOUT).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert!(mgr.resources.duels().duel_of(B_PID).is_none());
    assert!(!mgr.resources.duels().is_busy(B_PID));
    assert!(response_refused(&capture, Level::DEBUG, "expired"));
}

/// Decline tells both sides 878 and frees both.
#[tokio::test]
async fn decline_tells_both_sides() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let t0 = Instant::now();
    pending_a_to_b(&mut mgr, &tx, &mut rx, t0).await;
    respond(B_EID, &[0], &tx, &mut mgr, t0).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert_eq!(sent.len(), 2);
    assert!(!mgr.resources.duels().is_busy(A_PID) && !mgr.resources.duels().is_busy(B_PID));
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "duel.declined")
            && c.has_field("target_player_id", &A_PID.to_string())));
}

/// Accept puts both in one `StartPending` duel centred between them, tells
/// both, and starts both clients' countdown: `onTimerUpdate` type 14
/// (DuelTimer) on each player's own entity, 5 s long, expiring 5 s from
/// now on the game clock. Nothing else: no PvP flag, no `onDuelEntities*`,
/// and no harm, until the engage.
#[tokio::test]
async fn accept_starts_the_countdown_for_both() {
    let _capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let t0 = Instant::now();
    pending_a_to_b(&mut mgr, &tx, &mut rx, t0).await;
    let before = cimmeria_wire::mercury::game_clock::game_time_secs();
    respond(B_EID, &[1], &tx, &mut mgr, t0).await;
    let after = cimmeria_wire::mercury::game_clock::game_time_secs();
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_ACCEPTED.to_string()]);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_ACCEPTED.to_string()]);
    for eid in [A_EID, B_EID] {
        let timers = own(&sent, eid, 12);
        assert_eq!(timers.len(), 1, "one countdown to {eid}: {sent:?}");
        let t = &timers[0];
        assert_eq!(t.len(), 21);
        assert_eq!(t[4], 14, "Type = DuelTimer");
        assert_eq!(
            &t[5..9],
            &(eid as i32).to_le_bytes(),
            "SourceID = own entity"
        );
        assert_eq!(&t[13..17], &5.0f32.to_le_bytes(), "TotalTime");
        let complete = f32::from_le_bytes(t[17..21].try_into().unwrap());
        assert!(
            (before + 5.0..=after + 5.0).contains(&complete),
            "BigWorldTimeComplete {complete} is not game time + 5"
        );
    }
    assert_eq!(sent.len(), 4, "two lines and two countdowns: {sent:?}");
    let duel = *mgr.resources.duels().duel_of(A_PID).expect("A in the duel");
    assert_eq!(
        mgr.resources.duels().duel_of(B_PID).map(|d| d.duel_id),
        Some(duel.duel_id)
    );
    assert_eq!(
        duel.state,
        DuelState::StartPending {
            engage_at: t0 + COUNTDOWN
        }
    );
    assert_eq!(duel.centre, Vector3::new(2.5, 0.0, 0.0));
    assert!(!mgr.resources.duels().can_harm(A_PID, B_PID));
}

/// A malformed answer (not 0 or 1) is logged at WARN, not answered, and
/// leaves the challenge for a well-formed answer.
#[tokio::test]
async fn malformed_response_does_not_consume_the_challenge() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let t0 = Instant::now();
    pending_a_to_b(&mut mgr, &tx, &mut rx, t0).await;
    respond(B_EID, &[2], &tx, &mut mgr, t0).await;
    respond(B_EID, &[], &tx, &mut mgr, t0).await;
    assert!(drain(&mut rx).is_empty());
    assert!(mgr.resources.duels().pending_for(B_PID).is_some());
    let ev = capture
        .find_event(Level::WARN, "did not decode", "unknown_response")
        .expect("malformed row");
    assert!(
        ev.has_field("target_player_id", &A_PID.to_string()),
        "the malformed row names the challenger: {ev:?}"
    );
    assert!(capture
        .find_event(Level::WARN, "did not decode", "bad_length")
        .is_some());
}

/// The challenger logged off while the prompt was up: the accept starts no
/// duel and the responder is told 878.
#[tokio::test]
async fn accept_after_the_challenger_left_aborts() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let t0 = Instant::now();
    pending_a_to_b(&mut mgr, &tx, &mut rx, t0).await;
    mgr.destroy_entity(A_EID);
    respond(B_EID, &[1], &tx, &mut mgr, t0).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert_eq!(sent.len(), 1);
    assert!(mgr.resources.duels().duel_of(B_PID).is_none());
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "duel.accept_refused")
            && c.has_field("reason", "challenger_gone")));
    let skipped = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "duel.notify_skipped"))
        .expect("the gone challenger's line is skipped");
    assert!(skipped.has_field("player_id", &A_PID.to_string()));
    assert!(
        skipped.has_field("target_player_id", &B_PID.to_string()),
        "notify_skipped names the other duelist: {skipped:?}"
    );
}
