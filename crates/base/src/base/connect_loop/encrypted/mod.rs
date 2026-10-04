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
use cimmeria_mercury::channel::RxDelivery;
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{parse_incoming, ParsedPacket};

use crate::cell::messages::BaseToCellMsg;

use super::super::cooked_data::{handle_element_data_request, handle_version_info_request};
use super::super::cooked_sync;
use super::super::helpers::destroy_client_entities;
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
            // fine. The stable `reason` field makes a spike a one-query
            // alarm in SigNoz, and a client's plaintext login retry is
            // told apart from a real decrypt failure (see
            // `decrypt_reject`).
            decrypt_reject::log_decrypt_reject(connected, addr, account_id, raw, &e);
            return Ok(());
        }
    };

    // Full row to base.log, a counted 1-in-N sample to SigNoz (NA25).
    crate::firehose::log_decrypt_ok(&crate::firehose::DECRYPT_OK_SAMPLER, addr, &plaintext);

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
        // No `flags_names` (NT-31): this row is exported for every inbound
        // datagram, so a name string here costs an allocation per packet.
        // The byte has eight bits; `PACKET_FLAGS` names them.
        flags = pkt.flags,
        body_len = pkt.body.len(),
        seq = ?pkt.seq_id,
        acks = ?pkt.acks,
        "Decrypted packet received"
    );

    // Route the client's ACKs of OUR reliable packets to the per-session
    // Channel's TX window. Each ACK retires exactly the packet it names:
    // the client acks packets it is buffering behind a gap, so reading an
    // ACK as cumulative would retire the packet it is still waiting for
    // (see `cimmeria_mercury::channel` `ack` module doc). The Channel feeds
    // clean-round RTT samples to the per-peer adaptive RTO (Karn's
    // algorithm) and tracks the transmit hole the tick-side watchdog
    // reports.
    if !pkt.acks.is_empty() {
        if let Ok(clients) = connected.lock() {
            if let Some(state) = clients.get(&addr) {
                if let Ok(mut channel) = state.channel.lock() {
                    let holes_before = channel.tx_holes;
                    let retired = channel.process_ack_footer(&pkt.acks);
                    if channel.tx_holes > holes_before {
                        // The client acked a later reliable packet before an
                        // earlier one: that one was lost (or reordered).
                        // The rate per peer is the server->client loss rate
                        // the retransmit scan has to cover.
                        cimmeria_observability::counter!("mercury_tx_holes_total");
                    }
                    tracing::trace!(
                        %addr,
                        acks_consumed = pkt.acks.len(),
                        retired,
                        tx_window_len = channel.tx_window.len(),
                        srtt_ms = ?channel.rto().srtt().map(|d| d.as_millis()),
                        rto_ms = channel.rto().current().as_millis(),
                        "Channel TX window updated from client ACKs"
                    );
                }
            }
        }
    }

    // In-order delivery of the client's reliable stream, the gate a
    // BigWorld receiver runs (NA38): a reliable packet behind a gap waits
    // for the client's retransmit, a retransmitted duplicate (our ACK was
    // lost) is dropped instead of being dispatched a second time, and
    // unreliable packets (movement) go straight through.
    let Some(delivery) = receive_in_order(connected, addr, pkt) else {
        return Ok(());
    };
    if let Some(seq) = delivery.ack {
        tracing::trace!(%addr, client_seq = seq, "Queueing ACK for client reliable message");
        pending_acks.lock().unwrap().push(seq);
    }
    // Each bundle travels with the Mercury seq that carried it, so the
    // `useAbility` receipt row can name the packet the client logged when it
    // sent the press (ability-mechanics AB-T2).
    for (i, body) in delivery.bundles.iter().enumerate() {
        let packet_seq = delivery.bundle_seqs.get(i).copied().flatten();
        dispatch_client_bundle(
            body,
            packet_seq,
            transport,
            addr,
            key,
            account_id,
            connected,
            db_pool,
            resource_cache,
            entity_manager,
            cell_tx,
            entity_to_addr,
        )
        .await?;
    }
    Ok(())
}

/// Run one decrypted client packet through the session's
/// [`cimmeria_mercury::channel::Channel`] receive gate.
///
/// `None` when the session is gone (torn down between the lookup that
/// found its key and now) or its channel lock is poisoned; the packet is
/// dropped either way.
fn receive_in_order(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    pkt: ParsedPacket,
) -> Option<RxDelivery> {
    let clients = connected.lock().ok()?;
    let Some(state) = clients.get(&addr) else {
        tracing::debug!(
            %addr,
            reason = "session_gone",
            "client packet arrived after its session was removed -- dropped"
        );
        return None;
    };
    let mut channel = state.channel.lock().ok()?;
    match channel.receive_parsed(pkt) {
        Ok(delivery) => Some(delivery),
        Err(e) => {
            tracing::warn!(
                %addr,
                reason = "rx_reassembly_error",
                error = %e,
                "client packet rejected by the channel receive path"
            );
            None
        }
    }
}

