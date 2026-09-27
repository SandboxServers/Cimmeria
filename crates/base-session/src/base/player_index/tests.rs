//! D-SS13 name resolution, and the session-removing teardown paths
//! (`destroy_client_entities`, every documented reason) leaving no listing.
//! The logoff and world-entry paths are guarded next to their handlers
//! (`cimmeria-base` dispatch tests and `cimmeria-base-world-entry`).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::manager::EntityManager;

use super::*;
use crate::base::helpers::destroy_client_entities;
use crate::test_support::test_default_connected_client_state;

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

/// A session as world entry leaves it: listed, named, with a player id and
/// a player entity.
fn in_world(name: &str, player_id: i32) -> ConnectedClientState {
    let mut s = test_default_connected_client_state();
    s.player_name = Some(name.to_string());
    s.active_player_id = Some(player_id);
    s.player_entity_id = Some(1000 + player_id as u32);
    s.listed_online = true;
    s
}

fn index_of(
    sessions: Vec<(u16, ConnectedClientState)>,
) -> HashMap<SocketAddr, ConnectedClientState> {
    sessions.into_iter().map(|(p, s)| (addr(p), s)).collect()
}

fn found(port: u16, player_id: i32) -> NameLookup {
    NameLookup::Found(OnlinePlayer {
        addr: addr(port),
        player_id,
    })
}

#[test]
fn exact_name_is_found() {
    let clients = index_of(vec![(1, in_world("Lomiada", 7)), (2, in_world("Teal", 8))]);
    let index = OnlinePlayerIndex::new(&clients);
    assert_eq!(index.lookup("Lomiada"), found(1, 7));
    assert_eq!(index.lookup("Teal"), found(2, 8));
}

#[test]
fn case_folded_name_is_found_when_unique() {
    let clients = index_of(vec![(1, in_world("Lomiada", 7)), (2, in_world("Teal", 8))]);
    let index = OnlinePlayerIndex::new(&clients);
    assert_eq!(index.lookup("lomiada"), found(1, 7));
    assert_eq!(index.lookup("LOMIADA"), found(1, 7));
    assert_eq!(index.lookup("tEAL"), found(2, 8));
}

#[test]
fn exact_match_wins_over_case_fold_duplicates() {
    // `sgw_player.player_name` is UNIQUE but case-sensitive, so both exist.
    let clients = index_of(vec![(1, in_world("Bob", 1)), (2, in_world("bob", 2))]);
    let index = OnlinePlayerIndex::new(&clients);
    assert_eq!(index.lookup("Bob"), found(1, 1));
    assert_eq!(index.lookup("bob"), found(2, 2));
}

#[test]
fn case_fold_matching_two_characters_is_ambiguous() {
    let clients = index_of(vec![(1, in_world("Bob", 1)), (2, in_world("bob", 2))]);
    let index = OnlinePlayerIndex::new(&clients);
    assert_eq!(index.lookup("BOB"), NameLookup::Ambiguous);
    assert_eq!(index.lookup("bOb"), NameLookup::Ambiguous);
}

#[test]
fn two_sessions_with_the_same_exact_name_are_ambiguous() {
    // Transient (a duplicate login before the old session is reaped), but a
    // lookup must refuse rather than pick one.
    let clients = index_of(vec![(1, in_world("Bob", 1)), (2, in_world("Bob", 1))]);
    assert_eq!(
        OnlinePlayerIndex::new(&clients).lookup("Bob"),
        NameLookup::Ambiguous
    );
}

#[test]
fn missing_and_empty_names_are_not_found() {
    let clients = index_of(vec![(1, in_world("Lomiada", 7))]);
    let index = OnlinePlayerIndex::new(&clients);
    assert_eq!(index.lookup("Nobody"), NameLookup::NotFound);
    assert_eq!(index.lookup("Lomiad"), NameLookup::NotFound);
    assert_eq!(index.lookup("Lomiada "), NameLookup::NotFound);
    assert_eq!(index.lookup(""), NameLookup::NotFound);
    let empty = HashMap::new();
    assert_eq!(
        OnlinePlayerIndex::new(&empty).lookup("Lomiada"),
        NameLookup::NotFound
    );
}

#[test]
fn unlisted_or_incomplete_sessions_are_not_found() {
    let mut logging_off = in_world("Lomiada", 7);
    logging_off.listed_online = false;
    let mut char_select = in_world("Teal", 8);
    char_select.player_name = None;
    let mut no_player_id = in_world("Bratac", 9);
    no_player_id.active_player_id = None;
    let clients = index_of(vec![(1, logging_off), (2, char_select), (3, no_player_id)]);
    let index = OnlinePlayerIndex::new(&clients);
    for name in ["Lomiada", "Teal", "Bratac"] {
        assert_eq!(index.lookup(name), NameLookup::NotFound, "{name}");
    }
    assert_eq!(index.entries().count(), 0);
}

#[test]
fn lookup_online_locks_and_resolves() {
    let connected = Mutex::new(index_of(vec![(1, in_world("Lomiada", 7))]));
    assert_eq!(lookup_online(&connected, "lomiada"), found(1, 7));
    assert_eq!(lookup_online(&connected, "Teal"), NameLookup::NotFound);
}

/// Every reason `destroy_client_entities` is called with (disconnect,
/// inactivity timeout, send error i.e. a crashed client, duplicate login,
/// logoff): the session goes and its name stops resolving, while another
/// player's listing is untouched.
#[test]
fn destroy_client_entities_leaves_no_listing_for_any_reason() {
    for reason in [
        "client_disconnect",
        "inactivity_timeout",
        "send_error",
        "duplicate_login",
        "logoff",
    ] {
        let connected = Arc::new(Mutex::new(index_of(vec![
            (1, in_world("Lomiada", 7)),
            (2, in_world("Teal", 8)),
        ])));
        let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(1007u32, addr(1))])));
        assert_eq!(lookup_online(&connected, "Lomiada"), found(1, 7));

        destroy_client_entities(
            &connected,
            &entity_manager,
            addr(1),
            &None,
            &entity_to_addr,
            reason,
        );

        assert_eq!(
            lookup_online(&connected, "Lomiada"),
            NameLookup::NotFound,
            "{reason}: a torn-down session must not stay listed"
        );
        assert_eq!(
            lookup_online(&connected, "lomiada"),
            NameLookup::NotFound,
            "{reason}: nor resolve by case fold"
        );
        assert_eq!(lookup_online(&connected, "Teal"), found(2, 8), "{reason}");
        let clients = connected.lock().unwrap();
        assert_eq!(OnlinePlayerIndex::new(&clients).entries().count(), 1);
    }
}
