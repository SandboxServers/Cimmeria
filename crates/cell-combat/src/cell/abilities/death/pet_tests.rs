//! Pets PT-02 through the real death resolver (D-PT08): an owner's death
//! despawns its pet at once, and a pet's death leaves a corpse the pet sweep
//! removes 10 s later.

use std::time::{Duration, Instant};

use cimmeria_cell_world::cell::pets::{pet_owner_sweep_at, PET_CORPSE_DESPAWN};
use cimmeria_cell_world::test_fixtures::{
    assert_pet_fully_gone, drain_left_aoi_for, watched_pet_world, PET_FIXTURE_OTHER as OTHER,
    PET_FIXTURE_OWNER as OWNER,
};
use tokio::sync::mpsc;

use super::{kill_npc_out_of_band, resolve_death_for_test};
use crate::cell::combat::BSF_DEAD;

/// An NPC id nothing in the fixture uses: the killer.
const KILLER: u32 = 0x7000_0201;

/// The owner dies: its pet goes in the same call, not a sweep later, and
/// the owner (still in the world, in its Defeat Window) sees it leave.
#[tokio::test]
async fn owner_death_despawns_the_pet_immediately() {
    let (mut mgr, pet) = watched_pet_world();
    let (tx, mut rx) = mpsc::channel(256);

    assert!(resolve_death_for_test(OWNER, KILLER, &tx, &mut mgr).await);

    assert_eq!(drain_left_aoi_for(&mut rx, pet), vec![OWNER, OTHER]);
    assert_pet_fully_gone(&mgr, OWNER, pet);
}

/// The pet dies: it stays as a corpse (no NPC respawn is armed), and the
/// pet sweep despawns it once `PET_CORPSE_DESPAWN` has passed.
#[tokio::test]
async fn pet_death_leaves_a_corpse_that_despawns_after_the_timer() {
    let (mut mgr, pet) = watched_pet_world();
    let (tx, mut rx) = mpsc::channel(256);

    assert!(kill_npc_out_of_band(pet, KILLER, false, false, &tx, &mut mgr).await);
    let corpse = mgr.get_entity(pet).expect("a dead pet stays as a corpse");
    assert!(corpse.state_field & BSF_DEAD != 0);
    assert!(
        corpse.respawn_at.is_none(),
        "a pet never respawns as an NPC"
    );
    let _ = drain_left_aoi_for(&mut rx, pet);

    let t0 = Instant::now();
    assert_eq!(pet_owner_sweep_at(t0, &tx, &mut mgr).await, 0);
    assert_eq!(
        pet_owner_sweep_at(
            t0 + PET_CORPSE_DESPAWN - Duration::from_millis(1),
            &tx,
            &mut mgr
        )
        .await,
        0
    );
    assert!(mgr.get_entity(pet).is_some());
    assert_eq!(
        pet_owner_sweep_at(t0 + PET_CORPSE_DESPAWN, &tx, &mut mgr).await,
        1
    );
    assert_eq!(drain_left_aoi_for(&mut rx, pet), vec![OWNER, OTHER]);
    assert_pet_fully_gone(&mgr, OWNER, pet);
    assert!(mgr.get_entity(OWNER).is_some(), "the owner is untouched");
}
