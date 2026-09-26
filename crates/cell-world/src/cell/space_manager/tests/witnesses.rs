//! `get_witnesses_of` query — checks that the lookup is scoped to the
//! target's own space (no cross-instance bleed) and handles missing
//! entities cleanly.

use super::make_manager;

/// `get_witnesses_of` returns the player IDs whose current `witnesses`
/// set contains the target. Restricted to the target's space so
/// instanced worlds don't bleed across instances.
#[test]
fn get_witnesses_of_returns_only_players_in_the_targets_space() {
    let mut mgr = make_manager();
    // Player + NPC in Agnos.
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(900, "Agnos", [12.0, 0.0, 12.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    // Player + NPC in Castle (different startup space).
    mgr.create_entity(101, "Castle", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(901, "Castle", [12.0, 0.0, 12.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(101);

    let _ = mgr.compute_aoi_changes();

    // Player 100 sees NPC 900; player 101 sees NPC 901. Neither sees
    // the other space's NPC.
    let witnesses_900 = mgr.get_witnesses_of(900);
    assert_eq!(witnesses_900, vec![100]);
    let witnesses_901 = mgr.get_witnesses_of(901);
    assert_eq!(witnesses_901, vec![101]);
}

#[test]
fn get_witnesses_of_returns_empty_for_missing_entity() {
    let mgr = make_manager();
    assert!(mgr.get_witnesses_of(999_999).is_empty());
}
