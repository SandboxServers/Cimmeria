//! Per-session packet tap (issue #688, phase 5).
//!
//! A packet tap captures the **decoded** Mercury messages for ONE client
//! session, in both directions, into a bounded ring buffer that the lab MCP
//! endpoint drains on demand (`server_packet_tap_start` / `_read` / `_stop`).
//!
//! # Why it lives here
//!
//! [`super::log_inbound`] and [`super::log_outbound_entity_method`] are the two
//! points in the server where a Mercury message is available *decoded* and
//! attributed to a session — inbound by the peer [`SocketAddr`], outbound by
//! the witness entity id. Every other decoded seam funnels through these two.
//! The tap hangs off them so it sees exactly what the `wire.in` / `wire.out`
//! SigNoz stream sees, without a second decode path that could drift.
//!
//! # Per-session isolation
//!
//! The user names a session by its **player entity id** — the same id
//! `server_sessions` reports. A tap started for entity `E` records:
//!
//! - outbound messages whose `witness_id == E` (the outbound seam is already
//!   entity-keyed), and
//! - inbound messages from `E`'s socket address, resolved once at
//!   [`start`] time via `entity_to_addr` and stored in [`Registry::addr_to_entity`].
//!
//! A message for any other session key hits neither index and is never
//! recorded, so a tap on A cannot capture B's traffic. If `E` reconnects on a
//! new ephemeral port mid-tap, inbound capture stops until the tap is restarted
//! (the address binding is resolved once, at start) — acceptable for a live
//! debugging session and documented so it isn't mistaken for a bug.
//!
//! # Cost when no tap is active
//!
//! [`record_inbound`] / [`record_outbound`] run on the hot wire path. When no
//! tap is active they do a single relaxed atomic load and return — no lock, no
//! allocation. The decode/hex work happens only for a session that is actually
//! tapped.
//!
//! # Bounded ring
//!
//! Each tap holds at most `capacity` messages. When full, the oldest is
//! dropped and a per-ring `dropped` counter increments. [`read`] returns the
//! buffered messages *and* the dropped count, then clears both, so a caller
//! polling the tap always knows whether it missed anything between reads.

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// Default ring capacity when a tap is started without an explicit cap.
pub const DEFAULT_CAPACITY: usize = 500;

/// Hard upper bound on a tap's ring capacity. A caller-requested capacity is
/// clamped to this so a single tap cannot pin an unbounded amount of memory.
pub const MAX_CAPACITY: usize = 10_000;

/// Direction of a tapped message, server-relative.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Dir {
    /// Client → server (from the bundle scanner).
    In,
    /// Server → client (an entity-method fan-out).
    Out,
}

/// One decoded message captured by a tap.
#[derive(Clone, Debug, Serialize)]
pub struct TappedMessage {
    /// Server wall-clock capture time, milliseconds since the Unix epoch.
    pub ts_ms: u64,
    pub dir: Dir,
    /// Inbound Mercury message id (`0x01`..=`0xFF`); `None` for outbound.
    pub msg_id: Option<u8>,
    /// Flattened method index (cell / base method space), or `-1` for a
    /// system message with no method index.
    pub method_index: i32,
    /// Human-readable method / message name.
    pub msg_name: &'static str,
    /// Outbound only: the entity whose method is being sent.
    pub target_entity_id: Option<u32>,
    /// Payload length in bytes (before hex truncation).
    pub args_len: usize,
    /// First [`super::HEX_DUMP_CAP`] bytes of the payload, hex-encoded.
    pub args_hex: String,
    /// Structured decode when a schema decoder is registered for the method.
    pub decoded: Option<serde_json::Value>,
}

/// One tap's drained contents plus the dropped-since-last-read count.
#[derive(Clone, Debug, Serialize)]
pub struct TapReadResult {
    /// The player entity id this tap is bound to.
    pub entity_id: u32,
    /// Messages captured since the previous [`read`] (oldest first).
    pub messages: Vec<TappedMessage>,
    /// Messages dropped (oldest-first) because the ring was full since the
    /// previous read. `0` means no loss.
    pub dropped: u64,
    /// The ring's configured capacity.
    pub capacity: usize,
}

/// Status line for one active tap, for `server_packet_tap` listings.
#[derive(Clone, Debug, Serialize)]
pub struct TapStatus {
    pub entity_id: u32,
    pub buffered: usize,
    pub dropped: u64,
    pub capacity: usize,
}

/// A single session's bounded capture ring.
struct Ring {
    buf: VecDeque<TappedMessage>,
    capacity: usize,
    dropped: u64,
}

impl Ring {
    fn new(capacity: usize) -> Self {
        Self {
            buf: VecDeque::with_capacity(capacity.min(1024)),
            capacity,
            dropped: 0,
        }
    }

