//! `idle_parked`, `cleared_without_exit`, aggro-scan rejects,
//! `spawn_off_mesh`, `stuck`, `npc_ai.los`, the avatar-update flag, and
//! detector-state teardown.

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use tracing::Level;

use super::{
    add_npc, add_threat_player, ai_tick, castle_mgr, cellblock_mgr, movement_tick, rows, NPC,
    PLAYER,
};
use crate::cell::messages::CellToBaseMsg;
use crate::test_support::LogCapture;

const MESSHALL: [f32; 3] = [-96.25, 34.591, -91.59];

/// NA12 (S6): a fight that ends away from spawn sends the NPC home, so it is
/// never `idle_parked`. Before NA12 the first tick set Idle where the NPC
/// stood and this row fired with `reason=threat_empty`.
#[tokio::test]
async fn a_fight_that_ends_away_from_spawn_goes_home_and_is_not_parked() {
    let mut mgr = castle_mgr();
    add_npc(
        &mut mgr,
        "Castle",
        [20.0, 0.0, 0.0],
        Some([0.0; 3]),
        AiState::Fighting,
    );
    let logs = LogCapture::install();
    ai_tick(&mut mgr).await; // empty threat list -> Leashing
    ai_tick(&mut mgr).await; // no route in a meshless space: snap home, Idle
    assert!(
        rows(&logs, "npc_ai.idle_parked", "idle_parked").is_empty(),
        "{:#?}",
        logs.all()
    );
    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Idle);
    assert_eq!(npc.position, Vector3::new(0.0, 0.0, 0.0));
}

/// The one reset that still parks by design: a follower is reset where it
/// stands (it must not be yanked away from the player it escorts), so it
/// goes Idle away from spawn and the detector says so.
#[tokio::test]
async fn a_follower_reset_in_place_is_idle_parked() {
    let mut mgr = castle_mgr();
    add_npc(
        &mut mgr,
        "Castle",
        [20.0, 0.0, 0.0],
        Some([0.0; 3]),
        AiState::Fighting,
    );
    mgr.get_entity_mut(NPC).unwrap().follow_target_id = Some(999);
    let logs = LogCapture::install();
    ai_tick(&mut mgr).await; // -> Leashing
    ai_tick(&mut mgr).await; // follower: reset in place, Idle
    let found = rows(&logs, "npc_ai.idle_parked", "idle_parked");
    assert_eq!(found.len(), 1, "{:#?}", logs.all());
    assert_eq!(found[0].level, Level::INFO);
    assert!(
        found[0].has_field("reason", "leash_arrived"),
        "{:?}",
        found[0]
    );
    assert!(found[0].has_field("npc_to_spawn", "20.0"));
}

/// An aggressive NPC is still ticked when Idle, so it is not parked.
#[tokio::test]
async fn an_aggressive_npc_going_idle_is_not_parked() {
    let mut mgr = castle_mgr();
    add_npc(
        &mut mgr,
        "Castle",
        [20.0, 0.0, 0.0],
        Some([0.0; 3]),
        AiState::Fighting,
    );
    mgr.get_entity_mut(NPC).unwrap().aggro.override_level =
        Some(cimmeria_entity::cell_entity::MobAggression::Hostile);
    let logs = LogCapture::install();
    ai_tick(&mut mgr).await;
    assert!(rows(&logs, "npc_ai.idle_parked", "idle_parked").is_empty());
}

/// NA12 (S7): the leash drains the player before it clears the NPC's
/// threat, so `cleared_without_exit` stays silent through the whole leash
/// (entry and arrival) and the player leaves combat. Removing the drain in
/// `leash::begin_leash` brings the WARN back with `reason=leash_out`.
#[tokio::test]
async fn a_leash_drains_the_player_so_cleared_without_exit_stays_silent() {
    let mut mgr = castle_mgr();
    add_npc(
        &mut mgr,
        "Castle",
        [60.0, 0.0, 0.0],
        Some([0.0; 3]),
        AiState::Fighting,
    );
    add_threat_player(&mut mgr, "Castle", [70.0, 0.0, 0.0]);
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .threatened_mobs
        .insert(NPC);
    let logs = LogCapture::install();
    ai_tick(&mut mgr).await; // Fighting -> Leashing, player drained
    assert_eq!(
        mgr.get_entity(NPC).unwrap().ai_state(),
        AiState::Leashing,
        "control: the leash fired"
    );
    ai_tick(&mut mgr).await; // leash completes
    assert!(
        rows(&logs, "threat", "cleared_without_exit").is_empty(),
        "{:#?}",
        logs.all()
    );
    assert!(mgr.get_entity(PLAYER).unwrap().threatened_mobs.is_empty());
}

