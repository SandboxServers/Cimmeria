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
/// target come into range: the next AI tick is `attack_in_place`, which
/// clears the path and leaves the velocity (audit S1).
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
    let npc = mgr.get_entity(NPC).unwrap();
    assert!(
        npc.nav_path.is_empty(),
        "precondition: attack_in_place cleared the path"
    );
    assert!(
        npc.velocity[0] > 0.0,
        "precondition (today's bug): the velocity survives the stop"
    );
    mgr
}

/// **Acceptance: stale velocity after `attack_in_place`.** Revert-proof:
/// deleting the `report_stale_velocity` call in `after_movement_tick` (or
/// the `after_movement_tick` call in the message loop's helper used here)
/// leaves no row.
#[tokio::test]
async fn stale_velocity_fires_after_an_attack_in_place_stop() {
    let mut mgr = attack_in_place_after_a_chase().await;
    let logs = LogCapture::install();
    for _ in 0..=STALE_VELOCITY_TICKS {
        movement_tick(&mut mgr);
    }
    let found = rows(&logs, "movement.npc", "stale_velocity");
    assert_eq!(found.len(), 1, "one row (then throttled): {found:#?}");
    let row = &found[0];
    assert_eq!(row.level, Level::WARN);
    for (k, v) in [
        ("npc_id", "200"),
        ("path_state", "empty"),
        ("nav_path_len", "0"),
        ("ai_state", "fighting"),
        ("tag", "Test_Guard"),
        ("world", "Castle"),
        ("suppressed", "0"),
    ] {
        assert!(row.has_field(k, v), "{k}={v} missing: {row:?}");
    }
}

/// **Acceptance: after a leash** (the telemetry plan's
/// `animating_without_path`, folded into `stale_velocity` — see the
/// `movement` module docs). The NPC stops to shoot, the player backs off
/// past the leash radius, the leash snaps it home: the snap zeroes nothing,
/// so the NPC stands at spawn broadcasting its old chase velocity.
#[tokio::test]
async fn stale_velocity_fires_after_a_leash_snap() {
    let mut mgr = attack_in_place_after_a_chase().await;
    // The player leaves the leash radius around spawn.
    mgr.update_entity_position(super::PLAYER, [80.0, 0.0, 0.0], [0; 3], [0.0; 3]);
    ai_tick(&mut mgr).await; // Fighting -> Leashing
    ai_tick(&mut mgr).await; // leash snap -> Idle
    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Idle, "precondition: leash ran");
    assert!(npc.nav_path.is_empty());

    let logs = LogCapture::install();
    for _ in 0..=STALE_VELOCITY_TICKS {
        movement_tick(&mut mgr);
    }
    let found = rows(&logs, "movement.npc", "stale_velocity");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].has_field("ai_state", "idle"), "{:?}", found[0]);
    assert!(found[0].has_field("path_state", "empty"));
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
