//! NA10: a stopped NPC reports zero velocity, and the leash snap keeps the
//! spatial grid in sync.
//!
//! The AoI tick sends every witness an `EntityMoved` carrying the NPC's
//! stored velocity every 100 ms, whether or not the NPC moved. A site that
//! cleared `nav_path` without zeroing velocity left the client animating a
//! run in place (audit S1; SigNoz found 67 such mid-leg interruptions, all
//! `attack_in_place`). These tests run the real AI tick, the movement tick
//! and the AoI tick and read the velocity off the `EntityMoved` a witness
//! would receive.

use super::*;
use crate::cell::messages::CellToBaseMsg;
use cimmeria_common::Vector3;
use std::collections::VecDeque;
use tokio::sync::mpsc;

const NPC: u32 = 200;
const PLAYER: u32 = 101;

/// Chase speed the movement tick would have written: `move_speed` 0.6 u per
/// 100 ms tick = 6 u/s, the value the audit saw left on standing NPCs.
const CHASE_VELOCITY: [f32; 3] = [6.0, 0.0, 0.0];

fn add_witness(mgr: &mut SpaceManager, pos: [f32; 3]) {
    mgr.create_entity(PLAYER, "Castle", pos, [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(PLAYER as i32);
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    mgr.connect_entity(PLAYER);
    // Establish the witness set so the next pass emits `EntityMoved`.
    let _ = mgr.compute_aoi_changes();
}

async fn ai_tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(64);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

/// The velocity on the `EntityMoved` the witness receives for the NPC on the
/// next AoI pass.
fn next_entity_moved_velocity(mgr: &mut SpaceManager) -> [f32; 3] {
    mgr.compute_aoi_changes()
        .into_iter()
        .find_map(|m| match m {
            CellToBaseMsg::EntityMoved {
                witness_id: PLAYER,
                entity_id: NPC,
                velocity,
                ..
            } => Some(velocity),
            _ => None,
        })
        .expect("the witness must get an EntityMoved for the NPC")
}

/// Regression guard for S1. The NPC is mid-chase, with a route and the chase
/// velocity, when the target comes into range with line of sight. The fight
/// tick stops it to shoot. The next `EntityMoved` must say it is standing
/// still.
///
/// Reverting `attack_in_place` to a bare `nav_path.clear()` fails this: the
/// state does not change (Fighting stays Fighting), so nothing else zeroes
/// velocity, and the movement tick skips path-less NPCs.
#[tokio::test]
async fn attack_in_place_sends_zero_velocity_on_the_next_entity_moved() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    seed_default_ability(&mut mgr, 0, 30);
    add_witness(&mut mgr, [10.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.threat_list.insert(PLAYER, 10.0);
        npc.move_speed = 0.6;
        npc.nav_path = VecDeque::from([Vector3::new(4.0, 0.0, 0.0), Vector3::new(8.0, 0.0, 0.0)]);
        npc.velocity = CHASE_VELOCITY;
    }
    // Control: before the tick the witness is told the NPC is running.
    assert_eq!(next_entity_moved_velocity(&mut mgr), CHASE_VELOCITY);

    ai_tick(&mut mgr).await;
    crate::cell::service::ticks::npc_movement_tick(&mut mgr);

    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(
        npc.ai_state(),
        AiState::Fighting,
        "control: still fighting, so no state change can have stopped it"
    );
    assert!(npc.nav_path.is_empty(), "attack_in_place drops the route");
    assert_eq!(
        npc.position,
        Vector3::new(0.0, 0.0, 0.0),
        "control: the NPC stood still to shoot"
    );
    assert_eq!(
        next_entity_moved_velocity(&mut mgr),
        [0.0; 3],
        "an NPC standing to shoot must not be broadcast as moving (running in place)"
    );
}

/// Regression guard for S4. A Leashing NPC far from spawn, still carrying
/// its chase route and velocity, snaps home. Afterwards:
///
/// - the spatial grid finds it at spawn and not at the chase position (the
///   old code wrote `position` directly and left the grid indexing the chase
///   cell);
/// - it has no route and no velocity, so the movement tick does not walk it
///   back out along the stale path;
/// - it faces its authored spawn heading.
///
/// The NPC is put into Leashing with `force_ai_state`, so the route survives
/// into the leash handler, as it does when the Leashing transition is
/// arranged rather than reached. This isolates the leash's own stop from the
/// one the state transition performs.
#[tokio::test]
async fn leash_snap_updates_the_grid_drops_the_route_and_restores_facing() {
    // Chase position (140, 140) is grid cell (2, 2); spawn (0, 0) is (0, 0).
    let chase = [140.0, 0.0, 140.0];
    let mut mgr = make_ai_fixture([0.0; 3], chase);
    let spawn_facing = Vector3::new(0.0, 1.25, 0.0);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Leashing);
        npc.spawn_direction = Some(spawn_facing);
        npc.direction = Vector3::new(0.0, -2.0, 0.0);
        npc.move_speed = 0.6;
        npc.nav_path = VecDeque::from([Vector3::new(150.0, 0.0, 150.0)]);
        npc.velocity = CHASE_VELOCITY;
    }

    ai_tick(&mut mgr).await;
    crate::cell::service::ticks::npc_movement_tick(&mut mgr);

    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Idle, "control: leash completed");
    assert_eq!(npc.position, Vector3::new(0.0, 0.0, 0.0), "snapped home");
    assert!(npc.nav_path.is_empty(), "the stale chase route is dropped");
    assert_eq!(npc.velocity, [0.0; 3], "and the NPC is not moving");
    let facing = npc.direction;

    // Grid first: it is the half of S4 that nothing else would catch.
    let space_id = mgr.entity_space[&NPC];
    let space = &mgr.spaces[&space_id].space;
    assert!(
        space
            .get_entities_in_range(&Vector3::new(0.0, 0.0, 0.0), 1.0)
            .iter()
            .any(|e| e.0 == NPC as i32),
        "the spatial grid must index the NPC at spawn after the snap"
    );
    assert!(
        !space
            .get_entities_in_range(&Vector3::new(140.0, 0.0, 140.0), 1.0)
            .iter()
            .any(|e| e.0 == NPC as i32),
        "the grid must not still index the NPC at its chase position"
    );
    assert_eq!(facing, spawn_facing, "spawn facing restored");
}

