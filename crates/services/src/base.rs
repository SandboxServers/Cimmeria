//! BaseApp service — Mercury UDP listener (Phase 3 login handshake).
//!
//! After Phase 1+2 SOAP login the game client connects to the BaseApp via
//! Mercury UDP on port 32832 and sends a `baseAppLogin` packet containing
//! the 20-byte ticket issued by the auth server.
//!
//! # Connect flow
//!
//! 1. Client sends an unencrypted `baseAppLogin` packet (flags=0x41, 41 bytes).
//! 2. Server validates the ticket against the pending-logins map.
//! 3. Server derives the 32-byte AES key from the 64-char hex session key.
//! 4. Server sends an **encrypted** `BASEMSG_REPLY_MESSAGE` echoing the ticket.
//! 5. Server sends an **encrypted** time-sync bundle
//!    (`BASEMSG_UPDATE_FREQUENCY_NOTIFICATION`, `BASEMSG_TICK_SYNC`,
//!    `BASEMSG_SET_GAME_TIME`).
//!
//! All subsequent packets on the channel use the same AES-256-CBC + HMAC-MD5
//! encryption with IV = 16 zero bytes.
//!
//! See `docs/protocol/login-handshake.md` for the full wire-level spec.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;

use cimmeria_common::{EntityId, ServerConfig};

use cimmeria_mercury::packet::{parse_incoming, FLAG_HAS_REQUESTS, FLAG_HAS_SEQUENCE};

use crate::auth::PendingLogin;
use crate::mercury_ext::{build_connect_reply, build_time_sync};

// ── Error types ───────────────────────────────────────────────────────────────

/// Errors specific to the base service.
#[derive(Debug, thiserror::Error)]
pub enum BaseError {
    #[error("Entity {0} not found")]
    EntityNotFound(EntityId),

    #[error("Entity creation failed: {0}")]
    CreationFailed(String),

    #[error("Service not running")]
    NotRunning,

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}

// ── BaseService ────────────────────────────────────────────────────────────────

/// BaseApp service — manages persistent entity state and client connections.
///
/// In the original C++ architecture, this was the `BaseApp` process that:
/// - Accepted client connections on `shard_port` (default 32832) via Mercury UDP
/// - Managed base entity halves (persistent state)
/// - Routed entity messages between clients and CellApp
pub struct BaseService {
    /// Address the Mercury UDP listener binds to.
    pub listener_addr: SocketAddr,

    /// Whether the service is currently running.
    pub is_running: bool,

    /// Pending login handoffs shared with AuthService (ticket → PendingLogin).
    /// Wired by the orchestrator before `start()` is called.
    pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
}

impl BaseService {
    /// Create a new base service from server configuration.
    ///
    /// `pending_logins` starts as an empty map.  The orchestrator calls
    /// [`set_pending_logins`] before [`start`] to wire in the shared map from
    /// `AuthService`.
    pub fn new(config: &ServerConfig) -> Self {
        let listener_addr = format!("{}:{}", config.base_host, config.base_port)
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], config.base_port)));

        Self {
            listener_addr,
            is_running: false,
            pending_logins: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Wire in the `pending_logins` Arc from `AuthService`.
    ///
    /// Must be called before [`start`].
    pub fn set_pending_logins(
        &mut self,
        pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
    ) {
        self.pending_logins = pending_logins;
    }

    /// Start the Mercury UDP listener on `listener_addr`.
    ///
    /// Binds a UDP socket and spawns a background tokio task that handles
    /// incoming `baseAppLogin` connect packets.
    pub async fn start(&mut self) -> Result<(), BaseError> {
        tracing::info!(addr = %self.listener_addr, "Starting base service UDP listener");

        let socket = Arc::new(UdpSocket::bind(self.listener_addr).await?);
        tracing::info!(addr = %socket.local_addr().unwrap(), "Base service UDP socket bound");

        let pending_logins = Arc::clone(&self.pending_logins);

        tokio::spawn(async move {
            run_connect_loop(socket, pending_logins).await;
        });

        self.is_running = true;
        Ok(())
    }

    /// Stop the base service gracefully.
    pub async fn stop(&mut self) {
        tracing::info!("Stopping base service");
        // TODO: Signal the listener task to exit cleanly
        self.is_running = false;
    }

    /// Create a new base entity (stub).
    pub async fn create_base_entity(&self) -> Result<EntityId, BaseError> {
        if !self.is_running {
            return Err(BaseError::NotRunning);
        }
        tracing::debug!("Creating base entity");
        Ok(EntityId(0))
    }

    /// Destroy a base entity (stub).
    pub async fn destroy_base_entity(&self, entity_id: EntityId) -> Result<(), BaseError> {
        if !self.is_running {
            return Err(BaseError::NotRunning);
        }
        tracing::debug!(%entity_id, "Destroying base entity");
        Ok(())
    }
}

