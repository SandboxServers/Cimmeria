//! Pets PT-02 (A-31): a cross-space transfer (GM `.goto`/`.summon`/
//! `.gotolocation`/`.gotospace` to another space) leaves the pet behind,
//! despawned before the traveller is torn out, and a rejected transfer
//! leaves it alone.

use cimmeria_cell_world::test_fixtures::{
    assert_pet_fully_gone, drain_left_aoi_for, watched_pet_world, PET_FIXTURE_OTHER as OTHER,
    PET_FIXTURE_OWNER as OWNER,
};

use super::*;

#[tokio::test]
async fn transfer_to_another_world_despawns_the_pet() {
    let (mut mgr, pet) = watched_pet_world();
    let (tx, mut rx) = mpsc::channel(64);

    transfer_player_to_space(
        OWNER,
        &TransferDestination::in_world(CASTLE, [5.0, 0.0, 5.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect("transfer accepted");

    assert_eq!(drain_left_aoi_for(&mut rx, pet), vec![OTHER]);
    assert_pet_fully_gone(&mgr, OWNER, pet);
}

/// Negative: a refused transfer (unknown world) changes nothing, the pet
/// included.
#[tokio::test]
async fn rejected_transfer_keeps_the_pet() {
    let (mut mgr, pet) = watched_pet_world();
    let (tx, mut rx) = mpsc::channel(64);

    let result = transfer_player_to_space(
        OWNER,
        &TransferDestination::in_world("NoSuchWorld", [5.0, 0.0, 5.0]),
        &tx,
        &mut mgr,
    )
    .await;

    assert!(result.is_err());
    assert!(drain_left_aoi_for(&mut rx, pet).is_empty());
    assert_eq!(mgr.pets.pets_of(OWNER), vec![pet]);
}
