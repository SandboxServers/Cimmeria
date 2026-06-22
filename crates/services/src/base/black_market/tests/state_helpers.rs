//! Non-DB unit tests for the small leftover branches:
//! - `mod.rs` session-map resolvers (`player_name_for_entity`,
//!   `player_name_for_player_id`, `entity_id_for_player_id`),
//! - `validate.rs` negative-price rejection,
//! - `types.rs` invalid-UTF-8 string-body rejection.
//!
//! These need no database — they exercise pure logic and in-memory maps.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use crate::base::black_market::types::{auction_status, AuctionRow, BMSearchOptions};
use crate::base::black_market::validate::validate_create;
use crate::base::black_market::wire::error_code;
use crate::base::black_market::{
    entity_id_for_player_id, player_name_for_entity, player_name_for_player_id,
};
use crate::base::ConnectedClientState;
use crate::test_support::test_default_connected_client_state;

const ADDR: &str = "127.0.0.1:40000";

/// Build a `connected` map with one online session carrying a name, a DB
/// player_id, and an entity_id, plus its `entity_to_addr` mapping.
fn one_session(
    entity_id: u32,
    player_id: i32,
    name: &str,
) -> (
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let addr: SocketAddr = ADDR.parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_name = Some(name.to_string());
    state.active_player_id = Some(player_id);
    state.player_entity_id = Some(entity_id);

    let connected = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(addr, state);
        m
    }));
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(entity_id, addr);
        m
    }));
    (connected, entity_to_addr)
}

/// `player_name_for_entity` resolves through entity_to_addr → connected →
/// player_name for an online entity.
#[test]
fn player_name_for_entity_resolves_online() {
    let (connected, e2a) = one_session(42, 100, "Carter");
    assert_eq!(player_name_for_entity(42, &connected, &e2a), "Carter");
}

/// An entity with no address mapping yields an empty name (the cosmetic
/// fallback), not a panic.
#[test]
fn player_name_for_entity_unknown_entity_is_empty() {
    let (connected, e2a) = one_session(42, 100, "Carter");
    assert_eq!(
        player_name_for_entity(999, &connected, &e2a),
        "",
        "an unmapped entity resolves to the empty fallback name"
    );
}

/// `player_name_for_player_id` finds the session by DB player_id.
#[test]
fn player_name_for_player_id_resolves_online() {
    let (connected, e2a) = one_session(42, 100, "Carter");
    assert_eq!(
        player_name_for_player_id(100, &connected, &e2a),
        Some("Carter".to_string())
    );
}

/// An offline player_id resolves to `None`.
#[test]
fn player_name_for_player_id_offline_is_none() {
    let (connected, e2a) = one_session(42, 100, "Carter");
    assert_eq!(player_name_for_player_id(999, &connected, &e2a), None);
}

/// `entity_id_for_player_id` maps the DB player_id back to the live entity id.
#[test]
fn entity_id_for_player_id_resolves_online() {
    let (connected, _e2a) = one_session(42, 100, "Carter");
    assert_eq!(entity_id_for_player_id(100, &connected), Some(42));
}

/// An offline player_id has no live entity.
#[test]
fn entity_id_for_player_id_offline_is_none() {
    let (connected, _e2a) = one_session(42, 100, "Carter");
    assert_eq!(entity_id_for_player_id(999, &connected), None);
}

/// `validate_create` rejects a negative starting or buyout price even when the
/// item was owned (escrow succeeded). Bug shape: dropping the price-bound check
/// would let a seller list at a negative price. The existing validate tests
/// cover the not-owned branch but never the price branch.
#[test]
fn validate_create_rejects_negative_prices() {
    assert_eq!(
        validate_create(true, -1, 0).unwrap_err(),
        error_code::INVALID_ITEM,
        "negative starting price rejected"
    );
    assert_eq!(
        validate_create(true, 0, -1).unwrap_err(),
        error_code::INVALID_ITEM,
        "negative buyout price rejected"
    );
    // Sanity: a zero/positive pair with an owned item is accepted.
    assert!(validate_create(true, 0, 0).is_ok());
}

/// A `BMSearchOptions` whose string body is not valid UTF-8 must fail to
/// deserialize (return `None`), not panic or lossily decode. Pins the
/// `from_utf8(...).ok()?` branch in `read_string`.
#[test]
fn search_options_invalid_utf8_body_returns_none() {
    let mut buf = Vec::new();
    buf.push(0u8); // sort_id
    buf.extend_from_slice(&0i32.to_le_bytes()); // client_key
    buf.extend_from_slice(&0i32.to_le_bytes()); // sequence_id
    buf.push(0u8); // b_forward
                   // seller_name: declared length 2, body is invalid UTF-8 (0xFF 0xFE).
    buf.extend_from_slice(&2u32.to_le_bytes());
    buf.extend_from_slice(&[0xFF, 0xFE]);
    // Remaining fields would follow, but the bad UTF-8 must short-circuit first.
    buf.extend_from_slice(&0u32.to_le_bytes()); // bidder_name (empty)
    buf.extend_from_slice(&0u32.to_le_bytes()); // item_name (empty)
    for _ in 0..4 {
        buf.extend_from_slice(&0i32.to_le_bytes());
    }

    assert!(
        BMSearchOptions::from_wire(&buf).is_none(),
        "invalid UTF-8 in a string body must reject the whole message"
    );
}

/// Sanity check on the status constants used across the suite — guards against
/// an accidental renumber that would silently break every status assertion.
#[test]
fn auction_status_constants_are_stable() {
    assert_eq!(auction_status::ACTIVE, 0);
    assert_eq!(auction_status::SOLD, 1);
    assert_eq!(auction_status::CANCELLED, 2);
    assert_eq!(auction_status::EXPIRED, 3);
    // Touch AuctionRow's Default-free construction path indirectly: a row with
    // an explicit ACTIVE status is what validate_create's callers build on.
    let _ = AuctionRow {
        sequence_id: 1,
        seller_id: 1,
        item_id: 1,
        item_def_id: 1,
        stack_size: 1,
        durability: 0,
        charges: 0,
        starting_price: 0,
        buyout_price: 0,
        current_bid: 0,
        current_bidder: None,
        auction_length: 0,
        created_at: 0,
        expires_at: 0,
        status: auction_status::ACTIVE,
    };
}
