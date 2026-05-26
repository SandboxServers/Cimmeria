//! Channel tests — extracted from channel/mod.rs to keep that file under
//! the 500-line soft cap. Logically a single test module; `#[cfg(test)]`
//! is applied at the parent's `mod tests;` declaration.

use super::*;
use std::net::SocketAddr;

#[test]
fn new_channel_starts_connecting() {
    let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
    let ch = Channel::new(addr);
    assert_eq!(ch.state, ChannelState::Connecting);
    assert_eq!(ch.next_tx_seq, 0);
    assert_eq!(ch.expected_rx_seq, 0);
    assert_eq!(ch.remote_addr, addr);
    assert!(ch.tx_window.is_empty());
    assert!(ch.rx_window.is_empty());
}

#[test]
fn fresh_channel_not_timed_out() {
    let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
    let ch = Channel::new(addr);
    assert!(!ch.is_timed_out());
}

/// Helper: create a minimal valid Packet for channel tests.
fn test_packet() -> Packet {
    use crate::packet::PacketFlags;
    use bytes::Bytes;

    Packet::new(
        PacketFlags::default(),
        0, // sequence is overwritten by send_packet
        Bytes::from_static(&[0xDE, 0xAD]),
    )
}

fn test_addr() -> SocketAddr {
    "127.0.0.1:9000".parse().unwrap()
}

#[test]
fn mark_reliable_stores_packet() {
    let mut ch = Channel::new(test_addr());
    ch.send_packet(test_packet()).unwrap();

    assert_eq!(ch.tx_window.len(), 1);
    // send_packet stamps seq=0 (the first sequence number).
    assert_eq!(ch.tx_window[0].packet.sequence, 0);
    assert_eq!(ch.tx_window[0].retransmit_count, 0);
}

#[test]
fn on_ack_removes_packet() {
    let mut ch = Channel::new(test_addr());
    ch.send_packet(test_packet()).unwrap();
    assert_eq!(ch.tx_window.len(), 1);

    // Cumulative ACK for seq=0 should drain the window.
    ch.process_acks(0).unwrap();
    assert!(ch.tx_window.is_empty());
}

#[test]
fn check_retransmits_returns_expired() {
    let mut ch = Channel::new(test_addr());
    ch.send_packet(test_packet()).unwrap();

    // Default RtoConfig initial_srtt=500ms → initial RTO = 1500ms.
    // Backdate by 1600ms to be safely past the timeout regardless of
    // future tuning of the default initial_srtt.
    ch.tx_window[0].last_sent = std::time::Instant::now() - std::time::Duration::from_millis(1600);

    let retransmits = ch.check_timeouts();
    assert_eq!(
        retransmits.len(),
        0,
        "send_packet entries have no raw bytes — retransmit scan bumps the \
         counter but emits nothing for the bytes-empty entries. \
         register_sent_packet entries (with bytes) are tested separately."
    );
    // check_timeouts bumps the retransmit counter even when bytes are empty.
    assert_eq!(ch.tx_window[0].retransmit_count, 1);
}

#[test]
fn check_retransmits_empty_when_fresh() {
    let mut ch = Channel::new(test_addr());
    ch.send_packet(test_packet()).unwrap();

    // Immediately after send, the packet is well within the RTO window.
    let retransmits = ch.check_timeouts();
    assert!(retransmits.is_empty());
    assert_eq!(ch.tx_window[0].retransmit_count, 0);
}

// ── Split last_activity into last_sent / last_received ─────────────

#[test]
fn send_packet_updates_only_last_sent() {
    let mut ch = Channel::new(test_addr());
    // Backdate both clocks to a known past time so we can detect which
    // one moves forward.
    let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
    ch.last_sent = baseline;
    ch.last_received = baseline;

    ch.send_packet(test_packet()).unwrap();

    assert!(ch.last_sent > baseline, "send_packet must reset last_sent");
    assert_eq!(
        ch.last_received, baseline,
        "send_packet must NOT touch last_received"
    );
}

