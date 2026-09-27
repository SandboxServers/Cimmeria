//! Pets PT-02 at the content transport call sites: `Action::Teleport`
//! brings the owner's pet along only when the owner's snap was really sent,
//! and `Action::CrossWorldTeleport` leaves it behind, despawned.

use cimmeria_cell_world::test_fixtures::{
    assert_pet_fully_gone, drain_entity_moved_for, drain_left_aoi_for, watched_pet_world,
    PET_FIXTURE_OTHER as OTHER, PET_FIXTURE_OWNER as OWNER,
};

use super::*;

const DEST: [f32; 3] = [300.0, 0.0, -40.0];

fn one_action(action: Action) -> ResolvedActions {
    ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(1, action)],
    }
}

#[tokio::test]
async fn content_teleport_brings_the_pet_along() {
    let (mut mgr, pet) = watched_pet_world();
    let (tx, mut rx) = mpsc::channel(256);

    execute_actions(
        one_action(Action::Teleport {
            space_id: 0,
            position: DEST,
        }),
        OWNER,
        0,
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let p = mgr.get_entity(pet).unwrap().position;
    assert!(
        (p.x - DEST[0]).abs() < 3.0 && (p.z - DEST[2]).abs() < 3.0,
        "pet beside the teleported owner: {p:?}"
    );
    assert!(!drain_entity_moved_for(&mut rx, pet).is_empty());
}

/// The owner's snap never left the cell (the base channel is gone): its
/// client still shows the owner where it stood, so the pet stays too.
#[tokio::test]
async fn a_failed_content_teleport_leaves_the_pet() {
    let (mut mgr, pet) = watched_pet_world();
    let start = mgr.get_entity(pet).unwrap().position;
    let (tx, rx) = mpsc::channel(256);
    drop(rx);

    execute_actions(
        one_action(Action::Teleport {
            space_id: 0,
            position: DEST,
        }),
        OWNER,
        0,
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    assert_eq!(mgr.get_entity(pet).unwrap().position, start);
}

#[tokio::test]
async fn content_cross_world_teleport_despawns_the_pet() {
    let (mut mgr, pet) = watched_pet_world();
    let (tx, mut rx) = mpsc::channel(256);

    execute_actions(
        one_action(Action::CrossWorldTeleport {
            world_name: "Castle".to_string(),
            position: [5.0, 0.0, 5.0],
        }),
        OWNER,
        0,
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    assert!(mgr.get_entity(OWNER).is_none(), "the owner left");
    assert_eq!(drain_left_aoi_for(&mut rx, pet), vec![OTHER]);
    assert_pet_fully_gone(&mgr, OWNER, pet);
}
