//! Unit tests for the SGWBlackMarketManager cell methods (indices 61-66):
//! the decode helpers and the live `dispatch` command arms.

use super::*;

/// Decode a `BMCreateAuction` payload the way `dispatch` does, returning the
/// four parsed fields. Mirrors the production decode so a test can assert on
/// the result without a full async dispatch.
fn decode_create_auction(args: &[u8]) -> Option<(i32, i32, i32, u8)> {
    if args.len() < CREATE_AUCTION_LEN {
        return None;
    }
    let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    let starting_price = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
    let buyout_price = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
    let auction_length = args[12];
    Some((item_id, starting_price, buyout_price, auction_length))
}

/// A valid `BMCreateAuction` is exactly 13 bytes and decodes
/// `auctionLength` from the single byte at offset 12.
#[test]
fn create_auction_decodes_thirteen_byte_payload() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&777i32.to_le_bytes()); // itemInstanceId
    buf.extend_from_slice(&100i32.to_le_bytes()); // startingPrice
    buf.extend_from_slice(&5000i32.to_le_bytes()); // buyoutPrice
    buf.push(3u8); // auctionLength (UINT8)
    assert_eq!(buf.len(), CREATE_AUCTION_LEN);

    let (item, starting, buyout, length) =
        decode_create_auction(&buf).expect("13-byte payload should decode");
    assert_eq!(item, 777);
    assert_eq!(starting, 100);
    assert_eq!(buyout, 5000);
    assert_eq!(length, 3);
}

/// Regression guard for the UINT8 fix. `auctionLength` is the byte at offset
/// 12. If someone reverts to reading it as an INT32 (the old 16-byte
/// layout), they would consume bytes 12..16 — bytes that this 13-byte
/// payload does not have, so the old guard (`args.len() >= 16`) would
/// reject this valid message outright. We pin both facts:
///   1. a 13-byte buffer is accepted, and
///   2. the length byte is read from offset 12 (value 200, the high end of
///      a UINT8 that an INT32 read would smear across following bytes).
#[test]
fn create_auction_length_is_uint8_at_offset_12_not_int32() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&1i32.to_le_bytes());
    buf.extend_from_slice(&2i32.to_le_bytes());
    buf.extend_from_slice(&3i32.to_le_bytes());
    buf.push(200u8); // distinctive single-byte length
    assert_eq!(buf.len(), 13);

    // The fixed-size constant must be 13, not 16. A revert to INT32 would
    // push this back to 16 and break the accept path below.
    assert_eq!(CREATE_AUCTION_LEN, 13);

    let (_, _, _, length) = decode_create_auction(&buf).expect("13-byte payload must be accepted");
    assert_eq!(length, 200, "auctionLength must be the UINT8 at offset 12");
}

/// The `BMSearch` arm must parse the full 11-field `BMSearchOptions`.
/// (Behavioural coverage of the deserializer lives in
/// `base::black_market::types`; this confirms the cell arm uses it.)
#[test]
fn search_arm_parses_search_options() {
    let opts = BMSearchOptions {
        sort_id: 1,
        client_key: 11,
        sequence_id: 22,
        b_forward: 1,
        seller_name: "Carter".to_string(),
        bidder_name: String::new(),
        item_name: "Zat".to_string(),
        min_tc: 0,
        max_tc: 999,
        quality: 2,
        filter_flags: 8,
    };
    let mut wire = Vec::new();
    wire.push(opts.sort_id);
    wire.extend_from_slice(&opts.client_key.to_le_bytes());
    wire.extend_from_slice(&opts.sequence_id.to_le_bytes());
    wire.push(opts.b_forward);
    for s in [&opts.seller_name, &opts.bidder_name, &opts.item_name] {
        wire.extend_from_slice(&(s.len() as u32).to_le_bytes());
        wire.extend_from_slice(s.as_bytes());
    }
    wire.extend_from_slice(&opts.min_tc.to_le_bytes());
    wire.extend_from_slice(&opts.max_tc.to_le_bytes());
    wire.extend_from_slice(&opts.quality.to_le_bytes());
    wire.extend_from_slice(&opts.filter_flags.to_le_bytes());

    assert_eq!(BMSearchOptions::from_wire(&wire), Some(opts));
}

// ── live dispatch() coverage ──────────────────────────────────────────
//
// The decode-helper tests above prove the byte layout. These exercise the
// real async `dispatch(...)`: that each command arm forwards the correct
// `CellToBaseMsg::BM*` with the decoded fields, that player_id is resolved
// from the entity, and that the too-short / no-player_id branches bail
// without emitting a message.
//
// `SpaceManager`, `mpsc`, `CellToBaseMsg`, and the method-index consts are
// already in scope via the `use super::*` at the top of this module.

