//! Wireclient error type.
//!
//! One enum spans every failure mode a client can hit between SOAP Phase 1 and
//! the first encrypted entity method. Variants are deliberately specific —
//! tests assert against the variant, not against a stringified message.

use thiserror::Error;

/// Wireclient result alias.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    // ── Auth ────────────────────────────────────────────────────────────────
    /// SOAP transport failed (DNS, connect, body read, …). Carries the
    /// underlying `reqwest` error so the caller can distinguish "auth server
    /// unreachable" from "auth server rejected the payload".
    #[error("SOAP transport: {0}")]
    SoapTransport(#[from] reqwest::Error),

    /// HTTP-level status outside 200. The SOAP layer never returns 4xx/5xx
    /// for an auth failure (it returns 200 with an error envelope), so this
    /// variant only fires on routing or upstream-availability faults.
    #[error("SOAP HTTP status {status} from {endpoint}")]
    SoapStatus { status: u16, endpoint: &'static str },

    /// Phase 1 response missing a `Set-Cookie: SID=...` header. Either the
    /// server hit `<SGWLoginError>` (bad credentials, disabled account) or
    /// the cookie format drifted.
    #[error("Phase 1 response carried no SID cookie")]
    NoSidCookie,

    /// Phase 2 response did not advertise a `<ServerLocation>`. Most likely
    /// `<ServerSelectionError>` — wrong shard name, replayed SID, or
    /// expired pending login.
    #[error("Phase 2 response carried no ServerLocation")]
    NoServerLocation,

    /// A Phase 2 attribute the handshake needs is missing (one of `SessionKey`,
    /// `Ticket`, `IP`, `Port`).
    #[error("Phase 2 response missing attribute: {0}")]
    MissingAttribute(&'static str),

    /// `SessionKey` was not 64 hex chars (32 bytes). The Mercury encryption
    /// init would reject this anyway; catching at parse time gives a
    /// localised error.
    #[error("invalid session key length: expected 64 hex chars, got {0}")]
    InvalidSessionKeyLength(usize),

    /// `SessionKey` contained non-hex characters.
    #[error("invalid session key: {0}")]
    InvalidSessionKeyHex(#[from] hex::FromHexError),

    /// `Ticket` was not 20 chars. `parse_baseapp_login` on the server side
    /// expects exactly 20 ticket bytes.
    #[error("invalid ticket length: expected 20 chars, got {0}")]
    InvalidTicketLength(usize),

    /// `Port` did not parse as a `u16`.
    #[error("invalid port: {0}")]
    InvalidPort(String),

    /// `IP` attribute did not parse as a valid `IpAddr`.
    #[error("invalid IP: {0}")]
    InvalidIp(String),

    // ── Handshake ───────────────────────────────────────────────────────────
    /// UDP-level I/O fault during the Mercury phase-3 handshake.
    #[error("Mercury I/O: {0}")]
    MercuryIo(#[from] std::io::Error),

    /// The server replied but the reply did not match the expected
    /// `BASEMSG_REPLY_MESSAGE` (`0xFF`) shape. Carries the first byte of the
    /// decrypted body so the caller can log the actual msg ID.
    #[error("phase-3 reply: expected BASEMSG_REPLY_MESSAGE (0xFF), got msg_id 0x{0:02x}")]
    UnexpectedReplyMsg(u8),

    /// The phase-3 reply was longer than the spec'd 25-byte WORD_LENGTH or
    /// shorter than the minimum echo.
    #[error("phase-3 reply: invalid body length {0}")]
    InvalidReplyLength(usize),

    /// Mercury packet decryption failed — either the session key did not
    /// match what the server expected (most likely), or the wire format
    /// drifted.
    #[error("Mercury decryption: {0}")]
    Decryption(String),

    /// The reply's `request_id` did not echo our outbound request — almost
    /// certainly a wire-level desync (we read the wrong packet, or the
    /// server is talking to a different session).
    #[error("phase-3 reply: request_id mismatch (sent {sent}, got {got})")]
    RequestIdMismatch { sent: u32, got: u32 },

    /// The reply ticket did not echo the ticket we logged in with.
    #[error("phase-3 reply: ticket mismatch")]
    TicketMismatch,

    /// Mercury parse failure from `cimmeria_mercury::packet::parse_incoming`,
    /// surfaced verbatim so the wire-format tests catch any drift between
    /// the server-side and the client-side parser.
    #[error("Mercury parse: {0}")]
    MercuryParse(String),

    // ── Replay / trace ──────────────────────────────────────────────────────
    /// Session-trace JSON could not be parsed.
    #[error("session trace JSON: {0}")]
    TraceJson(#[from] serde_json::Error),

    /// Session-trace file could not be read.
    #[error("session trace I/O: {0}")]
    TraceIo(std::io::Error),
}
