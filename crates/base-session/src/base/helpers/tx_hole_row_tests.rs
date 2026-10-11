//! TG-MER-04 guard: the channel's `mercury.tx_hole` and `mercury.rx_order`
//! rows name the session's account and player.
//!
//! The channel knows only the peer address. The identity reaches it through
//! `collect_pending_retransmits`, which copies the session identity onto the
//! channel each tick (TG-MER-03); these rows reuse it. Both tests run that
//! sync first, as the tick loop does, then drive the receive-path call that
//! writes the row.

use super::*;

use crate::test_support::{test_default_connected_client_state, LogCapture};
use cimmeria_mercury::packet::{Bytes, Packet, ParsedPacket, FLAG_HAS_SEQUENCE, FLAG_RELIABLE};

/// A session for account 6 playing player 72, whose channel is `channel`.
fn named_session(
    addr: SocketAddr,
    channel: cimmeria_mercury::channel::Channel,
) -> Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> {
    let mut state = test_default_connected_client_state();
    state.account_id = 6;
    state.account_name = Some("mer04_login".to_string());
    state.active_player_id = Some(72);
    state.player_name = Some("Mer04 Carter".to_string());
    state.channel = Mutex::new(channel);
    Arc::new(Mutex::new(HashMap::from([(addr, state)])))
}

/// Server packets 1-3 are outstanding; the client's footer acks 1 and 3
/// before 2, so packet 2 is a transmit hole. The `tx_hole_open` row carries
/// the account and player pairs. Removing the fields from the row (or the
/// tick's sync) leaves every identity field off and fails the assertions.
#[test]
fn tx_hole_open_names_the_account() {
    let addr: SocketAddr = "127.0.0.1:52413".parse().unwrap();
    let mut channel = cimmeria_mercury::channel::Channel::new(addr);
    for seq in 1..=3 {
        channel
            .register_sent_packet(
                Packet::new(Default::default(), seq, Bytes::new()),
                Bytes::from(vec![0x3Cu8; 24]),
            )
            .unwrap();
    }
    let connected = named_session(addr, channel);
    // The tick's sync; nothing is past its RTO, so nothing is resent.
    assert!(collect_pending_retransmits(&connected, addr).is_empty());

    let capture = LogCapture::install();
    {
        let map = connected.lock().unwrap();
        let mut channel = map[&addr].channel.lock().unwrap();
        channel.process_ack_footer(&[3, 1]);
    }

    let events = capture.all();
    let row = events
        .iter()
        .find(|e| e.target == "mercury.tx_hole" && e.has_field("event", "tx_hole_open"))
        .unwrap_or_else(|| panic!("no tx_hole_open row: {events:#?}"));
    assert!(row.has_field("seq", "2"), "{row:#?}");
    assert!(row.has_field("account_id", "6"), "{row:#?}");
    assert!(row.has_field("account_name", "mer04_login"), "{row:#?}");
    assert!(row.has_field("player_id", "72"), "{row:#?}");
    assert!(row.has_field("player_name", "Mer04 Carter"), "{row:#?}");
}

/// The client's reliable packet 1 arrives, then 3: packet 3 is held behind
/// the gap and the `buffered` row names the account and player. Removing
/// the fields from the `rx_order` row fails the assertions.
#[test]
fn rx_order_buffered_names_the_account() {
    let addr: SocketAddr = "127.0.0.1:52414".parse().unwrap();
    let connected = named_session(addr, cimmeria_mercury::channel::Channel::new(addr));
    assert!(collect_pending_retransmits(&connected, addr).is_empty());

    let reliable = |seq: u32| ParsedPacket {
        flags: FLAG_RELIABLE | FLAG_HAS_SEQUENCE,
        body: Bytes::new(),
        seq_id: Some(seq),
        first_req_offset: None,
        frag_begin: None,
        frag_end: None,
        acks: Vec::new(),
    };
    let capture = LogCapture::install();
    {
        let map = connected.lock().unwrap();
        let mut channel = map[&addr].channel.lock().unwrap();
        channel.receive_parsed(reliable(1)).unwrap();
        channel.receive_parsed(reliable(3)).unwrap();
    }

    let events = capture.all();
    let row = events
        .iter()
        .find(|e| e.target == "mercury.rx_order" && e.has_field("event", "buffered"))
        .unwrap_or_else(|| panic!("no buffered row: {events:#?}"));
    assert!(row.has_field("seq", "3"), "{row:#?}");
    assert!(row.has_field("account_id", "6"), "{row:#?}");
    assert!(row.has_field("account_name", "mer04_login"), "{row:#?}");
    assert!(row.has_field("player_id", "72"), "{row:#?}");
    assert!(row.has_field("player_name", "Mer04 Carter"), "{row:#?}");
}