const TEST_ENTITY: u32 = 1;
const TEST_PLAYER: i32 = 4242;

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
    }
    mgr.connect_entity(TEST_ENTITY);
    let _ = mgr.compute_aoi_changes();
    mgr
}

/// Pack a valid 13-byte `BMCreateAuction` payload.
fn create_payload(item: i32, starting: i32, buyout: i32, length: u8) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&item.to_le_bytes());
    buf.extend_from_slice(&starting.to_le_bytes());
    buf.extend_from_slice(&buyout.to_le_bytes());
    buf.push(length);
    buf
}

/// CREATE_AUCTION forwards `BMCreateAuction` carrying every decoded field
/// and the entity-resolved player_id. Bug shape: if a field were decoded
/// from the wrong offset (the INT32-vs-UINT8 length hazard, a swapped
/// price pair) this lands a wrong value on the channel.
#[tokio::test]
async fn create_auction_forwards_decoded_fields() {
    let mut mgr = make_mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    let payload = create_payload(777, 100, 5000, 3);

    let handled = dispatch(TEST_ENTITY, CREATE_AUCTION, &payload, &tx, &mut mgr).await;
    assert!(handled, "dispatch must recognise CREATE_AUCTION");

    match rx
        .try_recv()
        .expect("CREATE_AUCTION must forward a message")
    {
        CellToBaseMsg::BMCreateAuction {
            entity_id,
            player_id,
            item_id,
            starting_price,
            buyout_price,
            auction_length,
        } => {
            assert_eq!(entity_id, TEST_ENTITY);
            assert_eq!(player_id, TEST_PLAYER, "player_id resolved from entity");
            assert_eq!(item_id, 777);
            assert_eq!(starting_price, 100);
            assert_eq!(buyout_price, 5000);
            assert_eq!(auction_length, 3, "UINT8 length at offset 12");
        }
        other => panic!("expected BMCreateAuction, got {other:?}"),
    }
}

/// PLACE_BID forwards `BMPlaceBid` with the decoded sequence_id + amount.
#[tokio::test]
async fn place_bid_forwards_decoded_fields() {
    let mut mgr = make_mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    let mut payload = Vec::new();
    payload.extend_from_slice(&55i32.to_le_bytes()); // sequence_id
    payload.extend_from_slice(&900i32.to_le_bytes()); // bid_amount

    let handled = dispatch(TEST_ENTITY, PLACE_BID, &payload, &tx, &mut mgr).await;
    assert!(handled, "dispatch must recognise PLACE_BID");

    match rx.try_recv().expect("PLACE_BID must forward a message") {
        CellToBaseMsg::BMPlaceBid {
            entity_id,
            player_id,
            sequence_id,
            bid_amount,
        } => {
            assert_eq!(entity_id, TEST_ENTITY);
            assert_eq!(player_id, TEST_PLAYER);
            assert_eq!(sequence_id, 55);
            assert_eq!(bid_amount, 900);
        }
        other => panic!("expected BMPlaceBid, got {other:?}"),
    }
}

/// CANCEL_AUCTION forwards `BMCancelAuction` with the decoded sequence_id.
#[tokio::test]
async fn cancel_auction_forwards_decoded_fields() {
    let mut mgr = make_mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    let payload = 321i32.to_le_bytes().to_vec();

    let handled = dispatch(TEST_ENTITY, CANCEL_AUCTION, &payload, &tx, &mut mgr).await;
    assert!(handled, "dispatch must recognise CANCEL_AUCTION");

    match rx
        .try_recv()
        .expect("CANCEL_AUCTION must forward a message")
    {
        CellToBaseMsg::BMCancelAuction {
            entity_id,
            player_id,
            sequence_id,
        } => {
            assert_eq!(entity_id, TEST_ENTITY);
            assert_eq!(player_id, TEST_PLAYER);
            assert_eq!(sequence_id, 321);
        }
        other => panic!("expected BMCancelAuction, got {other:?}"),
    }
}

