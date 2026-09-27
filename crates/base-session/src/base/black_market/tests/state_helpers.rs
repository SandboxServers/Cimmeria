//! Non-DB unit tests: the online-player lookup the settlement notices use,
//! the invalid-UTF-8 search decode, and the handlers' telemetry when the
//! server has no database (`BMUnavailable`).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use cimmeria_wire::black_market::{BMError, Decode};

use crate::base::black_market::send::BmNet;
use crate::base::black_market::types::{auction_status, BMSearchOptions};
use crate::base::black_market::{bid, cancel, create, search};
use crate::base::ConnectedClientState;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};

type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;
type EntityToAddr = Arc<Mutex<HashMap<u32, SocketAddr>>>;

/// One online session with an account, a DB player_id and an entity id.
fn one_session(entity_id: u32, player_id: i32, account_id: u32) -> (Connected, EntityToAddr) {
    let addr: SocketAddr = "127.0.0.1:40000".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.account_id = account_id;
    state.active_player_id = Some(player_id);
    state.player_entity_id = Some(entity_id);
    (
        Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
        Arc::new(Mutex::new(HashMap::from([(entity_id, addr)]))),
    )
}

/// `BmNet::entity_of` finds an online player's entity and `None` offline;
/// the sweep uses it to decide who gets `onBMAuctionRemove`.
#[test]
fn entity_of_resolves_online_players_only() {
    let (connected, e2a) = one_session(42, 100, 7);
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let net = BmNet {
        transport: &transport,
        connected: &connected,
        entity_to_addr: &e2a,
    };
    assert_eq!(net.entity_of(100), Some(42));
    assert_eq!(net.entity_of(999), None);
}

/// A `BMSearchOptions` whose string body is not valid UTF-8 must fail to
/// decode, not panic or lossily decode.
#[test]
fn search_options_invalid_utf8_body_is_refused() {
    let mut buf = vec![0u8];
    buf.extend_from_slice(&0i32.to_le_bytes()); // clientKey
    buf.extend_from_slice(&0i32.to_le_bytes()); // sequenceId
    buf.push(0); // bForward
    buf.extend_from_slice(&2u32.to_le_bytes());
    buf.extend_from_slice(&[0xFF, 0xFE]); // sellerName: invalid UTF-8
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes());
    for _ in 0..4 {
        buf.extend_from_slice(&0i32.to_le_bytes());
    }
    assert!(BMSearchOptions::decode(&buf).is_err());
}

#[test]
fn auction_status_constants_are_stable() {
    assert_eq!(auction_status::ACTIVE, 0);
    assert_eq!(auction_status::SOLD, 1);
    assert_eq!(auction_status::CANCELLED, 2);
    assert_eq!(auction_status::EXPIRED, 3);
}

/// With no database every handler refuses with `BMUnavailable`: the
/// refusal row carries `reason`, `error_id` and the actor (`account_id`
/// from the session, `player_id`), each handler span records the account,
/// and each press is answered with `onBMError`. Bug shape: the branch
/// returned silently, so the button did nothing.
#[tokio::test]
async fn no_database_refuses_every_request_visibly() {
    const ENTITY: u32 = 0x7000_A9F1;
    const PLAYER: i32 = 0x7000_A0F1;
    const ACCOUNT: u32 = 0x7000_A0F2;

    let capture = LogCapture::install();
    let (connected, e2a) = one_session(ENTITY, PLAYER, ACCOUNT);
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());

    search::handle_search(
        ENTITY,
        PLAYER,
        BMSearchOptions::default(),
        &None,
        &transport,
        &connected,
        &e2a,
    )
    .await;
    create::handle_create_auction(
        ENTITY, PLAYER, 1, 10, 20, 3, &None, &transport, &connected, &e2a,
    )
    .await;
    bid::handle_place_bid(ENTITY, PLAYER, 1, 10, &None, &transport, &connected, &e2a).await;
    cancel::handle_cancel_auction(ENTITY, PLAYER, 1, &None, &transport, &connected, &e2a).await;

    let refusals: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|e| e.message_contains("Black Market request refused"))
        .collect();
    assert_eq!(refusals.len(), 4, "{:#?}", capture.all());
    for (ev, op) in refusals.iter().zip(["search", "create", "bid", "cancel"]) {
        assert!(ev.has_field("op", op), "{:?}", ev.fields);
        assert!(ev.has_field("reason", BMError::BMUnavailable.reason()));
        assert!(ev.has_field("error_id", "1"), "{:?}", ev.fields);
        assert!(ev.has_field("account_id", &ACCOUNT.to_string()));
        assert!(ev.has_field("player_id", &PLAYER.to_string()));
    }
    assert!(capture.span_recorded("account_id", &ACCOUNT.to_string()));
    let answered = capture
        .all()
        .iter()
        .filter(|e| e.has_field("method", "onBMError") && e.has_field("sent", "true"))
        .count();
    assert_eq!(answered, 4, "every press gets onBMError");
}
