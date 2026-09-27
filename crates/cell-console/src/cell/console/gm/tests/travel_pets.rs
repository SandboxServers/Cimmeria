//! Pets PT-02 at the native GM travel call sites: `gmGotoXYZ`, `gmGoto` and
//! `gmSummon` move the moved player's pet beside it; `gmGotoLocation` leaves
//! it behind, despawned. One test per call site, so removing any one hook
//! fails its own test.

use super::*;
use crate::cell::messages::CellToBaseMsg;
use cimmeria_cell_world::test_fixtures::{
    assert_pet_fully_gone, watched_pet_world, PET_FIXTURE_OTHER as OTHER,
    PET_FIXTURE_OWNER as OWNER,
};
use tokio::sync::mpsc;

/// The pet world with both players promoted to GameMaster.
fn gm_pet_world() -> (SpaceManager, u32) {
    let (mut mgr, pet) = watched_pet_world();
    for id in [OWNER, OTHER] {
        mgr.get_entity_mut(id).unwrap().access_level = 2;
    }
    (mgr, pet)
}

fn assert_pet_near(mgr: &SpaceManager, pet: u32, at: [f32; 3]) {
    let p = mgr.get_entity(pet).unwrap().position;
    assert!(
        (p.x - at[0]).abs() < 3.0 && (p.z - at[2]).abs() < 3.0,
        "pet at {p:?}, expected beside {at:?}"
    );
}

#[tokio::test]
async fn gm_goto_xyz_brings_the_pet_along() {
    let (mut mgr, pet) = gm_pet_world();
    let (tx, _rx) = mpsc::channel(256);
    let mut args = Vec::new();
    for c in [300.0f32, 0.0, -40.0] {
        args.extend_from_slice(&c.to_le_bytes());
    }
    assert!(dispatch(OWNER, GM_GOTO_XYZ, &args, &tx, &mut mgr, &test_engine()).await);
    assert_pet_near(&mgr, pet, [300.0, 0.0, -40.0]);
}

#[tokio::test]
async fn gm_goto_brings_the_pet_along() {
    let (mut mgr, pet) = gm_pet_world();
    mgr.update_position_preserving_facing(OTHER, [200.0, 0.0, 200.0], [0.0; 3]);
    let (tx, _rx) = mpsc::channel(256);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, &OTHER.to_string());
    assert!(dispatch(OWNER, GM_GOTO, &args, &tx, &mut mgr, &test_engine()).await);
    assert_pet_near(&mgr, pet, [200.0, 0.0, 200.0]);
}

#[tokio::test]
async fn gm_summon_brings_the_summoned_players_pet_along() {
    let (mut mgr, pet) = gm_pet_world();
    mgr.update_position_preserving_facing(OTHER, [200.0, 0.0, 200.0], [0.0; 3]);
    let (tx, _rx) = mpsc::channel(256);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, &OWNER.to_string());
    assert!(dispatch(OTHER, GM_SUMMON, &args, &tx, &mut mgr, &test_engine()).await);
    assert_pet_near(&mgr, pet, [200.0, 0.0, 200.0]);
}

#[tokio::test]
async fn gm_goto_location_despawns_the_pet() {
    let (mut mgr, pet) = gm_pet_world();
    let (tx, mut rx) = mpsc::channel(256);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "Castle");
    for c in [5.0f32, 0.0, 5.0] {
        args.extend_from_slice(&c.to_le_bytes());
    }
    assert!(
        dispatch(
            OWNER,
            GM_GOTO_LOCATION,
            &args,
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
    assert!(drain(&mut rx).iter().any(|m| matches!(
        m,
        CellToBaseMsg::GateTravel {
            entity_id: OWNER,
            ..
        }
    )));
    assert_pet_fully_gone(&mgr, OWNER, pet);
}
