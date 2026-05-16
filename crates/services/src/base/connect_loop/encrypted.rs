//! Per-datagram dispatch for established encrypted channels.
//!
//! Decrypts the datagram, parses the Mercury packet, queues an ACK if the
//! client message is reliable, then walks the bundle (the body can carry
//! multiple back-to-back messages) and dispatches each one. The arm bodies
//! for the larger families (account base methods, cell entity methods)
//! delegate to sibling modules to keep this match readable.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{parse_incoming, FLAG_RELIABLE};

use crate::cell::messages::BaseToCellMsg;

use super::super::cooked_data::{handle_element_data_request, handle_version_info_request};
use super::super::helpers::{destroy_client_entities, to_hex};
use super::super::resources::ResourceCache;
use super::super::world_entry::handle_enable_entities;
use super::super::ConnectedClientState;
use super::{account_arms, cell_arms, read_constant_payload, read_word_length_payload};

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

    // Route the client's ACKs of OUR reliable packets to the per-session
    // Channel's TX window (issue #308). The Channel drains its window
    // cumulatively up through each acked sequence and feeds RTT samples
    // to the per-peer adaptive RTO (Karn's algorithm — only clean rounds
    // contribute, retransmitted-packet samples are excluded internally).
    if !pkt.acks.is_empty() {
        if let Ok(clients) = connected.lock() {
            if let Some(state) = clients.get(&addr) {
                if let Ok(mut channel) = state.channel.lock() {
                    for &ack_seq in &pkt.acks {
                        if let Err(e) = channel.process_acks(ack_seq) {
                            tracing::warn!(%addr, ack_seq, error = %e, "channel.process_acks failed");
                        }
                    }
                    tracing::trace!(
                        %addr,
                        acks_consumed = pkt.acks.len(),
                        tx_window_len = channel.tx_window.len(),
                        srtt_ms = ?channel.rto().srtt().map(|d| d.as_millis()),
                        rto_ms = channel.rto().current().as_millis(),
                        "Channel TX window updated from client ACKs"
                    );
                }
            }
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
                handle_enable_entities(
                    socket,
                    addr,
                    key,
                    account_id,
                    connected,
                    db_pool,
                    entity_manager,
                    cell_tx,
                    entity_to_addr,
                )
                .await?;
            }
            // AVATAR_UPDATE_EXPLICIT (0x03) -- client movement update (40 bytes)
            // Wire: [spaceId:u32][vehicleId:u32][pos:3xf32][vel:3xf32][dir:3xi8][flags:u8][cells:3xu8][updateId:u8]
            // Note: first field is spaceId, NOT entityId. Entity is the authenticated player.
            0x03 => {
                if payload.len() >= 40 {
                    if let Some(ref tx) = cell_tx {
                        // Look up the player entity_id from connection state
                        let entity_id = connected
                            .lock()
                            .unwrap()
                            .get(&addr)
                            .and_then(|c| c.player_entity_id);
                        if let Some(entity_id) = entity_id {
                            // payload[0..4] = spaceId (not used here -- client confirms which space)
                            // payload[4..8] = vehicleId (unused)
                            let pos = [
                                f32::from_le_bytes([
                                    payload[8],
                                    payload[9],
                                    payload[10],
                                    payload[11],
                                ]),
                                f32::from_le_bytes([
                                    payload[12],
                                    payload[13],
                                    payload[14],
                                    payload[15],
                                ]),
                                f32::from_le_bytes([
                                    payload[16],
                                    payload[17],
                                    payload[18],
                                    payload[19],
                                ]),
                            ];
                            let vel = [
                                f32::from_le_bytes([
                                    payload[20],
                                    payload[21],
                                    payload[22],
                                    payload[23],
                                ]),
                                f32::from_le_bytes([
                                    payload[24],
                                    payload[25],
                                    payload[26],
                                    payload[27],
                                ]),
                                f32::from_le_bytes([
                                    payload[28],
                                    payload[29],
                                    payload[30],
                                    payload[31],
                                ]),
                            ];
                            let dir = [payload[32] as i8, payload[33] as i8, payload[34] as i8];
                            tracing::trace!(
                                entity_id,
                                ?pos,
                                "AVATAR_UPDATE_EXPLICIT -> CellService"
                            );
                            let _ = tx
                                .send(BaseToCellMsg::EntityMove {
                                    entity_id,
                                    position: pos,
                                    direction: dir,
                                    velocity: vel,
                                })
                                .await;
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
                handle_version_info_request(socket, addr, key, payload, connected, resource_cache)
                    .await?;
            }
            0xC1 => {
                handle_element_data_request(socket, addr, key, payload, connected, resource_cache)
                    .await?;
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
                account_arms::dispatch_base_method(
                    id,
                    payload,
                    addr,
                    socket,
                    key,
                    account_id,
                    connected,
                    db_pool,
                    entity_manager,
                    cell_tx,
                    entity_to_addr,
                )
                .await?;
            }
            // ── Cell entity method calls (0x80-0xBF range) ──
            // Wire format (from bundle.cpp + entity_message_handler.cpp):
            //   Direct (0-60):  [msg_id = methodId + 0x80][word_len][entityId: u32][args]
            //   Sub-slot (61+): [msg_id = 0xBD][word_len][entityId: u32][sub_index: u8][args]
            // The 4-byte entityId prefix is ALWAYS present and must be stripped.
            id if (0x80..=0xBF).contains(&id) => {
                if cell_arms::dispatch_cell_method(
                    id,
                    payload,
                    addr,
                    socket,
                    key,
                    connected,
                    cell_tx,
                    entity_to_addr,
                    db_pool,
                )
                .await
                .is_break()
                {
                    continue;
                }
            }
            _ => {
                tracing::trace!(%addr, msg_id = format_args!("{:#04x}", msg_id), payload_len = payload.len(), "Unhandled client message");
            }
        }
    }

    Ok(())
}
