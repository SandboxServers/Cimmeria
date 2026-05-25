use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{parse_incoming, FLAG_HAS_REQUESTS, FLAG_HAS_SEQUENCE};

use crate::auth::PendingLogin;
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_connect_reply, build_logged_off, build_time_sync};

use super::helpers::{destroy_client_entities, to_hex};
use super::tick_sync::run_tick_loop;
use super::ConnectedClientState;

/// Validate ticket, send Phase 3 reply + time-sync, register the encrypted channel.
///
/// `level = "info"` because login is low-frequency and high-signal —
/// every successful auth shows up in SigNoz as a span you can drill
/// into for ticket lookup, duplicate-eviction, and tick-loop spawn.
#[tracing::instrument(
    name = "base.login",
    level = "info",
    skip_all,
    fields(peer = %addr, request_id, account_id = tracing::field::Empty),
)]
pub(crate) async fn handle_login(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    request_id: u32,
    ticket: &str,
    pending_logins: &Arc<Mutex<HashMap<String, PendingLogin>>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    // Backfill account_id onto the parent span now that the ticket has
    // resolved — every nested call below (duplicate eviction, channel
    // register, tick-loop spawn) gets correlated under the same
    // account in SigNoz.
    tracing::Span::current().record("account_id", login.account_id);

    let key = decode_session_key(&login.session_key)?;

    // ── Duplicate login detection (KI-7) ────────────────────────────────────
    // If this account already has an active session, evict the old one first.
    // C++ checks ChannelManager.isPlayerOnline() at play-character time
    // (Account.py:286-290), but we also guard at login to prevent stale sessions.
    {
        let evict_addr: Option<(SocketAddr, [u8; 32])> = {
            let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
            clients.iter().find_map(|(existing_addr, c)| {
                if c.account_id == login.account_id && *existing_addr != addr {
                    Some((*existing_addr, c.key))
                } else {
                    None
                }
            })
        };
        if let Some((old_addr, old_key)) = evict_addr {
            tracing::warn!(
                account_id = login.account_id,
                %old_addr,
                %addr,
                "Duplicate login -- evicting old session"
            );
            // Send LOGGED_OFF to the old client so it gets an immediate teardown.
            let (acks, seq) = {
                let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
                if let Some(c) = clients.get_mut(&old_addr) {
                    let acks: Vec<u32> = c.pending_acks.lock().unwrap().drain(..).collect();
                    let seq = c
                        .next_seq
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                        & cimmeria_mercury::packet::SEQUENCE_MASK;
                    (acks, seq)
                } else {
                    (vec![], 0)
                }
            };
            let pkt = build_logged_off(&old_key, seq, &acks);
            let _ = transport.send_to(&pkt, old_addr).await;
            destroy_client_entities(
                connected,
                entity_manager,
                old_addr,
                cell_tx,
                entity_to_addr,
                "duplicate_login",
            );
        }
    }

    tracing::info!(
        account_id = login.account_id,
        "Phase 3 authenticated; sending reply and time-sync"
    );

    // connect_reply at seq=1.
    let reply = build_connect_reply(request_id, ticket.as_bytes(), &key, 1);
    tracing::trace!(%addr, len = reply.len(), hex = %to_hex(&reply), "UDP_OUT connect_reply");
    transport.send_to(&reply, addr).await?;

    // time-sync bundle at seq=2.
    let sync = build_time_sync(&key, 2);
    tracing::trace!(%addr, len = sync.len(), hex = %to_hex(&sync), "UDP_OUT time_sync");
    transport.send_to(&sync, addr).await?;

    // Register for Phase 4 encrypted traffic.
    let (pending_acks_arc, last_recv_arc, next_seq_unreliable_arc, cancelled_arc) = {
        let next_seq = Arc::new(AtomicU32::new(3));
        let next_seq_unreliable = Arc::new(AtomicU32::new(0));
        let pending_acks = Arc::new(Mutex::new(Vec::new()));
        let last_recv = Arc::new(Mutex::new(Instant::now()));
        let cancelled = Arc::new(AtomicBool::new(false));
        // `next_seq` (reliable) is consumed by `ConnectedClientState` below and by
        // application-packet paths that pull from `clients[addr].next_seq`; the
        // tick-loop only needs `next_seq_unreliable`, so the reliable counter
        // does not appear in the spawn-argument tuple.
        let arcs = (
            Arc::clone(&pending_acks),
            Arc::clone(&last_recv),
            Arc::clone(&next_seq_unreliable),
            Arc::clone(&cancelled),
        );

        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        clients.insert(
            addr,
            ConnectedClientState {
                enc: MercuryEncryption::from_session_key(key),
                key,
                account_id: login.account_id,
                access_level: login.access_level,
                char_list_sent: false,
                world_entry_sent: false,
                pending_player_entity_id: None,
                player_entity_id: None,
                next_seq,
                next_seq_unreliable,
                pending_acks,
                last_recv,
                account_entity_id: entity_manager.lock().unwrap().create_entity("Account").0 as u32,
                next_data_id: 0,
                pending_world_entry: None,
                pending_player_load_data: None,
                pending_map_loaded: None,
                pending_client_ready: None,
                deferred_aoi_msgs: Vec::new(),
                cached_appearance_args: None,
                cached_tint_args: None,
                weapon_holstered: true,
                cancelled,
                cinematic_spam_cancel: Arc::new(AtomicBool::new(false)),
                player_name: None,
                player_level: None,
                player_archetype: None,
                world_name: None,
                player_xp: None,
                player_training_points: None,
                active_player_id: None,
                pending_destination_ring_id: None,
                channel: Mutex::new(cimmeria_mercury::channel::Channel::new(addr)),
            },
        );
        arcs
    };

    tracing::info!(%addr, "Phase 3 complete -- channel registered, starting tick-sync");

    // Start tick-sync immediately after Phase 3 (matches C++ onConnected() behavior).
    // The C++ server starts gameTick() as soon as the Account entity is created,
    // BEFORE the client sends ENABLE_ENTITIES. This provides:
    // 1. ACK delivery for client reliable messages (AUTHENTICATE, ENABLE_ENTITIES)
    // 2. A running game clock so the client can play the stargate transition animation
    tokio::spawn(run_tick_loop(
        Arc::clone(transport),
        addr,
        key,
        next_seq_unreliable_arc,
        pending_acks_arc,
        last_recv_arc,
        cancelled_arc,
        Arc::clone(connected),
        Arc::clone(entity_manager),
        cell_tx.clone(),
        Arc::clone(entity_to_addr),
    ));

    Ok(())
}