/// The detector itself still fires on a clear that skipped the drain (a new
/// clear path added without one), with the reason it was given.
#[tokio::test]
async fn an_undrained_clear_is_still_reported() {
    let mut mgr = castle_mgr();
    add_npc(
        &mut mgr,
        "Castle",
        [0.0; 3],
        Some([0.0; 3]),
        AiState::Fighting,
    );
    add_threat_player(&mut mgr, "Castle", [10.0, 0.0, 0.0]);
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .threatened_mobs
        .insert(NPC);
    let logs = LogCapture::install();
    crate::cell::service::npc_ai::detectors::threat::check_cleared(
        &mut mgr,
        NPC,
        crate::cell::service::npc_ai::detectors::threat::ThreatClear::TargetLost,
        std::time::Instant::now(),
    );
    let found = rows(&logs, "threat", "cleared_without_exit");
    assert_eq!(found.len(), 1, "{:#?}", logs.all());
    assert_eq!(found[0].level, Level::WARN);
    assert!(
        found[0].has_field("reason", "target_lost"),
        "{:?}",
        found[0]
    );
    assert!(found[0].has_field("player_id", "101"));
}

/// Today's only scan rejects are named, per pair; nobody qualifying says so.
#[tokio::test]
async fn the_idle_scan_names_its_rejects() {
    let mut mgr = castle_mgr();
    add_npc(&mut mgr, "Castle", [0.0; 3], Some([0.0; 3]), AiState::Idle);
    mgr.get_entity_mut(NPC).unwrap().aggro.override_level =
        Some(cimmeria_entity::cell_entity::MobAggression::Hostile);
    add_threat_player(&mut mgr, "Castle", [10.0, 0.0, 0.0]);
    {
        let npc = mgr.get_entity_mut(NPC).unwrap();
        npc.threat_list.clear();
        npc.faction = 0; // same as the player
    }
    let logs = LogCapture::install();
    ai_tick(&mut mgr).await;
    let rejected = rows(&logs, "npc_ai.aggro_scan", "candidate_rejected");
    assert_eq!(rejected.len(), 1, "{:#?}", logs.all());
    assert!(rejected[0].has_field("reason", "same_faction"));
    assert!(rejected[0].has_field("player_id", "101"));
    assert!(
        rejected[0].has_field("aggro_radius", "18.0"),
        "{:?}",
        rejected[0]
    );
    assert_eq!(rows(&logs, "npc_ai.aggro_scan", "no_candidates").len(), 1);
    ai_tick(&mut mgr).await; // inside both sample windows
    assert_eq!(
        rows(&logs, "npc_ai.aggro_scan", "candidate_rejected").len(),
        1
    );
}

/// A spawn hovering 2 units over its floor passes `is_point_valid` but not
/// `find_path`'s start box (S9). Reported once per spawn id.
#[test]
fn a_hovering_spawn_is_spawn_off_mesh_once() {
    let (mut mgr, _) = cellblock_mgr();
    add_npc(
        &mut mgr,
        "Castle_CellBlock",
        [MESSHALL[0], MESSHALL[1] + 2.0, MESSHALL[2]],
        None,
        AiState::Idle,
    );
    mgr.get_entity_mut(NPC).unwrap().spawn_id = Some(29);
    let logs = LogCapture::install();
    crate::cell::service::npc_ai::detectors::spawn::check_spawn(&mut mgr, NPC);
    crate::cell::service::npc_ai::detectors::spawn::check_spawn(&mut mgr, NPC);
    let found = rows(&logs, "spawner.npc_behaviour", "spawn_off_mesh");
    assert_eq!(found.len(), 1, "{:#?}", logs.all());
    assert_eq!(found[0].level, Level::WARN);
    assert!(found[0].has_field("on_navmesh", "true"), "{:?}", found[0]);
    assert!(found[0].has_field("gate", "start_box"));
}

/// An NPC that never gets a route at all (meshless space, no stale path)
/// is stuck too: `no_path` counts without a path. Revert-proof: requiring
/// a non-empty path for `no_path` again leaves no row.
#[tokio::test]
async fn a_chaser_that_never_gets_a_path_is_stuck() {
    let mut mgr = castle_mgr();
    add_npc(&mut mgr, "Castle", [0.0; 3], None, AiState::Fighting);
    add_threat_player(&mut mgr, "Castle", [45.0, 0.0, 0.0]);
    assert!(mgr.get_entity(NPC).unwrap().nav_path.is_empty());
    let logs = LogCapture::install();
    for _ in 0..3 {
        ai_tick(&mut mgr).await;
    }
    let found = rows(&logs, "npc_ai", "stuck");
    assert_eq!(found.len(), 1, "{:#?}", logs.all());
    assert!(found[0].has_field("decision_outcome", "no_path"));
    assert!(found[0].has_field("nav_path_len", "0"));
}

