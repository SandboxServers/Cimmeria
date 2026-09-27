//! Pet teardown (A-31): every owner departure ends with the pet gone from
//! the world, a `LeftAoI` to whoever could see it, and an empty registry.

use tokio::sync::mpsc;

use super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::pets::pet_owner_sweep;
use cimmeria_wire::state_field::BSF_DEAD;

/// Drain `rx` into the witnesses sent `LeftAoI` for `entity`.
fn left_aoi_witnesses(rx: &mut mpsc::Receiver<CellToBaseMsg>, entity: u32) -> Vec<u32> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::LeftAoI {
            witness_id,
            entity_id,
        } = msg
        {
            if entity_id == entity {
                out.push(witness_id);
            }
        }
    }
    out.sort_unstable();
    out
}

/// A pet with its owner plus a second player (`OTHER`) watching it, after
/// one AoI tick so both witness sets hold the pet.
fn watched_pet() -> (SpaceManager, u32) {
    let (mut mgr, pet) = world_with_pet();
    add_pet_owner(&mut mgr, OTHER, "Agnos", [12.0, 0.0, 12.0], 5);
    let _ = mgr.compute_aoi_changes();
    (mgr, pet)
}

fn assert_pet_gone(mgr: &SpaceManager, pet: u32) {
    assert!(mgr.get_entity(pet).is_none(), "pet entity removed");
    assert!(mgr.pets.is_empty(), "registry emptied");
    assert!(mgr.pets.pets_of(OWNER).is_empty());
    for space in mgr.spaces.values() {
        for e in space.entities.values() {
            assert!(
                !e.witnesses.contains(&cimmeria_common::EntityId(pet as i32)),
                "no witness set still holds the pet"
            );
        }
    }
}

/// Owner disconnect despawns the pet at once (`forget_owner` from
/// `disconnect_entity`), with `LeftAoI` to the other player.
#[tokio::test]
async fn owner_disconnect_despawns_the_pet() {
    let (mut mgr, pet) = watched_pet();
    let (tx, mut rx) = mpsc::channel(64);

    mgr.disconnect_entity(OWNER, &tx).await;

    let witnesses = left_aoi_witnesses(&mut rx, pet);
    assert!(
        witnesses.contains(&OTHER),
        "the other player must be told the pet left: {witnesses:?}"
    );
    assert_pet_gone(&mgr, pet);
}

/// Owner in another space (a transfer that destroyed and re-created it
/// elsewhere, or any path PT-02 has not made explicit): the sweep despawns
/// the pet with `LeftAoI` to the player still next to it.
#[tokio::test]
async fn owner_in_another_space_is_swept() {
    let (mut mgr, pet) = watched_pet();
    let (tx, mut rx) = mpsc::channel(64);
    // Move the owner to Castle the way a transfer does: destroy, re-create.
    mgr.destroy_entity(OWNER);
    add_pet_owner(&mut mgr, OWNER, "Castle", [5.0, 0.0, 5.0], 12);
    assert!(
        mgr.get_entity(pet).is_some(),
        "destroy_entity alone leaves the pet"
    );

    let despawned = pet_owner_sweep(&tx, &mut mgr).await;

    assert_eq!(despawned, 1);
    assert_eq!(left_aoi_witnesses(&mut rx, pet), vec![OTHER]);
    assert_pet_gone(&mgr, pet);
}

/// Owner destroyed with no replacement (any sync teardown path): swept.
#[tokio::test]
async fn owner_gone_is_swept() {
    let (mut mgr, pet) = watched_pet();
    let (tx, mut rx) = mpsc::channel(64);
    mgr.destroy_entity(OWNER);

    assert_eq!(pet_owner_sweep(&tx, &mut mgr).await, 1);
    assert_eq!(left_aoi_witnesses(&mut rx, pet), vec![OTHER]);
    assert_pet_gone(&mgr, pet);
}

/// D-PT08: owner death despawns the pet.
#[tokio::test]
async fn dead_owner_is_swept() {
    let (mut mgr, pet) = watched_pet();
    let (tx, mut rx) = mpsc::channel(64);
    mgr.get_entity_mut(OWNER).unwrap().set_state_flag(BSF_DEAD);

    assert_eq!(pet_owner_sweep(&tx, &mut mgr).await, 1);
    let witnesses = left_aoi_witnesses(&mut rx, pet);
    assert_eq!(witnesses, vec![OWNER, OTHER]);
    assert_pet_gone(&mgr, pet);
}

/// Negative: a live owner beside its pet keeps it.
#[tokio::test]
async fn healthy_owner_keeps_the_pet() {
    let (mut mgr, pet) = watched_pet();
    let (tx, mut rx) = mpsc::channel(64);
    assert_eq!(pet_owner_sweep(&tx, &mut mgr).await, 0);
    assert!(left_aoi_witnesses(&mut rx, pet).is_empty());
    assert!(mgr.get_entity(pet).is_some());
    assert_eq!(mgr.pets.pets_of(OWNER), vec![pet]);
}

/// Instanced-space teardown: the owner is the last player in a
/// Castle_CellBlock instance and leaves, so the space and the pet in it are
/// destroyed. Nobody is left to see it; the registry must still be empty.
#[tokio::test]
async fn instanced_space_teardown_empties_the_registry() {
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Castle_CellBlock", [0.0; 3], 12);
    let space = mgr.get_entity_space_id(OWNER).unwrap();
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 0)
        .unwrap();
    assert_eq!(mgr.get_entity_space_id(pet), Some(space));
    let _ = mgr.compute_aoi_changes();

    mgr.destroy_entity(OWNER);

    assert!(!mgr.spaces.contains_key(&space), "instance torn down");
    assert_pet_gone(&mgr, pet);
    let (tx, _rx) = mpsc::channel(8);
    assert_eq!(
        pet_owner_sweep(&tx, &mut mgr).await,
        0,
        "nothing left to sweep"
    );
}

/// A pet removed by any other path (GM `.despawn`, a death sweep) leaves
/// the registry through `destroy_entity`.
#[tokio::test]
async fn despawning_the_pet_directly_scrubs_the_registry() {
    let (mut mgr, pet) = watched_pet();
    let (tx, _rx) = mpsc::channel(64);
    let _ = mgr.despawn_npc(pet, &tx).await;
    assert_pet_gone(&mgr, pet);
}
