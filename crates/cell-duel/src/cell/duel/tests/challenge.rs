//! The cell-side challenge checks (audit CAT-M-12). Each refusal asserts the
//! challenger's line, that the target is sent nothing, that nothing was
//! stored, and the `duel.challenge_refused` event with its `reason`
//! (TESTING.md type 12).

use crate::cell::duel::DuelResources;
use std::time::Instant;

use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_wire::cell::client_methods::duel::{
    TEXT_ALREADY_IN_DUEL, TEXT_CHALLENGE_SELF, TEXT_CHALLENGE_SENT, TEXT_NOT_CLOSE_ENOUGH,
    TEXT_PAIR_COOLDOWN, TEXT_TARGET_BUSY, TEXT_TARGET_NOT_ONLINE,
};

use super::*;
use crate::cell::duel::limits::PAIR_COOLDOWN;
use crate::test_support::{LogCapture, LogCaptureGuard};

const A: (u32, i32) = (A_EID, A_PID);
const B: (u32, i32) = (B_EID, B_PID);
const C: (u32, i32) = (C_EID, C_PID);

fn refused(capture: &LogCaptureGuard, reason: &str) -> bool {
    capture.all().iter().any(|c| {
        c.level == Level::DEBUG
            && c.target == "duel"
            && c.has_field("event", "duel.challenge_refused")
            && c.has_field("reason", reason)
    })
}

/// Assert a refusal: exactly one line, to the challenger; nothing stored.
fn assert_refused(sent: &[Sent], mgr: &SpaceManager, text: &str) {
    assert_eq!(sent.len(), 1, "one line and nothing else: {sent:?}");
    assert_eq!(lines_to(sent, A_EID), vec![text.to_string()]);
    assert!(
        mgr.resources.duels().pending_for(B_PID).is_none(),
        "a refused challenge must not be stored"
    );
}

/// The happy path: the target gets `onDuelChallenge` [143] with the
/// challenger's entity id and an empty squad list, byte for byte; the
/// challenger gets its acknowledgement; the challenge is stored.
#[tokio::test]
async fn challenge_prompts_the_target_byte_exact() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let t0 = Instant::now();
    challenge(&mut mgr, &tx, A, B, t0).await;
    let sent = drain(&mut rx);
    assert_eq!(
        sent[0],
        Sent {
            entity_id: B_EID,
            method_index: 143,
            args: vec![A_EID as u8, 0, 0, 0, 0, 0, 0, 0],
            witness: None,
        }
    );
    assert_eq!(
        lines_to(&sent, A_EID),
        vec![TEXT_CHALLENGE_SENT.to_string()]
    );
    assert_eq!(sent.len(), 2);
    let p = mgr.resources.duels().pending_for(B_PID).expect("stored");
    assert_eq!(p.challenger, A_PID);
    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "duel.challenge_sent"))
        .expect("duel.challenge_sent event");
    assert!(ev.has_field("player_id", &A_PID.to_string()));
    assert!(ev.has_field("target_player_id", &B_PID.to_string()));
    assert!(ev.has_field("duel_id", &p.duel_id.to_string()));
}

#[tokio::test]
async fn challenge_rejects_self() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    challenge(&mut mgr, &tx, A, A, Instant::now()).await;
    let sent = drain(&mut rx);
    assert_eq!(sent.len(), 1);
    assert_eq!(
        lines_to(&sent, A_EID),
        vec![TEXT_CHALLENGE_SELF.to_string()]
    );
    assert!(!mgr.resources.duels().is_busy(A_PID));
    assert!(refused(&capture, "self_challenge"));
}

