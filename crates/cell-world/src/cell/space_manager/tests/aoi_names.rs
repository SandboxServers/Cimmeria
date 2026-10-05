//! Rule 6 on the AoI enter/leave lines (NT-23): `aoi.entity_enter` and
//! `aoi.entity_leave` name the witness, the observee (with an NPC's
//! template pair, D-NT5) and the world, not only their IDs.
//!
//! Each assertion fails if its name field is dropped from the event: the
//! fields are looked up only inside the `tracing::enabled!` branch, so a
//! line that stops emitting one carries no other trace of it.

use cimmeria_names::{NameBook, Table};
use tracing::Level;

use super::make_manager;
use crate::test_support::LogCapture;

const PLAYER: u32 = 101;
/// Ids no seed row uses, so the global NameBook can't collide with a real one.
const TEMPLATE_ID: i32 = 990_101;
const NAME_ID: i32 = 990_102;

fn install_test_book() {
    let mut book = NameBook::empty();
    book.insert(Table::Templates, TEMPLATE_ID.into(), "NT23_Jaffa_Template");
    book.insert(Table::Texts, NAME_ID.into(), "Jaffa Guard");
    cimmeria_names::global().store(book);
}

/// A connected, named player at the origin of Agnos and a named NPC in its
/// view radius. Returns the NPC's id.
fn scene(mgr: &mut crate::cell::space_manager::SpaceManager) -> u32 {
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .stamp_log_names(Some("Daniel Jackson"), Some("djackson"));
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [5.0, 0.0, 5.0], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(npc).unwrap();
    e.template_id = Some(TEMPLATE_ID);
    e.name_id = Some(NAME_ID);
    npc
}

fn assert_names(ev: &crate::test_support::Captured, npc: u32, what: &str) {
    for (key, want) in [
        ("witness_id", PLAYER.to_string()),
        ("witness_name", "Daniel Jackson".to_string()),
        ("entity_id", npc.to_string()),
        ("entity_name", "Jaffa Guard".to_string()),
        ("template_id", TEMPLATE_ID.to_string()),
        ("template_name", "NT23_Jaffa_Template".to_string()),
        ("world", "Agnos".to_string()),
    ] {
        assert!(
            ev.has_field(key, &want),
            "{what}: expected {key}={want}; got {ev:#?}"
        );
    }
}

#[test]
fn aoi_enter_and_leave_name_the_witness_the_npc_and_the_world() {
    install_test_book();
    let capture = LogCapture::install();
    let mut mgr = make_manager();
    let npc = scene(&mut mgr);

    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes_for_player(PLAYER);
    let enter = capture
        .all()
        .into_iter()
        .find(|e| e.target == "aoi.entity_enter" && e.has_field("entity_id", &npc.to_string()))
        .expect("the NPC's AoI entry must log");
    assert_eq!(enter.level, Level::DEBUG);
    assert_names(&enter, npc, "aoi.entity_enter");

    // Far outside the leave radius: the next diff drops it from view.
    mgr.update_entity_position(npc, [2000.0, 0.0, 2000.0], [0; 3], [0.0; 3]);
    let _ = mgr.compute_aoi_changes();
    let leave = capture
        .all()
        .into_iter()
        .find(|e| e.target == "aoi.entity_leave" && e.has_field("entity_id", &npc.to_string()))
        .expect("the NPC's AoI exit must log");
    assert_names(&leave, npc, "aoi.entity_leave");

    cimmeria_names::global().store(NameBook::empty());
}
