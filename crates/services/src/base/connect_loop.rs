use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{parse_incoming, FLAG_HAS_REQUESTS, FLAG_HAS_SEQUENCE, FLAG_RELIABLE};

use crate::cell::messages::BaseToCellMsg;

use super::ConnectedClientState;
use super::character::{handle_create_character, handle_delete_character, handle_request_character_visuals};
use super::cooked_data::{handle_element_data_request, handle_version_info_request};
use super::dispatch::{dispatch_sgw_player_base_method, sgw_player_base};
use super::helpers::{destroy_client_entities, to_hex};
use super::login::{handle_log_off, handle_login, parse_baseapp_login};
use super::resources::ResourceCache;
use super::world_entry::{
    handle_enable_entities, handle_map_loaded_phase_b, handle_on_client_ready, handle_play_character,
};

/// Main receive loop -- one per running `BaseService`.
pub(crate) async fn run_connect_loop(
    socket: Arc<UdpSocket>,
    pending_logins: Arc<Mutex<HashMap<String, crate::auth::PendingLogin>>>,
    db_pool: Option<Arc<PgPool>>,
    resource_cache: Option<Arc<ResourceCache>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: Arc<Mutex<EntityManager>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {

    let mut buf = [0u8; 4096];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, addr)) => {
                tracing::trace!(%addr, len, hex = %to_hex(&buf[..len]), "UDP_IN");
                if let Err(e) = handle_datagram(
                    &socket,
                    addr,
                    &buf[..len],
                    &pending_logins,
                    &connected,
                    &db_pool,
                    &resource_cache,
                    &entity_manager,
                    &cell_tx,
                    &entity_to_addr,
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
                    tracing::debug!("UDP recv: WSAECONNRESET (10054) -- remote closed, continuing");
                    continue;
                }
                tracing::error!("Base service UDP recv error (fatal): {e}");
                break;
            }
        }
    }
}

/// Dispatch a single incoming UDP datagram.
async fn handle_datagram(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    raw: &[u8],
    pending_logins: &Arc<Mutex<HashMap<String, crate::auth::PendingLogin>>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    resource_cache: &Option<Arc<ResourceCache>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if raw.is_empty() {
        return Ok(());
    }

    // Check for an established encrypted channel first.
    let channel_key: Option<(MercuryEncryption, [u8; 32], u32, Arc<Mutex<Vec<u32>>>, Arc<Mutex<Instant>>)> = {
        let clients = connected
            .lock()
            .map_err(|_| "connected lock poisoned")?;
        clients.get(&addr).map(|c| {
            (
                c.enc.clone(),
                c.key,
                c.account_id,
                Arc::clone(&c.pending_acks),
                Arc::clone(&c.last_recv),
            )
        })
    };

    if let Some((enc, key, account_id, pending_acks, last_recv)) = channel_key {
        // Update last-recv timestamp on every packet from this client.
        *last_recv.lock().unwrap() = Instant::now();
        return handle_encrypted_datagram(
            socket, addr, raw, enc, key, account_id,
            &pending_acks, connected, db_pool, resource_cache, entity_manager, cell_tx,
            entity_to_addr,
        ).await;
    }

    // Not yet connected -- only accept the unencrypted Phase 3 login (flags=0x41).
    let login_flags = FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE; // 0x41
    if raw[0] != login_flags {
        tracing::trace!(%addr, flags = raw[0], "Ignoring packet from unknown addr (flags={:#04x})", raw[0]);
        return Ok(());
    }

    match parse_baseapp_login(raw) {
        Ok((request_id, ticket_str)) => {
            tracing::info!(%addr, ticket = %ticket_str, "baseAppLogin received");
            handle_login(socket, addr, request_id, &ticket_str, pending_logins, connected, entity_manager, cell_tx, entity_to_addr).await
        }
        Err(e) => {
            tracing::warn!(%addr, "Failed to parse baseAppLogin: {e}");
            Ok(())
        }
    }
}