#[test]
fn receive_packet_updates_only_last_received() {
    use crate::packet::PacketFlags;
    use bytes::Bytes;

    let mut ch = Channel::new(test_addr());
    let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
    ch.last_sent = baseline;
    ch.last_received = baseline;

    // Inbound packet at expected_rx_seq=0.
    let pkt = Packet::new(PacketFlags::default(), 0, Bytes::from_static(&[0xAB]));
    ch.receive_packet(pkt).unwrap();

    assert!(
        ch.last_received > baseline,
        "receive_packet must reset last_received"
    );
    assert_eq!(
        ch.last_sent, baseline,
        "receive_packet must NOT touch last_sent"
    );
}

#[test]
fn process_acks_updates_only_last_received() {
    let mut ch = Channel::new(test_addr());
    // Need a packet in flight for the ACK to drain.
    ch.send_packet(test_packet()).unwrap();

    let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
    ch.last_sent = baseline;
    ch.last_received = baseline;

    ch.process_acks(0).unwrap();

    // ACK is peer-originated data — counts as receive, not send.
    assert!(
        ch.last_received > baseline,
        "process_acks must reset last_received"
    );
    assert_eq!(
        ch.last_sent, baseline,
        "process_acks must NOT touch last_sent"
    );
}

#[test]
fn keepalive_due_when_we_havent_sent_in_a_while() {
    let mut ch = Channel::new(test_addr());
    // Force last_sent into the past beyond KEEPALIVE_INTERVAL_MS, but
    // keep last_received fresh (peer is talking to us).
    ch.last_sent = std::time::Instant::now()
        - std::time::Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 100);
    ch.last_received = std::time::Instant::now();

    // Inbound peer traffic must NOT suppress our send-side keepalive
    // — NAT entries time out per direction.
    assert!(
        ch.keepalive_due(),
        "keepalive must be due based on OUR send-side silence, regardless of peer activity"
    );
}

#[test]
fn keepalive_not_due_right_after_sending() {
    let mut ch = Channel::new(test_addr());
    ch.send_packet(test_packet()).unwrap();
    // Just sent; no keepalive needed for at least KEEPALIVE_INTERVAL_MS.
    assert!(!ch.keepalive_due());
}

#[test]
fn is_timed_out_fires_on_silent_peer_even_if_we_keep_sending() {
    let mut ch = Channel::new(test_addr());
    // Simulate "we keep blasting world updates at a dead client":
    // last_sent is fresh, but last_received is stale past the
    // configured INACTIVITY_TIMEOUT_MS.
    ch.last_sent = std::time::Instant::now();
    ch.last_received = std::time::Instant::now()
        - std::time::Duration::from_millis(consts::INACTIVITY_TIMEOUT_MS + 100);

    // Conflated `last_activity` would have been refreshed by our own
    // sends — so dead clients would never be reaped. Splitting the
    // clocks closes that hole.
    assert!(
        ch.is_timed_out(),
        "is_timed_out must trigger on peer silence regardless of our outgoing traffic"
    );
}

#[test]
fn is_timed_out_does_not_fire_when_peer_is_chatty() {
    let mut ch = Channel::new(test_addr());
    // Both sides active recently — nothing to disconnect.
    ch.last_sent = std::time::Instant::now();
    ch.last_received = std::time::Instant::now();
    assert!(!ch.is_timed_out());
}

#[test]
fn touch_sent_only_moves_send_clock() {
    let mut ch = Channel::new(test_addr());
    let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
    ch.last_sent = baseline;
    ch.last_received = baseline;

    ch.touch_sent();

    assert!(ch.last_sent > baseline, "touch_sent must move last_sent");
    assert_eq!(
        ch.last_received, baseline,
        "touch_sent must NOT move last_received"
    );
}

#[test]
fn touch_received_only_moves_receive_clock() {
    let mut ch = Channel::new(test_addr());
    let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
    ch.last_sent = baseline;
    ch.last_received = baseline;

    ch.touch_received();

    assert!(
        ch.last_received > baseline,
        "touch_received must move last_received"
    );
    assert_eq!(
        ch.last_sent, baseline,
        "touch_received must NOT move last_sent"
    );
}