/// The target is online but in another space: text 877, nothing stored.
#[tokio::test]
async fn challenge_rejects_cross_space() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    // Move B to Harset at the same coordinates.
    mgr.destroy_entity(B_EID);
    add_player(&mut mgr, B_EID, B_PID, 600, "Harset", [5.0, 0.0, 0.0]);
    let (tx, mut rx) = mpsc::channel(16);
    challenge(&mut mgr, &tx, A, B, Instant::now()).await;
    assert_refused(&drain(&mut rx), &mgr, TEXT_NOT_CLOSE_ENOUGH);
    assert!(refused(&capture, "cross_space"));
}

/// D-SS19: 20 units. At 20 the challenge goes out; just past it, text 877.
#[tokio::test]
async fn challenge_rejects_out_of_range() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    mgr.get_entity_mut(B_EID).unwrap().position.x = 20.5;
    let (tx, mut rx) = mpsc::channel(16);
    challenge(&mut mgr, &tx, A, B, Instant::now()).await;
    assert_refused(&drain(&mut rx), &mgr, TEXT_NOT_CLOSE_ENOUGH);
    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("reason", "out_of_range"))
        .expect("out_of_range event");
    assert!(ev.has_field("distance", "20.5"));

    mgr.get_entity_mut(B_EID).unwrap().position.x = 20.0;
    challenge(&mut mgr, &tx, A, B, Instant::now()).await;
    assert!(
        mgr.resources.duels().pending_for(B_PID).is_some(),
        "exactly at range is allowed"
    );
}

/// The target already has a challenge to answer (from C): the challenger
/// is refused with the target-busy line, and C's challenge is untouched.
#[tokio::test]
async fn challenge_rejects_when_target_busy() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let t0 = Instant::now();
    challenge(&mut mgr, &tx, C, B, t0).await;
    let c_challenge = *mgr.resources.duels().pending_for(B_PID).unwrap();
    drain(&mut rx);

    challenge(&mut mgr, &tx, A, B, t0).await;
    let sent = drain(&mut rx);
    assert_eq!(sent.len(), 1);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_TARGET_BUSY.to_string()]);
    assert_eq!(mgr.resources.duels().pending_for(B_PID), Some(&c_challenge));
    assert!(refused(&capture, "target_busy"));
}

/// The challenger already has a challenge out: text 873.
#[tokio::test]
async fn challenge_rejects_when_challenger_busy() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let t0 = Instant::now();
    challenge(&mut mgr, &tx, A, C, t0).await;
    drain(&mut rx);
    challenge(&mut mgr, &tx, A, B, t0).await;
    assert_refused(&drain(&mut rx), &mgr, TEXT_ALREADY_IN_DUEL);
    assert!(refused(&capture, "challenger_busy"));
}

/// D-SS21: after B declines, A cannot challenge B again for 60 s.
#[tokio::test]
async fn challenge_rejects_during_pair_cooldown() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let t0 = Instant::now();
    challenge(&mut mgr, &tx, A, B, t0).await;
    crate::cell::duel::response::handle_at(B_EID, &[0], &tx, &mut mgr, t0).await;
    drain(&mut rx);

    challenge(&mut mgr, &tx, A, B, t0 + PAIR_COOLDOWN / 2).await;
    assert_refused(&drain(&mut rx), &mgr, TEXT_PAIR_COOLDOWN);
    assert!(refused(&capture, "pair_cooldown"));

    challenge(&mut mgr, &tx, A, B, t0 + PAIR_COOLDOWN).await;
    assert!(mgr.resources.duels().pending_for(B_PID).is_some());
}

/// The base resolved B, but B's entity is gone (or recycled to another
/// player) by the time the cell handles it: "not online", nothing stored.
#[tokio::test]
async fn challenge_rejects_a_target_entity_that_no_longer_matches() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    mgr.get_entity_mut(B_EID).unwrap().player_id = Some(9999);
    let (tx, mut rx) = mpsc::channel(16);
    challenge(&mut mgr, &tx, A, B, Instant::now()).await;
    assert_refused(&drain(&mut rx), &mgr, TEXT_TARGET_NOT_ONLINE);
    assert!(refused(&capture, "target_gone"));
}
