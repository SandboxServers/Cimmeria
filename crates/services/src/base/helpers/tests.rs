use super::*;
use tracing::Level;
// TestTransport needed for the negative-logging log-capture guards below.
use crate::test_support::TestTransport;

/// `to_hex` formats each byte as two uppercase hex digits, separated
/// by single spaces. Pin the format so a refactor that swaps to
/// lowercase or drops the separator doesn't silently change every
/// trace log.
#[test]
fn to_hex_formats_bytes_as_uppercase_with_space_separator() {
    assert_eq!(to_hex(&[]), "");
    assert_eq!(to_hex(&[0x00]), "00");
    assert_eq!(to_hex(&[0xAB, 0xCD]), "AB CD");
    assert_eq!(
        to_hex(&[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]),
        "12 34 56 78 9A BC DE F0"
    );
}

/// `to_hex` zero-pads single-digit byte values. A regression that
/// drops the `:02X` width specifier would emit "1 2" for [0x01, 0x02]
/// instead of "01 02", breaking deterministic log diffs.
#[test]
fn to_hex_zero_pads_single_digit_bytes() {
    assert_eq!(to_hex(&[0x01, 0x02, 0x0F]), "01 02 0F");
}

/// Regression guard for the reliable / unreliable seq stream split
/// (PR #317). The bug: a single shared counter meant every unreliable
/// emission consumed a slot in the reliable stream the client expects
/// to be contiguous, leaving permanent holes that stalled the session.
///
/// This test asserts the wire-format-correct shape: bumping the
/// unreliable counter does NOT advance the reliable one, and vice
/// versa. A future refactor that re-merges them (a tempting
/// "simplification") will fail this test before it ships.
///
/// See `spec.protocol.mercury-wire-format` §1.7 and the disassembly
/// of `UnAckedHandler::queueAckForPacket` for why this invariant is
/// load-bearing on the client side.
#[test]
fn reliable_and_unreliable_seq_counters_are_independent() {
    let state = crate::test_support::test_default_connected_client_state();

    // Both counters start at the same value (0). They live in separate
    // dedup state on the receiver (`inSeqAt` at +0x50 vs the unreliable
    // structure at +0x128), so a shared starting value does not collide.
    let r0 = state.next_seq.load(Ordering::Relaxed);
    let u0 = state.next_seq_unreliable.load(Ordering::Relaxed);
    assert_eq!(r0, 0);
    assert_eq!(u0, 0);

    // Bumping the unreliable counter must NOT advance the reliable one.
    let u_first = state.next_unreliable_seq();
    assert_eq!(u_first, 0, "first unreliable seq is the initial value");
    assert_eq!(
        state.next_seq.load(Ordering::Relaxed),
        0,
        "reliable counter must NOT advance when an unreliable packet is sent",
    );

    // Bumping the reliable counter must NOT advance the unreliable one.
    let r_first =
        state.next_seq.fetch_add(1, Ordering::Relaxed) & cimmeria_mercury::packet::SEQUENCE_MASK;
    assert_eq!(r_first, 0, "first reliable seq is the initial value");
    assert_eq!(
        state.next_seq_unreliable.load(Ordering::Relaxed),
        1,
        "unreliable counter must NOT advance when a reliable packet is sent",
    );

    // Interleaved sequence: R, U, R, U, R. Each stream is monotonic
    // independently, regardless of interleave order — this is exactly
    // the shape that broke before the fix.
    let _r_second =
        state.next_seq.fetch_add(1, Ordering::Relaxed) & cimmeria_mercury::packet::SEQUENCE_MASK;
    let _u_second = state.next_unreliable_seq();
    let r_third =
        state.next_seq.fetch_add(1, Ordering::Relaxed) & cimmeria_mercury::packet::SEQUENCE_MASK;
    let u_third = state.next_unreliable_seq();
    let r_fourth =
        state.next_seq.fetch_add(1, Ordering::Relaxed) & cimmeria_mercury::packet::SEQUENCE_MASK;

    assert_eq!(r_third, 2, "reliable stream stays contiguous (0,1,2,...)");
    assert_eq!(
        r_fourth, 3,
        "reliable stream stays contiguous (0,1,2,3,...)"
    );
    assert_eq!(u_third, 2, "unreliable stream stays contiguous (0,1,2,...)");
}

