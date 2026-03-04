//! BaseApp service — Mercury UDP listener.
//!
//! # Protocol phases
//!
//! ## Phase 3 — initial connect (unencrypted)
//!
//! 1. Client sends `baseAppLogin` (flags=0x41, 41 bytes, **unencrypted**).
//! 2. Server validates the ticket against the pending-logins map.
//! 3. Server sends **encrypted** `BASEMSG_REPLY_MESSAGE` (seq=1) echoing the ticket.
//! 4. Server sends **encrypted** time-sync bundle (seq=2).
//! 5. Connection is registered; all further traffic on this channel is encrypted.
//!
//! ## Phase 4 — character list
//!
//! 1. Client sends login confirmation (msg_id=0x01, **encrypted**).
//! 2. Server decrypts, identifies msg_id=0x01, sends **encrypted** character list
//!    (seq=3): game-state (0x05) + empty char list (0x82, count=0).
//! 3. Server spawns a per-connection tick-sync task that sends `BASEMSG_TICK_SYNC`
//!    every 100 ms (seq=4, 5, …) to prevent REASON_INACTIVITY disconnects.
//!
//! See `docs/protocol/login-handshake.md` for the full wire-level spec.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;

use cimmeria_common::{EntityId, ServerConfig};
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{parse_incoming, FLAG_HAS_REQUESTS, FLAG_HAS_SEQUENCE, FLAG_RELIABLE};

use crate::auth::PendingLogin;
use crate::mercury_ext::{
    build_char_list_response, build_connect_reply, build_ongoing_tick_sync, build_time_sync,
};

// ── Hex dump helper ──────────────────────────────────────────────────────────

/// Format a byte slice as a hex string for trace logging.
fn to_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")
}

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

// ── Per-connection state ──────────────────────────────────────────────────────

/// State held for each client that has completed the Phase 3 handshake.
struct ConnectedClientState {
    /// Encryption context (for decrypting client packets).
    enc: MercuryEncryption,
    /// Raw 32-byte AES-256 key (for spawning the tick-sync task).
    key: [u8; 32],
    /// `true` once the Phase 4 character list has been sent to this client.
    char_list_sent: bool,
    /// Client reliable-message sequence IDs that need to be ACKed.
    /// Drained by the tick-sync loop (or the char_list response) and
    /// piggybacked on the next outgoing packet.
    pending_acks: Arc<Mutex<Vec<u32>>>,
    /// Timestamp of the last received packet from this client.
    /// Used by the tick-sync loop to detect dead clients.
    last_recv: Arc<Mutex<Instant>>,
}

// ── BaseService ───────────────────────────────────────────────────────────────

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
    pub async fn start(&mut self) -> Result<(), BaseError> {
        tracing::info!(addr = %self.listener_addr, "Starting base service UDP listener");

        tracing::trace!(addr = %self.listener_addr, "Binding UDP socket for base service");
        let socket = Arc::new(UdpSocket::bind(self.listener_addr).await.map_err(|e| {
            tracing::error!(addr = %self.listener_addr, error = %e, "Failed to bind base UDP socket");
            e
        })?);
        tracing::info!(addr = %socket.local_addr().unwrap(), "Base service UDP socket bound");

        let pending_logins = Arc::clone(&self.pending_logins);

        tracing::trace!("Spawning base service UDP receive loop");
        tokio::spawn(async move {
            tracing::trace!("Base service UDP receive loop started");
            run_connect_loop(socket, pending_logins).await;
            tracing::trace!("Base service UDP receive loop exited");
        });

        self.is_running = true;
        Ok(())
    }

    /// Stop the base service gracefully.
    pub async fn stop(&mut self) {
        tracing::info!("Stopping base service");
        // TODO: signal the listener task to exit cleanly
        self.is_running = false;
        tracing::trace!("Base service stopped");
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

// ── UDP receive loop ──────────────────────────────────────────────────────────

/// Main receive loop — one per running `BaseService`.
async fn run_connect_loop(
    socket: Arc<UdpSocket>,
    pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
) {
    // Per-connection state; allocated once per loop, shared across datagram handlers.
    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let mut buf = [0u8; 4096];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, addr)) => {
                tracing::trace!(%addr, len, hex = %to_hex(&buf[..len]), "UDP_IN");
                // We immediately await the handler, so borrowing buf here is fine.
                if let Err(e) = handle_datagram(
                    &socket,
                    addr,
                    &buf[..len],
                    &pending_logins,
                    &connected,
                )
                .await
                {
                    tracing::warn!(%addr, "Datagram handler error: {e}");
                }
            }
            Err(e) => {
                // On Windows, WSAECONNRESET (10054) arrives on the UDP socket
                // when a previous send_to targeted a closed port (ICMP port
                // unreachable).  This is per-destination, NOT a socket failure.
                if e.raw_os_error() == Some(10054) {
                    tracing::debug!("UDP recv: WSAECONNRESET (10054) — remote closed, continuing");
                    continue;
                }
                tracing::error!("Base service UDP recv error (fatal): {e}");
                break;
            }
        }
    }
}

