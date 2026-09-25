//! A stationary NPC fires across furniture the navmesh cuts out, and a
//! mobile NPC still walks around it (NPC AI restoration NA16, audit S11).
//!
//! The Find Ambernol drone (spawn 10, `castle_cellblock`) held fire at
//! 12-16 m in 17 of 19 recorded fights (`stationary_holds`,
//! `has_los=false`). The player takes the vial from the med-station desk.
//! That desk is a hole in the navmesh, so the ray from the drone reads
//! `Blocked` although the desk is about 1 m high and the drone sees over
//! it. The drone is pinned (`is_stationary`) and cannot step around the
//! desk, so it never fired until the player walked up to it.
//!
//! Uses the real `castle_cellblock.nav`, injected into the `Castle` fixture
//! space (the production loader keys off a cwd-relative path the test
//! harness does not satisfy). The geometry diagnosis itself is pinned in
//! `cimmeria-entity`'s `navigation::line_of_sight_policy_tests`.

use super::{make_ai_fixture, seed_default_ability, seed_target_with_threat};
use crate::cell::combat::NPC_DEFAULT_ABILITY;
use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::navigation::{LineOfSight, NavMesh};
use tokio::sync::mpsc;

/// Spawn 10's seeded position.
const DRONE: [f32; 3] = [-220.257, 66.744, -121.375];
/// South of the desk, 15.2 m from the drone, on the drone's floor.
const SOUTH_OF_THE_DESK: [f32; 3] = [-234.0, 65.6, -127.7];

/// NPC 200 at the drone's spawn, fighting player 100 south of the desk,
/// with the real Cellblock mesh attached. `None` in a fixture-less
/// checkout.
fn drone_fighting_across_the_desk(is_stationary: bool) -> Option<SpaceManager> {
    let nav_path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !nav_path.exists() {
        return None;
    }
    let navmesh = NavMesh::load(nav_path).expect("load castle_cellblock.nav");

    let mut mgr = make_ai_fixture(DRONE, DRONE);
    seed_default_ability(&mut mgr, 0, 30);
    seed_target_with_threat(&mut mgr, 200, 100, SOUTH_OF_THE_DESK);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.abilities.add_ability(NPC_DEFAULT_ABILITY);
        npc.is_stationary = is_stationary;
    }
    let space_id = mgr.entity_space[&200];
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(navmesh);

    assert_eq!(
        mgr.line_of_sight(200, 100),
        LineOfSight::Blocked,
        "precondition: the navmesh reads the desk as a wall. If a rebuilt mesh \
         answers otherwise, this test no longer exercises the S11 shape"
    );
    Some(mgr)
}

async fn tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(64);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

/// The regression guard. Before NA16 the pinned drone landed in
/// `stationary_holds` here on every tick and never started its cooldown.
#[tokio::test]
async fn the_stationary_drone_fires_across_the_med_station_desk() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let Some(mut mgr) = drone_fighting_across_the_desk(true) else {
        return;
    };
    assert!(
        mgr.attack_line_of_sight(200, 100, true),
        "a stationary attacker on the same storey is not stopped by a navmesh Blocked"
    );

    let capture = LogCapture::install();
    tick(&mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.abilities.is_on_cooldown(NPC_DEFAULT_ABILITY),
        "the drone must fire at a player 15 m away across a 1 m desk; no cooldown \
         means it held fire, which is audit S11"
    );
    assert!(
        capture
            .find_message(Level::INFO, "NPC AI: stationary mob holding fire")
            .is_none(),
        "the drone must not take the `stationary_holds` branch. Captured: {:#?}",
        capture.all()
    );
}

/// The control: a mobile NPC in the same spot keeps the strict verdict. It
/// holds fire and paths toward the player, which is how it gets a clear
/// line in a few steps. Only the pinned case, which cannot walk around the
/// desk, relaxes `Blocked`.
#[tokio::test]
async fn a_mobile_npc_still_walks_around_the_desk_instead_of_firing() {
    let Some(mut mgr) = drone_fighting_across_the_desk(false) else {
        return;
    };
    assert!(!mgr.attack_line_of_sight(200, 100, false));

    tick(&mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        !npc.abilities.is_on_cooldown(NPC_DEFAULT_ABILITY),
        "a mobile NPC must not fire through a navmesh Blocked"
    );
    assert!(
        !npc.nav_path.is_empty(),
        "a mobile NPC with a Blocked line must path toward its target"
    );
}
