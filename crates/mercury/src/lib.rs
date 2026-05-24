//! # cimmeria-mercury
//!
//! Implementation of the Mercury network protocol used by Stargate Worlds.
//! Mercury is a reliable UDP protocol with fragmentation, channel management,
//! and AES-256-CBC encryption. This implementation must produce wire output
//! that is **byte-identical** to the original C++ BigWorld/Mercury stack.
//!
//! Additionally provides TCP inter-service framing via the Unified protocol
//! (length-prefixed messages between Auth, Base, and Cell services).

pub mod bundle;
pub mod channel;
pub mod channel_bundle;
pub mod clock;
pub mod codec;
pub mod encryption;
pub mod messages;
pub mod packet;
pub mod transport;
pub mod unified;
pub mod unpacker;

/// Recording `Transport` fake for byte-exact fan-out assertions. Available to
/// mercury's own tests and to consumer crates that enable the `test-support`
/// feature; excluded from production builds. See [`test_transport`].
#[cfg(any(test, feature = "test-support"))]
pub mod test_transport;

/// Tier 2 loopback Mercury session harness. Pairs two `Channel`s on real
/// loopback `UdpSocket`s and runs end-to-end protocol tests covering reliable
/// delivery under simulated loss, fragment reassembly, keepalive cadence,
/// encryption round-trip, the handshake, ack aggregation, and adaptive RTO
/// convergence. Available to mercury's own tests and to consumer crates that
/// enable the `test-harness` feature; excluded from production builds. See
/// `docs/architecture/mercury-loopback-harness.md` for the rationale and
/// for how this differs from [`test_transport`] (Tier 1, byte-exact fan-out)
/// and from the wireclient (#281, content-aware end-to-end).
#[cfg(any(test, feature = "test-harness"))]
pub mod test_harness;

/// Mercury protocol constants — these MUST match the C++ implementation exactly.
///
/// Values are derived from reverse-engineering the original BigWorld Mercury
/// networking layer as used by Stargate Worlds (protocol version 391).
pub mod consts {
    /// Maximum UDP packet size including headers (MTU-safe).
    pub const PACKET_MAX_SIZE: usize = 1472;

    /// Size of the Mercury packet header (1-byte flags field).
    pub const HEADER_SIZE: usize = 1;

    /// Maximum payload body per packet after header and footers.
    pub const MAX_BODY: usize = 1348;

    /// Receive window size for reliable sequencing.
    pub const RX_WINDOW_SIZE: usize = 64;

    /// Transmit window size — limits unacknowledged in-flight reliable
    /// packets per channel.
    ///
    /// Pinned at **32** because the SGW client's `ChannelInternal` slot
    /// store is sized from a runtime config value at `[ChannelInternal+0x2C]`
    /// that defaults to 32 in the unmodified binary. Sending more than 32
    /// in-flight reliable packets would let two seqs differing by 32 collide
    /// on the same slot in the client's `[+0x40]` hash table (mask at
    /// `[+0x44]` = `capacity - 1`), causing `queueAckForPacket` at
    /// `ghidra://SGW.exe@0x0158cba0` to overwrite the older unacked entry —
    /// the server would then phantom-ack the older seq when the newer one
    /// was actually acked.
    ///
    /// This constant can be raised once the SGW.exe binary is patched to
    /// widen the slot store (a 3-byte patch at VA `0x0158C801` rewriting
    /// the capacity push). Until that ships, server-side overflow is
    /// handled by the deferred-send queue (see [`MAX_UNSENT_PACKETS`])
    /// rather than by raising this value.
    pub const TX_WINDOW_SIZE: usize = 32;

    /// Per-channel cap on the deferred-send queue used when the TX window
    /// is full.
    ///
    /// When `Channel::register_sent_packet` would overflow `TX_WINDOW_SIZE`,
    /// the packet is held in a per-channel queue and promoted into the TX
    /// window as ACKs free slots. The queue prevents the silent-best-effort
    /// downgrade that the previous overflow path performed.
    ///
    /// The cap exists so a genuinely dead channel (peer has stopped acking
    /// entirely) cannot grow unbounded. 1024 = 32× the TX window, which
    /// gives a comfortable transient-stall margin while keeping per-channel
    /// memory bounded. When the cap is hit the new send returns an error;
    /// the inactivity timeout will reap the channel shortly thereafter.
    pub const MAX_UNSENT_PACKETS: usize = 1024;

    /// Milliseconds before a reliable packet is considered lost and retransmitted.
    pub const ACK_TIMEOUT_MS: u64 = 700;

    /// Maximum retransmission attempts before the channel is considered dead.
    pub const MAX_RETRIES: u32 = 20;

    /// Per-tick retransmission work budget — `mercury-wire-format` spec
    /// §1.7 + §2.4.1 R14 + §2.10 S7.
    ///
    /// `Channel::check_timeouts` processes at most this many expired
    /// TX-window entries per scan before yielding. Mirrors the SGW
    /// client's `UnAckedHandler::checkResendTimers` at
    /// `ghidra://SGW.exe@0x0158c420`, which carries an IEEE 754 `5.0`
    /// budget counter that decrements per processed entry and exits
    /// when negative. Caps the cost of one tick in the pathological
    /// "every TX-window entry is past RTO" case (a sustained link
    /// stall would otherwise have us blast 32 retransmits in a single
    /// tick, possibly making the congestion worse).
    pub const RETRANSMIT_BUDGET_PER_TICK: usize = 5;

    /// Keepalive interval in milliseconds for idle channels.
    pub const KEEPALIVE_INTERVAL_MS: u64 = 1000;

    /// How long a channel may go without observing any peer-originated
    /// packet before it's considered dead and reaped. Mirrors C++ Mercury's
    /// `client_inactivity_timeout` rather than reusing keepalive math —
    /// the two are separate dimensions (we want to keep NAT mappings warm
    /// far more frequently than we want to declare a peer dead).
    pub const INACTIVITY_TIMEOUT_MS: u64 = 300_000;

    /// Maximum number of fragments a single message may be split across.
    pub const MAX_FRAGMENTS: usize = 64;

    /// Protocol version exchanged during channel creation handshake.
    pub const PROTOCOL_VERSION: u32 = 391;
}
