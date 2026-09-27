//! Pets PT-02 (A-31): a respawn takes the owner's pet with it. Cross-world,
//! the pet stays behind and is despawned before the owner leaves;
//! same-world (a GM respawn of a living owner), it lands beside the owner at
//! the respawn point.

use cimmeria_cell_world::cell::pets::PET_SPAWN_OFFSET;
use cimmeria_cell_world::test_fixtures::{
    assert_pet_fully_gone, drain_left_aoi_for, watched_pet_world, PET_FIXTURE_OTHER as OTHER,
    PET_FIXTURE_OWNER as OWNER,
};
use tokio::sync::mpsc;

use super::super::handle_respawn;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::spawner::RespawnerDef;

/// Cross-world respawn: the pet is despawned before the owner's destroy,
/// the other player sees it go, and the owner (whose client is about to be
/// reset by the gate-travel back half) gets no `LeftAoI` for it.
#[tokio::test]
async fn cross_world_respawn_despawns_the_pet() {
    let (mut mgr, pet) = watched_pet_world();
    mgr.respawners.push(RespawnerDef {
        respawner_id: 90,
        world_name: "Castle".to_string(),
        name: "Castle hub".to_string(),
        pos: [5.0, 0.0, 5.0],
    });
    let (tx, mut rx) = mpsc::channel(256);

    handle_respawn(OWNER, 90, &tx, &mut mgr).await;

    assert!(mgr.get_entity(OWNER).is_none(), "cross-world branch taken");
    assert_eq!(drain_left_aoi_for(&mut rx, pet), vec![OTHER]);
    assert_pet_fully_gone(&mgr, OWNER, pet);
}

/// Same-world respawn of a living owner: the pet lands
/// `PET_SPAWN_OFFSET` behind the respawn point (the reanchor zeroes the
/// owner's yaw, so "behind" is -z), and both witnesses get the move.
#[tokio::test]
async fn same_world_respawn_brings_the_pet_along() {
    let (mut mgr, pet) = watched_pet_world();
    mgr.respawners.push(RespawnerDef {
        respawner_id: 91,
        world_name: "Agnos".to_string(),
        name: "Agnos hub".to_string(),
        pos: [200.0, 3.0, 150.0],
    });
    let (tx, mut rx) = mpsc::channel(256);

    handle_respawn(OWNER, 91, &tx, &mut mgr).await;

    let expect = [200.0, 3.0, 150.0 - PET_SPAWN_OFFSET];
    let p = mgr
        .get_entity(pet)
        .expect("the pet survives a same-world respawn");
    let got = [p.position.x, p.position.y, p.position.z];
    for (g, w) in got.iter().zip(expect) {
        assert!((g - w).abs() < 1e-3, "pet at {got:?}, want {expect:?}");
    }
    assert_eq!(mgr.pets.pets_of(OWNER), vec![pet]);

    // Queued behind the owner's own `ReanchorPlayer`, like every other
    // same-space move.
    let mut order = Vec::new();
    let mut witnesses = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::ReanchorPlayer { .. } => order.push("reanchor"),
            CellToBaseMsg::EntityMoved {
                witness_id,
                entity_id,
                ..
            } if entity_id == pet => {
                order.push("pet_moved");
                witnesses.push(witness_id);
            }
            _ => {}
        }
    }
    assert_eq!(order.first(), Some(&"reanchor"), "{order:?}");
    witnesses.sort_unstable();
    assert_eq!(witnesses, vec![OWNER, OTHER]);
}

/// The owner's `ReanchorPlayer` never left the cell (the base channel is
/// gone): its client still shows the owner where it stood, so the pet
/// stays there too. The rest of the respawn still runs.
#[tokio::test]
async fn a_failed_same_world_reanchor_leaves_the_pet() {
    let (mut mgr, pet) = watched_pet_world();
    mgr.respawners.push(RespawnerDef {
        respawner_id: 91,
        world_name: "Agnos".to_string(),
        name: "Agnos hub".to_string(),
        pos: [200.0, 3.0, 150.0],
    });
    let start = mgr.get_entity(pet).unwrap().position;
    let (tx, rx) = mpsc::channel(256);
    drop(rx);

    handle_respawn(OWNER, 91, &tx, &mut mgr).await;

    assert_eq!(mgr.get_entity(pet).unwrap().position, start);
    assert_eq!(mgr.pets.pets_of(OWNER), vec![pet]);
}

/// The cross-world `GateTravel` never left the cell: the owner is not torn
/// out of its space with no transfer in flight, and neither is its pet.
#[tokio::test]
async fn a_failed_cross_world_respawn_keeps_the_owner_and_the_pet() {
    let (mut mgr, pet) = watched_pet_world();
    mgr.respawners.push(RespawnerDef {
        respawner_id: 90,
        world_name: "Castle".to_string(),
        name: "Castle hub".to_string(),
        pos: [5.0, 0.0, 5.0],
    });
    let (tx, rx) = mpsc::channel(256);
    drop(rx);

    handle_respawn(OWNER, 90, &tx, &mut mgr).await;

    assert!(mgr.get_entity(OWNER).is_some(), "the owner stays");
    assert!(mgr.get_entity(pet).is_some(), "so does its pet");
    assert_eq!(mgr.pets.pets_of(OWNER), vec![pet]);
}
