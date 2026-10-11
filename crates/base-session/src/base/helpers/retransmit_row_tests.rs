//! TG-MER-02 guard: a reliable resend writes one `mercury.retransmit` row
//! that carries its cause fields, and nothing else on the resend path.
//!
//! The row is written by the channel's scan (`Channel::check_timeouts`),
//! which the tick loop reaches through `collect_pending_retransmits`. The
//! test drives the real tick loop, so it sees both the channel's row and
//! anything the loop itself logs around the resend.

use std::time::{Duration, Instant};

use super::*;
use tracing::Level;

use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use cimmeria_mercury::packet::{Bytes, Packet};

/// A clock the test moves by hand, so the channel sees the entry as past its
/// RTO without the test waiting seconds of real time.
struct ManualClock {
    base: Instant,
    offset: Mutex<Duration>,
}

impl cimmeria_mercury::clock::Clock for ManualClock {
    fn now(&self) -> Instant {
        self.base + *self.offset.lock().unwrap()
    }
}

/// An entry unacked past the RTO ceiling (4 s) is resent on the first tick.
/// Its row says `event=retransmit` with `age_ms` (time since the first send,
/// at least the 5 s the clock moved), the peer's silence, and the datagram's
/// fingerprint. The tick loop's old "RTO fired before ACK" DEBUG row, which
/// duplicated it per resend, is gone. Reverting the cause fields drops
/// `event`/`age_ms`; restoring the duplicate row trips the last assertion.
#[tokio::test]
async fn retransmit_row_has_age_and_no_duplicate() {
    let addr: SocketAddr = "127.0.0.1:52411".parse().unwrap();
    let clock = Arc::new(ManualClock {
        base: Instant::now(),
        offset: Mutex::new(Duration::ZERO),
    });
    let wire = vec![0xA5u8; 48];
    let mut channel = cimmeria_mercury::channel::Channel::with_clock(addr, clock.clone());
    channel
        .register_sent_packet(
            Packet::new(Default::default(), 1, Bytes::new()),
            Bytes::from(wire.clone()),
        )
        .unwrap();
    let mut state = test_default_connected_client_state();
    state.channel = Mutex::new(channel);
    let cancelled = Arc::clone(&state.cancelled);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    // Past the RTO ceiling: the entry is expired on the loop's first scan.
    *clock.offset.lock().unwrap() += Duration::from_secs(5);

    let capture = LogCapture::install();
    // The loop runs until cancelled; a few 100 ms ticks are enough for the
    // first scan. The manual clock does not move again, so the backed-off
    // RTO keeps later scans quiet.
    let _ = tokio::time::timeout(
        Duration::from_millis(450),
        crate::base::tick_sync::run_tick_loop(
            Arc::new(TestTransport::new()),
            addr,
            [0u8; 32],
            Default::default(),
            Arc::new(std::sync::atomic::AtomicU32::new(0)),
            Arc::new(Mutex::new(Vec::new())),
            Arc::new(Mutex::new(Instant::now())),
            Arc::clone(&cancelled),
            Arc::clone(&connected),
            Arc::new(Mutex::new(cimmeria_entity::manager::EntityManager::new())),
            None,
            Arc::new(Mutex::new(HashMap::new())),
            None,
        ),
    )
    .await;
    cancelled.store(true, std::sync::atomic::Ordering::Relaxed);

    let events = capture.all();
    let rows: Vec<_> = events
        .iter()
        .filter(|e| e.target == "mercury.retransmit")
        .collect();
    assert_eq!(rows.len(), 1, "one resend, one row: {events:#?}");
    let row = rows[0];
    assert_eq!(row.level, Level::INFO, "{row:#?}");
    assert!(row.has_field("event", "retransmit"), "{row:#?}");
    assert!(row.has_field("seq", "1"), "{row:#?}");
    let age_ms: u64 = row
        .fields
        .get("age_ms")
        .unwrap_or_else(|| panic!("no age_ms: {row:#?}"))
        .parse()
        .expect("age_ms is a number");
    assert!(age_ms >= 5_000, "age_ms {age_ms} must cover the 5 s");
    row.fields
        .get("peer_last_rx_ms")
        .unwrap_or_else(|| panic!("no peer_last_rx_ms: {row:#?}"))
        .parse::<u64>()
        .expect("peer_last_rx_ms is a number");
    assert!(
        row.has_field(
            "wire_fingerprint",
            &cimmeria_mercury::instrumentation::wire_fingerprint(&wire)
        ),
        "{row:#?}"
    );
    // No clean RTT sample and no recorded head: both are left off, never
    // written as "None".
    assert!(!row.fields.contains_key("srtt_ms"), "{row:#?}");
    assert!(!row.fields.contains_key("msg_id"), "{row:#?}");

    assert!(
        !events.iter().any(|e| e.message_contains("RTO fired")),
        "the duplicate per-resend row is gone: {events:#?}"
    );
}