#[test]
fn check_timeouts_bumps_last_sent_when_retransmitting() {
    // A channel that's actively retransmitting a lost reliable packet
    // is putting bytes on the wire — the keepalive helper must observe
    // that activity and not emit redundant pings on top of the retries.
    //
    // Registers via the bytes-bearing path (#308 retransmit driver
    // commit) so the emitted retransmits vec is non-empty.
    let mut ch = Channel::new(test_addr());
    let mut pkt = test_packet();
    pkt.sequence = 0;
    ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"on-wire"))
        .unwrap();
    // Backdate both: the entry's last_sent so check_timeouts sees it as
    // expired (past the 1500ms default initial RTO), AND Channel.last_sent
    // so we can detect whether it moves.
    let backdate = std::time::Instant::now() - std::time::Duration::from_millis(1600);
    ch.tx_window[0].last_sent = backdate;
    ch.last_sent = backdate;

    let retransmits = ch.check_timeouts();
    assert_eq!(
        retransmits.len(),
        1,
        "expired packet should be retransmitted (raw bytes emitted)"
    );
    assert!(
        ch.last_sent > backdate,
        "check_timeouts emitting retransmits must bump Channel.last_sent"
    );
}

#[test]
fn check_timeouts_does_not_bump_last_sent_when_no_retransmits() {
    // A no-op pass (nothing expired) shouldn't perturb the keepalive
    // timer — otherwise the periodic tick would silently keep the
    // channel "active" forever.
    let mut ch = Channel::new(test_addr());
    ch.send_packet(test_packet()).unwrap();
    let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
    ch.last_sent = baseline;

    let retransmits = ch.check_timeouts();
    assert!(retransmits.is_empty());
    assert_eq!(
        ch.last_sent, baseline,
        "no-op check_timeouts must leave last_sent untouched"
    );
}

// ── register_sent_packet (services-layer migration helper) ────────

/// `register_sent_packet` adds an entry to the TX window with the
/// packet's pre-assigned sequence, without consuming `next_tx_seq`.
/// Used during the services-layer migration where the legacy send
/// path owns sequence assignment via a session-local `AtomicU32`.
#[test]
fn register_sent_packet_records_entry_without_consuming_next_tx_seq() {
    let mut ch = Channel::new(test_addr());

    let mut pkt = test_packet();
    pkt.sequence = 42; // caller pre-assigned

    let next_tx_seq_before = ch.next_tx_seq;
    let raw = bytes::Bytes::from_static(&[0xCA, 0xFE]);
    ch.register_sent_packet(pkt, raw.clone()).unwrap();

    assert_eq!(ch.tx_window.len(), 1);
    assert_eq!(
        ch.tx_window[0].packet.sequence, 42,
        "register_sent_packet must preserve caller-assigned sequence"
    );
    assert_eq!(
        ch.tx_window[0].raw_bytes, raw,
        "register_sent_packet must retain raw_bytes for retransmit"
    );
    assert_eq!(
        ch.next_tx_seq, next_tx_seq_before,
        "register_sent_packet must NOT increment next_tx_seq — that counter is owned by the legacy send path during the migration"
    );
    assert_eq!(ch.tx_window[0].retransmit_count, 0);
}

/// A subsequent ack of the registered sequence drains the entry just
/// like a `send_packet`-originated one, and feeds the RTT sample to
/// the RTO smoother (clean round — `retransmit_count == 0`).
#[test]
fn registered_packet_is_acked_and_samples_rto_normally() {
    let mut ch = Channel::with_rto_config(test_addr(), fast_rto_config());

    let mut pkt = test_packet();
    pkt.sequence = 7;
    ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"raw"))
        .unwrap();
    // Backdate so the ack measures ~80ms RTT.
    ch.tx_window[0].last_sent = std::time::Instant::now() - std::time::Duration::from_millis(80);

    ch.process_acks(7).unwrap();
    assert!(
        ch.tx_window.is_empty(),
        "ack must drain the registered entry"
    );
    let srtt = ch.rto().srtt().expect("srtt set after ack");
    assert!(
        srtt >= std::time::Duration::from_millis(75)
            && srtt <= std::time::Duration::from_millis(100),
        "registered packets sample RTO same as send_packet ones, got {srtt:?}"
    );
}

