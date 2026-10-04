//! Entity labels and the departed-entity ring (NT-02, Rule 6): a recycled
//! entity ID is named after whoever held the slot at the time asked about,
//! never after its current occupant.

use std::time::{Duration, SystemTime};

use cimmeria_names::{NameBook, Table};

use super::make_manager;
use crate::cell::space_manager::{EntityNames, SpaceManager, DEPARTED_RETENTION};

/// Agnos: not instanced, so destroying its last player keeps the space.
const AGNOS: u32 = 65536;
const SLOT: u32 = 100;

/// Ids no seed row uses, so the global NameBook these tests write into
/// can't collide with a real one.
const TEMPLATE_ID: i32 = 990_001;
const NAME_ID: i32 = 990_002;

fn t(secs: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000 + secs)
}

/// A player named `name` in the slot, created at `created`.
fn occupy(mgr: &mut SpaceManager, name: &str, created: SystemTime) {
    mgr.create_entity(SLOT, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(SLOT).unwrap();
    e.character_name = Some(name.to_string());
    e.created_at = created;
}

fn vacate(mgr: &mut SpaceManager, at: SystemTime) {
    mgr.departed.set_now(at);
    mgr.destroy_entity(SLOT);
}

#[test]
fn a_recycled_id_is_named_after_whoever_held_it_at_the_time() {
    let mut mgr = make_manager();
    occupy(&mut mgr, "Daniel", t(0));
    vacate(&mut mgr, t(100));
    occupy(&mut mgr, "Vala", t(100));

    assert_eq!(
        mgr.entity_label_at(AGNOS, SLOT, t(50)),
        Some("Daniel"),
        "before the recycle the slot was Daniel's: naming it after its current \
         occupant would pin Daniel's row on Vala"
    );
    assert_eq!(
        mgr.entity_label_at(AGNOS, SLOT, t(100)),
        Some("Vala"),
        "half-open lifetimes: the hand-over instant is the new occupant's"
    );
    assert_eq!(mgr.entity_label_at(AGNOS, SLOT, t(150)), Some("Vala"));
    assert_eq!(mgr.entity_label(SLOT), Some("Vala"));
    assert_eq!(
        mgr.entity_label_at(AGNOS, SLOT, t(0) - Duration::from_secs(1)),
        None,
        "before anyone held the slot it has no name"
    );
    assert_eq!(
        mgr.entity_label_at(AGNOS + 1, SLOT, t(50)),
        None,
        "another space"
    );
}

#[test]
fn a_timestamp_older_than_the_ring_is_unnamed() {
    let mut mgr = make_manager();
    occupy(&mut mgr, "Daniel", t(0));
    vacate(&mut mgr, t(10));
    // Another departure well past retention evicts Daniel's row.
    let later = t(10) + DEPARTED_RETENTION + Duration::from_secs(1);
    occupy(&mut mgr, "Vala", later - Duration::from_secs(5));
    vacate(&mut mgr, later);

    assert_eq!(mgr.entity_label_at(AGNOS, SLOT, t(5)), None);
    assert_eq!(
        mgr.entity_label_at(AGNOS, SLOT, later - Duration::from_secs(1)),
        Some("Vala"),
        "the row that evicted it is still answered from"
    );
}

/// NPC despawns are the bulk of a busy space's departures. They must not
/// push a departed player out of its ring inside the retention window.
#[test]
fn npc_despawns_do_not_evict_a_departed_player() {
    let mut mgr = make_manager();
    occupy(&mut mgr, "Daniel", t(0));
    vacate(&mut mgr, t(10));
    mgr.departed.set_now(t(20));
    for i in 0..5_000 {
        let id = 200_000 + i;
        mgr.spawn_npc(id, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        mgr.destroy_entity(id);
    }
    assert_eq!(
        mgr.entity_label_at(AGNOS, SLOT, t(5)),
        Some("Daniel"),
        "5,000 NPC rows evicted a player who left 10 s earlier"
    );
}

/// The book is process-global; install one that names the test template.
fn install_test_book() {
    let mut book = NameBook::empty();
    book.insert(
        Table::Templates,
        TEMPLATE_ID.into(),
        "NT02_Jaffa_Guard_Template",
    );
    book.insert(Table::Texts, NAME_ID.into(), "Jaffa Guard");
    book.insert_template_name_id(TEMPLATE_ID.into(), NAME_ID.into());
    cimmeria_names::global().store(book);
}

fn spawn_guard(mgr: &mut SpaceManager, id: u32) {
    mgr.spawn_npc(id, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let e = mgr.get_entity_mut(id).unwrap();
    e.template_id = Some(TEMPLATE_ID);
    e.name_id = Some(NAME_ID);
}

#[test]
fn an_npc_carries_its_display_name_and_template_pair() {
    install_test_book();
    let mut mgr = make_manager();
    spawn_guard(&mut mgr, 900);

    assert_eq!(mgr.entity_label(900), Some("Jaffa Guard"));
    assert_eq!(
        mgr.entity_names(900),
        EntityNames {
            entity_name: Some("Jaffa Guard"),
            template_id: Some(TEMPLATE_ID),
            template_name: Some("NT02_Jaffa_Guard_Template"),
        }
    );
}

#[test]
fn an_unnamed_entity_has_no_label_rather_than_a_blank_one() {
    let mut mgr = make_manager();
    occupy(&mut mgr, "", t(0));
    assert_eq!(mgr.entity_label(SLOT), None);
    assert_eq!(mgr.entity_names(SLOT), EntityNames::default());
}

/// An instance goes when its last player leaves, NPCs and all. Those NPCs
/// must still be nameable from the ring afterwards.
#[test]
fn npcs_of_a_destroyed_instance_stay_nameable() {
    install_test_book();
    let mut mgr = make_manager();
    let space_id = mgr
        .create_entity(SLOT, "SGC_W1", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.connect_entity(SLOT);
    mgr.create_entity_in_space(901, space_id, [0.0; 3], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(901).unwrap();
    e.template_id = Some(TEMPLATE_ID);
    e.name_id = Some(NAME_ID);
    let born = mgr.get_entity(901).unwrap().created_at;

    mgr.destroy_entity(SLOT);
    assert!(
        !mgr.spaces.contains_key(&space_id),
        "fixture: the instance is gone"
    );

    assert_eq!(
        mgr.entity_label_at(space_id, 901, born),
        Some("Jaffa Guard")
    );
}