/// TX-window pressure regression guard. The split-counter design only
/// helps if the tick-sync emit path also avoids registering its
/// packets in the reliable Channel's TX window. Without this guard,
/// a refactor could re-introduce a `shadow_register_reliable_send`
/// call on the tickSync path (the way `send_to_witness_reliable`
/// does for actual reliable packets) and silently start filling
/// the 32-slot window again.
///
/// The test calls [`tick_sync_packet`] — the same function `run_tick_loop`
/// delegates to — so any registration logic added *inside that function*
/// will cause the assertion to fire. Note the guard's scope: a
/// `shadow_register_reliable_send` call added *at the call site in
/// `run_tick_loop`* (outside the helper) would still slip past this test.
///
/// [`tick_sync_packet`]: crate::base::tick_sync::tick_sync_packet
#[test]
fn tick_sync_emission_does_not_consume_reliable_tx_window_slots() {
    use crate::base::tick_sync::tick_sync_packet;
    use cimmeria_mercury::packet::{Bytes, Packet, PacketFlags};

    let state = crate::test_support::test_default_connected_client_state();

    // Burst: fill the TX window with 30 reliable application packets.
    // Mirrors the world-entry shape (charList + versionInfo +
    // resourceFragments + createBasePlayer + mapLoaded fragments).
    {
        let mut ch = state.channel.lock().unwrap();
        for seq in 0..30u32 {
            let pkt = Packet::new(PacketFlags::default(), seq, Bytes::new());
            ch.register_sent_packet(pkt, Bytes::new())
                .expect("register_sent_packet must succeed under window cap");
        }
        assert_eq!(ch.tx_window.len(), 30, "TX window seeded with 30 reliable");
    }

    // Run 10 tick iterations via the same function `run_tick_loop` calls.
    // The UDP send is omitted — the TX-window-pressure failure mode is
    // about register_sent_packet calls, not socket I/O.
    for tick in 0..10u32 {
        let (_seq_id, _pkt) = tick_sync_packet(&state.next_seq_unreliable, &state.key, tick, &[]);
    }

    // The reliable TX window must be unchanged. If a future refactor
    // accidentally registers tickSync seqs into the window, this
    // assertion will fire (`tx_window.len() == 40` instead of 30).
    let ch = state.channel.lock().unwrap();
    assert_eq!(
        ch.tx_window.len(),
        30,
        "tickSync emission must not consume reliable TX window slots — \
         the split-counter design's invariant. If this fires, check that \
         `tick_sync_packet` still avoids calling `shadow_register_reliable_send`."
    );
}

/// The encapsulating accessor [`ConnectedClientState::next_unreliable_seq`]
/// must mask its return value to the 28-bit Mercury sequence space.
/// A regression that drops the `SEQUENCE_MASK` would let the counter
/// roll into the reserved high 4 bits and corrupt the flags byte on
/// the wire (the failure shape from issue #292).
#[test]
fn next_unreliable_seq_masks_to_28_bit_space() {
    let state = crate::test_support::test_default_connected_client_state();
    // Pre-load the counter near the wrap point.
    state
        .next_seq_unreliable
        .store(cimmeria_mercury::packet::SEQUENCE_MASK, Ordering::Relaxed);

    let seq = state.next_unreliable_seq();
    assert_eq!(
        seq,
        cimmeria_mercury::packet::SEQUENCE_MASK,
        "last value before wrap is the mask itself"
    );

    let wrapped = state.next_unreliable_seq();
    assert_eq!(
        wrapped, 0,
        "next call after wrap masks back to 0 — the 4 reserved high \
         bits must never leak into the seq footer"
    );
}

