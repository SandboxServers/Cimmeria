//! Pets PT-02 at the `.`-console travel call sites: a same-space snap
//! (`.gotoxyz`, the console travel path) and `.location` move the moved
//! player's pet beside it. One test per call site, so removing any one hook
//! fails its own test.

use cimmeria_cell_world::test_fixtures::{
    watched_pet_world, PET_FIXTURE_OTHER as OTHER, PET_FIXTURE_OWNER as OWNER,
};
use tokio::sync::mpsc;

use crate::cell::space_manager::SpaceManager;

/// The pet world with both players promoted to GameMaster.
fn gm_pet_world() -> (SpaceManager, u32) {
    let (mut mgr, pet) = watched_pet_world();
    for id in [OWNER, OTHER] {
        mgr.get_entity_mut(id).unwrap().access_level = 2;
    }
    (mgr, pet)
}

async fn run(mgr: &mut SpaceManager, caller: u32, line: &str) {
    let (tx, _rx) = mpsc::channel(256);
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    crate::cell::console::handle_console_command(caller, line, &tx, mgr, &engine).await;
}

fn assert_pet_near(mgr: &SpaceManager, pet: u32, at: [f32; 3]) {
    let owner = mgr.get_entity(OWNER).unwrap().position;
    assert!(
        (owner.x - at[0]).abs() < 1e-3 && (owner.z - at[2]).abs() < 1e-3,
        "precondition: the command moved the owner, now at {owner:?}"
    );
    let p = mgr.get_entity(pet).unwrap().position;
    assert!(
        (p.x - at[0]).abs() < 3.0 && (p.z - at[2]).abs() < 3.0,
        "pet at {p:?}, expected beside {at:?}"
    );
}

#[tokio::test]
async fn console_travel_snap_brings_the_pet_along() {
    let (mut mgr, pet) = gm_pet_world();
    run(&mut mgr, OWNER, ".gotoxyz 300 0 -40").await;
    assert_pet_near(&mgr, pet, [300.0, 0.0, -40.0]);
}

#[tokio::test]
async fn console_location_brings_the_pet_along() {
    let (mut mgr, pet) = gm_pet_world();
    // `.location` acts on the selection: another GM places the owner.
    mgr.get_entity_mut(OTHER).unwrap().current_target_id = Some(OWNER as i32);
    run(&mut mgr, OTHER, ".location 300 0 -40").await;
    assert_pet_near(&mgr, pet, [300.0, 0.0, -40.0]);
}
