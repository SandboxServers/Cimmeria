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
pub mod codec;
pub mod encryption;
pub mod messages;
pub mod nub;
pub mod packet;
pub mod unified;
pub mod unpacker;

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

    /// Transmit window size — limits unacknowledged in-flight packets.
    pub const TX_WINDOW_SIZE: usize = 45;

    /// Milliseconds before a reliable packet is considered lost and retransmitted.
    pub const ACK_TIMEOUT_MS: u64 = 700;

    /// Maximum retransmission attempts before the channel is considered dead.
    pub const MAX_RETRIES: u32 = 20;

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

    /// How long an in-progress fragment reassembly is kept before being
    /// discarded as orphaned. Tied to the worst-case interval over which
    /// the remaining fragments could still arrive — generous because UDP
    /// reordering can stretch fragment delivery, and the cost of holding
    /// a few KB longer is far less than the cost of dropping a usable
    /// bundle that was about to complete.
    pub const FRAGMENT_REASSEMBLY_TIMEOUT_MS: u64 = 30_000;

    /// Protocol version exchanged during channel creation handshake.
    pub const PROTOCOL_VERSION: u32 = 391;
}