// ──────────────────────────────────────────────────────────────────
// Negative-logging regression guards.
//
// These tests fail if the warn/debug log level on the witness-miss
// and disconnect paths is reverted to the original `trace!`. Per
// TESTING.md, a regression guard must fail when the fix is reverted;
// these do that by asserting on both the level AND the `reason`
// structured field — so a generic level-only revert (e.g. someone
// demoting back to `trace`) AND a field-removing revert both trip.
//
// The bug shape this guards: a witness AoI packet silently dropped
// at `trace!` was the single biggest blind spot for the missing-
// entity-update class of bugs (#288 spawn glitches). Promoting to
// warn! with a stable `reason` field makes the drop greppable in
// ops.
// ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn send_to_witness_emits_warn_when_entity_to_addr_misses() {
    use crate::test_support::LogCapture;
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    let capture = LogCapture::install();

    let transport: Arc<dyn cimmeria_mercury::transport::Transport> =
        Arc::new(TestTransport::default());
    let connected = Arc::new(Mutex::new(
        HashMap::<SocketAddr, ConnectedClientState>::new(),
    ));
    // Deliberately empty — the witness has no addr mapping.
    let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));

    send_to_witness(
        &transport,
        &connected,
        &entity_to_addr,
        999, // witness_id not in map
        |_key, _seq, _acks| vec![],
    )
    .await;

    let found = capture.find_event(
        Level::WARN,
        "no client addr for witness",
        "entity_to_addr_miss",
    );
    assert!(
        found.is_some(),
        "negative-logging convention: AoI witness-miss must emit WARN with reason=entity_to_addr_miss; \
         reverting to trace!/debug! breaks ops visibility of the #288-class spawn glitches. \
         Captured events: {:#?}",
        capture.all()
    );
}

#[tokio::test]
async fn send_to_witness_reliable_emits_warn_when_entity_to_addr_misses() {
    use crate::test_support::LogCapture;
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    let capture = LogCapture::install();

    let transport: Arc<dyn cimmeria_mercury::transport::Transport> =
        Arc::new(TestTransport::default());
    let connected = Arc::new(Mutex::new(
        HashMap::<SocketAddr, ConnectedClientState>::new(),
    ));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));

    send_to_witness_reliable(
        &transport,
        &connected,
        &entity_to_addr,
        42,
        |_key, _seq, _acks| vec![],
    )
    .await;

    assert!(
        capture
            .find_event(
                Level::WARN,
                "no client addr for witness",
                "entity_to_addr_miss"
            )
            .is_some(),
        "negative-logging convention: reliable AoI witness-miss must emit WARN with reason=entity_to_addr_miss"
    );
}

#[tokio::test]
async fn send_bundle_to_witness_reliable_emits_warn_when_entity_to_addr_misses() {
    use crate::test_support::LogCapture;
    use cimmeria_mercury::channel_bundle::ChannelBundle;
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    let capture = LogCapture::install();

    let transport: Arc<dyn cimmeria_mercury::transport::Transport> =
        Arc::new(TestTransport::default());
    let connected = Arc::new(Mutex::new(
        HashMap::<SocketAddr, ConnectedClientState>::new(),
    ));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));

    let bundle = ChannelBundle::new(true);
    send_bundle_to_witness_reliable(&transport, &connected, &entity_to_addr, 7, bundle).await;

    assert!(
        capture
            .find_event(
                Level::WARN,
                "no client addr for witness",
                "entity_to_addr_miss"
            )
            .is_some(),
        "negative-logging convention: bundle AoI witness-miss must emit WARN with reason=entity_to_addr_miss"
    );
}

// ──────────────────────────────────────────────────────────────────
// Disconnect/debug-path guards: addr is mapped but the
// client is not in `connected` (the post-handshake disconnect race
// window). The unreliable / reliable / bundle helpers all log this
// at `debug!` with `reason="client_disconnected"`. Promoting back
// to `trace!` would silence the path; the guards pin both the
// level AND the reason.
// ──────────────────────────────────────────────────────────────────