// ── UDP connect loop ──────────────────────────────────────────────────────────

/// Main receive loop for the Mercury UDP listener.
///
/// Reads datagrams and dispatches each to [`handle_datagram`].
async fn run_connect_loop(
    socket: Arc<UdpSocket>,
    pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
) {
    let mut buf = [0u8; 1500];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, addr)) => {
                let data = &buf[..len];
                if let Err(e) =
                    handle_datagram(&socket, addr, data, &pending_logins).await
                {
                    tracing::warn!(%addr, "Connect handler error: {e}");
                }
            }
            Err(e) => {
                tracing::error!("Base service UDP recv error: {e}");
                break;
            }
        }
    }
}

/// Handle a single incoming UDP datagram.
///
/// Expects an unencrypted `baseAppLogin` packet (flags=0x41).  All other
/// flag patterns are silently dropped here; post-authentication packets are
/// handled once per-connection state is added in a future milestone.
async fn handle_datagram(
    socket: &UdpSocket,
    addr: SocketAddr,
    raw: &[u8],
    pending_logins: &Arc<Mutex<HashMap<String, PendingLogin>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use cimmeria_mercury::packet::{FLAG_HAS_REQUESTS, FLAG_HAS_SEQUENCE};

    if raw.len() < 1 {
        return Ok(());
    }

    let flags = raw[0];

    // Only handle the initial unencrypted connect packet.
    let expected = FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE; // 0x41
    if flags != expected {
        tracing::trace!(%addr, flags, "Ignoring non-connect packet (flags={flags:#04x})");
        return Ok(());
    }

    match parse_baseapp_login(raw) {
        Ok((request_id, ticket_str)) => {
            tracing::info!(%addr, ticket = %ticket_str, "baseAppLogin received");
            handle_login(socket, addr, request_id, &ticket_str, pending_logins).await
        }
        Err(e) => {
            tracing::warn!(%addr, "Failed to parse baseAppLogin: {e}");
            Ok(())
        }
    }
}

/// Parse the `baseAppLogin` packet body.
///
/// Wire layout (41 bytes total):
/// ```text
/// [0x41]              flags
/// [0x00]              msg_id = CLIENTMSG_BASEAPP_LOGIN
/// [u16 LE]            WORD_LENGTH = 25
/// [u32 LE]            requestId  (+ request framing: read before accountId)
/// [u16 LE]            nextReqOffset = 0
/// [u32 LE]            accountId
/// [u8]                ticketLen = 20
/// [20 bytes]          ticket (20 ASCII hex chars)
/// [u16 LE]            footer: first_req_offset = 1
/// [u32 LE]            footer: seq_id
/// ```
fn parse_baseapp_login(
    raw: &[u8],
) -> Result<(u32, String), Box<dyn std::error::Error + Send + Sync>> {
    let pkt = parse_incoming(raw)?;

    // Validate flags
    if pkt.flags != (FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE) {
        return Err(format!("unexpected flags: {:#04x}", pkt.flags).into());
    }

    let body = &pkt.body;
    if body.len() < 34 {
        return Err(format!("body too short: {} bytes (expected 34)", body.len()).into());
    }

    // body[0]    = msg_id (should be 0x00)
    // body[1..2] = WORD_LENGTH (little-endian, should be 25)
    // body[3..6] = requestId (u32 LE) — from request framing
    // body[7..8] = nextReqOffset (u16 LE) — from request framing
    // body[9..12] = accountId (u32 LE)
    // body[13]   = ticketLen (u8, should be 20)
    // body[14..33] = ticket (20 bytes)

    let msg_id = body[0];
    if msg_id != 0x00 {
        return Err(format!("unexpected msg_id: {:#04x}", msg_id).into());
    }

    let word_len = u16::from_le_bytes([body[1], body[2]]);
    if word_len != 25 {
        return Err(format!("unexpected WORD_LENGTH: {}", word_len).into());
    }

    let request_id = u32::from_le_bytes([body[3], body[4], body[5], body[6]]);
    // body[7..8] = nextReqOffset — ignored
    let _account_id = u32::from_le_bytes([body[9], body[10], body[11], body[12]]);
    let ticket_len = body[13] as usize;

    if ticket_len != 20 {
        return Err(format!("unexpected ticketLen: {}", ticket_len).into());
    }
    if body.len() < 14 + ticket_len {
        return Err("body truncated at ticket".into());
    }

    let ticket_bytes = &body[14..14 + ticket_len];
    let ticket_str = String::from_utf8(ticket_bytes.to_vec())?;

    Ok((request_id, ticket_str))
}