/// Parse the `baseAppLogin` packet body.
pub(crate) fn parse_baseapp_login(
    raw: &[u8],
) -> Result<(u32, String), Box<dyn std::error::Error + Send + Sync>> {
    let pkt = parse_incoming(raw)?;

    if pkt.flags != (FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE) {
        return Err(format!("unexpected flags: {:#04x}", pkt.flags).into());
    }

    let body = &pkt.body;
    if body.len() < 34 {
        return Err(format!("body too short: {} bytes (expected >= 34)", body.len()).into());
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
pub(crate) fn decode_session_key(
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

/// Handle `logOff` (0xC2) -- the client is leaving the session.
///
/// Both "Change Server" and "Back" buttons in the character select UI call this.
/// The C++ reference (`Account.py:logOff`) was a no-op (`pass`), which meant the
/// server never cleaned up -- leaking sessions indefinitely.
///
/// Client flow (from Ghidra analysis of FUN_00def9f0):
///   1. Both buttons emit `Event_NetOut_LogOff` -> sends logOff (0xC2)
///   2. `relogin()` also sets a relogin flag before step 1
///   3. Client fires `gotoLogin` state transition
///   4. Client waits for Mercury channel to die (server closes or timeout)
///   5. `Event_Net_Disconnected` fires -> disconnect callback (FUN_00de9bb0):
///      - If relogin flag set -> `EntityManager::disconnected()` + SOAP re-login
///      - If not set -> shows "disconnected from network" UI
///
/// We send `LOGGED_OFF` (0x37) to trigger an immediate channel teardown on the
/// client side (via `ServerConnection::loggedOff -> Mercury::disconnect`).
/// Without it, the client waits 30s for the Mercury channel timeout.
pub(crate) async fn handle_log_off(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(%addr, "Client requests logOff -- sending LOGGED_OFF and cleaning up");

    // Signal the tick-sync loop to stop and grab seq/acks for the final packet.
    let (acks, seq) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let client = clients.get_mut(&addr).ok_or("no session for addr")?;
        client
            .cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let acks: Vec<u32> = client.pending_acks.lock().unwrap().drain(..).collect();
        let seq = client
            .next_seq
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            & cimmeria_mercury::packet::SEQUENCE_MASK;
        (acks, seq)
    };

    // Send LOGGED_OFF (0x37) with reason=0 to trigger immediate client-side
    // channel teardown.  This fires ServerConnection::loggedOff -> Event_Net_Disconnected
    // which triggers the relogin flow (Change Server) or disconnect UI (Back).
    let pkt = build_logged_off(&key, seq, &acks);
    transport.send_to(&pkt, addr).await?;
    tracing::debug!(%addr, seq, "Sent LOGGED_OFF (0x37)");

    // Destroy entities and remove from connected map.
    destroy_client_entities(
        connected,
        entity_manager,
        addr,
        cell_tx,
        entity_to_addr,
        "logoff",
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Decode rejects non-hex characters with a clear error rather than
    /// silently producing zero bytes. Auth feeds this raw from the
    /// upstream POST body, so a malformed key must surface as a Phase 3
    /// reject, not a silent connection with a wrong key.
    #[test]
    fn decode_session_key_rejects_invalid_hex() {
        // 64 chars, all non-hex characters (Z is outside [0-9A-Fa-f]).
        let s = "ZZ".repeat(32);
        assert!(decode_session_key(&s).is_err());
    }

    /// Mixed lowercase hex must round-trip identically to uppercase —
    /// the auth service may emit either depending on upstream casing.
    #[test]
    fn decode_session_key_accepts_lowercase_hex() {
        let upper = "AABBCCDD".repeat(8);
        let lower = "aabbccdd".repeat(8);
        assert_eq!(
            decode_session_key(&upper).unwrap(),
            decode_session_key(&lower).unwrap(),
            "uppercase and lowercase hex must decode to the same key bytes"
        );
    }

    #[test]
    fn parse_baseapp_login_valid() {
        let mut raw = Vec::new();
        raw.push(0x41u8); // flags
        raw.push(0x00u8); // msg_id
        raw.extend_from_slice(&25u16.to_le_bytes());
        raw.extend_from_slice(&0xCAFEBABEu32.to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(&1u32.to_le_bytes());
        raw.push(20u8);
        raw.extend_from_slice(b"ABCDEF1234567890ABCD");
        raw.extend_from_slice(&1u16.to_le_bytes());
        raw.extend_from_slice(&3u32.to_le_bytes());

        assert_eq!(raw.len(), 41);

        let (req_id, ticket) = parse_baseapp_login(&raw).unwrap();
        assert_eq!(req_id, 0xCAFEBABE);
        assert_eq!(ticket, "ABCDEF1234567890ABCD");
    }

    /// Truncated body (less than the 34-byte fixed header for a valid
    /// `baseAppLogin`) must reject — the auth → base seam is the first
    /// validation point a malformed Phase 3 packet hits, and silently
    /// continuing past truncation would index OOB on the ticket bytes.
    ///
    /// The packet IS well-formed at the `parse_incoming` layer (valid
    /// flags + footers); only the body is short. The error must come
    /// from `parse_baseapp_login`'s own `body.len() < 34` check, not
    /// from underflow in the upstream packet parser.
    #[test]
    fn parse_baseapp_login_rejects_truncated_body() {
        // FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE = 0x41. With these flags the
        // upstream parser strips two trailing footers from the buffer
        // before yielding the body (innermost → outermost):
        //   * first_req_offset (u16) — where requests begin in the body
        //   * seq_id           (u32)
        // Layout:
        //   raw = [flags][..body..][first_req_offset u16][seq_id u32]
        // We give a 13-byte body (msg_id 0x00 + word_len 25u16 + 10
        // zero filler). 13 < 34, so parse_baseapp_login's own length
        // guard fires AFTER the upstream parser succeeds.
        let mut raw = vec![0x41u8]; // flags
        raw.push(0x00u8); // msg_id
        raw.extend_from_slice(&25u16.to_le_bytes()); // word_len (well-formed)
        raw.extend_from_slice(&[0u8; 10]); // zero filler — body total = 13 bytes
        raw.extend_from_slice(&0u16.to_le_bytes()); // first_req_offset footer
        raw.extend_from_slice(&7u32.to_le_bytes()); // seq_id footer

        let err = parse_baseapp_login(&raw)
            .expect_err("13-byte body must trigger the 34-byte length guard");
        let msg = err.to_string();
        assert!(
            msg.contains("body too short") || msg.contains("34"),
            "error must come from parse_baseapp_login's body-length check, got: {msg}"
        );
    }

    /// An unexpected msg_id must reject — the parser is keyed on 0x00
    /// (`baseAppLogin`). Without this guard a misrouted packet of a
    /// different shape would surface as a confusing "ticketLen wrong"
    /// error instead of the clear msg_id mismatch.
    #[test]
    fn parse_baseapp_login_rejects_wrong_msg_id() {
        let mut raw = vec![0x41u8, 0x99u8]; // msg_id = 0x99 (not 0x00)
        raw.extend_from_slice(&25u16.to_le_bytes());
        raw.extend_from_slice(&0u32.to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(&1u32.to_le_bytes());
        raw.push(20u8);
        raw.extend_from_slice(b"ABCDEF1234567890ABCD");
        raw.extend_from_slice(&1u16.to_le_bytes());
        raw.extend_from_slice(&3u32.to_le_bytes());
        let err = parse_baseapp_login(&raw).unwrap_err().to_string();
        assert!(
            err.contains("msg_id"),
            "error message should mention msg_id, got: {err}"
        );
    }

    // ── Auth → base login handoff seam ─────────────────────────────────────
    //
    // The Phase 3 handoff is the auth↔base seam: auth created a
    // PendingLogin during shard select, and base consumes it when the
    // client connects via Mercury UDP. Without these tests, a regression
    // in the handoff (forgotten ticket consume, wrong key copied into
    // ConnectedClientState, missing duplicate-login evict) silently
    // corrupts every subsequent encrypted packet.
    //
    // These tests use a recording `TestTransport` so the `transport.send_to`
    // calls inside handle_login complete successfully — most assertions are
    // about state after the handoff, not packet bytes. The
    // `login_phases_emit_ordered_byte_sequence` test below is the exception:
    // it inspects the recorded fan-out to pin the phase 1→4 wire sequence.

    use crate::auth::PendingLogin;
    use crate::test_support::TestTransport;
    use cimmeria_entity::manager::EntityManager;

    fn make_pending_login(account_id: u32, key_byte: u8) -> PendingLogin {
        PendingLogin {
            account_id,
            access_level: 0,
            ticket: "TICKET00000000000001".to_string(),
            // 32-byte key encoded as 64 hex chars, all the same byte.
            session_key: format!("{:02X}", key_byte).repeat(32),
            created: Instant::now(),
        }
    }

    /// Helper: a recording transport so the `transport.send_to` calls inside
    /// handle_login are captured rather than hitting a real socket. Tests that
    /// only assert post-handoff state ignore the recorded bytes; the phase
    /// sequence test drains them.
    fn make_transport() -> Arc<dyn Transport> {
        Arc::new(TestTransport::new())
    }

    /// Stop the tick-sync loop spawned by handle_login. Without this,
    /// the spawned task leaks across tests (it sleeps + sends every
    /// 100ms forever). Setting cancelled=true on the loop's flag ends
    /// it on the next tick.
    fn cancel_session(
        connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        addr: SocketAddr,
    ) {
        if let Ok(map) = connected.lock() {
            if let Some(c) = map.get(&addr) {
                c.cancelled
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    /// Phase 3 happy path: a valid ticket gets consumed, the connected
    /// map gets a fully-populated ConnectedClientState, and the account
    /// entity is created in the entity manager. Pins the auth → base
    /// seam end-to-end at the state level.
    #[tokio::test]
    async fn login_consumes_ticket_and_registers_connected_client_state() {
        let transport = make_transport();
        let addr: SocketAddr = "127.0.0.1:55555".parse().unwrap();

        let pending_logins = Arc::new(Mutex::new(HashMap::new()));
        let connected = Arc::new(Mutex::new(HashMap::new()));
        let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
        let cell_tx = None;

        let pending = make_pending_login(0xDEAD_BEEF, 0xAB);
        let ticket = pending.ticket.clone();
        let expected_account_id = pending.account_id;
        pending_logins
            .lock()
            .unwrap()
            .insert(ticket.clone(), pending);

        // Pre-seed sanity: ticket present, no connected entries.
        assert_eq!(pending_logins.lock().unwrap().len(), 1);
        assert!(connected.lock().unwrap().is_empty());

        handle_login(
            &transport,
            addr,
            0xCAFE_BABE,
            &ticket,
            &pending_logins,
            &connected,
            &entity_manager,
            &cell_tx,
            &entity_to_addr,
        )
        .await
        .expect("Phase 3 handoff");

        // Ticket consumed — Phase 3 is single-shot.
        assert!(
            pending_logins.lock().unwrap().is_empty(),
            "ticket must be removed from pending_logins after consumption"
        );

        // Connected client state populated with the right account_id and
        // a key matching the session_key bytes. Any drift here breaks
        // the entire encrypted channel.
        let state = connected
            .lock()
            .unwrap()
            .get(&addr)
            .map(|c| {
                (
                    c.account_id,
                    c.access_level,
                    c.key,
                    c.world_entry_sent,
                    c.char_list_sent,
                )
            })
            .expect("connected entry present");
        assert_eq!(state.0, expected_account_id, "account_id from PendingLogin");
        assert_eq!(state.1, 0, "access_level from PendingLogin");
        assert_eq!(state.2, [0xABu8; 32], "key matches the decoded session_key");
        assert!(
            !state.3,
            "world_entry_sent starts false (no playCharacter yet)"
        );
        assert!(!state.4, "char_list_sent starts false");

        cancel_session(&connected, addr);
    }

    /// Domain F (fan-out byte test): with no duplicate session, `handle_login`
    /// emits exactly two packets, in order — the Phase 3 connect-reply (seq 1)
    /// then the initial time-sync bundle (seq 2) — both to the connecting
    /// addr, byte-exact. This pins the auth→base handshake *wire* sequence the
    /// state-level tests above can't see (no old-addr cleanup fires because
    /// `connected` starts empty). The spawned tick-sync loop sleeps 100 ms
    /// before its first send, and we cancel it immediately, so a synchronous
    /// drain captures only the handshake.
    #[tokio::test]
    async fn login_emits_ordered_connect_reply_then_time_sync_bytes() {
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let addr: SocketAddr = "127.0.0.1:55556".parse().unwrap();

        let pending_logins = Arc::new(Mutex::new(HashMap::new()));
        let connected = Arc::new(Mutex::new(HashMap::new()));
        let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
        let cell_tx = None;

        let key = [0xABu8; 32];
        let request_id = 0xCAFE_BABEu32;
        let pending = make_pending_login(0xDEAD_BEEF, 0xAB);
        let ticket = pending.ticket.clone();
        pending_logins
            .lock()
            .unwrap()
            .insert(ticket.clone(), pending);

        handle_login(
            &dyn_transport,
            addr,
            request_id,
            &ticket,
            &pending_logins,
            &connected,
            &entity_manager,
            &cell_tx,
            &entity_to_addr,
        )
        .await
        .expect("Phase 3 handoff");

        // Stop the tick loop before it can append a third packet.
        cancel_session(&connected, addr);

        let sent = transport.drain();
        assert_eq!(
            sent.len(),
            2,
            "no duplicate session ⇒ exactly connect_reply + time_sync (no old-addr cleanup)"
        );
        assert_eq!(sent[0].0, addr, "connect_reply goes to the connecting addr");
        assert_eq!(sent[1].0, addr, "time_sync goes to the connecting addr");
        assert_eq!(
            sent[0].1,
            build_connect_reply(request_id, ticket.as_bytes(), &key, 1),
            "phase-3 connect_reply bytes (seq 1)"
        );
        assert_eq!(
            sent[1].1,
            build_time_sync(&key, 2),
            "initial time_sync bytes (seq 2)"
        );
    }

    /// Phase 3 with an unknown ticket must NOT register a connected
    /// state — the function logs and returns Ok. Without this, a
    /// replayed-ticket packet from a stale client could create a
    /// half-initialized ConnectedClientState (no key, no account).
    #[tokio::test]
    async fn login_with_unknown_ticket_does_not_register_state() {
        let transport = make_transport();
        let addr: SocketAddr = "127.0.0.1:55556".parse().unwrap();

        let pending_logins = Arc::new(Mutex::new(HashMap::new()));
        let connected = Arc::new(Mutex::new(HashMap::new()));
        let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
        let cell_tx = None;

        // pending_logins is empty — any ticket lookup must miss.
        handle_login(
            &transport,
            addr,
            1,
            "DOES_NOT_EXIST_00000",
            &pending_logins,
            &connected,
            &entity_manager,
            &cell_tx,
            &entity_to_addr,
        )
        .await
        .expect("Phase 3 with unknown ticket returns Ok and logs");

        assert!(
            connected.lock().unwrap().is_empty(),
            "unknown ticket must not produce a ConnectedClientState"
        );
    }

    /// Duplicate-login eviction (KI-7): when a second Phase 3 lands for
    /// the same account_id from a different SocketAddr, the prior
    /// session is evicted — its addr removed from `connected` and
    /// LOGGED_OFF dispatched to the old client. Without this, two
    /// active sessions per account corrupt entity state on both sides.
    #[tokio::test]
    async fn second_login_for_same_account_evicts_first_session() {
        let transport = make_transport();
        let addr_a: SocketAddr = "127.0.0.1:55557".parse().unwrap();
        let addr_b: SocketAddr = "127.0.0.1:55558".parse().unwrap();

        let pending_logins = Arc::new(Mutex::new(HashMap::new()));
        let connected = Arc::new(Mutex::new(HashMap::new()));
        let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
        let cell_tx = None;

        const ACCOUNT_ID: u32 = 0xC0FF_EE42;

        // Session A: original login.
        let mut p_a = make_pending_login(ACCOUNT_ID, 0x01);
        p_a.ticket = "TICKETA000000000001A".to_string();
        let ticket_a = p_a.ticket.clone();
        pending_logins.lock().unwrap().insert(ticket_a.clone(), p_a);
        handle_login(
            &transport,
            addr_a,
            1,
            &ticket_a,
            &pending_logins,
            &connected,
            &entity_manager,
            &cell_tx,
            &entity_to_addr,
        )
        .await
        .expect("first login");
        assert!(
            connected.lock().unwrap().contains_key(&addr_a),
            "session A registered after first login"
        );

        // Session B: same account_id, different addr — must evict A.
        let mut p_b = make_pending_login(ACCOUNT_ID, 0x02);
        p_b.ticket = "TICKETB000000000002B".to_string();
        let ticket_b = p_b.ticket.clone();
        pending_logins.lock().unwrap().insert(ticket_b.clone(), p_b);
        handle_login(
            &transport,
            addr_b,
            2,
            &ticket_b,
            &pending_logins,
            &connected,
            &entity_manager,
            &cell_tx,
            &entity_to_addr,
        )
        .await
        .expect("second login");

        let map = connected.lock().unwrap();
        assert!(
            !map.contains_key(&addr_a),
            "addr_a (session A) must be evicted when account_id collides on a new addr",
        );
        assert!(
            map.contains_key(&addr_b),
            "addr_b (session B) takes the slot"
        );
        assert_eq!(
            map.get(&addr_b).unwrap().key,
            [0x02u8; 32],
            "session B's key must come from its OWN PendingLogin, not A's stale one"
        );
        drop(map);

        cancel_session(&connected, addr_b);
    }
}
