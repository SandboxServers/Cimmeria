//! Pets PT-02 at the stargate call site: a traveller's pet stays behind
//! (D-PT01), despawned in `perform_gate_travel` once the `GateTravel` send
//! is confirmed; a refused transfer keeps it.

use cimmeria_cell_world::test_fixtures::{
    add_pet_owner, assert_pet_fully_gone, seed_pet_template, PET_FIXTURE_TEMPLATE_ID,
};
use tokio::sync::mpsc;

use super::super::handle_dial_gate;
use super::{engine, grant_all_addresses, make_manager_with_stargates, strip_stargate_regions};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

const OWNER: u32 = 1;
const OTHER: u32 = 2;

/// Agnos with no gate volume (the dial travels at once), an owner with a
/// pet, and a second player who sees the pet.
fn fixture() -> (SpaceManager, u32) {
    let mut mgr = make_manager_with_stargates();
    strip_stargate_regions(&mut mgr);
    seed_pet_template(&mut mgr, PET_FIXTURE_TEMPLATE_ID);
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    grant_all_addresses(&mut mgr, OWNER);
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 0)
        .unwrap();
    add_pet_owner(&mut mgr, OTHER, "Agnos", [12.0, 0.0, 12.0], 5);
    let _ = mgr.compute_aoi_changes();
    (mgr, pet)
}

#[tokio::test]
async fn gate_travel_despawns_the_pet() {
    let (mut mgr, pet) = fixture();
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_dial_gate(OWNER, 2, 0, &tx, &mut mgr, &engine()).await);

    let msgs: Vec<CellToBaseMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(msgs.iter().any(|m| matches!(
        m,
        CellToBaseMsg::GateTravel {
            entity_id: OWNER,
            ..
        }
    )));
    let left: Vec<u32> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::LeftAoI {
                witness_id,
                entity_id,
            } if *entity_id == pet => Some(*witness_id),
            _ => None,
        })
        .collect();
    assert_eq!(
        left,
        vec![OTHER],
        "the traveller gets no LeftAoI, the other player does"
    );
    assert_pet_fully_gone(&mgr, OWNER, pet);
}

/// The base channel is gone, so no `GateTravel` is enqueued: the owner stays
/// and so does its pet.
#[tokio::test]
async fn a_failed_gate_travel_keeps_the_pet() {
    let (mut mgr, pet) = fixture();
    let (tx, rx) = mpsc::channel(256);
    drop(rx);

    handle_dial_gate(OWNER, 2, 0, &tx, &mut mgr, &engine()).await;

    assert!(mgr.get_entity(OWNER).is_some(), "the owner stays");
    assert!(mgr.get_entity(pet).is_some(), "so does the pet");
    assert_eq!(mgr.pets.pets_of(OWNER), vec![pet]);
}
