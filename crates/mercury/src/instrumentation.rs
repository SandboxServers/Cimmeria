//! Mercury packet instrumentation — emits structured `tracing` events
//! for every packet entering or leaving the server.
//!
//! # Why this module exists
//!
//! Mercury packets cross several seams (channel send/receive, Unified
//! TCP codec encode/decode). Each could emit ad-hoc `tracing::info!`
//! calls, but the field naming would drift and the downstream SigNoz
//! query "show me packets" would need three different filters.
//!
//! Centralising the field schema in two helpers means a single
//! `target = "mercury.packet"` filter catches every wire-level event
//! across both transports — and the LLM consumer (Cimmeria-MCP) gets
//! a stable, documented schema to query against.
//!
//! # Event schema
//!
//! Every event uses `target = "mercury.packet"`. Common fields:
//!
//! | Field | Type | Meaning |
//! |---|---|---|
//! | `dir` | `"in"` or `"out"` | Server-relative direction. |
//! | `transport` | `"udp"` or `"tcp"` | UDP = client-facing Mercury, TCP = inter-service Unified. |
//! | `len` | u32 | Payload size in bytes (excludes framing). |
//! | `seq` | u32 (UDP only) | Mercury sequence number — wraps at 28 bits. |
//! | `flags` | u8 (UDP only) | Mercury PacketFlags bitfield. |
//! | `msg_id` | u8 (TCP only) | Unified protocol message ID — see message catalog. |
//! | `peer` | string (optional) | Peer SocketAddr / channel id, if known at the call site. |
//!
//! All events are emitted at **info** level — they're the load-bearing
//! analytical surface, not debug noise. Downstream filters can
//! down-sample if volume becomes a problem; we'd rather have more
//! data than less in the SigNoz store.
//!
//! # Cost
//!
//! `tracing::info!` with no subscribers attached is a single atomic
//! load + branch (the global subscriber check). With the OTLP layer
//! attached, the event is queued onto the batch exporter's channel —
//! the actual OTLP send happens on a background tokio task. Net cost
//! on the hot send/receive path: a handful of nanoseconds.

/// Direction of a packet relative to the server.
#[derive(Copy, Clone, Debug)]
pub enum Direction {
    /// Server received this packet from a peer.
    In,
    /// Server is about to send (or just sent) this packet to a peer.
    Out,
}

impl Direction {
    /// Stable string label that goes onto the tracing event. We use a
    /// `&'static str` rather than the Debug repr so SigNoz column
    /// values stay stable across Rust edition / formatter changes.
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::In => "in",
            Direction::Out => "out",
        }
    }
}

/// Record a UDP Mercury packet crossing the wire.
///
/// Called from [`crate::channel::Channel::send_packet`] (outbound) and
/// [`crate::channel::Channel::receive_packet`] (inbound). Cheap when no
/// subscriber is attached; cheap-but-batched when SigNoz/OTLP is active.
///
/// `peer` takes `impl std::fmt::Display` (in practice `&SocketAddr`) so
/// the hot path doesn't allocate a `String` per packet purely to format
/// the address. `tracing`'s `%` formatter writes the Display impl
/// directly into the event without intermediate heap.
#[inline]
pub fn record_udp_packet(
    dir: Direction,
    seq: u32,
    flags: u8,
    payload_len: usize,
    peer: impl std::fmt::Display,
) {
    tracing::info!(
        target: "mercury.packet",
        dir = dir.as_str(),
        transport = "udp",
        seq,
        flags,
        len = payload_len,
        peer = %peer,
        "mercury_packet"
    );
}

/// Record a TCP Unified inter-service frame crossing the wire.
///
/// Called from the [`crate::unified::UnifiedCodec`] encode/decode
/// implementations. The `msg_id` is the load-bearing field — it's how
/// the consumer crate routes the payload to a handler, so it's the
/// natural primary key for downstream analytical queries.
#[inline]
pub fn record_tcp_frame(dir: Direction, msg_id: u8, payload_len: usize) {
    tracing::info!(
        target: "mercury.packet",
        dir = dir.as_str(),
        transport = "tcp",
        msg_id,
        len = payload_len,
        "unified_frame"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Direction labels must be stable strings — downstream consumers
    /// (SigNoz dashboards, Cimmeria-MCP queries) key off these literal
    /// values, so a typo or rename would silently break analytical
    /// queries with no compile-time signal.
    #[test]
    fn direction_labels_are_stable() {
        assert_eq!(Direction::In.as_str(), "in");
        assert_eq!(Direction::Out.as_str(), "out");
    }

    /// The record functions must not panic regardless of input — they
    /// are called on the hot wire path and a panic here would tear
    /// down the entire connection. (tracing!() macros are guaranteed
    /// not to panic, but the test pins the invariant against future
    /// drift, e.g. someone adding a non-tracing assertion inside the
    /// helper.)
    #[test]
    fn record_helpers_do_not_panic() {
        use std::net::SocketAddr;
        let peer: SocketAddr = "127.0.0.1:0".parse().unwrap();
        record_udp_packet(Direction::In, 0, 0, 0, peer);
        record_udp_packet(Direction::Out, u32::MAX, 0xFF, usize::MAX, peer);
        record_tcp_frame(Direction::In, 0, 0);
        record_tcp_frame(Direction::Out, u8::MAX, usize::MAX);
    }
}
