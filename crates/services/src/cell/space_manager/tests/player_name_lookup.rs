//! `find_online_player_by_name` — the P44 exact-match online-player-name
//! resolver that `.goto` / `.summon` (P46) build on.
//!
//! Legacy reference: `deprecated/python/cell/commands/Player.py:298-341`
//! probes `name in PlayersByName` (a raw Python dict-key test, hence
//! case-sensitive and exact) and then separately checks that the resolved
//! player is bound to a space. These tests pin both halves of that
//! contract plus the character-name uniqueness defense.

use super::make_manager;
use crate::cell::space_manager::PlayerNameLookup;

/// Create a connected player in `world` and cache `name` on it, the same
/// way `BaseToCellMsg::InitPlayerState` does at world entry.
fn spawn_named_player(
    mgr: &mut crate::cell::space_manager::SpaceManager,
    entity_id: u32,
    world: &str,
    name: &str,
) -> u32 {
    let space_id = mgr
        .create_entity(entity_id, world, [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(entity_id);
    mgr.get_entity_mut(entity_id).unwrap().character_name = Some(name.to_string());
    space_id
}

/// Happy path: an exact name resolves to the right entity id *and* the
/// space id of the instance that player is actually in — not merely "a"
/// space of the right world. P45's transfer primitive depends on that
/// exactness for instanced worlds.
#[test]
fn exact_name_resolves_to_entity_and_its_actual_space() {
    let mut mgr = make_manager();
    let agnos_space = spawn_named_player(&mut mgr, 100, "Agnos", "Daniel");
    let castle_space = spawn_named_player(&mut mgr, 101, "Castle", "Teal'c");
    assert_ne!(
        agnos_space, castle_space,
        "fixture precondition: the two players must be in different spaces"
    );

    assert_eq!(
        mgr.find_online_player_by_name("Daniel"),
        PlayerNameLookup::Found {
            entity_id: 100,
            space_id: agnos_space,
        }
    );
    assert_eq!(
        mgr.find_online_player_by_name("Teal'c"),
        PlayerNameLookup::Found {
            entity_id: 101,
            space_id: castle_space,
        }
    );
}

/// The scan is CellApp-wide: a player in an *instanced* space, which the
/// caller has no way to name up front, is still resolvable. Same scope as
/// P04's `.players` fix (`all_player_entity_ids` walks `self.spaces`).
#[test]
fn lookup_spans_every_loaded_space_including_instances() {
    let mut mgr = make_manager();
    // "Castle_CellBlock" is instanced in the test spaces XML, so this
    // allocates a fresh space rather than reusing a startup one.
    let instance_space = spawn_named_player(&mut mgr, 200, "Castle_CellBlock", "Vala");
    assert!(
        !mgr.all_spaces()
            .iter()
            .any(|(sid, w)| *sid == instance_space && w == "Agnos"),
        "fixture precondition: the instanced space must be distinct from the startup spaces"
    );

    assert_eq!(
        mgr.find_online_player_by_name("Vala"),
        PlayerNameLookup::Found {
            entity_id: 200,
            space_id: instance_space,
        }
    );
}

/// Case-sensitivity: legacy's `name in PlayersByName` is a Python dict-key
/// test on `str`, so `"daniel"` is simply a different key from `"Daniel"`.
/// No evidence anywhere in `Player.py` / `SGWPlayer.py` normalizes case,
/// so a case-mismatch is a miss, not a hit.
#[test]
fn case_mismatch_and_near_miss_do_not_resolve() {
    let mut mgr = make_manager();
    spawn_named_player(&mut mgr, 100, "Agnos", "Daniel");

    for probe in [
        "daniel", "DANIEL", "DanieL", "Danie", "Daniell", "Daniel ", " Daniel",
    ] {
        assert_eq!(
            mgr.find_online_player_by_name(probe),
            PlayerNameLookup::NotFound,
            "{probe:?} must not resolve -- exact, case-sensitive match only"
        );
    }
}

/// A name nobody online carries is `NotFound`, which P46 maps to legacy's
/// `"Player is not available on this CellApp"`.
#[test]
fn unknown_name_is_not_found() {
    let mut mgr = make_manager();
    spawn_named_player(&mut mgr, 100, "Agnos", "Daniel");

    assert_eq!(
        mgr.find_online_player_by_name("Mitchell"),
        PlayerNameLookup::NotFound
    );
    assert_eq!(
        mgr.find_online_player_by_name(""),
        PlayerNameLookup::NotFound
    );
}

/// NPCs are invisible to this lookup: their display name lives on
/// `npc_name`, and `character_name` stays `None`. A name-matching probe
/// against an NPC's display name must not resolve, or `.summon` would try
/// to drive a cross-world transfer on an entity with no client.
#[test]
fn npc_display_names_are_not_matched() {
    let mut mgr = make_manager();
    mgr.create_entity(900, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(900).unwrap().npc_name = Some("Jaffa Guard".to_string());

    assert_eq!(
        mgr.find_online_player_by_name("Jaffa Guard"),
        PlayerNameLookup::NotFound
    );
}

/// The two failure shapes are distinct. A named entity that is no longer
/// registered in its space's player set is `InTransition`, not `NotFound`
/// — legacy's `destination.space is None` branch, which produces a
/// different GM error string (`"Player is not on any reachable space"`).
///
/// The state is built the same way `disconnect_entity` builds it: remove
/// the id from `space.players` while the named entity is still in
/// `space.entities`.
#[test]
fn named_player_absent_from_player_set_is_in_transition_not_not_found() {
    let mut mgr = make_manager();
    let space_id = spawn_named_player(&mut mgr, 100, "Agnos", "Daniel");
    assert_eq!(
        mgr.find_online_player_by_name("Daniel"),
        PlayerNameLookup::Found {
            entity_id: 100,
            space_id,
        },
        "precondition: resolvable while registered as a player"
    );

    // Mid-teardown / unbound: `disconnect_entity` does exactly this
    // before `destroy_entity` removes the entity itself.
    mgr.spaces.get_mut(&space_id).unwrap().players.remove(&100);

    assert_eq!(
        mgr.find_online_player_by_name("Daniel"),
        PlayerNameLookup::InTransition { entity_id: 100 },
        "an unbound-but-named player must be distinguishable from an unknown name"
    );
}

/// Defense-in-depth for the character-name uniqueness invariant: if it is
/// ever violated, the lookup must not silently resolve to one of the
/// candidates. The scan walks `HashMap`s, so "first match" is unstable
/// across runs — a nondeterministic `.goto` would teleport a GM to a
/// different player on each invocation.
#[test]
fn duplicate_names_are_reported_not_silently_resolved() {
    let mut mgr = make_manager();
    spawn_named_player(&mut mgr, 100, "Agnos", "Daniel");
    spawn_named_player(&mut mgr, 101, "Castle", "Daniel");

    assert_eq!(
        mgr.find_online_player_by_name("Daniel"),
        PlayerNameLookup::Ambiguous {
            entity_ids: vec![100, 101],
        },
        "duplicate names must surface as Ambiguous with sorted ids"
    );
}

/// The ambiguity check fires before the in-space/in-transition split, so a
/// duplicate is reported even when only one of the candidates is actually
/// reachable. Picking "the reachable one" would be a silent resolution of
/// a broken invariant, and would still be arbitrary if both were bound.
#[test]
fn duplicate_names_are_reported_even_when_only_one_is_in_a_space() {
    let mut mgr = make_manager();
    let agnos_space = spawn_named_player(&mut mgr, 100, "Agnos", "Daniel");
    spawn_named_player(&mut mgr, 101, "Castle", "Daniel");
    mgr.spaces
        .get_mut(&agnos_space)
        .unwrap()
        .players
        .remove(&100);

    assert_eq!(
        mgr.find_online_player_by_name("Daniel"),
        PlayerNameLookup::Ambiguous {
            entity_ids: vec![100, 101],
        }
    );
}

/// The lookup is read-only: it must not mutate space or entity state.
/// Cheap to assert and cheap to break, since it takes `&self` today but
/// P45/P46 will call it from mutating command paths.
#[test]
fn lookup_is_non_mutating() {
    let mut mgr = make_manager();
    let space_id = spawn_named_player(&mut mgr, 100, "Agnos", "Daniel");
    let before = mgr.get_entity(100).unwrap().position;
    let players_before = mgr.spaces[&space_id].players.clone();

    let _ = mgr.find_online_player_by_name("Daniel");
    let _ = mgr.find_online_player_by_name("Nobody");

    assert_eq!(mgr.get_entity(100).unwrap().position, before);
    assert_eq!(mgr.spaces[&space_id].players, players_before);
    assert_eq!(mgr.get_entity_space_id(100), Some(space_id));
}