/// Every AI state change stops the NPC. Fighting -> Leashing (target left
/// the leash radius) used to keep the chase route and velocity, which the
/// next movement ticks walked.
#[tokio::test]
async fn leash_out_transition_stops_the_npc() {
    // NA12: the leash measures the NPC, so the NPC stands past the band.
    let mut mgr = make_ai_fixture([0.0; 3], [60.0, 0.0, 0.0]);
    add_witness(&mut mgr, [90.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.threat_list.insert(PLAYER, 10.0);
        npc.nav_path = VecDeque::from([Vector3::new(70.0, 0.0, 0.0)]);
        npc.velocity = CHASE_VELOCITY;
    }

    ai_tick(&mut mgr).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Leashing, "control");
    assert!(npc.nav_path.is_empty());
    assert_eq!(npc.velocity, [0.0; 3]);
}

/// Aggro preempt: a patrolling NPC that takes damage drops its patrol leg
/// and stands still until the fight handler routes it, instead of being
/// broadcast at patrol speed with no path.
#[tokio::test]
async fn aggro_preempt_stops_a_patrolling_npc() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    add_witness(&mut mgr, [10.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Patrol);
        npc.nav_path = VecDeque::from([Vector3::new(0.0, 0.0, 30.0)]);
        npc.velocity = [0.0, 0.0, 4.0];
    }

    let _ = crate::cell::combat::generate_threat(
        &mut mgr,
        PLAYER,
        NPC,
        10.0,
        crate::cell::combat::AggroCause::Damage,
    );

    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Fighting, "control: preempted");
    assert!(npc.nav_path.is_empty());
    assert_eq!(npc.velocity, [0.0; 3]);
}

/// NA10 wire guard. The Fighting entry used to send every witness a
/// `WitnessEntityMethod` with method index 1 and the one-byte payload
/// `[CombatAdvance]`. On the client, witness method 1 is `onSequence`, so
/// that was a truncated Kismet-sequence trigger. No movement-type message
/// exists server-to-client, so nothing may go out. The cache is still
/// recorded, which is the control that the broadcast path actually ran.
#[tokio::test]
async fn fighting_entry_sends_no_one_byte_method_1_to_witnesses() {
    use cimmeria_entity::cell_entity::MobMovementType;

    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    seed_default_ability(&mut mgr, 0, 30);
    add_witness(&mut mgr, [10.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.threat_list.insert(PLAYER, 10.0);
        npc.last_movement_type = None;
    }

    let (tx, mut rx) = mpsc::channel(256);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    assert_eq!(
        mgr.get_entity(NPC).unwrap().last_movement_type,
        Some(MobMovementType::CombatAdvance),
        "control: the Fighting entry recorded its movement type"
    );
    let bogus: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok())
        .filter(|m| {
            matches!(
                m,
                CellToBaseMsg::WitnessEntityMethod {
                    entity_id: NPC,
                    method_index: 1,
                    args,
                    ..
                } if args.len() == 1
            )
        })
        .collect();
    assert!(
        bogus.is_empty(),
        "a one-byte method-1 witness call is a truncated onSequence: {bogus:?}"
    );
}