/// Dispatch a single incoming UDP datagram.
///
/// Two paths:
/// - **Established channel** (addr in `connected`) → decrypt and handle.
/// - **Phase 3 login** (flags=0x41, addr not yet connected) → parse and respond.
async fn handle_datagram(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    raw: &[u8],
    pending_logins: &Arc<Mutex<HashMap<String, PendingLogin>>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if raw.is_empty() {
        return Ok(());
    }

    // Check for an established encrypted channel first.
    let channel_key: Option<(MercuryEncryption, [u8; 32], Arc<Mutex<Vec<u32>>>, Arc<Mutex<Instant>>)> = {
        let clients = connected
            .lock()
            .map_err(|_| "connected lock poisoned")?;
        clients.get(&addr).map(|c| {
            (
                c.enc.clone(),
                c.key,
                Arc::clone(&c.pending_acks),
                Arc::clone(&c.last_recv),
            )
        })
    };

    if let Some((enc, key, pending_acks, last_recv)) = channel_key {
        // Update last-recv timestamp on every packet from this client.
        *last_recv.lock().unwrap() = Instant::now();
        return handle_encrypted_datagram(socket, addr, raw, enc, key, &pending_acks, connected)
            .await;
    }

    // Not yet connected — only accept the unencrypted Phase 3 login (flags=0x41).
    let login_flags = FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE; // 0x41
    if raw[0] != login_flags {
        tracing::trace!(%addr, flags = raw[0], "Ignoring packet from unknown addr (flags={:#04x})", raw[0]);
        return Ok(());
    }

    match parse_baseapp_login(raw) {
        Ok((request_id, ticket_str)) => {
            tracing::info!(%addr, ticket = %ticket_str, "baseAppLogin received");
            handle_login(socket, addr, request_id, &ticket_str, pending_logins, connected).await
        }
        Err(e) => {
            tracing::warn!(%addr, "Failed to parse baseAppLogin: {e}");
            Ok(())
        }
    }
}

/// Handle an encrypted datagram from a known connected client.
async fn handle_encrypted_datagram(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    raw: &[u8],
    enc: MercuryEncryption,
    key: [u8; 32],
    pending_acks: &Arc<Mutex<Vec<u32>>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let plaintext = match enc.decrypt(raw) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(%addr, "Decryption failed (bad HMAC?): {e}");
            return Ok(());
        }
    };

    tracing::trace!(%addr, len = plaintext.len(), hex = %to_hex(&plaintext), "DECRYPT_OK");

    let pkt = match parse_incoming(&plaintext) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(%addr, "Packet parse failed after decrypt: {e}");
            return Ok(());
        }
    };

    tracing::debug!(
        %addr,
        flags = pkt.flags,
        body_len = pkt.body.len(),
        seq = ?pkt.seq_id,
        acks = ?pkt.acks,
        "Decrypted packet received"
    );

    // Queue an ACK for any reliable message the client sends.
    if pkt.flags & FLAG_RELIABLE != 0 {
        if let Some(seq) = pkt.seq_id {
            tracing::trace!(%addr, client_seq = seq, "Queueing ACK for client reliable message");
            pending_acks.lock().unwrap().push(seq);
        }
    }

    // Phase 4: first client message after Phase 3 is msg_id=0x01 (login confirmation).
    if pkt.body.first().copied() == Some(0x01) {
        handle_login_confirmation(socket, addr, key, connected).await?;
    }

    Ok(())
}

/// Send the character list (Phase 4) once, then start the tick-sync heartbeat.
async fn handle_login_confirmation(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Guard: only send once per connection.  Also grab shared state Arcs.
    let arcs = {
        let mut clients = connected
            .lock()
            .map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            if !c.char_list_sent {
                c.char_list_sent = true;
                Some((Arc::clone(&c.pending_acks), Arc::clone(&c.last_recv)))
            } else {
                None
            }
        } else {
            tracing::warn!(%addr, "login_confirmation: addr not in connected map");
            None
        }
    };

    let (pending_acks_arc, last_recv_arc) = match arcs {
        Some(a) => a,
        None => return Ok(()),
    };

    tracing::info!(%addr, "Phase 4: sending character list (empty → triggers char creation)");

    // Drain any pending ACKs (at minimum the client's login confirmation seq=0).
    let acks: Vec<u32> = {
        let mut pending = pending_acks_arc.lock().unwrap();
        pending.drain(..).collect()
    };
    tracing::trace!(%addr, ?acks, "Piggybacking ACKs on char_list");

    // Character list at seq=3 (connect_reply=1, time_sync=2).
    let pkt = build_char_list_response(&key, 3, &acks);
    tracing::trace!(%addr, len = pkt.len(), hex = %to_hex(&pkt), "UDP_OUT char_list");
    socket.send_to(&pkt, addr).await?;

    tracing::info!(%addr, "Phase 4 complete — spawning tick-sync task");

    // Kick off the 100 ms heartbeat (seq starts at 4).
    let socket_clone = Arc::clone(socket);
    tokio::spawn(run_tick_loop(
        socket_clone,
        addr,
        key,
        pending_acks_arc,
        last_recv_arc,
    ));

    Ok(())
}

