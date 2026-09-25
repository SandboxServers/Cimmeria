//! `movement.npc event=stale_velocity`: the running-in-place detector.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use tracing::Level;

use super::{add_npc, add_threat_player, ai_tick, castle_mgr, movement_tick, rows, NPC};
use crate::cell::service::npc_ai::detectors::movement::{
    after_movement_tick, STALE_VELOCITY_TICKS,
};
use crate::test_support::LogCapture;

/// Chase until the movement tick has written a chase velocity, then let the
/// target come into range: the next AI tick is `attack_in_place`. Before
/// NA10 that cleared the path and left the chase velocity (audit S1); since
/// NA10 it goes through `stop_npc_movement`, which zeroes it.
async fn attack_in_place_after_a_chase() -> crate::cell::space_manager::SpaceManager {
    let mut mgr = castle_mgr();
    add_npc(
        &mut mgr,
        "Castle",
        [0.0, 0.0, 0.0],
        Some([0.0; 3]),
        AiState::Fighting,
    );
    add_threat_player(&mut mgr, "Castle", [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.move_speed = 0.6;
        // Mid-leg: a path still heading somewhere.
        npc.nav_path.push_back(Vector3::new(4.0, 0.0, 0.0));
    }
    movement_tick(&mut mgr);
    assert!(
        mgr.get_entity(NPC).unwrap().velocity[0] > 0.0,
        "precondition: the movement tick wrote a chase velocity"
    );
    ai_tick(&mut mgr).await;
    assert!(
        mgr.get_entity(NPC).unwrap().nav_path.is_empty(),
        "precondition: attack_in_place cleared the path"
    );
    mgr
}

/// **Regression guard for NA10** (this was NA02's positive case before
/// NA10 landed). After `attack_in_place` the velocity is zeroed, so the
/// running-in-place detector stays silent. Removing NA10's
/// `stop_npc_movement` at the attack-in-place site brings the WARN back.
///
/// Silence alone proves nothing if the detector is dead: this guard relies
/// on `stale_velocity_fires_on_a_stalled_path_with_velocity` proving the
/// detector does fire, and on its own precondition that the velocity is
/// zeroed.
#[tokio::test]
async fn stale_velocity_is_silent_after_an_attack_in_place_stop() {
    let mut mgr = attack_in_place_after_a_chase().await;
    assert_eq!(
        mgr.get_entity(NPC).unwrap().velocity,
        [0.0; 3],
        "NA10: attack_in_place zeroes the chase velocity"
    );
    let logs = LogCapture::install();
    for _ in 0..=STALE_VELOCITY_TICKS {
        movement_tick(&mut mgr);
    }
    assert!(rows(&logs, "movement.npc", "stale_velocity").is_empty());
}

/// **Regression guard for NA10, leash.** The snap goes through
/// `snap_npc_to`, which stops the NPC, so it no longer stands at spawn
/// broadcasting its old chase velocity.
///
/// Like the guard above, a silence-only test: it relies on
/// `stale_velocity_fires_on_a_stalled_path_with_velocity` for proof that
/// the detector is alive.
#[tokio::test]
async fn stale_velocity_is_silent_after_a_leash_snap() {
    let mut mgr = castle_mgr();
    add_npc(
        &mut mgr,
        "Castle",
        [20.0, 0.0, 0.0],
        Some([0.0; 3]),
        AiState::Leashing,
    );
    mgr.get_entity_mut(NPC).unwrap().velocity = [6.0, 0.0, 0.0];
    ai_tick(&mut mgr).await; // leash snap -> Idle
    assert_eq!(mgr.get_entity(NPC).unwrap().ai_state(), AiState::Idle);
    let logs = LogCapture::install();
    for _ in 0..=STALE_VELOCITY_TICKS {
        movement_tick(&mut mgr);
    }
    assert!(rows(&logs, "movement.npc", "stale_velocity").is_empty());
}

/// **Positive case: a stalled path that still carries a velocity.** The
/// NPC holds a route and a chase velocity but does not move across ticks
/// (the detector pass is driven alone, standing in for movement ticks that
/// did not advance it). Revert-proof: deleting the `report_stale_velocity`
/// call in `after_movement_tick` leaves no row.
#[test]
fn stale_velocity_fires_on_a_stalled_path_with_velocity() {
    let mut mgr = castle_mgr();
    add_npc(&mut mgr, "Castle", [0.0; 3], None, AiState::Fighting);
    {
        let npc = mgr.get_entity_mut(NPC).unwrap();
        npc.velocity = [6.0, 0.0, 0.0];
        npc.nav_path.push_back(Vector3::new(10.0, 0.0, 0.0));
    }
    let logs = LogCapture::install();
    let t0 = Instant::now();
    for i in 0..=u64::from(STALE_VELOCITY_TICKS) {
        after_movement_tick(&mut mgr, t0 + Duration::from_millis(100 * i));
    }
    let found = rows(&logs, "movement.npc", "stale_velocity");
    assert_eq!(found.len(), 1, "{found:#?}");
    let row = &found[0];
    assert_eq!(row.level, Level::WARN);
    for (k, v) in [
        ("npc_id", "200"),
        ("path_state", "stalled"),
        ("nav_path_len", "1"),
        ("ai_state", "fighting"),
        ("tag", "Test_Guard"),
        ("world", "Castle"),
        ("suppressed", "0"),
    ] {
        assert!(row.has_field(k, v), "{k}={v} missing: {row:?}");
    }
}

/// A walking NPC is not stale, however long it walks.
#[test]
fn a_moving_npc_is_never_stale() {
    let mut mgr = castle_mgr();
    add_npc(&mut mgr, "Castle", [0.0; 3], None, AiState::Patrol);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.move_speed = 0.5;
        npc.nav_path.push_back(Vector3::new(50.0, 0.0, 0.0));
    }
    let logs = LogCapture::install();
    for _ in 0..20 {
        movement_tick(&mut mgr);
    }
    assert!(rows(&logs, "movement.npc", "stale_velocity").is_empty());
}

/// Throttle: one row per 10 s per NPC, and the next row carries the count
/// of what it skipped. Driven with a synthetic clock.
#[test]
fn stale_velocity_is_throttled_and_counts_suppressed() {
    let mut mgr = castle_mgr();
    add_npc(&mut mgr, "Castle", [0.0; 3], None, AiState::Fighting);
    mgr.get_entity_mut(NPC).unwrap().velocity = [6.0, 0.0, 0.0];
    let logs = LogCapture::install();
    let t0 = Instant::now();
    // 1 baseline + 3 still ticks -> first row; 5 more inside the window.
    for i in 0..9u64 {
        after_movement_tick(&mut mgr, t0 + Duration::from_millis(100 * i));
    }
    after_movement_tick(&mut mgr, t0 + Duration::from_secs(11));
    let found = rows(&logs, "movement.npc", "stale_velocity");
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(found[0].has_field("suppressed", "0"));
    assert!(
        found[1].has_field("suppressed", "5"),
        "the second row reports the five throttled occurrences: {:?}",
        found[1]
    );
}
