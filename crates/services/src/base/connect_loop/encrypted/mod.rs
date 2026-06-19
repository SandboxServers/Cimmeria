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

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
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
///
/// This is the load-bearing dispatch seam for established sessions —
/// every encrypted UDP packet from a logged-in client passes through
/// here. The `#[instrument]` parents Mercury packet events, account
/// ID, and any downstream method calls under one trace, so SigNoz
/// shows "what did this packet do?" as a single tree per datagram.
///
/// `level = "debug"` because the recv-loop sibling [`handle_datagram`]
/// already pays the per-packet span cost at debug; this nested span
/// adds the account_id correlation field and keeps the trace coherent
/// across the decrypt → parse → dispatch chain.
#[tracing::instrument(
    name = "base.encrypted_datagram",
    level = "debug",
    skip_all,
    fields(peer = %addr, account_id, raw_len = raw.len()),
)]
pub(crate) async fn handle_encrypted_datagram(
    transport: &Arc<dyn Transport>,
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
            // Not a session teardown — the next packet may decrypt
            // fine. Stable `reason` field makes "spike in HMAC fails"
            // a one-query alarm in SigNoz (key rollover gone wrong,
            // MITM attempt, replay attack window).
            tracing::warn!(
                %addr,
                disconnect_reason = "decrypt_fail",
                error = %e,
                "Decryption failed (bad HMAC?)"
            );
            return Ok(());
        }
    };

    tracing::trace!(%addr, len = plaintext.len(), hex = %to_hex(&plaintext), "DECRYPT_OK");

    let pkt = match parse_incoming(&plaintext) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(%addr, "Packet parse failed after decrypt: {e}");
            // Discord errors-channel: a decrypted-but-undecodable datagram is
            // a wire-format break worth a structured embed (the generic warn
            // harvest would lose the addr/kind structure). The sender's
            // bounded queue + per-channel rate limit absorb a flood.
            cimmeria_discord::emit_wire_format_error("parse_incoming", Some(addr), e.to_string());
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
    // Channel's TX window. The Channel drains its window cumulatively
    // up through each acked sequence and feeds RTT samples
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
        let payload_result = read_client_message_payload(msg_id, body, &mut offset);

        let payload = match payload_result {
            Some(p) => p,
            None => {
                tracing::trace!(%addr, msg_id = format_args!("{:#04x}", msg_id), "Bundle truncated");
                break;
            }
        };

        tracing::debug!(%addr, msg_id = format_args!("{:#04x}", msg_id), payload_len = payload.len(), "Client bundle message");

        crate::wire_log::log_inbound(addr, msg_id, payload);

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
                    transport,
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
                destroy_client_entities(
                    connected,
                    entity_manager,
                    addr,
                    cell_tx,
                    entity_to_addr,
                    "client_disconnect",
                );
            }
            // VIEWPORT_ACK (0x09)
            0x09 => {
                tracing::trace!(%addr, "Client sent VIEWPORT_ACK");
            }
            // REQUEST_ENTITY_UPDATE (0x07) -- client wants re-sync for one or
            // more entities it thinks it's missing or has stale state for.
            //
            // Wire (spec §2.5.2): `[u32 header][N × u32 entity_id]`. The
            // header semantic is unknown (likely flags or last-known-revision);
            // payload is safe to decode by skipping the first 4 bytes and
            // reading u32 ids until the length is exhausted.
            //
            // Forward to the cell, which re-emits a synthetic `EnteredAoI`
            // per requested entity that is currently in the witness's AoI.
            // Out-of-AoI requests are dropped on the cell side -- the client
            // must not be able to probe arbitrary ids.
            0x07 => {
                let entity_ids = parse_request_entity_update(payload);
                if entity_ids.is_empty() {
                    tracing::debug!(
                        %addr,
                        payload_len = payload.len(),
                        "REQUEST_ENTITY_UPDATE with no decoded entity ids -- ignoring"
                    );
                } else {
                    let witness_id = connected
                        .lock()
                        .unwrap()
                        .get(&addr)
                        .and_then(|c| c.player_entity_id);
                    if let Some(witness_id) = witness_id {
                        if let Some(tx) = cell_tx {
                            let count = entity_ids.len();
                            tracing::info!(
                                %addr,
                                witness_id,
                                count,
                                "REQUEST_ENTITY_UPDATE -> cell::RequestEntityUpdate"
                            );
                            if let Err(e) = tx
                                .send(BaseToCellMsg::RequestEntityUpdate {
                                    witness_id,
                                    entity_ids,
                                })
                                .await
                            {
                                tracing::warn!(
                                    %addr,
                                    witness_id,
                                    count,
                                    "REQUEST_ENTITY_UPDATE: cell send failed -- request dropped: {e}"
                                );
                            }
                        } else {
                            tracing::debug!(
                                %addr,
                                witness_id,
                                count = entity_ids.len(),
                                "REQUEST_ENTITY_UPDATE: no cell channel -- ignoring"
                            );
                        }
                    } else {
                        tracing::warn!(
                            %addr,
                            count = entity_ids.len(),
                            reason = "no_player_entity",
                            "REQUEST_ENTITY_UPDATE before player entity is connected -- dropping"
                        );
                    }
                }
            }

            // ── Protocol-level cooked-data messages ──
            //
            // These are not part of the active entity's base-method namespace.
            // The client sends them both before and after entering the world.
            0xC0 => {
                handle_version_info_request(
                    transport,
                    addr,
                    key,
                    payload,
                    connected,
                    resource_cache,
                )
                .await?;
            }
            0xC1 => {
                handle_element_data_request(
                    transport,
                    addr,
                    key,
                    payload,
                    connected,
                    resource_cache,
                )
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
                    transport,
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
                    transport,
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

/// Per-msg_id payload-length dispatch for the inbound client bundle.
///
/// Reads exactly one message's payload starting at `*offset` and
/// advances `*offset` past it. Returns `None` only on truncation —
/// the caller breaks the bundle scan in that case.
///
/// Two framing flavors per `messages.cpp::ClientMessageList`:
///
/// - **CONSTANT_LENGTH**: fixed-size payload with no length prefix.
///   Width pinned per message in the table below. `read_constant_payload`
///   advances by exactly that many bytes.
/// - **WORD_LENGTH**: payload prefixed by `u16` little-endian length.
///   `read_word_length_payload` reads the prefix, advances 2 bytes,
///   then advances by `prefix` bytes.
///
/// **0x0B (`restoreClientAck`) is CONSTANT_LENGTH = 4**, per
/// spec §2.5.2 and the sole emitter at
/// `ghidra://SGW.exe@0x00dd8bc9` (writes literal `i32 = 0`).
/// Parsing it as WORD_LENGTH reads the first two ack bytes as a
/// `u16` length = 0, then misinterprets the remaining two ack bytes
/// as the next msg_id (`0x00 0x00` → dispatches to `baseAppLogin`),
/// cascade-failing every subsequent message in the bundle. The
/// regression guard `restore_client_ack_consumes_exactly_four_bytes`
/// pins this.
fn read_client_message_payload<'a>(
    msg_id: u8,
    body: &'a [u8],
    offset: &mut usize,
) -> Option<&'a [u8]> {
    match msg_id {
        // --- System messages with CONSTANT_LENGTH ---
        // 0x02: AVATAR_UPD_IMPLICIT (CONSTANT_LENGTH = 36)
        0x02 => read_constant_payload(body, offset, 36),
        // 0x03: AVATAR_UPDATE_EXPLICIT (CONSTANT_LENGTH = 40)
        0x03 => read_constant_payload(body, offset, 40),
        // 0x04: AVATAR_UPDW_IMPLICIT (CONSTANT_LENGTH = 36)
        0x04 => read_constant_payload(body, offset, 36),
        // 0x05: AVATAR_UPDW_EXPLICIT (CONSTANT_LENGTH = 40)
        0x05 => read_constant_payload(body, offset, 40),
        // 0x06: SWITCH_INTERFACE (CONSTANT_LENGTH = 0)
        0x06 => read_constant_payload(body, offset, 0),
        // 0x08: ENABLE_ENTITIES (CONSTANT_LENGTH = 8)
        0x08 => read_constant_payload(body, offset, 8),
        // 0x09: VIEWPORT_ACK (CONSTANT_LENGTH = 8)
        0x09 => read_constant_payload(body, offset, 8),
        // 0x0A: VEHICLE_ACK (CONSTANT_LENGTH = 8)
        0x0A => read_constant_payload(body, offset, 8),
        // 0x0B: RESTORE_CLIENT_ACK (CONSTANT_LENGTH = 4 — see doc above)
        0x0B => read_constant_payload(body, offset, 4),
        // 0x0C: DISCONNECT (CONSTANT_LENGTH = 1)
        0x0C => read_constant_payload(body, offset, 1),

        // --- System messages with WORD_LENGTH ---
        // 0x07: REQUEST_ENTITY_UPDATE (WORD_LENGTH)
        0x07 => read_word_length_payload(body, offset),

        // --- Entity method calls (0xC0+): always WORD_LENGTH ---
        //
        // 0x0D `entityMessage` is intentionally NOT in the table:
        // its wire byte is `0x80..0xFE` (cell method `m | 0x80`,
        // base method `m | 0xC0`), NEVER the literal 0x0D. The
        // wildcard arm catches both ranges as WORD_LENGTH per
        // `ServerConnection_startEntityMessage` (0x00dd6a60) and
        // `ServerConnection_startProxyMessage` (0x00dd6980). See
        // audit doc §2.11 row `0x0D` for the disposition.
        _ => read_word_length_payload(body, offset),
    }
}

/// Parse a `requestEntityUpdate` (msg `0x07`) payload.
///
/// Wire layout (spec §2.5.2): `[u32 header][N × u32 entity_id]`. The header's
/// exact semantic (flags? last-known-revision?) is not documented; the spec
/// notes the payload is safe to decode by skipping the first 4 bytes and
/// reading consecutive u32s. Trailing bytes that don't form a complete u32
/// are dropped.
///
/// Returns an empty `Vec` when the payload is shorter than the 4-byte header.
fn parse_request_entity_update(payload: &[u8]) -> Vec<u32> {
    if payload.len() < 4 {
        return Vec::new();
    }
    payload[4..]
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

#[cfg(test)]
mod tests;