/// Handle an encrypted datagram from a known connected client.
pub(crate) async fn handle_encrypted_datagram(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    raw: &[u8],
    enc: MercuryEncryption,
    key: [u8; 32],
    account_id: u32,
    pending_acks: &Arc<Mutex<Vec<u32>>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    resource_cache: &Option<Arc<ResourceCache>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
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

    // Parse the client bundle.
    let body = &pkt.body;
    if body.is_empty() {
        return Ok(());
    }

    let mut offset = 0;

    // First message may be authenticate (0x01, WORD_LENGTH).
    // The C++ reference server ignores this message -- entity creation happens
    // on ENABLE_ENTITIES (0x08) so the client's entity system is ready.
    if body[offset] == 0x01 {
        offset += 1; // skip msg_id
        if offset + 2 <= body.len() {
            let word_len = u16::from_le_bytes([body[offset], body[offset + 1]]) as usize;
            offset += 2 + word_len;
        }
        tracing::debug!(%addr, "AUTHENTICATE received -- ignored (entity created on ENABLE_ENTITIES)");

        if offset >= body.len() {
            return Ok(());
        }
    }

    // Scan remaining messages in the bundle.
    //
    // Client messages come in two flavours:
    //   - System messages (0x00-0x0D): use CONSTANT_LENGTH or WORD_LENGTH per the
    //     ClientMessageList table in messages.cpp.
    //   - Entity method calls (0xC0+): always WORD_LENGTH (u16 prefix).
    while offset < body.len() {
        let msg_id = body[offset];
        offset += 1;

        // Determine payload length based on message format.
        // System messages (0x00-0x0D) have defined formats; entity methods use WORD_LENGTH.
        let payload_result = match msg_id {
            // --- System messages with CONSTANT_LENGTH ---
            // 0x02: AVATAR_UPD_IMPLICIT (CONSTANT_LENGTH = 36)
            0x02 => read_constant_payload(body, &mut offset, 36),
            // 0x03: AVATAR_UPDATE_EXPLICIT (CONSTANT_LENGTH = 40)
            0x03 => read_constant_payload(body, &mut offset, 40),
            // 0x04: AVATAR_UPDW_IMPLICIT (CONSTANT_LENGTH = 36)
            0x04 => read_constant_payload(body, &mut offset, 36),
            // 0x05: AVATAR_UPDW_EXPLICIT (CONSTANT_LENGTH = 40)
            0x05 => read_constant_payload(body, &mut offset, 40),
            // 0x06: SWITCH_INTERFACE (CONSTANT_LENGTH = 0)
            0x06 => read_constant_payload(body, &mut offset, 0),
            // 0x08: ENABLE_ENTITIES (CONSTANT_LENGTH = 8)
            0x08 => read_constant_payload(body, &mut offset, 8),
            // 0x09: VIEWPORT_ACK (CONSTANT_LENGTH = 8)
            0x09 => read_constant_payload(body, &mut offset, 8),
            // 0x0A: VEHICLE_ACK (CONSTANT_LENGTH = 8)
            0x0A => read_constant_payload(body, &mut offset, 8),
            // 0x0C: DISCONNECT (CONSTANT_LENGTH = 1)
            0x0C => read_constant_payload(body, &mut offset, 1),

            // --- System messages with WORD_LENGTH ---
            // 0x07: REQUEST_ENTITY_UPDATE (WORD_LENGTH)
            0x07 => read_word_length_payload(body, &mut offset),
            // 0x0B: RESTORE_CLIENT_ACK (WORD_LENGTH)
            0x0B => read_word_length_payload(body, &mut offset),

            // --- Entity method calls (0xC0+): always WORD_LENGTH ---
            _ => read_word_length_payload(body, &mut offset),
        };

        let payload = match payload_result {
            Some(p) => p,
            None => {
                tracing::trace!(%addr, msg_id = format_args!("{:#04x}", msg_id), "Bundle truncated");
                break;
            }
        };

        tracing::debug!(%addr, msg_id = format_args!("{:#04x}", msg_id), payload_len = payload.len(), "Client bundle message");

        // Dispatch message.
        //
        // The client cache methods are protocol-level messages that keep the
        // same wire IDs both at character select and in-world:
        //   0xC0=versionInfoRequest, 0xC1=elementDataRequest
        //
        // Account base methods start after those protocol IDs:
        //   0xC2=logOff, 0xC3=createCharacter, 0xC4=playCharacter,
        //   0xC5=deleteCharacter, 0xC6=requestCharacterVisuals, 0xC7=onClientVersion
        match msg_id {
            // ── System messages ──
            // ENABLE_ENTITIES (0x08) -- client re-enables entity system after RESET_ENTITIES
            0x08 => {
                tracing::info!(%addr, "Client sent ENABLE_ENTITIES");
                handle_enable_entities(socket, addr, key, account_id, connected, db_pool, entity_manager, cell_tx, entity_to_addr).await?;
            }
            // AVATAR_UPDATE_EXPLICIT (0x03) -- client movement update (40 bytes)
            // Wire: [spaceId:u32][vehicleId:u32][pos:3xf32][vel:3xf32][dir:3xi8][flags:u8][cells:3xu8][updateId:u8]
            // Note: first field is spaceId, NOT entityId. Entity is the authenticated player.
            0x03 => {
                if payload.len() >= 40 {
                    if let Some(ref tx) = cell_tx {
                        // Look up the player entity_id from connection state
                        let entity_id = connected.lock().unwrap()
                            .get(&addr)
                            .and_then(|c| c.player_entity_id);
                        if let Some(entity_id) = entity_id {
                            // payload[0..4] = spaceId (not used here -- client confirms which space)
                            // payload[4..8] = vehicleId (unused)
                            let pos = [
                                f32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]),
                                f32::from_le_bytes([payload[12], payload[13], payload[14], payload[15]]),
                                f32::from_le_bytes([payload[16], payload[17], payload[18], payload[19]]),
                            ];
                            let vel = [
                                f32::from_le_bytes([payload[20], payload[21], payload[22], payload[23]]),
                                f32::from_le_bytes([payload[24], payload[25], payload[26], payload[27]]),
                                f32::from_le_bytes([payload[28], payload[29], payload[30], payload[31]]),
                            ];
                            let dir = [payload[32] as i8, payload[33] as i8, payload[34] as i8];
                            tracing::trace!(entity_id, ?pos, "AVATAR_UPDATE_EXPLICIT -> CellService");
                            let _ = tx.send(BaseToCellMsg::EntityMove {
                                entity_id,
                                position: pos,
                                direction: dir,
                                velocity: vel,
                            }).await;
                        }
                    }
                }
            }
            // DISCONNECT (0x0C)
            0x0C => {
                tracing::info!(%addr, "Client sent DISCONNECT");
                destroy_client_entities(connected, entity_manager, addr, cell_tx, entity_to_addr);
            }
            // VIEWPORT_ACK (0x09)
            0x09 => {
                tracing::trace!(%addr, "Client sent VIEWPORT_ACK");
            }

            // ── Protocol-level cooked-data messages ──
            //
            // These are not part of the active entity's base-method namespace.
            // The client sends them both before and after entering the world.
            0xC0 => {
                handle_version_info_request(
                    socket, addr, key, payload, connected, resource_cache,
                ).await?;
            }
            0xC1 => {
                handle_element_data_request(
                    socket, addr, key, payload, connected, resource_cache,
                ).await?;
            }

            // ── Entity base method calls (0xC0+) ──
            //
            // ACCOUNT entity (character select):
            //   Account:     0xC2=logOff, 0xC3=createCharacter, 0xC4=playCharacter,
            //                0xC5=deleteCharacter, 0xC6=requestCharacterVisuals, 0xC7=onClientVersion
            //
            // SGWPLAYER entity (in-world):
            //   SGWPlayer base methods use their own namespace after the global
            //   protocol-level cache messages above.
            //   (Other interfaces and SGWPlayer own methods at higher indices)
            id if id >= 0xC0 => {
                let (in_world, player_name) = {
                    let clients = connected.lock().unwrap();
                    match clients.get(&addr) {
                        Some(c) => (c.player_entity_id.is_some(), c.player_name.clone()),
                        None => (false, None),
                    }
                };

                if in_world {
                    match id {
                        sgw_player_base::ON_CLIENT_READY => {
                            handle_on_client_ready(addr, connected, cell_tx).await?;
                        }
                        _ => {
                            // SGWPlayer base method dispatch
                            dispatch_sgw_player_base_method(
                                id, payload, &player_name, addr, socket, key,
                                connected, entity_manager, cell_tx, entity_to_addr,
                            ).await?;
                        }
                    }
                } else {
                    // Account base method dispatch
                    match id {
                        0xC2 => {
                            handle_log_off(
                                socket, addr, key, connected, entity_manager, cell_tx, entity_to_addr,
                            ).await?;
                        }
                        0xC3 => {
                            tracing::info!(%addr, "Client requests createCharacter");
                            handle_create_character(
                                socket, addr, key, account_id, payload,
                                connected, db_pool,
                            ).await?;
                        }
                        0xC4 => {
                            let player_id = if payload.len() >= 4 {
                                i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]])
                            } else {
                                0
                            };
                            tracing::info!(%addr, player_id, "Client requests playCharacter");
                            handle_play_character(socket, addr, key, account_id, player_id, connected, db_pool, entity_manager, cell_tx).await?;
                        }
                        0xC5 => {
                            let player_id = if payload.len() >= 4 {
                                i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]])
                            } else { 0 };
                            tracing::info!(%addr, player_id, "Client requests deleteCharacter");
                            handle_delete_character(
                                socket, addr, key, account_id, player_id,
                                connected, db_pool,
                            ).await?;
                        }
                        0xC6 => {
                            let player_id = if payload.len() >= 4 {
                                i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]])
                            } else { 0 };
                            tracing::debug!(%addr, player_id, "Client sent requestCharacterVisuals");
                            handle_request_character_visuals(
                                socket, addr, key, player_id, connected, db_pool,
                            ).await?;
                        }
                        0xC7 => {
                            tracing::debug!(%addr, "Client sent onClientVersion -- acknowledged");
                        }
                        _ => {
                            tracing::trace!(%addr, msg_id = format_args!("{:#04x}", id), "Unhandled Account base method");
                        }
                    }
                }
            }
            // ── Cell entity method calls (0x80-0xBF range) ──
            // After world entry, the client sends cell method calls as:
            //   Direct:   msg_id = cellMethodIndex | 0x80 (indices 0-60)
            //   Extended: msg_id = 0xBD, payload contains subIndex
            // These are forwarded to the CellService for dispatch.
            id if (0x80..=0xBF).contains(&id) => {
                // ── Phase 5b-B trigger ──
                // After Phase 5b-A, the client sends the exposed SGWPlayer cell
                // method `mapLoaded` (index 25, msg_id 0x99). The C++ server
                // waits for that specific method before sending VIEWPORT + CELL +
                // POSITION + the full entity data bundle.
                let phase_b_pending = {
                    let clients = connected.lock().unwrap();
                    clients.get(&addr).map_or(false, |c| c.pending_world_entry_phase_b.is_some())
                };
                if phase_b_pending && id == 0x99 {
                    if let Err(e) = handle_map_loaded_phase_b(
                        socket, addr, key, connected, cell_tx, entity_to_addr, db_pool,
                    ).await {
                        tracing::error!(%addr, error = %e, "Phase 5b-B failed");
                    }
                }

                let player_eid = {
                    let clients = connected.lock().unwrap();
                    clients.get(&addr).and_then(|c| c.player_entity_id)
                };
                if let Some(player_eid) = player_eid {
                    if phase_b_pending && id != 0x99 {
                        tracing::trace!(
                            %addr,
                            msg_id = format_args!("{:#04x}", id),
                            "Ignoring cell method until mapLoaded arrives"
                        );
                        continue;
                    }
                    if id == 0xBD {
                        // Extended encoding: subIndex is first byte of payload
                        if !payload.is_empty() {
                            let sub_index = payload[0] as u16;
                            let args = if payload.len() > 1 { payload[1..].to_vec() } else { Vec::new() };
                            if let Some(ref tx) = cell_tx {
                                let _ = tx.send(BaseToCellMsg::CellMethodCall {
                                    entity_id: player_eid,
                                    method_index: sub_index + 61,
                                    args,
                                }).await;
                            }
                        }
                    } else {
                        let method_index = (id - 0x80) as u16;
                        if let Some(ref tx) = cell_tx {
                            let _ = tx.send(BaseToCellMsg::CellMethodCall {
                                entity_id: player_eid,
                                method_index,
                                args: payload.to_vec(),
                            }).await;
                        }
                    }
                } else {
                    tracing::trace!(%addr, msg_id = format_args!("{:#04x}", id), "Cell method before world entry -- ignored");
                }
            }
            _ => {
                tracing::trace!(%addr, msg_id = format_args!("{:#04x}", msg_id), payload_len = payload.len(), "Unhandled client message");
            }
        }
    }

    Ok(())
}