/// Per-connection tick-sync heartbeat task.
///
/// Sends `BASEMSG_TICK_SYNC` every 100 ms to keep the client connection alive.
/// Any pending client ACKs are piggybacked onto the next tick-sync packet.
/// The task exits when:
/// - The UDP send fails (socket closed or OS error).
/// - No data has been received from the client for 60 seconds.
async fn run_tick_loop(
    socket: Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    pending_acks: Arc<Mutex<Vec<u32>>>,
    last_recv: Arc<Mutex<Instant>>,
) {
    const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(60);

    // seq=4 follows: connect_reply(1), time_sync(2), char_list(3).
    let mut seq_id: u32 = 4;
    let mut tick: u32 = 0;

    tracing::debug!(%addr, "Tick-sync loop started");

    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check if the client has gone silent.
        let idle = last_recv.lock().unwrap().elapsed();
        if idle > INACTIVITY_TIMEOUT {
            tracing::info!(
                %addr,
                idle_secs = idle.as_secs(),
                "Tick-sync stopping: client inactive for {}s",
                idle.as_secs()
            );
            break;
        }

        // Drain any ACKs queued by the receive loop.
        let acks: Vec<u32> = {
            let mut pending = pending_acks.lock().unwrap();
            pending.drain(..).collect()
        };
        if !acks.is_empty() {
            tracing::trace!(%addr, ?acks, seq_id, "Piggybacking ACKs on tick_sync");
        }

        let pkt = build_ongoing_tick_sync(&key, seq_id, tick, &acks);
        if let Err(e) = socket.send_to(&pkt, addr).await {
            tracing::debug!(%addr, "Tick-sync stopped (send error): {e}");
            break;
        }

        if tick % 100 == 0 {
            tracing::debug!(%addr, tick, seq_id, "Tick-sync heartbeat (every 100th)");
        }

        seq_id = seq_id.wrapping_add(1);
        tick = tick.wrapping_add(1);
    }
}

// ── Phase 3 login helpers ─────────────────────────────────────────────────────

/// Validate ticket, send Phase 3 reply + time-sync, register the encrypted channel.
async fn handle_login(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    request_id: u32,
    ticket: &str,
    pending_logins: &Arc<Mutex<HashMap<String, PendingLogin>>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Consume the pending login entry.
    let login = {
        let mut map = pending_logins
            .lock()
            .map_err(|_| "pending_logins lock poisoned")?;
        map.remove(ticket)
    };

    let login = match login {
        Some(l) => l,
        None => {
            tracing::warn!("Unknown or already-consumed ticket: {ticket}");
            return Ok(());
        }
    };

    let key = decode_session_key(&login.session_key)?;

    tracing::info!(
        account_id = login.account_id,
        "Phase 3 authenticated; sending reply and time-sync"
    );

    // connect_reply at seq=1.
    let reply = build_connect_reply(request_id, ticket.as_bytes(), &key, 1);
    tracing::trace!(%addr, len = reply.len(), hex = %to_hex(&reply), "UDP_OUT connect_reply");
    socket.send_to(&reply, addr).await?;

    // time-sync bundle at seq=2.
    let sync = build_time_sync(&key, 2);
    tracing::trace!(%addr, len = sync.len(), hex = %to_hex(&sync), "UDP_OUT time_sync");
    socket.send_to(&sync, addr).await?;

    // Register for Phase 4 encrypted traffic.
    {
        let mut clients = connected
            .lock()
            .map_err(|_| "connected lock poisoned")?;
        clients.insert(
            addr,
            ConnectedClientState {
                enc: MercuryEncryption::from_session_key(key),
                key,
                char_list_sent: false,
                pending_acks: Arc::new(Mutex::new(Vec::new())),
                last_recv: Arc::new(Mutex::new(Instant::now())),
            },
        );
    }

    tracing::info!(%addr, "Phase 3 complete — channel registered");
    Ok(())
}

/// Parse the `baseAppLogin` packet body.
///
/// Wire layout (41 bytes total):
/// ```text
/// [0x41]              flags
/// [0x00]              msg_id = CLIENTMSG_BASEAPP_LOGIN
/// [u16 LE]            WORD_LENGTH = 25
/// [u32 LE]            requestId
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

    if pkt.flags != (FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE) {
        return Err(format!("unexpected flags: {:#04x}", pkt.flags).into());
    }

    let body = &pkt.body;
    if body.len() < 34 {
        return Err(format!("body too short: {} bytes (expected ≥ 34)", body.len()).into());
    }

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
