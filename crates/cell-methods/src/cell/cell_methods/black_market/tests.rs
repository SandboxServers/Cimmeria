//! Unit tests for the SGWBlackMarketManager cell methods (indices 61-66):
//! the decode through the shared codec, the auctioneer authority gate on
//! 62-64, the refusals' `onBMError`, and the session signal at logout.

use super::*;
use crate::test_support::LogCapture;
use cimmeria_wire::black_market::{BMSearchOptions, Encode};

const TEST_ENTITY: u32 = 1;
const TEST_PLAYER: i32 = 4242;
const TEST_ACCOUNT: u32 = 0x7000_A0F1;

/// One-player Castle space whose single entity carries a DB `player_id`,
/// so `resolve_player_id` succeeds. Mirrors the `combatant.rs` fixture.
fn make_mgr_with_player() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(TEST_ENTITY, "Castle", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(TEST_ENTITY) {
        p.is_player = true;
        p.player_id = Some(TEST_PLAYER);
        p.account_id = Some(TEST_ACCOUNT);
    }
    mgr.connect_entity(TEST_ENTITY);
    let _ = mgr.compute_aoi_changes();
    mgr
}

/// [`make_mgr_with_player`] plus an auctioneer at `pos` that the server has
/// opened the window at (what `open_black_market` does after an interact).
fn make_mgr_at_auctioneer(pos: [f32; 3]) -> (SpaceManager, u32) {
    let mut mgr = make_mgr_with_player();
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Castle", pos, [0.0; 3]).unwrap();
    mgr.get_entity_mut(TEST_ENTITY)
        .unwrap()
        .last_interaction_target = Some(npc);
    mgr.black_market.open(TEST_PLAYER, npc);
    (mgr, npc)
}

/// A `BMCreateAuction` payload in `.def` order, written out by hand:
/// item, buyout, length (one byte), starting.
fn create_payload(item: i32, buyout: i32, length: u8, starting: i32) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&item.to_le_bytes());
    buf.extend_from_slice(&buyout.to_le_bytes());
    buf.push(length);
    buf.extend_from_slice(&starting.to_le_bytes());
    buf
}

/// The next message, which must be a `CellToBaseMsg::BlackMarket`.
fn next_bm(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> BlackMarketCellToBase {
    match rx.try_recv().expect("forwarded") {
        CellToBaseMsg::BlackMarket(msg) => msg,
        other => panic!("expected BlackMarket, got {other:?}"),
    }
}

/// The `onBMError` ids the cell sent, in order.
fn bm_errors(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<i32> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_BM_ERROR,
            args,
        } = msg
        {
            assert_eq!(entity_id, TEST_ENTITY, "onBMError goes to the caller");
            out.push(i32::from_le_bytes(args[..4].try_into().unwrap()));
        } else {
            panic!("unexpected message {msg:?}");
        }
    }
    out
}

// ── decode (S4) and forwarding ─────────────────────────────────────────

/// S4 regression guard: `BMCreateAuction` is decoded in `.def` order,
/// `item, buyout, length:u8, starting`. The branch's order (`item, starting,
/// buyout, length`) would read buyout 7777 as the starting price and a
/// length of 0x2B out of the starting price's bytes.
#[tokio::test]
async fn create_auction_decodes_def_order_and_forwards() {
    let (mut mgr, _) = make_mgr_at_auctioneer([2.0, 0.0, 0.0]);
    let (tx, mut rx) = mpsc::channel(8);
    let payload = create_payload(0x0101_0101, 7777, 4, 300);
    assert_eq!(payload.len(), 13);

    assert!(dispatch(TEST_ENTITY, CREATE_AUCTION, &payload, &tx, &mut mgr).await);

    match rx.try_recv().expect("forwarded") {
        CellToBaseMsg::BlackMarket(BlackMarketCellToBase::CreateAuction {
            entity_id,
            player_id,
            item_id,
            starting_price,
            buyout_price,
            auction_length,
        }) => {
            assert_eq!((entity_id, player_id), (TEST_ENTITY, TEST_PLAYER));
            assert_eq!(item_id, 0x0101_0101);
            assert_eq!(buyout_price, 7777);
            assert_eq!(auction_length, 4);
            assert_eq!(starting_price, 300);
        }
        other => panic!("expected CreateAuction, got {other:?}"),
    }
}

#[tokio::test]
async fn place_bid_and_cancel_forward_decoded_fields() {
    let (mut mgr, _) = make_mgr_at_auctioneer([2.0, 0.0, 0.0]);
    let (tx, mut rx) = mpsc::channel(8);
    let mut bid = 55i32.to_le_bytes().to_vec();
    bid.extend_from_slice(&1234i32.to_le_bytes());
    assert!(dispatch(TEST_ENTITY, PLACE_BID, &bid, &tx, &mut mgr).await);
    assert!(
        dispatch(
            TEST_ENTITY,
            CANCEL_AUCTION,
            &66i32.to_le_bytes(),
            &tx,
            &mut mgr
        )
        .await
    );

    assert_eq!(
        next_bm(&mut rx),
        BlackMarketCellToBase::PlaceBid {
            entity_id: TEST_ENTITY,
            player_id: TEST_PLAYER,
            sequence_id: 55,
            bid_amount: 1234,
        }
    );
    assert_eq!(
        next_bm(&mut rx),
        BlackMarketCellToBase::CancelAuction {
            entity_id: TEST_ENTITY,
            player_id: TEST_PLAYER,
            sequence_id: 66,
        }
    );
}

