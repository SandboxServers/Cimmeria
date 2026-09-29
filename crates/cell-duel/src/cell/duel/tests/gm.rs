//! The GM abort and status read behind `.duel_end` / `.duel_status` (SS-U2).

use crate::cell::duel::DuelResources;
use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_common::Vector3;
use cimmeria_wire::cell::client_methods::duel::TEXT_DUEL_ABORTED;

use super::*;
use crate::cell::duel::gm::{gm_end, online_name, status, DuelStatus};
use crate::cell::duel::response::handle_at as respond;
use crate::cell::duel::{DuelRegistry, GmAborted};
use crate::test_support::LogCapture;

const A: (u32, i32) = (A_EID, A_PID);
const B: (u32, i32) = (B_EID, B_PID);

/// A duel in the countdown clears for both players, and no pair cooldown
/// starts: the pair may challenge again at once.
#[test]
fn gm_abort_ends_a_duel_for_both_without_a_cooldown() {
    let mut r = DuelRegistry::default();
    let t0 = Instant::now();
    let p = r.open_challenge(1, 2, t0).unwrap();
    r.take_pending_for(2, t0).unwrap();
    r.start_duel(&p, 1, Vector3::new(0.0, 0.0, 0.0), t0);

    let aborted = r.gm_abort(2).expect("the target's duel ends");
    assert!(matches!(aborted, GmAborted::Duel(_)));
    assert_eq!(aborted.players(), (1, 2));
    assert_eq!(aborted.stage(), "countdown");
    assert!(!r.is_busy(1) && !r.is_busy(2));
    assert_eq!(r.cooldown_count(), 0, "a GM abort is not a decline");
    assert!(r.open_challenge(1, 2, t0).is_ok());
}

/// A pending challenge clears from either end: named by its target or by
/// its challenger.
#[test]
fn gm_abort_withdraws_a_challenge_from_either_side() {
    let t0 = Instant::now();
    for named in [1, 2] {
        let mut r = DuelRegistry::default();
        let p = r.open_challenge(1, 2, t0).unwrap();
        assert_eq!(r.gm_abort(named), Some(GmAborted::Challenge(p)));
        assert!(r.is_idle(), "nothing left after aborting via {named}");
    }
    let mut r = DuelRegistry::default();
    assert_eq!(r.gm_abort(1), None, "nothing to end");
}

/// `status` names the one entry a player has, from their side.
#[test]
fn status_reports_each_side() {
    let t0 = Instant::now();
    let mut r = DuelRegistry::default();
    assert_eq!(status(&r, 1), DuelStatus::Idle);
    let p = r.open_challenge(1, 2, t0).unwrap();
    assert_eq!(status(&r, 1), DuelStatus::Challenging(p));
    assert_eq!(status(&r, 2), DuelStatus::Challenged(p));
    assert_eq!(status(&r, 3), DuelStatus::Idle);
    r.take_pending_for(2, t0).unwrap();
    let d = r.start_duel(&p, 1, Vector3::new(0.0, 0.0, 0.0), t0);
    assert_eq!(status(&r, 1), DuelStatus::InDuel(d));
    assert_eq!(status(&r, 2), DuelStatus::InDuel(d));
}

/// End to end on the cell: an accepted duel, then `gm_end` named by the
/// challenger. Both players get "Duel aborted", both entries are gone, and
/// the `duel.gm_ended` row carries the GM's ids and the subject.
#[tokio::test]
async fn gm_end_tells_both_and_clears_the_duel() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(32);
    let t0 = Instant::now();
    challenge(&mut mgr, &tx, A, B, t0).await;
    respond(B_EID, &[1], &tx, &mut mgr, t0).await;
    drain(&mut rx);
    assert!(mgr.resources.duels().duel_of(A_PID).is_some());

    // C (entity 30, account 700) acts as the GM.
    let aborted = gm_end(&tx, &mut mgr, C_EID, A_PID).await.unwrap();
    assert_eq!(aborted.players(), (A_PID, B_PID));

    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert_eq!(sent.len(), 2);
    assert!(
        mgr.resources.duels().duel_of(A_PID).is_none()
            && mgr.resources.duels().duel_of(B_PID).is_none()
    );
    assert!(!mgr.resources.duels().is_busy(A_PID) && !mgr.resources.duels().is_busy(B_PID));

    let row = capture
        .all()
        .into_iter()
        .find(|c| c.target == "duel" && c.has_field("event", "duel.gm_ended"))
        .expect("duel.gm_ended row");
    assert!(row.has_field("account_id", "700"));
    assert!(row.has_field("player_id", &C_PID.to_string()));
    assert!(row.has_field("entity_id", &C_EID.to_string()));
    assert!(row.has_field("subject_player_id", &A_PID.to_string()));
    assert!(row.has_field("opponent_player_id", &B_PID.to_string()));
    assert!(row.has_field("stage", "countdown"));
}

/// Nothing to end: `None`, and nothing is sent.
#[tokio::test]
async fn gm_end_with_nothing_to_end_sends_nothing() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(8);
    assert!(gm_end(&tx, &mut mgr, C_EID, A_PID).await.is_none());
    assert!(drain(&mut rx).is_empty());
}

#[test]
fn online_name_resolves_a_connected_player() {
    let mut mgr = make_mgr();
    mgr.get_entity_mut(A_EID).unwrap().character_name = Some("Ana".into());
    assert_eq!(online_name(&mgr, A_PID).as_deref(), Some("Ana"));
    assert_eq!(online_name(&mgr, 424_242), None);
}