// ── Adaptive RTO integration ──────────────────────────────────────

use super::rto::RtoConfig;

/// Tight RtoConfig for tests — sub-second RTO bounds keep test runtimes
/// short without changing the algorithm's behavior.
fn fast_rto_config() -> RtoConfig {
    RtoConfig {
        min: std::time::Duration::from_millis(10),
        max: std::time::Duration::from_millis(500),
        initial_srtt: std::time::Duration::from_millis(50),
    }
}

/// Clean ack (the packet was never retransmitted) feeds an RTT sample
/// into the per-channel RTO smoother. After one sample, srtt should
/// reflect the observed round-trip time.
#[test]
fn process_acks_samples_rto_on_clean_round() {
    let mut ch = Channel::with_rto_config(test_addr(), fast_rto_config());
    ch.send_packet(test_packet()).unwrap();
    // Backdate the entry's last_sent so the ack measures ~80ms RTT.
    ch.tx_window[0].last_sent = std::time::Instant::now() - std::time::Duration::from_millis(80);

    assert_eq!(ch.rto().srtt(), None, "no samples yet");
    ch.process_acks(0).unwrap();
    assert!(ch.tx_window.is_empty(), "ack must drain the entry");

    let srtt = ch.rto().srtt().expect("srtt set after first ack");
    // Allow a few ms slack for instant-now() drift between the backdate
    // and the process_acks call.
    assert!(
        srtt >= std::time::Duration::from_millis(75)
            && srtt <= std::time::Duration::from_millis(100),
        "srtt ~= 80ms, got {srtt:?}",
    );
}

/// Karn's algorithm: an ack of a retransmitted packet MUST NOT update
/// the RTO smoother, because we don't know which copy of the send the
/// ack corresponds to. Pin so a regression that drops the
/// `retransmit_count == 0` guard surfaces here.
#[test]
fn process_acks_skips_rto_sample_on_retransmitted_packet_karn() {
    let mut ch = Channel::with_rto_config(test_addr(), fast_rto_config());
    ch.send_packet(test_packet()).unwrap();
    // Simulate that the packet was retransmitted at some point — we don't
    // care when, only that retransmit_count > 0 by ack time.
    ch.tx_window[0].retransmit_count = 1;
    ch.tx_window[0].last_sent = std::time::Instant::now() - std::time::Duration::from_millis(80);

    ch.process_acks(0).unwrap();
    assert!(ch.tx_window.is_empty(), "ack must still drain the entry");
    assert_eq!(
        ch.rto().srtt(),
        None,
        "Karn's algorithm: retransmitted-packet samples must be excluded",
    );
}

/// `check_timeouts` uses the adaptive RTO, not a fixed constant. With
/// initial_srtt=50ms → initial RTO=150ms (3 × srtt clamped), a 200ms
/// backdate fires the timeout; a 50ms backdate does not.
#[test]
fn check_timeouts_fires_at_adaptive_rto_not_fixed_constant() {
    let mut ch = Channel::with_rto_config(test_addr(), fast_rto_config());
    let mut pkt = test_packet();
    pkt.sequence = 0;
    ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"raw"))
        .unwrap();
    // 50ms ago — under the 150ms initial RTO.
    ch.tx_window[0].last_sent = std::time::Instant::now() - std::time::Duration::from_millis(50);
    assert!(
        ch.check_timeouts().is_empty(),
        "50ms < 150ms RTO — no retransmit yet"
    );

    // 200ms ago — past the 150ms initial RTO.
    ch.tx_window[0].last_sent = std::time::Instant::now() - std::time::Duration::from_millis(200);
    let retx = ch.check_timeouts();
    assert_eq!(
        retx.len(),
        1,
        "200ms > 150ms RTO — retransmit fires (with raw bytes)"
    );
}