/// Chasing with a stale path and no route: the NPC never closes (the
/// `no_path` branch enqueues nothing). Three AI ticks without progress.
#[tokio::test]
async fn a_chase_that_never_closes_is_stuck() {
    let mut mgr = castle_mgr();
    add_npc(&mut mgr, "Castle", [0.0; 3], None, AiState::Fighting);
    add_threat_player(&mut mgr, "Castle", [45.0, 0.0, 0.0]);
    mgr.get_entity_mut(NPC)
        .unwrap()
        .nav_path
        .push_back(Vector3::new(-20.0, 0.0, 0.0));
    let logs = LogCapture::install();
    for _ in 0..3 {
        ai_tick(&mut mgr).await;
    }
    let found = rows(&logs, "npc_ai", "stuck");
    assert_eq!(found.len(), 1, "{:#?}", logs.all());
    assert_eq!(found[0].level, Level::WARN);
    assert!(found[0].has_field("target_id", "101"));
}

/// A blocked line of sight is logged with its ray, sampled per pair.
#[test]
fn a_blocked_line_of_sight_is_logged_with_its_ray() {
    let (mut mgr, _) = cellblock_mgr();
    add_npc(
        &mut mgr,
        "Castle_CellBlock",
        MESSHALL,
        None,
        AiState::Fighting,
    );
    add_threat_player(&mut mgr, "Castle_CellBlock", [-400.0, 0.2, -400.0]);
    let logs = LogCapture::install();
    let los = mgr.line_of_sight(NPC, PLAYER);
    let _ = mgr.line_of_sight(NPC, PLAYER);
    assert_ne!(los, cimmeria_entity::navigation::LineOfSight::Clear);
    let found = rows(&logs, "npc_ai.los", "blocked");
    assert_eq!(found.len(), 1, "sampled per pair: {found:#?}");
    assert!(found[0].has_field("result", los.label()));
    assert!(found[0].has_field("eye_height_used", "0.0"));
}

/// The AoI relay says whether the NPC it is sending a velocity for actually
/// moved: `Some(false)` with a non-zero velocity is running in place.
#[test]
fn the_aoi_relay_carries_npc_moved_since_last() {
    let mut mgr = castle_mgr();
    add_npc(&mut mgr, "Castle", [0.0; 3], None, AiState::Fighting);
    add_threat_player(&mut mgr, "Castle", [5.0, 0.0, 0.0]);
    mgr.get_entity_mut(NPC).unwrap().velocity = [6.0, 0.0, 0.0];
    movement_tick(&mut mgr);
    movement_tick(&mut mgr);
    let moved: Vec<Option<bool>> = mgr
        .compute_aoi_changes()
        .into_iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMoved {
                entity_id,
                npc_moved_since_last,
                ..
            } if entity_id == NPC => Some(npc_moved_since_last),
            _ => None,
        })
        .collect();
    assert_eq!(moved, vec![Some(false)]);
}

/// Every per-entity detector map is released by both teardown paths, map
/// by map, and only for the entity torn down: a bystander id outside the
/// torn-down space keeps its slots. `destroy_space` also releases the per-world idle
/// summary throttle.
#[tokio::test]
async fn detector_state_is_released_on_destroy_entity_and_destroy_space() {
    const OTHER: u32 = 300;
    for via_space in [false, true] {
        let mut mgr = castle_mgr();
        add_npc(
            &mut mgr,
            "Castle",
            [20.0, 0.0, 0.0],
            Some([0.0; 3]),
            AiState::Fighting,
        );
        let now = std::time::Instant::now();
        mgr.npc_detectors.fill_all_for_test(NPC, PLAYER, now);
        // A bystander id with state of its own, which must survive.
        mgr.npc_detectors.fill_all_for_test(OTHER, PLAYER + 1, now);
        mgr.npc_detectors
            .admit_world_summary("Castle", now, std::time::Duration::from_secs(1));
        for (map, n) in mgr.npc_detectors.slots_for(NPC) {
            assert!(n > 0, "precondition: {map} holds a slot for the NPC");
        }
        if via_space {
            let sid = mgr.get_entity_space_id(NPC).unwrap();
            mgr.destroy_space(sid);
            assert!(
                !mgr.npc_detectors.tracks_world_summary("Castle"),
                "destroy_space releases the per-world summary throttle"
            );
        } else {
            mgr.destroy_entity(NPC);
        }
        for (map, n) in mgr.npc_detectors.slots_for(NPC) {
            assert_eq!(
                n, 0,
                "via_space={via_space}: {map} still holds the destroyed NPC -- a \
                 recycled id would inherit its throttle window"
            );
        }
        for (map, n) in mgr.npc_detectors.slots_for(OTHER) {
            assert!(n > 0, "via_space={via_space}: {map} dropped a bystander");
        }
        assert_eq!(
            mgr.npc_detectors
                .slots_for(NPC)
                .iter()
                .map(|(_, n)| n)
                .sum::<usize>(),
            0,
            "via_space={via_space}: a recycled id must not inherit a throttle window"
        );
    }
}
