//! Phase filter in the AoI (PH-01b): a phased entity is introduced to, and
//! kept in the view of, only the players who hold its phase.

use cimmeria_entity::cell_entity::{PhaseKey, PhaseState};

use super::{make_manager, SpaceManager};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::PhaseError;

const A: u32 = 100;
const B: u32 = 101;
const PROP: u32 = 900;

fn key() -> PhaseKey {
    PhaseKey::new("tollana_nest").unwrap()
}

/// Two connected players and a prop 10 m away, the prop in `key()`'s phase.
fn world() -> SpaceManager {
    let mut mgr = make_manager();
    mgr.create_entity(A, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(B, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(A);
    mgr.connect_entity(B);
    mgr.spawn_npc(PROP, "Agnos", [20.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(PROP)
        .unwrap()
        .extensions
        .insert(PhaseState {
            entity_phase: Some(key()),
            ..Default::default()
        });
    mgr
}

fn entered(events: &[CellToBaseMsg], witness: u32, entity: u32) -> usize {
    events
        .iter()
        .filter(|e| {
            matches!(e, CellToBaseMsg::EnteredAoI { witness_id, entity_id, .. }
                if *witness_id == witness && *entity_id == entity)
        })
        .count()
}

fn left(events: &[CellToBaseMsg], witness: u32, entity: u32) -> usize {
    events
        .iter()
        .filter(|e| {
            matches!(e, CellToBaseMsg::LeftAoI { witness_id, entity_id }
                if *witness_id == witness && *entity_id == entity)
        })
        .count()
}

#[test]
fn phased_prop_is_not_introduced_without_the_phase() {
    let mut mgr = world();
    let events = mgr.compute_aoi_changes();
    assert_eq!(entered(&events, A, PROP), 0, "A does not hold the phase");
    assert!(mgr.get_witnesses_of(PROP).is_empty());
}

#[test]
fn joining_the_phase_introduces_once() {
    let mut mgr = world();
    mgr.compute_aoi_changes();
    let events = mgr.set_player_phase(A, &key(), true).unwrap();
    assert_eq!(entered(&events, A, PROP), 1);
    let all_enters = events
        .iter()
        .filter(|e| matches!(e, CellToBaseMsg::EnteredAoI { .. }))
        .count();
    assert_eq!(all_enters, 1, "only the prop enters: {events:?}");
    assert!(mgr.set_player_phase(A, &key(), true).unwrap().is_empty());
    for _ in 0..10 {
        assert_eq!(entered(&mgr.compute_aoi_changes(), A, PROP), 0);
    }
}

#[test]
fn leaving_the_phase_sends_one_leave() {
    let mut mgr = world();
    mgr.set_player_phase(A, &key(), true).unwrap();
    mgr.compute_aoi_changes();
    assert!(mgr.get_witnesses_of(PROP).contains(&A));
    let events = mgr.set_player_phase(A, &key(), false).unwrap();
    assert_eq!(left(&events, A, PROP), 1);
    assert!(!mgr.get_witnesses_of(PROP).contains(&A));
    assert_eq!(left(&mgr.compute_aoi_changes(), A, PROP), 0);
    // Re-gaining the phase is a fresh create, exactly one EnteredAoI.
    let events = mgr.set_player_phase(A, &key(), true).unwrap();
    assert_eq!(entered(&events, A, PROP), 1, "re-gain: {events:?}");
    assert_eq!(left(&events, A, PROP), 0);
}

#[test]
fn two_players_see_only_their_phase() {
    let mut mgr = world();
    mgr.set_player_phase(A, &key(), true).unwrap();
    for _ in 0..50 {
        mgr.compute_aoi_changes();
    }
    assert_eq!(mgr.get_witnesses_of(PROP), vec![A]);
    assert!(mgr.get_witnesses_of(A).contains(&B));
    assert!(mgr.get_witnesses_of(B).contains(&A));
}

#[test]
fn unphased_entities_are_unaffected() {
    let mut mgr = world();
    mgr.spawn_npc(901, "Agnos", [10.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    let events = mgr.compute_aoi_changes();
    assert_eq!(entered(&events, A, 901), 1);
    assert_eq!(entered(&events, B, 901), 1);
}

#[test]
fn set_player_phase_rejects_non_players() {
    let mut mgr = world();
    assert_eq!(
        mgr.set_player_phase(PROP, &key(), true).unwrap_err(),
        PhaseError::NotAPlayer
    );
    assert_eq!(
        mgr.set_player_phase(999_999, &key(), true).unwrap_err(),
        PhaseError::PlayerMissing
    );
}
