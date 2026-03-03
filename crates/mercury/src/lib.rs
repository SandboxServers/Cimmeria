//! # cimmeria-mercury
//!
//! Implementation of the Mercury network protocol used by Stargate Worlds.
//! Mercury is a reliable UDP protocol with fragmentation, channel management,
//! and AES-256-CBC encryption. This implementation must produce wire output
//! that is **byte-identical** to the original C++ BigWorld/Mercury stack.
//!
//! Additionally provides TCP inter-service framing via the Unified protocol
//! (length-prefixed messages between Auth, Base, and Cell services).

pub mod packet;
pub mod channel;
pub mod nub;
pub mod bundle;
pub mod unpacker;
pub mod encryption;
pub mod unified;
pub mod codec;
pub mod messages;

/// Mercury protocol constants — these MUST match the C++ implementation exactly.
///
/// Values are derived from reverse-engineering the original BigWorld Mercury
/// networking layer as used by Stargate Worlds (protocol version 391).
pub mod consts {
    /// Maximum UDP packet size including headers (MTU-safe).
    pub const PACKET_MAX_SIZE: usize = 1472;

    /// Size of the Mercury packet header (sequence + flags packed into u32).
    pub const HEADER_SIZE: usize = 4;

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

    /// Maximum number of fragments a single message may be split across.
    pub const MAX_FRAGMENTS: usize = 64;

    /// Protocol version exchanged during channel creation handshake.
    pub const PROTOCOL_VERSION: u32 = 391;
}
