//! SS-D2: the engage, the PvP-flag fan-out, the safety ends and the AoI
//! replay.
//!
//! A (challenger) and B (target) duel; C is a bystander who has both in AoI
//! from the start; D arrives mid-duel. `archetype_id` is set so the players
//! are introducible and the AoI tick builds real witness sets.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_common::Vector3;
use cimmeria_wire::cell::client_methods::duel::{
    build_on_duel_entities_set, build_pvp_flag, TEXT_DUEL_ABORTED, TEXT_DUEL_ENGAGED,
};
use cimmeria_wire::state_field::BSF_IN_COMBAT;

use super::*;
use crate::cell::duel::limits::{COUNTDOWN, ENGAGED_LIMIT};
use crate::cell::duel::tick::run_at;
use crate::cell::duel::DuelState;
use crate::test_support::LogCapture;

const A: (u32, i32) = (A_EID, A_PID);
const B: (u32, i32) = (B_EID, B_PID);
/// A player who arrives mid-duel: entity 40, player 4000.
const D_EID: u32 = 40;
const D_PID: i32 = 4000;

const PVP_FLAG: u16 = 7;
const STATE_FIELD: u16 = 19;
const DUEL_SET: u16 = 151;
const DUEL_CLEAR: u16 = 153;

/// `make_mgr`'s three players, introducible, with their witness sets built.
fn aoi_mgr() -> SpaceManager {
    let mut mgr = make_mgr();
    for eid in [A_EID, B_EID, C_EID] {
        mgr.get_entity_mut(eid).unwrap().archetype_id = Some(1);
    }
    let _ = mgr.compute_aoi_changes();
    mgr
}

/// Challenge, accept, and run the tick past the countdown. Returns the
/// instant of the engage; the channel is drained up to the accept.
async fn engage(
    mgr: &mut SpaceManager,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) -> Instant {
    let t0 = Instant::now();
    challenge(mgr, tx, A, B, t0).await;
    crate::cell::duel::response::handle_at(B_EID, &[1], tx, mgr, t0).await;
    drain(rx);
    run_at(tx, mgr, t0 + COUNTDOWN).await;
    t0 + COUNTDOWN
}

/// The witness routings of `method_index` about `entity_id`, as
/// `(witness, args)`.
fn to_witnesses(sent: &[Sent], entity_id: u32, method_index: u16) -> Vec<(u32, Vec<u8>)> {
    let mut out: Vec<(u32, Vec<u8>)> = sent
        .iter()
        .filter(|s| s.entity_id == entity_id && s.method_index == method_index)
        .filter_map(|s| s.witness.map(|w| (w, s.args.clone())))
        .collect();
    out.sort();
    out
}