/// Helper: build an (entity_to_addr, connected) pair where the
/// witness has an addr mapping but no `ConnectedClientState`. This
/// is exactly the post-handshake disconnect window the debug log
/// guards.
fn staged_disconnect_maps(
    witness_id: u32,
    addr: SocketAddr,
) -> (
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let connected = Arc::new(Mutex::new(
        HashMap::<SocketAddr, ConnectedClientState>::new(),
    ));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(witness_id, addr)])));
    (connected, entity_to_addr)
}

#[tokio::test]
async fn send_to_witness_emits_debug_when_client_disconnected() {
    use crate::test_support::LogCapture;

    let capture = LogCapture::install();
    let transport: Arc<dyn cimmeria_mercury::transport::Transport> =
        Arc::new(TestTransport::default());
    let witness_addr: SocketAddr = "127.0.0.1:55101".parse().unwrap();
    let (connected, entity_to_addr) = staged_disconnect_maps(111, witness_addr);

    send_to_witness(
        &transport,
        &connected,
        &entity_to_addr,
        111,
        |_key, _seq, _acks| vec![],
    )
    .await;

    assert!(
        capture
            .find_event(
                Level::DEBUG,
                "client disconnected mid-send",
                "client_disconnected"
            )
            .is_some(),
        "negative-logging convention: unreliable disconnect path must emit DEBUG with reason=client_disconnected"
    );
}

#[tokio::test]
async fn send_to_witness_reliable_emits_debug_when_client_disconnected() {
    use crate::test_support::LogCapture;

    let capture = LogCapture::install();
    let transport: Arc<dyn cimmeria_mercury::transport::Transport> =
        Arc::new(TestTransport::default());
    let witness_addr: SocketAddr = "127.0.0.1:55102".parse().unwrap();
    let (connected, entity_to_addr) = staged_disconnect_maps(222, witness_addr);

    send_to_witness_reliable(
        &transport,
        &connected,
        &entity_to_addr,
        222,
        |_key, _seq, _acks| vec![],
    )
    .await;

    assert!(
        capture
            .find_event(
                Level::DEBUG,
                "client disconnected mid-send",
                "client_disconnected"
            )
            .is_some(),
        "negative-logging convention: reliable disconnect path must emit DEBUG with reason=client_disconnected"
    );
}

#[tokio::test]
async fn send_bundle_to_witness_reliable_emits_debug_when_client_disconnected() {
    use crate::test_support::LogCapture;
    use cimmeria_mercury::channel_bundle::ChannelBundle;

    let capture = LogCapture::install();
    let transport: Arc<dyn cimmeria_mercury::transport::Transport> =
        Arc::new(TestTransport::default());
    let witness_addr: SocketAddr = "127.0.0.1:55103".parse().unwrap();
    let (connected, entity_to_addr) = staged_disconnect_maps(333, witness_addr);

    let bundle = ChannelBundle::new(true);
    send_bundle_to_witness_reliable(&transport, &connected, &entity_to_addr, 333, bundle).await;

    assert!(
        capture
            .find_event(
                Level::DEBUG,
                "client disconnected mid-send",
                "client_disconnected"
            )
            .is_some(),
        "negative-logging convention: bundle disconnect path must emit DEBUG with reason=client_disconnected"
    );
}

// ──────────────────────────────────────────────────────────────────
// Disconnect-reason plumbing guards: every cleanup path passes a
// stable `&'static str` reason label through `destroy_client_entities`,
// which stamps it onto the "Client entities cleaned up" log as the
// `disconnect_reason` field. SigNoz dashboards pivot on that field —
// if a caller drifts to a different label (or stops passing one),
// reason-based queries break silently.
// ──────────────────────────────────────────────────────────────────