/// Validate the ticket, then send the encrypted reply + time-sync packets.
async fn handle_login(
    socket: &UdpSocket,
    addr: SocketAddr,
    request_id: u32,
    ticket: &str,
    pending_logins: &Arc<Mutex<HashMap<String, PendingLogin>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Consume the pending login entry (fails if ticket unknown or already used).
    let login = {
        let mut map = pending_logins.lock().map_err(|_| "pending_logins lock poisoned")?;
        map.remove(ticket)
    };

    let login = match login {
        Some(l) => l,
        None => {
            tracing::warn!("Unknown or already-consumed ticket: {ticket}");
            return Ok(());
        }
    };

    // Decode the 64-char hex session key to 32 bytes.
    let key = decode_session_key(&login.session_key)?;

    tracing::info!(
        account_id = login.account_id,
        "Phase 3 authenticated; sending reply and time sync"
    );

    // Build and send the encrypted reply packet (seq_id = 1).
    let reply = build_connect_reply(request_id, ticket.as_bytes(), &key, 1);
    socket.send_to(&reply, addr).await?;

    // Build and send the encrypted time-sync bundle (seq_id = 2).
    let sync = build_time_sync(&key, 2);
    socket.send_to(&sync, addr).await?;

    tracing::info!(%addr, "Phase 3 complete — client connected");
    Ok(())
}

/// Decode a 64-character uppercase-hex session key to a 32-byte array.
fn decode_session_key(
    hex: &str,
) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
    if hex.len() != 64 {
        return Err(format!("session key must be 64 hex chars, got {}", hex.len()).into());
    }
    let mut key = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        key[i] = (hi << 4) | lo;
    }
    Ok(key)
}

fn hex_nibble(b: u8) -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(format!("invalid hex character: {}", b as char).into()),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_service_is_not_running() {
        let config = ServerConfig::default();
        let svc = BaseService::new(&config);
        assert!(!svc.is_running);
        assert_eq!(svc.listener_addr.port(), 32832);
    }

    #[tokio::test]
    async fn start_sets_running() {
        let mut config = ServerConfig::default();
        config.base_port = 0; // OS-assigned port to avoid conflicts in tests
        let mut svc = BaseService::new(&config);
        svc.start().await.unwrap();
        assert!(svc.is_running);
    }

    #[tokio::test]
    async fn create_entity_fails_when_not_running() {
        let config = ServerConfig::default();
        let svc = BaseService::new(&config);
        let result = svc.create_base_entity().await;
        assert!(result.is_err());
    }

    #[test]
    fn decode_session_key_valid() {
        let hex = "AABBCCDD".repeat(8); // 64 chars
        let key = decode_session_key(&hex).unwrap();
        assert_eq!(key[0], 0xAA);
        assert_eq!(key[1], 0xBB);
        assert_eq!(key[2], 0xCC);
        assert_eq!(key[3], 0xDD);
    }

    #[test]
    fn decode_session_key_too_short() {
        let result = decode_session_key("AABB");
        assert!(result.is_err());
    }

    #[test]
    fn parse_baseapp_login_valid() {
        // Construct a valid 41-byte baseAppLogin packet.
        let mut raw = Vec::new();
        raw.push(0x41u8); // flags
        raw.push(0x00u8); // msg_id
        raw.extend_from_slice(&25u16.to_le_bytes()); // WORD_LENGTH
        raw.extend_from_slice(&0xCAFEBABEu32.to_le_bytes()); // requestId
        raw.extend_from_slice(&0u16.to_le_bytes()); // nextReqOffset
        raw.extend_from_slice(&1u32.to_le_bytes()); // accountId
        raw.push(20u8); // ticketLen
        raw.extend_from_slice(b"ABCDEF1234567890ABCD"); // ticket
        raw.extend_from_slice(&1u16.to_le_bytes()); // first_req_offset
        raw.extend_from_slice(&3u32.to_le_bytes()); // seq_id

        assert_eq!(raw.len(), 41);

        let (req_id, ticket) = parse_baseapp_login(&raw).unwrap();
        assert_eq!(req_id, 0xCAFEBABE);
        assert_eq!(ticket, "ABCDEF1234567890ABCD");
    }
}