/// **Type 8.** At the engage each duelist gets `onDuelEntitiesSet([A, B])`,
/// its own PvP flag set to 1, `BSF_InCombat` and the "begun" line; the flag
/// and the state field also reach every witness: the other duelist and the
/// bystander. The pair may now harm each other, nobody else.
#[tokio::test]
async fn engage_fans_the_pvp_flag_to_both_duelists_and_a_witness() {
    let capture = LogCapture::install();
    let mut mgr = aoi_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    engage(&mut mgr, &tx, &mut rx).await;
    let sent = drain(&mut rx);

    let set = build_on_duel_entities_set(&[A_EID as i32, B_EID as i32]);
    let on = build_pvp_flag(true);
    for (me, other) in [(A_EID, B_EID), (B_EID, A_EID)] {
        assert_eq!(own(&sent, me, DUEL_SET), vec![set.clone()], "151 to {me}");
        assert_eq!(own(&sent, me, PVP_FLAG), vec![on.clone()], "own flag {me}");
        assert_eq!(
            to_witnesses(&sent, me, PVP_FLAG),
            vec![
                (other.min(C_EID), on.clone()),
                (other.max(C_EID), on.clone())
            ],
            "flag about {me} to the other duelist and the bystander"
        );
        let state = own(&sent, me, STATE_FIELD);
        assert_eq!(state.len(), 1, "one state field to {me}");
        let bits = u32::from_le_bytes(state[0][..4].try_into().unwrap());
        assert_ne!(bits & BSF_IN_COMBAT, 0, "{me} is in combat");
        assert_eq!(to_witnesses(&sent, me, STATE_FIELD).len(), 2);
        assert_eq!(lines_to(&sent, me), vec![TEXT_DUEL_ENGAGED.to_string()]);
        let e = mgr.get_entity(me).unwrap();
        assert!(
            e.threatened_mobs.contains(&other),
            "the partner is the combat source"
        );
    }
    assert!(
        own(&sent, C_EID, DUEL_SET).is_empty(),
        "the bystander gets no 151"
    );
    assert!(mgr.duels.can_harm(A_PID, B_PID) && mgr.duels.can_harm(B_PID, A_PID));
    assert!(!mgr.duels.can_harm(A_PID, C_PID) && !mgr.duels.can_harm(C_PID, A_PID));
    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "duel.engaged"))
        .expect("duel.engaged row");
    for (k, v) in [
        ("account_id", "500"),
        ("player_id", &A_PID.to_string()[..]),
        ("entity_id", &A_EID.to_string()[..]),
        ("target_player_id", &B_PID.to_string()[..]),
    ] {
        assert!(ev.has_field(k, v), "duel.engaged {k}={v}: {ev:?}");
    }
}

/// A player who comes into range mid-duel gets each duelist's flag right
/// after the create; a non-duelist in the same view brings no flag.
#[tokio::test]
async fn witness_entering_mid_duel_gets_the_current_flag() {
    let mut mgr = aoi_mgr();
    add_player(&mut mgr, D_EID, D_PID, 800, "Agnos", [900.0, 0.0, 0.0]);
    mgr.get_entity_mut(D_EID).unwrap().archetype_id = Some(1);
    let (tx, mut rx) = mpsc::channel(256);
    engage(&mut mgr, &tx, &mut rx).await;
    drain(&mut rx);

    mgr.get_entity_mut(D_EID).unwrap().position = Vector3::new(0.0, 0.0, -5.0);
    let events = mgr.compute_aoi_changes_for_player(D_EID);
    let flag_to_d: Vec<u32> = events
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id: D_EID,
                entity_id,
                method_index: PVP_FLAG,
                args,
                entity_is_player: true,
            } if *args == build_pvp_flag(true) => Some(*entity_id),
            _ => None,
        })
        .collect();
    let mut sorted = flag_to_d.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, vec![A_EID, B_EID], "D learns both flags, not C's");
    for eid in [A_EID, B_EID] {
        let created = events.iter().position(|m| {
            matches!(m, CellToBaseMsg::EnteredAoI { witness_id: D_EID, entity_id, .. } if *entity_id == eid)
        });
        let flagged = events.iter().position(|m| {
            matches!(m, CellToBaseMsg::WitnessEntityMethod { witness_id: D_EID, entity_id, method_index: PVP_FLAG, .. } if *entity_id == eid)
        });
        assert!(created < flagged, "the flag follows the create for {eid}");
    }
}