/// Build a populated `connected` map + entity_manager pair so
/// `destroy_client_entities` actually has a session to tear down
/// (not the "already cleaned up" short-circuit path).
fn staged_connected_session(
    addr: SocketAddr,
    account_eid: u32,
) -> (
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<cimmeria_entity::manager::EntityManager>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let mut state = crate::test_support::test_default_connected_client_state();
    state.account_entity_id = account_eid;
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_manager = Arc::new(Mutex::new(cimmeria_entity::manager::EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));
    (connected, entity_manager, entity_to_addr)
}

#[test]
fn destroy_client_entities_stamps_disconnect_reason_on_cleanup_log() {
    use crate::test_support::LogCapture;

    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:55401".parse().unwrap();
    let (connected, entity_manager, entity_to_addr) = staged_connected_session(addr, 42);

    destroy_client_entities(
        &connected,
        &entity_manager,
        addr,
        &None,
        &entity_to_addr,
        "client_disconnect",
    );

    let event = capture
        .find_message(Level::INFO, "Client entities cleaned up")
        .expect("expected INFO `Client entities cleaned up` event after destroy");
    assert!(
        event.has_field("disconnect_reason", "client_disconnect"),
        "destroy_client_entities must stamp the caller's reason onto the \
         disconnect_reason field — SigNoz dashboards pivot on it. Got: {:?}",
        event.fields,
    );
}

/// Pin the full set of documented `disconnect_reason` labels. A new
/// disconnect site that invents a fresh label (or a refactor that
/// drops one) will be caught by this list compared to grep/lint.
#[test]
fn destroy_client_entities_accepts_all_documented_reasons() {
    use crate::test_support::LogCapture;

    // Every label that appears at a documented call site of
    // `destroy_client_entities` (helpers.rs doc, helpers.rs line 232ish).
    // If you add a new reason, add it here AND to the doc comment.
    let documented_reasons = [
        "client_disconnect",
        "inactivity_timeout",
        "send_error",
        "duplicate_login",
        "logoff",
    ];

    for (i, reason) in documented_reasons.iter().enumerate() {
        let capture = LogCapture::install();
        let addr: SocketAddr = format!("127.0.0.1:{}", 55500 + i).parse().unwrap();
        let (connected, entity_manager, entity_to_addr) =
            staged_connected_session(addr, 100 + i as u32);

        destroy_client_entities(
            &connected,
            &entity_manager,
            addr,
            &None,
            &entity_to_addr,
            reason,
        );

        let event = capture
            .find_message(Level::INFO, "Client entities cleaned up")
            .unwrap_or_else(|| panic!("missing cleanup log for reason `{reason}`"));
        assert!(
            event.has_field("disconnect_reason", reason),
            "reason `{reason}` must round-trip into the disconnect_reason field; got fields: {:?}",
            event.fields,
        );
    }
}

/// Idempotent-cleanup short-circuit (no session at addr) must still
/// log the `disconnect_reason` so the operator can see "we tried to
/// clean up but it was already gone" without losing the reason.
#[test]
fn destroy_client_entities_logs_reason_on_already_cleaned_short_circuit() {
    use crate::test_support::LogCapture;

    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:55402".parse().unwrap();
    let connected = Arc::new(Mutex::new(
        HashMap::<SocketAddr, ConnectedClientState>::new(),
    ));
    let entity_manager = Arc::new(Mutex::new(cimmeria_entity::manager::EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));

    destroy_client_entities(
        &connected,
        &entity_manager,
        addr,
        &None,
        &entity_to_addr,
        "inactivity_timeout",
    );

    let event = capture
        .find_message(Level::DEBUG, "already cleaned up")
        .expect("expected DEBUG `already cleaned up` event on idempotent re-call");
    assert!(
        event.has_field("disconnect_reason", "inactivity_timeout"),
        "short-circuit path must still carry the reason; got fields: {:?}",
        event.fields,
    );
}
