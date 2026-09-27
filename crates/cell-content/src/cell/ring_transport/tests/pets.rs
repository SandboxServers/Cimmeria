//! Pets PT-02 (A-31) on the ring: a same-world passenger's pet appears
//! beside it when the passenger reappears at the destination
//! (`Effect::ShowPlayer`), not when the passenger is moved while still
//! hidden (`Effect::TeleportPlayer`). A cross-world passenger's pet stays
//! behind, despawned before the passenger's destroy.

use cimmeria_cell_world::test_fixtures::{
    assert_pet_fully_gone, drain_entity_moved_for, drain_left_aoi_for, watched_pet_world,
    PET_FIXTURE_OTHER as OTHER, PET_FIXTURE_OWNER as OWNER,
};

use super::*;
use crate::cell::ring_transport::transporter::Effect;

const DEST: [f32; 3] = [400.0, 0.0, 300.0];

#[tokio::test]
async fn same_world_ring_moves_the_pet_when_the_owner_reappears() {
    let (mut mgr, pet) = watched_pet_world();
    let start = mgr.get_entity(pet).unwrap().position;
    let (tx, mut rx) = mpsc::channel(256);
    let engine = ChainEngine::new();

    dispatch_effects(
        vec![
            Effect::HidePlayer { entity_id: OWNER },
            Effect::TeleportPlayer {
                entity_id: OWNER,
                position: DEST,
                world_name: "Agnos".to_string(),
                destination_region_id: 2,
            },
        ],
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    assert_eq!(
        mgr.get_entity(pet).unwrap().position,
        start,
        "the pet must not arrive while its owner is still hidden"
    );
    assert!(drain_entity_moved_for(&mut rx, pet).is_empty());

    dispatch_effects(
        vec![Effect::ShowPlayer { entity_id: OWNER }],
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    let p = mgr.get_entity(pet).unwrap().position;
    assert!(
        (p.x - DEST[0]).abs() < 3.0 && (p.z - DEST[2]).abs() < 3.0,
        "pet beside the arrived owner: {p:?}"
    );
    let witnesses: Vec<u32> = drain_entity_moved_for(&mut rx, pet)
        .into_iter()
        .map(|m| m.0)
        .collect();
    assert_eq!(witnesses, vec![OWNER, OTHER]);
}

#[tokio::test]
async fn cross_world_ring_despawns_the_pet() {
    let (mut mgr, pet) = watched_pet_world();
    let (tx, mut rx) = mpsc::channel(256);
    let engine = ChainEngine::new();

    dispatch_effects(
        vec![Effect::TeleportCrossWorld {
            entity_id: OWNER,
            position: [5.0, 0.0, 5.0],
            world_name: "Castle".to_string(),
            destination_region_id: 2,
        }],
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(mgr.get_entity(OWNER).is_none(), "the passenger left");
    assert_eq!(drain_left_aoi_for(&mut rx, pet), vec![OTHER]);
    assert_pet_fully_gone(&mgr, OWNER, pet);
}