/// Walk one complete client bundle (the body can carry several
/// back-to-back messages) and dispatch each message.
async fn dispatch_client_bundle(
    body: &[u8],
    packet_seq: Option<u32>,
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    resource_cache: &Option<Arc<ResourceCache>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        // At character select the client cache methods come first:
        //   0xC0=versionInfoRequest, 0xC1=elementDataRequest
        // In-world the same IDs are SGWPlayer.chatJoin / chatLeave.
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
                            // payload[0..4] = spaceId the client claims it is
                            // in. The cell never writes against it (the server
                            // `entity_space` binding is authoritative); it is
                            // forwarded only so the cell can warn on a
                            // server↔client space divergence.
                            // payload[4..8] = vehicleId (unused)
                            let claimed_space_id = u32::from_le_bytes([
                                payload[0], payload[1], payload[2], payload[3],
                            ]);
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
                                    claimed_space_id,
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
                    transport,
                    db_pool,
                    "client_disconnect",
                );
            }
            // VIEWPORT_ACK (0x09)
            0x09 => {
                tracing::trace!(%addr, "Client sent VIEWPORT_ACK");
            }
            // REQUEST_ENTITY_UPDATE (0x07) -- the client's cache-stamp
            // handshake, fired once per non-player entity from
            // `EntityManager::onEntityEnter` for EVERY normal AoI entry, not
            // just recovery from a dropped `createEntity`.
            //
            // Wire (spec §2.5.2, corrected 2026-09-28 per issue #838):
            // `[u32 entityId][N × u32 cacheStamp]`, N always 0 on this client
            // build. See
            // `docs/reverse-engineering/findings/request-entity-update-cache-stamp.md`
            // for the RE evidence.
            //
            // Forward to the cell: an id already in the witness's AoI gets no
            // reply (the client already has full state from its original
            // CREATE_ENTITY); an id outside it is refused (anti-probe -- the
            // client must not be able to probe arbitrary ids).
            0x07 => {
                let entity_ids = parse_request_entity_update(payload);
                if entity_ids.is_empty() {
                    tracing::warn!(
                        %addr,
                        account_id,
                        payload_len = payload.len(),
                        reason = "payload_too_short",
                        "REQUEST_ENTITY_UPDATE: payload shorter than the 4-byte entity id -- dropping"
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
                            tracing::debug!(
                                %addr,
                                account_id,
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
                                    account_id,
                                    witness_id,
                                    count,
                                    "REQUEST_ENTITY_UPDATE: cell send failed -- request dropped: {e}"
                                );
                            }
                        } else {
                            tracing::debug!(
                                %addr,
                                account_id,
                                witness_id,
                                count = entity_ids.len(),
                                "REQUEST_ENTITY_UPDATE: no cell channel -- ignoring"
                            );
                        }
                    } else {
                        tracing::warn!(
                            %addr,
                            account_id,
                            count = entity_ids.len(),
                            reason = "no_player_entity",
                            "REQUEST_ENTITY_UPDATE before player entity is connected -- dropping"
                        );
                    }
                }
            }

            // ── Account-phase cooked-data messages ──
            //
            // 0xC0/0xC1 are the ClientCache methods only while the Account
            // entity is active (character select). Once the player entity
            // exists they are SGWPlayer.chatJoin / chatLeave (Communicator
            // indices 0 and 1) and fall through to the base-method dispatch
            // below. Routing them here in-world read a chatJoin's WSTRING
            // (length, "ch...") as a category and version, and answered it
            // with an InvalidateAll that emptied category 16 on every login
            // (#840).
            0xC0 if !cooked_sync::in_world(connected, addr) => {
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
            0xC1 if !cooked_sync::in_world(connected, addr) => {
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
                    resource_cache,
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
                    packet_seq,
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
/// Wire layout (spec §2.5.2, corrected 2026-09-28 per issue #838): `[u32
/// entityId][N × u32 cacheStamp]`. The client's `EntityManager::onEntityEnter`
/// (`ghidra://SGW.exe@0x00dd24f0`) sends this once per non-player entity
/// entering its AoI, with `N` always 0 on this client build -- BigWorld's
/// cache-stamp versioning is never populated. See
/// `docs/reverse-engineering/findings/request-entity-update-cache-stamp.md`
/// for the full RE evidence. The pre-#838 layout here, `[u32 header][N × u32
/// entity_id]`, was wrong: it read the entity id as a discardable header and
/// treated the (always-empty) cache-stamp tail as the entity id list, so
/// every real payload decoded to zero ids.
///
/// The cache-stamp values themselves are not interpreted -- the server has
/// no per-property cache to diff them against, and every observed client
/// build sends none.
///
/// Returns an empty `Vec` when the payload is shorter than the 4-byte entity
/// id, else a single-element `Vec` containing it. Trailing cache-stamp bytes
/// are consumed (ignored) rather than left to desync the bundle.
fn parse_request_entity_update(payload: &[u8]) -> Vec<u32> {
    if payload.len() < 4 {
        return Vec::new();
    }
    let entity_id = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    vec![entity_id]
}

mod decrypt_reject;

#[cfg(test)]
mod cache_routing_tests;
#[cfg(test)]
mod decrypt_reject_tests;
#[cfg(test)]
mod rx_order_tests;
#[cfg(test)]
mod tests;