    /// Push, dropping the oldest (and counting it) when at capacity.
    fn push(&mut self, msg: TappedMessage) {
        if self.buf.len() >= self.capacity {
            self.buf.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        self.buf.push_back(msg);
    }
}

/// The global tap table. Two indexes into the same set of rings: `rings` is
/// keyed by the player entity id (outbound + the read/stop API), and
/// `addr_to_entity` resolves an inbound peer address to its entity id.
#[derive(Default)]
struct Registry {
    rings: HashMap<u32, Ring>,
    addr_to_entity: HashMap<SocketAddr, u32>,
}

fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

/// Number of active taps — the hot-path fast-bail gate. Kept in sync with
/// `Registry::rings.len()` under the registry lock.
static ACTIVE: AtomicUsize = AtomicUsize::new(0);

/// Start (or restart) a tap on session `entity_id`, whose socket is `addr`.
///
/// `capacity` is clamped to `[1, MAX_CAPACITY]`; `None` uses [`DEFAULT_CAPACITY`].
/// Restarting an existing tap replaces its ring (buffered messages and dropped
/// count are discarded) and returns `true`; a fresh tap returns `false`.
pub fn start(entity_id: u32, addr: SocketAddr, capacity: Option<usize>) -> bool {
    let cap = capacity.unwrap_or(DEFAULT_CAPACITY).clamp(1, MAX_CAPACITY);
    let mut reg = registry().lock().unwrap();
    // Drop any stale address binding pointing at this entity from a prior tap
    // (e.g. a restart after a reconnect on a different port).
    reg.addr_to_entity.retain(|_, &mut e| e != entity_id);
    let replaced = reg.rings.insert(entity_id, Ring::new(cap)).is_some();
    reg.addr_to_entity.insert(addr, entity_id);
    ACTIVE.store(reg.rings.len(), Ordering::Relaxed);
    replaced
}

/// Stop the tap on `entity_id`, discarding its ring. Returns `true` if a tap
/// was active.
pub fn stop(entity_id: u32) -> bool {
    let mut reg = registry().lock().unwrap();
    let existed = reg.rings.remove(&entity_id).is_some();
    reg.addr_to_entity.retain(|_, &mut e| e != entity_id);
    ACTIVE.store(reg.rings.len(), Ordering::Relaxed);
    existed
}

/// Drain the tap on `entity_id`: return its buffered messages and dropped
/// count, then clear both. `None` if no tap is active for that entity.
pub fn read(entity_id: u32) -> Option<TapReadResult> {
    let mut reg = registry().lock().unwrap();
    let ring = reg.rings.get_mut(&entity_id)?;
    let messages: Vec<TappedMessage> = ring.buf.drain(..).collect();
    let dropped = ring.dropped;
    let capacity = ring.capacity;
    ring.dropped = 0;
    Some(TapReadResult {
        entity_id,
        messages,
        dropped,
        capacity,
    })
}

/// Snapshot every active tap (entity id, buffered count, dropped, capacity).
pub fn active_taps() -> Vec<TapStatus> {
    let reg = registry().lock().unwrap();
    let mut out: Vec<TapStatus> = reg
        .rings
        .iter()
        .map(|(&entity_id, ring)| TapStatus {
            entity_id,
            buffered: ring.buf.len(),
            dropped: ring.dropped,
            capacity: ring.capacity,
        })
        .collect();
    out.sort_unstable_by_key(|s| s.entity_id);
    out
}

/// `true` when at least one tap is active — the hot-path fast bail.
#[inline]
fn any_active() -> bool {
    ACTIVE.load(Ordering::Relaxed) != 0
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Record an inbound message against the tap for `peer`'s session, if one is
/// active. Called from [`super::log_inbound`]. No-op (single atomic load) when
/// no tap is active.
pub(super) fn record_inbound(peer: SocketAddr, msg_id: u8, payload: &[u8]) {
    if !any_active() {
        return;
    }
    let mut reg = registry().lock().unwrap();
    let Some(&entity_id) = reg.addr_to_entity.get(&peer) else {
        return;
    };
    let Some(ring) = reg.rings.get_mut(&entity_id) else {
        return;
    };
    let method_index: i32 = match msg_id {
        0xBD => payload.get(4).map_or(-1, |s| 61 + i32::from(*s)),
        0x80..=0xBC => i32::from(msg_id - 0x80),
        0xC0..=0xFF => i32::from(msg_id - 0xC0),
        _ => -1,
    };
    ring.push(TappedMessage {
        ts_ms: now_ms(),
        dir: Dir::In,
        msg_id: Some(msg_id),
        method_index,
        msg_name: super::client_names::inbound_msg_name_with_payload(msg_id, payload),
        target_entity_id: None,
        args_len: payload.len(),
        args_hex: super::hex_truncate(payload),
        decoded: super::decoders::decode_inbound(msg_id, payload),
    });
}

/// Record an outbound entity-method call against the tap for the `witness_id`
/// session, if one is active. Called from [`super::log_outbound_entity_method`].
/// No-op (single atomic load) when no tap is active.
pub(super) fn record_outbound(
    witness_id: u32,
    target_entity_id: u32,
    method_index: u16,
    args: &[u8],
) {
    if !any_active() {
        return;
    }
    let mut reg = registry().lock().unwrap();
    let Some(ring) = reg.rings.get_mut(&witness_id) else {
        return;
    };
    ring.push(TappedMessage {
        ts_ms: now_ms(),
        dir: Dir::Out,
        msg_id: None,
        method_index: i32::from(method_index),
        msg_name: super::client_names::outbound_method_name(method_index),
        target_entity_id: Some(target_entity_id),
        args_len: args.len(),
        args_hex: super::hex_truncate(args),
        decoded: super::decoders::decode_outbound(method_index, args),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    /// The ring is bounded: pushing more than `capacity` messages drops the
    /// OLDEST and reports the dropped count on the next read. Regression shape:
    /// an unbounded ring (or a drop-newest ring) would fail one of the two
    /// assertions — the tap must never grow without bound, and a `read` must
    /// tell the caller it lost the earliest messages.
    #[test]
    fn ring_bound_drops_oldest_and_counts() {
        let eid = 0xA688_0001;
        start(eid, addr(40001), Some(3));

        // Push 5 inbound; capacity 3 → 2 dropped, newest 3 retained.
        for i in 0..5u8 {
            record_inbound(addr(40001), 0x80 + i, &[i]);
        }

        let r = read(eid).expect("tap must be active");
        assert_eq!(r.capacity, 3);
        assert_eq!(r.dropped, 2, "oldest 2 of 5 must be dropped at cap 3");
        assert_eq!(r.messages.len(), 3, "ring holds at most capacity");
        // The retained window is the NEWEST three (msg_ids 0x82,0x83,0x84).
        let ids: Vec<u8> = r.messages.iter().filter_map(|m| m.msg_id).collect();
        assert_eq!(
            ids,
            vec![0x82, 0x83, 0x84],
            "must retain newest, drop oldest"
        );

        // A second read after no new traffic is empty with dropped reset.
        let r2 = read(eid).expect("tap still active");
        assert!(r2.messages.is_empty());
        assert_eq!(r2.dropped, 0, "dropped count resets after read");

        stop(eid);
    }

    /// A tap on session A must not capture session B's traffic — in EITHER
    /// direction. Regression shape: keying the ring on the wrong index (or a
    /// single shared ring) would leak B's inbound (different addr) or B's
    /// outbound (different witness id) into A's read.
    #[test]
    fn per_session_isolation_both_directions() {
        let a = 0xA688_0010;
        let b = 0xA688_0011;
        start(a, addr(40010), Some(100));
        start(b, addr(40011), Some(100));

        // Inbound from B's address, outbound to B's witness id.
        record_inbound(addr(40011), 0x90, &[1, 2, 3]);
        record_outbound(b, b, 5, &[9, 9]);
        // One inbound + one outbound genuinely for A.
        record_inbound(addr(40010), 0x91, &[4]);
        record_outbound(a, 12345, 7, &[7]);

        let ra = read(a).expect("A active");
        assert_eq!(ra.messages.len(), 2, "A sees only its own two messages");
        assert!(
            ra.messages.iter().all(|m| m.target_entity_id != Some(b)),
            "A must not capture any message addressed to B"
        );
        assert_eq!(ra.dropped, 0);

        let rb = read(b).expect("B active");
        assert_eq!(rb.messages.len(), 2, "B still holds its own two messages");

        stop(a);
        stop(b);
    }

    /// After `stop`, the hot-path recorders are inert for that session and a
    /// read returns `None`.
    #[test]
    fn stop_makes_session_inert() {
        let eid = 0xA688_0020;
        start(eid, addr(40020), None);
        record_inbound(addr(40020), 0x80, &[0]);
        assert!(stop(eid), "stop reports the tap existed");
        assert!(read(eid).is_none(), "no tap after stop");
        // A late packet for the stopped session must not resurrect a ring.
        record_inbound(addr(40020), 0x80, &[0]);
        assert!(read(eid).is_none(), "stopped session stays inert");
    }

    /// Capacity is clamped into `[1, MAX_CAPACITY]` so a caller cannot request
    /// an unbounded (or zero-length) ring.
    #[test]
    fn capacity_is_clamped() {
        let eid = 0xA688_0030;
        start(eid, addr(40030), Some(usize::MAX));
        let status = active_taps()
            .into_iter()
            .find(|s| s.entity_id == eid)
            .expect("tap present");
        assert_eq!(status.capacity, MAX_CAPACITY, "huge cap clamps to MAX");
        stop(eid);

        start(eid, addr(40030), Some(0));
        let status = active_taps()
            .into_iter()
            .find(|s| s.entity_id == eid)
            .expect("tap present");
        assert_eq!(status.capacity, 1, "zero cap clamps up to 1");
        stop(eid);
    }
}