/// `BMSearch` is read-only and needs no auctioneer; it forwards the decoded
/// options for the base to scope.
#[tokio::test]
async fn search_forwards_without_an_auctioneer() {
    let mut mgr = make_mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    let opts = BMSearchOptions {
        client_key: 2,
        item_name: "Naq".into(),
        quality: 2000,
        ..Default::default()
    };
    let wire = opts.to_bytes().unwrap();
    assert!(dispatch(TEST_ENTITY, SEARCH, &wire, &tx, &mut mgr).await);
    assert_eq!(
        next_bm(&mut rx),
        BlackMarketCellToBase::Search {
            entity_id: TEST_ENTITY,
            player_id: TEST_PLAYER,
            options: opts,
        }
    );
}

/// Short and over-long payloads forward nothing and log the payload length
/// and the decode reason (BM-02 telemetry), with the actor's ids.
#[tokio::test]
async fn bad_payloads_forward_nothing_and_log_the_reason() {
    let capture = LogCapture::install();
    let (mut mgr, _) = make_mgr_at_auctioneer([2.0, 0.0, 0.0]);
    let (tx, mut rx) = mpsc::channel(8);

    assert!(dispatch(TEST_ENTITY, PLACE_BID, &[1, 2, 3], &tx, &mut mgr).await);
    let mut long = create_payload(1, 2, 3, 4);
    long.push(0);
    assert!(dispatch(TEST_ENTITY, CREATE_AUCTION, &long, &tx, &mut mgr).await);
    assert!(rx.try_recv().is_err(), "nothing forwarded");

    let ev = capture
        .find_message(tracing::Level::WARN, "BMPlaceBid: payload did not decode")
        .expect("decode warn");
    assert!(ev.has_field("arg_len", "3"), "{:?}", ev.fields);
    assert!(ev.has_field("reason", "truncated"), "{:?}", ev.fields);
    assert!(ev.has_field("account_id", &TEST_ACCOUNT.to_string()));
    assert!(ev.has_field("player_id", &TEST_PLAYER.to_string()));
    assert!(capture.span_recorded("account_id", &TEST_ACCOUNT.to_string()));
    assert!(capture.span_recorded("player_id", &TEST_PLAYER.to_string()));
    let ev = capture
        .find_message(
            tracing::Level::WARN,
            "BMCreateAuction: payload did not decode",
        )
        .expect("trailing-bytes warn");
    assert!(ev.has_field("reason", "trailing_bytes"), "{:?}", ev.fields);
}

#[tokio::test]
async fn create_auction_with_no_player_id_forwards_nothing() {
    let (mut mgr, _) = make_mgr_at_auctioneer([2.0, 0.0, 0.0]);
    mgr.get_entity_mut(TEST_ENTITY).unwrap().player_id = None;
    let (tx, mut rx) = mpsc::channel(8);
    let payload = create_payload(1, 0, 5, 10);
    assert!(dispatch(TEST_ENTITY, CREATE_AUCTION, &payload, &tx, &mut mgr).await);
    assert!(rx.try_recv().is_err());
}

// ── authority (CWE-862) ──────────────────────────────────────────────────

/// The authority regression guard: with no `onBMOpen` from the server, 62,
/// 63 and 64 are each answered `onBMError(NotAtAuctioneer)` and nothing
/// reaches the base. Removing the gate forwards all three.
#[tokio::test]
async fn trades_without_an_auctioneer_session_are_refused() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    let mut bid = 1i32.to_le_bytes().to_vec();
    bid.extend_from_slice(&100i32.to_le_bytes());

    assert!(
        dispatch(
            TEST_ENTITY,
            CREATE_AUCTION,
            &create_payload(1, 0, 5, 10),
            &tx,
            &mut mgr
        )
        .await
    );
    assert!(dispatch(TEST_ENTITY, PLACE_BID, &bid, &tx, &mut mgr).await);
    assert!(
        dispatch(
            TEST_ENTITY,
            CANCEL_AUCTION,
            &1i32.to_le_bytes(),
            &tx,
            &mut mgr
        )
        .await
    );

    let not_at = BMError::NotAtAuctioneer.id();
    assert_eq!(bm_errors(&mut rx), vec![not_at, not_at, not_at]);
    let ev = capture
        .find_message(
            tracing::Level::INFO,
            "Black Market request refused on the cell",
        )
        .expect("refusal row");
    assert!(
        ev.has_field("reason", "not_at_auctioneer"),
        "{:?}",
        ev.fields
    );
    assert!(ev.has_field("access", "no_bm_session"), "{:?}", ev.fields);
    assert!(ev.has_field("error_id", &not_at.to_string()));
    assert!(ev.has_field("account_id", &TEST_ACCOUNT.to_string()));
    assert!(ev.has_field("player_id", &TEST_PLAYER.to_string()));
}

