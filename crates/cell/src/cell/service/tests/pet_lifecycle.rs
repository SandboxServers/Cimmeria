//! Pets PT-02 (A-31): the base's `DestroyEntity` for an owner despawns the
//! pet in the same call, before the owner's destroy, instead of leaving it to
//! the next pet sweep.

use cimmeria_cell_pets::PetsPlugin;
use cimmeria_cell_world::cell::plugin::CellPlugins;
use cimmeria_cell_world::test_fixtures::{
    assert_pet_fully_gone, drain_left_aoi_for, watched_pet_world, PET_FIXTURE_OTHER as OTHER,
    PET_FIXTURE_OWNER as OWNER,
};
use tokio::sync::mpsc;

use crate::cell::service::base_messages::lifecycle::flush_and_destroy;

#[tokio::test]
async fn destroy_entity_for_an_owner_despawns_the_pet() {
    let (mut mgr, pet) = watched_pet_world();
    // The pet despawn is the pets plugin's base-destroy hook (#962).
    mgr.install_plugins(CellPlugins::build(&[&PetsPlugin]).unwrap());
    let (tx, mut rx) = mpsc::channel(64);

    flush_and_destroy(OWNER, &tx, &mut mgr).await;

    assert!(mgr.get_entity(OWNER).is_none());
    // The owner's session is closing, so only the other player is told.
    assert_eq!(drain_left_aoi_for(&mut rx, pet), vec![OTHER]);
    assert_pet_fully_gone(&mgr, OWNER, pet);
}