/// SEARCH parses `BMSearchOptions` off the wire, resolves `player_id`, and
/// forwards `CellToBaseMsg::BMSearch` to the base. Guards that the forward
/// now happens (regression: before the serve was wired this arm forwarded
/// nothing; if the `send` call is removed the channel stays empty and this
/// test fails).
#[tokio::test]
async fn search_arm_forwards_bm_search_msg() {
    let mut mgr = make_mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);

    // Minimal valid BMSearchOptions: scalars + three empty strings + 4 i32.
    let mut wire = Vec::new();
    wire.push(2u8); // sort_id (non-zero so we can assert it landed)
    wire.extend_from_slice(&11i32.to_le_bytes()); // client_key
    wire.extend_from_slice(&22i32.to_le_bytes()); // sequence_id
    wire.push(1u8); // b_forward
    for _ in 0..3 {
        wire.extend_from_slice(&0u32.to_le_bytes()); // empty STRING
    }
    wire.extend_from_slice(&0i32.to_le_bytes()); // min_tc
    wire.extend_from_slice(&999i32.to_le_bytes()); // max_tc
    wire.extend_from_slice(&0i32.to_le_bytes()); // quality
    wire.extend_from_slice(&0i32.to_le_bytes()); // filter_flags

    let handled = dispatch(TEST_ENTITY, SEARCH, &wire, &tx, &mut mgr).await;
    assert!(handled, "dispatch must recognise SEARCH");

    match rx.try_recv().expect("SEARCH must forward a CellToBaseMsg::BMSearch") {
        CellToBaseMsg::BMSearch {
            entity_id,
            player_id,
            options,
        } => {
            assert_eq!(entity_id, TEST_ENTITY);
            assert_eq!(player_id, TEST_PLAYER, "player_id resolved from entity");
            assert_eq!(options.sort_id, 2, "sort_id decoded from wire");
            assert_eq!(options.client_key, 11);
            assert_eq!(options.sequence_id, 22);
            assert_eq!(options.b_forward, 1);
            assert_eq!(options.max_tc, 999);
        }
        other => panic!("expected BMSearch, got {other:?}"),
    }
}

/// An entity with no `player_id` must NOT forward an auction op — keying a
/// create/bid/cancel on a fallback player_id=0 would target a sentinel row.
/// Bug shape: dropping the `resolve_player_id` guard (defaulting to 0)
/// would leak a message here.
#[tokio::test]
async fn create_auction_with_no_player_id_forwards_nothing() {
    let mut mgr = make_mgr_with_player();
    // Strip the player_id so resolution returns None.
    if let Some(p) = mgr.get_entity_mut(TEST_ENTITY) {
        p.player_id = None;
    }
    let (tx, mut rx) = mpsc::channel(8);
    let payload = create_payload(1, 2, 3, 1);

    let handled = dispatch(TEST_ENTITY, CREATE_AUCTION, &payload, &tx, &mut mgr).await;
    assert!(handled, "method is still recognised even when it bails");
    assert!(
        rx.try_recv().is_err(),
        "no player_id must drop the op, not misroute it to player_id=0",
    );
}

/// A too-short CREATE_AUCTION payload (12 bytes, missing the length byte)
/// must bail before resolving/forwarding. Guards the `>= 13` length gate.
#[tokio::test]
async fn create_auction_short_payload_forwards_nothing() {
    let mut mgr = make_mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    let short = vec![0u8; CREATE_AUCTION_LEN - 1];

    let handled = dispatch(TEST_ENTITY, CREATE_AUCTION, &short, &tx, &mut mgr).await;
    assert!(handled);
    assert!(
        rx.try_recv().is_err(),
        "a 12-byte payload must be rejected, not partially decoded",
    );
}

/// A too-short PLACE_BID payload (7 bytes, need 8) bails. Guards `>= 8`.
#[tokio::test]
async fn place_bid_short_payload_forwards_nothing() {
    let mut mgr = make_mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);

    let handled = dispatch(TEST_ENTITY, PLACE_BID, &[0u8; 7], &tx, &mut mgr).await;
    assert!(handled);
    assert!(
        rx.try_recv().is_err(),
        "7-byte bid payload must be rejected"
    );
}

/// A too-short CANCEL_AUCTION payload (3 bytes, need 4) bails. Guards `>= 4`.
#[tokio::test]
async fn cancel_auction_short_payload_forwards_nothing() {
    let mut mgr = make_mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);

    let handled = dispatch(TEST_ENTITY, CANCEL_AUCTION, &[0u8; 3], &tx, &mut mgr).await;
    assert!(handled);
    assert!(
        rx.try_recv().is_err(),
        "3-byte cancel payload must be rejected",
    );
}

/// The watch arms (START/STOP) are recognised (handled=true) and forward
/// nothing — they are Phase-1 logging stubs.
#[tokio::test]
async fn watch_arms_are_handled_and_forward_nothing() {
    let mut mgr = make_mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    let auction_id = 99i32.to_le_bytes().to_vec();

    assert!(dispatch(TEST_ENTITY, START_WATCHING, &auction_id, &tx, &mut mgr).await);
    assert!(dispatch(TEST_ENTITY, STOP_WATCHING, &auction_id, &tx, &mut mgr).await);
    assert!(
        rx.try_recv().is_err(),
        "watch arms are logging stubs — no CellToBaseMsg",
    );
}

/// An unknown method index falls through so the outer router can chain.
#[tokio::test]
async fn dispatch_returns_false_for_unhandled_method() {
    let mut mgr = make_mgr_with_player();
    let (tx, _rx) = mpsc::channel(8);
    assert!(
        !dispatch(TEST_ENTITY, 9999, &[], &tx, &mut mgr).await,
        "unknown method must return false",
    );
}