/// A player who walked away from the auctioneer is refused with the
/// distance logged.
#[tokio::test]
async fn trades_out_of_range_are_refused_with_the_distance() {
    let capture = LogCapture::install();
    let (mut mgr, _) = make_mgr_at_auctioneer([40.0, 0.0, 0.0]);
    let (tx, mut rx) = mpsc::channel(8);
    assert!(
        dispatch(
            TEST_ENTITY,
            CANCEL_AUCTION,
            &9i32.to_le_bytes(),
            &tx,
            &mut mgr
        )
        .await
    );
    assert_eq!(bm_errors(&mut rx), vec![BMError::NotAtAuctioneer.id()]);
    let ev = capture
        .find_message(
            tracing::Level::INFO,
            "Black Market request refused on the cell",
        )
        .expect("refusal row");
    assert!(
        ev.has_field("access", "auctioneer_out_of_range"),
        "{:?}",
        ev.fields
    );
    let dist: f32 = ev.fields["distance"].parse().expect("distance logged");
    assert!((dist - 40.0).abs() < 0.01, "{:?}", ev.fields);
}

/// Talking to another NPC after opening the window ends the right to trade.
#[tokio::test]
async fn a_new_interaction_target_refuses_trades() {
    let (mut mgr, npc) = make_mgr_at_auctioneer([2.0, 0.0, 0.0]);
    mgr.get_entity_mut(TEST_ENTITY)
        .unwrap()
        .last_interaction_target = Some(npc + 1);
    let (tx, mut rx) = mpsc::channel(8);
    assert!(
        dispatch(
            TEST_ENTITY,
            CANCEL_AUCTION,
            &9i32.to_le_bytes(),
            &tx,
            &mut mgr
        )
        .await
    );
    assert_eq!(bm_errors(&mut rx), vec![BMError::NotAtAuctioneer.id()]);
}

// ── watch list (D4), routing ─────────────────────────────────────────────

/// D4: the watch list is deferred, so 65 and 66 answer
/// `onBMError(WatchUnavailable)` instead of silence.
#[tokio::test]
async fn watch_arms_answer_watch_unavailable() {
    let mut mgr = make_mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    let item = 42i32.to_le_bytes();
    assert!(dispatch(TEST_ENTITY, START_WATCHING, &item, &tx, &mut mgr).await);
    assert!(dispatch(TEST_ENTITY, STOP_WATCHING, &item, &tx, &mut mgr).await);
    let w = BMError::WatchUnavailable.id();
    assert_eq!(bm_errors(&mut rx), vec![w, w]);
}

/// Index 67 is past this interface: the router must offer it to SGWPlayer,
/// and claiming it would swallow an SGWPlayer method.
#[tokio::test]
async fn non_bm_index_is_declined() {
    let mut mgr = make_mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    assert!(!dispatch(TEST_ENTITY, STOP_WATCHING + 1, &[0; 8], &tx, &mut mgr).await);
    assert!(!dispatch(TEST_ENTITY, 9999, &[], &tx, &mut mgr).await);
    assert!(rx.try_recv().is_err(), "nothing forwarded");
}

// ── session signal at logout ────────────────────────────────────────────

/// `bm.open_without_client_call`: logged at logout when the window was
/// opened and the client never called; not logged once any call arrived.
#[tokio::test]
async fn logout_flags_a_window_the_client_never_answered() {
    let capture = LogCapture::install();
    let (mut mgr, _) = make_mgr_at_auctioneer([2.0, 0.0, 0.0]);
    on_disconnect(TEST_ENTITY, &mut mgr);
    let ev = capture
        .find_message(
            tracing::Level::INFO,
            "Black Market opened this session but the client never called it",
        )
        .expect("signal logged");
    assert!(ev.has_field("player_id", &TEST_PLAYER.to_string()));
    assert!(ev.has_field("account_id", &TEST_ACCOUNT.to_string()));
    assert!(mgr.black_market.get(TEST_PLAYER).is_none(), "session ended");
}

#[tokio::test]
async fn logout_after_a_client_call_logs_no_signal() {
    let capture = LogCapture::install();
    let (mut mgr, _) = make_mgr_at_auctioneer([2.0, 0.0, 0.0]);
    let (tx, _rx) = mpsc::channel(8);
    let wire = BMSearchOptions::default().to_bytes().unwrap();
    assert!(dispatch(TEST_ENTITY, SEARCH, &wire, &tx, &mut mgr).await);
    on_disconnect(TEST_ENTITY, &mut mgr);
    assert!(capture
        .find_message(
            tracing::Level::INFO,
            "Black Market opened this session but the client never called it",
        )
        .is_none());
}