/// `check_timeouts` calls `rto.on_retransmit` ONCE per scan, not per
/// retransmitted entry. Doubling the backoff per-entry would compound
/// catastrophically if the TX window has multiple packets in flight
/// during a stall — pin this so a refactor moving the `on_retransmit`
/// into the per-entry loop is caught at test time.
#[test]
fn check_timeouts_doubles_rto_once_per_scan_not_per_entry() {
    let mut ch = Channel::with_rto_config(test_addr(), fast_rto_config());
    for seq in 0..3 {
        let mut pkt = test_packet();
        pkt.sequence = seq;
        ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"raw"))
            .unwrap();
    }

    // Backdate ALL three entries past the 150ms initial RTO.
    let backdate = std::time::Instant::now() - std::time::Duration::from_millis(200);
    for entry in ch.tx_window.iter_mut() {
        entry.last_sent = backdate;
    }

    let rto_before = ch.rto().current();
    let retx = ch.check_timeouts();
    assert_eq!(retx.len(), 3, "all three expired");

    let rto_after = ch.rto().current();
    // Doubled once (per scan), not three times (per entry).
    assert_eq!(
        rto_after,
        rto_before * 2,
        "RTO must double once per scan, not per retransmitted entry"
    );
}

/// Per-tick retransmit work budget of 5. With 7 expired entries in the
/// TX window, a single `check_timeouts` scan processes exactly 5 (the
/// spec-mandated budget). The remaining 2 wait for the next tick.
/// Pinning this protects against a refactor that drops the budget and
/// lets a saturated link blast the whole TX window in one tick
/// (worsening congestion). The cap mirrors
/// `UnAckedHandler::checkResendTimers` at `ghidra://SGW.exe@0x0158c420`.
#[test]
fn check_timeouts_caps_retransmits_per_scan_at_five_per_spec() {
    let mut ch = Channel::with_rto_config(test_addr(), fast_rto_config());
    // Register 7 packets with bytes so they're all eligible for retransmit.
    for seq in 0..7 {
        let mut pkt = test_packet();
        pkt.sequence = seq;
        ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"raw"))
            .unwrap();
    }
    // All 7 backdated past the 150ms RTO.
    let backdate = std::time::Instant::now() - std::time::Duration::from_millis(200);
    for entry in ch.tx_window.iter_mut() {
        entry.last_sent = backdate;
    }

    let retx = ch.check_timeouts();
    assert_eq!(
        retx.len(),
        consts::RETRANSMIT_BUDGET_PER_TICK,
        "first scan must cap at the per-tick budget (5), even with 7 expired"
    );
    // The first 5 entries had retransmit_count bumped + last_sent refreshed.
    // The remaining 2 are untouched, ready for next scan.
    for (i, entry) in ch.tx_window.iter().enumerate() {
        if i < consts::RETRANSMIT_BUDGET_PER_TICK {
            assert_eq!(entry.retransmit_count, 1, "first 5 retransmitted once");
        } else {
            assert_eq!(entry.retransmit_count, 0, "remaining 2 untouched");
        }
    }
}

/// A no-op `check_timeouts` (no entries expired) MUST NOT call
/// `rto.on_retransmit` — the backoff is supposed to fire on actual
/// retransmits, not on every scan-where-nothing-happened.
#[test]
fn check_timeouts_does_not_touch_rto_when_no_retransmits() {
    let mut ch = Channel::with_rto_config(test_addr(), fast_rto_config());
    ch.send_packet(test_packet()).unwrap();
    // Fresh — well within RTO.

    let rto_before = ch.rto().current();
    let retx = ch.check_timeouts();
    assert!(retx.is_empty());
    assert_eq!(
        ch.rto().current(),
        rto_before,
        "no retransmits means no backoff"
    );
}