/// Read a CONSTANT_LENGTH payload (no length prefix, fixed size).
fn read_constant_payload<'a>(body: &'a [u8], offset: &mut usize, size: usize) -> Option<&'a [u8]> {
    if *offset + size > body.len() {
        return None;
    }
    let payload = &body[*offset..*offset + size];
    *offset += size;
    Some(payload)
}

/// Read a WORD_LENGTH payload (u16 length prefix).
fn read_word_length_payload<'a>(body: &'a [u8], offset: &mut usize) -> Option<&'a [u8]> {
    if *offset + 2 > body.len() {
        return None;
    }
    let word_len = u16::from_le_bytes([body[*offset], body[*offset + 1]]) as usize;
    *offset += 2;
    if *offset + word_len > body.len() {
        return None;
    }
    let payload = &body[*offset..*offset + word_len];
    *offset += word_len;
    Some(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_constant_length_0x02() {
        // AVATAR_UPD_IMPLICIT: CONSTANT_LENGTH = 36
        let buf = vec![0xAAu8; 36];
        let mut offset = 0usize;
        let payload = read_constant_payload(&buf, &mut offset, 36);
        assert!(payload.is_some());
        assert_eq!(offset, 36);
        assert_eq!(payload.unwrap().len(), 36);
    }

    #[test]
    fn scanner_constant_length_0x04() {
        // AVATAR_UPDW_IMPLICIT: CONSTANT_LENGTH = 36
        let buf = vec![0xBBu8; 36];
        let mut offset = 0usize;
        let payload = read_constant_payload(&buf, &mut offset, 36);
        assert!(payload.is_some());
        assert_eq!(offset, 36);
        assert_eq!(payload.unwrap().len(), 36);
    }

    #[test]
    fn scanner_constant_length_0x05() {
        // AVATAR_UPDW_EXPLICIT: CONSTANT_LENGTH = 40
        let buf = vec![0xCCu8; 40];
        let mut offset = 0usize;
        let payload = read_constant_payload(&buf, &mut offset, 40);
        assert!(payload.is_some());
        assert_eq!(offset, 40);
        assert_eq!(payload.unwrap().len(), 40);
    }
}
