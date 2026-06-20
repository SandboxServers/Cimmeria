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
                account_name: Some(login.account_name.clone()),
                access_level: login.access_level,
                dnd_message: None,
                char_list_sent: false,
                world_entry_sent: false,
                pending_player_entity_id: None,
                player_entity_id: None,
                next_seq,
                next_seq_unreliable,
                pending_acks,
                last_recv,
                connected_at: Instant::now(),
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

    // Discord auth-channel ping. Character isn't selected yet at Phase 3
    // (that happens at playCharacter), so the name is `None` here — the
    // world-entry emit carries the character once it's known.
    cimmeria_discord::emit_player_login(
        login.account_id,
        Some(login.account_name.clone()),
        None,
        addr,
    );

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
    // Snapshot the identity + session duration here too, before
    // `destroy_client_entities` removes the session from the map — the
    // Discord logout emit below needs them.
    let (acks, seq, account_id, account_name, player_name, session_secs) = {
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
        let account_id = client.account_id;
        let account_name = client.account_name.clone();
        let player_name = client.player_name.clone();
        let session_secs = client.connected_at.elapsed().as_secs();
        (
            acks,
            seq,
            account_id,
            account_name,
            player_name,
            session_secs,
        )
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

    // Discord auth-channel: clean logout carries the session length. The
    // lower-level teardown in `destroy_client_entities` also fires a
    // `PlayerDisconnect { reason: Clean }` — by design (see the
    // `DisconnectReason::Clean` doc): logout is the gameplay-level event,
    // disconnect the connection-level one.
    cimmeria_discord::emit_player_logout(account_id, account_name, player_name, session_secs);

    Ok(())
}

#[cfg(test)]
mod tests;