/// The safety end: an engaged duel that reaches `ENGAGED_LIMIT` is ended
/// and everything the engage set is cleared: the flag back to 0 for the
/// duelists and their witnesses, 153, `BSF_InCombat`, the registry.
#[tokio::test]
async fn engaged_limit_ends_the_duel_and_clears_both_flags() {
    let capture = LogCapture::install();
    let mut mgr = aoi_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    let engaged_at = engage(&mut mgr, &tx, &mut rx).await;
    drain(&mut rx);
    assert!(matches!(
        mgr.duels.duel_of(A_PID).unwrap().state,
        DuelState::Engaged { .. }
    ));

    run_at(
        &tx,
        &mut mgr,
        engaged_at + ENGAGED_LIMIT - Duration::from_millis(1),
    )
    .await;
    assert!(drain(&mut rx).is_empty(), "nothing before the limit");

    run_at(&tx, &mut mgr, engaged_at + ENGAGED_LIMIT).await;
    let sent = drain(&mut rx);
    let off = build_pvp_flag(false);
    for (me, other) in [(A_EID, B_EID), (B_EID, A_EID)] {
        assert_eq!(
            own(&sent, me, PVP_FLAG),
            vec![off.clone()],
            "own flag off {me}"
        );
        assert_eq!(
            to_witnesses(&sent, me, PVP_FLAG),
            vec![
                (other.min(C_EID), off.clone()),
                (other.max(C_EID), off.clone())
            ]
        );
        assert_eq!(
            own(&sent, me, DUEL_CLEAR),
            vec![Vec::<u8>::new()],
            "153 to {me}"
        );
        let state = own(&sent, me, STATE_FIELD);
        let bits = u32::from_le_bytes(state[0][..4].try_into().unwrap());
        assert_eq!(bits & BSF_IN_COMBAT, 0, "{me} left combat");
        assert!(mgr.get_entity(me).unwrap().threatened_mobs.is_empty());
        assert_eq!(lines_to(&sent, me), vec![TEXT_DUEL_ABORTED.to_string()]);
    }
    assert!(!mgr.duels.can_harm(A_PID, B_PID));
    assert!(mgr.duels.is_idle() || mgr.duels.duel_of(A_PID).is_none());
    assert!(!mgr.duels.is_busy(A_PID) && !mgr.duels.is_busy(B_PID));
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "duel.ended") && c.has_field("reason", "engaged_limit")));
}

/// A duelist who leaves the world ends the duel on the next tick; the one
/// left behind is fully cleared and told.
#[tokio::test]
async fn duelist_leaving_the_world_ends_the_duel() {
    let capture = LogCapture::install();
    let mut mgr = aoi_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    let engaged_at = engage(&mut mgr, &tx, &mut rx).await;
    drain(&mut rx);

    mgr.destroy_entity(B_EID);
    run_at(&tx, &mut mgr, engaged_at + Duration::from_millis(100)).await;
    let sent = drain(&mut rx);
    assert_eq!(own(&sent, A_EID, PVP_FLAG), vec![build_pvp_flag(false)]);
    assert_eq!(own(&sent, A_EID, DUEL_CLEAR).len(), 1);
    assert!(mgr.get_entity(A_EID).unwrap().threatened_mobs.is_empty());
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert!(!mgr.duels.is_busy(A_PID));
    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "duel.ended"))
        .expect("duel.ended row");
    assert!(ev.has_field("reason", "duelist_gone"), "{ev:?}");
    assert!(ev.has_field("target_cleared", "false"), "{ev:?}");
}

/// **Type 12.** A duelist who left during the countdown: no engage, nothing
/// flagged, 878 to the one still here, and a `duel.engage_refused` row with
/// `reason = duelist_gone`.
#[tokio::test]
async fn countdown_end_with_a_duelist_gone_is_refused() {
    let capture = LogCapture::install();
    let mut mgr = aoi_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    let t0 = Instant::now();
    challenge(&mut mgr, &tx, A, B, t0).await;
    crate::cell::duel::response::handle_at(B_EID, &[1], &tx, &mut mgr, t0).await;
    drain(&mut rx);

    mgr.destroy_entity(A_EID);
    run_at(&tx, &mut mgr, t0 + COUNTDOWN).await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert!(own(&sent, B_EID, PVP_FLAG).is_empty() && own(&sent, B_EID, DUEL_SET).is_empty());
    assert!(!mgr.duels.is_busy(B_PID));
    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "duel.engage_refused"))
        .expect("engage_refused row");
    for (k, v) in [
        ("reason", "duelist_gone"),
        ("gone", "challenger"),
        ("player_id", &A_PID.to_string()[..]),
        ("target_player_id", &B_PID.to_string()[..]),
    ] {
        assert!(ev.has_field(k, v), "engage_refused {k}={v}: {ev:?}");
    }
}
