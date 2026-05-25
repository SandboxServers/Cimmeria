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
                destroy_client_entities(connected, entity_manager, addr, cell_tx, entity_to_addr);
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
            // Without a handler, this request is silently dropped (audit
            // finding N3 / issue #289). Forward to the cell, which re-emits a
            // synthetic `EnteredAoI` per requested entity that is currently
            // in the witness's AoI. Out-of-AoI requests are dropped on the
            // cell side -- the client must not be able to probe arbitrary ids.
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
mod tests {
    use super::*;

    /// Pin the framing of `restoreClientAck` (msg 0x0B) at
    /// CONSTANT_LENGTH = 4. Spec §2.5.2 names this — the sole
    /// emitter at `ghidra://SGW.exe@0x00dd8bc9` writes a literal
    /// `i32 = 0`, with no `u16` length prefix in front of it.
    ///
    /// Bug shape: parsing it as WORD_LENGTH would read the first
    /// two ack bytes as a length prefix (`0x00 0x00` → length 0),
    /// advance only 2 bytes past the msg_id, then read the
    /// remaining two ack bytes as the NEXT msg_id (`0x00 0x00` →
    /// `baseAppLogin`) and cascade-fail. This guard reproduces the
    /// exact bundle layout the bug fires on:
    ///
    /// ```text
    ///   [0x0B][0x00 0x00 0x00 0x00][0x07][0x05 0x00][...5-byte body]
    ///    ack         payload         next msg_id  u16 len
    /// ```
    ///
    /// Correct behavior: after reading the 0x0B + its 4-byte body,
    /// `offset` lands at 5, pointing exactly at the next msg_id
    /// (`0x07`, REQUEST_ENTITY_UPDATE). Reverting to
    /// `read_word_length_payload` lands at 3 (1 msg_id + 2 length
    /// prefix), reads `0x00` as next msg_id, and the assertion at
    /// the bottom trips.
    #[test]
    fn restore_client_ack_consumes_exactly_four_bytes() {
        // Bundle: 0x0B + 4-byte ack body + a real 0x07 message
        // (WORD_LENGTH) with a 5-byte payload. The framing-bug fix
        // is observable as "we read 0x07 as the next msg_id, not
        // 0x00", which only holds when 0x0B consumes 4 bytes.
        let bundle = [
            0x0B, // restoreClientAck
            0x00, 0x00, 0x00, 0x00, // ack body (i32 = 0)
            0x07, // REQUEST_ENTITY_UPDATE
            0x05, 0x00, // u16 length = 5
            0xDE, 0xAD, 0xBE, 0xEF, 0x42, // payload
        ];

        // First message: consume the ack.
        let mut offset = 1; // past msg_id 0x0B
        let ack_payload = read_client_message_payload(0x0B, &bundle, &mut offset)
            .expect("0x0B must produce a payload — it's CONSTANT_LENGTH = 4");
        assert_eq!(
            ack_payload,
            &[0x00, 0x00, 0x00, 0x00],
            "ack payload must be the literal i32 = 0 (four zero bytes)"
        );
        assert_eq!(
            offset, 5,
            "offset must advance to exactly 5 (1 msg_id + 4 body). \
             Pre-fix WORD_LENGTH parse advances to 3 (1 + 2 prefix + 0 length), \
             leaving two ack bytes unconsumed and misaligning every following message."
        );

        // Second message: confirm we land on 0x07 (the canary).
        let next_msg_id = bundle[offset];
        assert_eq!(
            next_msg_id, 0x07,
            "next msg_id must be 0x07 (REQUEST_ENTITY_UPDATE). Pre-fix this \
             would be 0x00 (baseAppLogin) because the WORD_LENGTH bug skips \
             only 2 bytes of the 4-byte ack, leaking 0x00 0x00 into the next \
             msg_id slot."
        );

        offset += 1;
        let req_payload = read_client_message_payload(0x07, &bundle, &mut offset)
            .expect("0x07 must produce a payload");
        assert_eq!(
            req_payload,
            &[0xDE, 0xAD, 0xBE, 0xEF, 0x42],
            "downstream message must round-trip cleanly: if 0x0B framing is \
             right, the parser arrives at 0x07's length prefix and reads the \
             5-byte payload as expected"
        );
        assert_eq!(
            offset,
            bundle.len(),
            "final offset must consume the entire bundle"
        );
    }

    /// Negative pin: a 0x0B payload truncated below 4 bytes must
    /// return `None`, signalling the bundle scan to break — NOT a
    /// silent advance past the end of `body`.
    #[test]
    fn restore_client_ack_truncation_returns_none() {
        let bundle = [0x0B, 0x00, 0x00, 0x00]; // only 3 ack bytes, not 4
        let mut offset = 1;
        assert!(
            read_client_message_payload(0x0B, &bundle, &mut offset).is_none(),
            "truncated 0x0B body must return None so the caller breaks the \
             bundle loop with a 'truncated' trace — silently advancing past \
             the end would corrupt all downstream offset arithmetic."
        );
    }

    /// Round-trip pin for the unchanged CONSTANT_LENGTH entries —
    /// catches a future refactor that swaps the dispatch arms with
    /// each other. Pre-fix this passed; the bug was the missing
    /// 0x0B row, not these.
    #[test]
    fn constant_length_dispatch_widths_match_spec() {
        let cases: &[(u8, usize)] = &[
            (0x02, 36), // AVATAR_UPD_IMPLICIT
            (0x03, 40), // AVATAR_UPDATE_EXPLICIT
            (0x04, 36), // AVATAR_UPDW_IMPLICIT
            (0x05, 40), // AVATAR_UPDW_EXPLICIT
            (0x06, 0),  // SWITCH_INTERFACE
            (0x08, 8),  // ENABLE_ENTITIES
            (0x09, 8),  // VIEWPORT_ACK
            (0x0A, 8),  // VEHICLE_ACK
            (0x0B, 4),  // RESTORE_CLIENT_ACK
            (0x0C, 1),  // DISCONNECT
        ];
        for &(msg_id, expected_width) in cases {
            // Construct a fresh body with exactly `expected_width`
            // bytes of payload after the (implicit) msg_id slot.
            let body: Vec<u8> = vec![0xAA; expected_width];
            let mut offset = 0;
            let payload =
                read_client_message_payload(msg_id, &body, &mut offset).unwrap_or_else(|| {
                    panic!(
                        "msg {msg_id:#04x} CONSTANT_LENGTH should accept a body of \
                         exactly {expected_width} bytes"
                    )
                });
            assert_eq!(
                payload.len(),
                expected_width,
                "msg {msg_id:#04x} produced wrong payload width"
            );
            assert_eq!(
                offset, expected_width,
                "msg {msg_id:#04x} must advance offset by exactly {expected_width}"
            );
        }
    }

    /// Pin the WORD_LENGTH default arm by msg_id family. Two
    /// distinct classes share the wildcard path:
    ///
    /// - **0x07** — `requestEntityUpdate`, the only system msg_id
    ///   in the WORD_LENGTH group. An explicit arm so the test
    ///   catches a regression where it slips into a different
    ///   width by accident.
    /// - **0xC2** — sample base-method msg_id from the `0xC0+`
    ///   entity-method range. `messages.cpp` documents every
    ///   `0x80..0xFE` byte as WORD_LENGTH per
    ///   `ServerConnection_startEntityMessage` (0x00dd6a60) and
    ///   `ServerConnection_startProxyMessage` (0x00dd6980). The
    ///   `_ => read_word_length_payload` wildcard arm handles
    ///   the whole range, so one representative msg_id is enough
    ///   to pin the contract.
    ///
    /// Together these guard against a future refactor that adds
    /// a special-case `0x07` arm with the wrong width, or that
    /// changes the wildcard arm to CONSTANT (which would
    /// catastrophically break every entity-method call).
    #[test]
    fn word_length_dispatch_arms_consume_u16_prefix_then_payload() {
        // Sweep both 0x07 (named arm) and 0xC2 (wildcard sample) so
        // a single test trips when EITHER arm regresses.
        for &msg_id in &[0x07u8, 0xC2u8] {
            // body: [len_lo, len_hi, payload...]
            let payload_bytes: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF];
            let mut body = Vec::with_capacity(2 + payload_bytes.len());
            body.extend_from_slice(&(payload_bytes.len() as u16).to_le_bytes());
            body.extend_from_slice(payload_bytes);

            let mut offset = 0;
            let read = read_client_message_payload(msg_id, &body, &mut offset)
                .unwrap_or_else(|| panic!("msg {msg_id:#04x} WORD_LENGTH must produce a payload"));
            assert_eq!(
                read, payload_bytes,
                "msg {msg_id:#04x} WORD_LENGTH must return the post-prefix payload bytes"
            );
            assert_eq!(
                offset,
                2 + payload_bytes.len(),
                "msg {msg_id:#04x} WORD_LENGTH must advance offset by 2 (prefix) + payload_len"
            );
        }
    }

    /// Truncated WORD_LENGTH prefix (only one byte of the u16) on
    /// the wildcard arm must return None so the bundle loop
    /// breaks cleanly. Symmetric to the CONSTANT-truncation guard
    /// above; ensures every dispatch arm refuses to silently
    /// over-read.
    #[test]
    fn word_length_truncated_prefix_returns_none() {
        let body = [0x05u8]; // missing the high byte of the u16 prefix
        let mut offset = 0;
        assert!(
            read_client_message_payload(0xC2, &body, &mut offset).is_none(),
            "truncated WORD_LENGTH prefix must return None — silently advancing \
             past the end of `body` would corrupt every downstream offset."
        );
    }

    // --- parse_request_entity_update parser pins (msg 0x07 body) ---

    /// Three ids round-trip cleanly with a zero header.
    #[test]
    fn parses_header_plus_three_ids() {
        // [u32 header = 0][u32 100][u32 200][u32 300] = 16 bytes
        let mut body = Vec::new();
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&100u32.to_le_bytes());
        body.extend_from_slice(&200u32.to_le_bytes());
        body.extend_from_slice(&300u32.to_le_bytes());
        assert_eq!(parse_request_entity_update(&body), vec![100, 200, 300]);
    }

    /// Header-only payload (4 bytes) decodes to an empty id list — that's
    /// the no-op case, not an error.
    #[test]
    fn header_only_payload_decodes_empty() {
        let body = [0u8; 4];
        assert_eq!(parse_request_entity_update(&body), Vec::<u32>::new());
    }

    /// Sub-header payloads (< 4 bytes) defensively return empty. The dispatch
    /// arm relies on this to no-op without panicking when the client (or a
    /// fuzzer) sends a malformed body.
    #[test]
    fn truncated_payload_returns_empty() {
        for len in 0..4 {
            let body = vec![0u8; len];
            assert!(
                parse_request_entity_update(&body).is_empty(),
                "expected empty result for {len}-byte body"
            );
        }
    }

    /// Trailing bytes that don't form a complete u32 are dropped — the parser
    /// reads as many whole ids as the length allows.
    #[test]
    fn trailing_partial_id_is_dropped() {
        // header + one full id (8 bytes) + 3 trailing bytes that can't form a u32
        let mut body = Vec::new();
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&42u32.to_le_bytes());
        body.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        assert_eq!(parse_request_entity_update(&body), vec![42]);
    }

    /// Header value is opaque — non-zero header bytes do NOT change which ids
    /// are decoded. Documents the "skip 4, then read ids" contract.
    #[test]
    fn non_zero_header_does_not_affect_id_decode() {
        let mut body = Vec::new();
        body.extend_from_slice(&0xDEADBEEFu32.to_le_bytes());
        body.extend_from_slice(&7u32.to_le_bytes());
        body.extend_from_slice(&8u32.to_le_bytes());
        assert_eq!(parse_request_entity_update(&body), vec![7, 8]);
    }

    /// Endianness: little-endian u32s only.
    #[test]
    fn ids_are_little_endian() {
        // header(0) + bytes for id = 0x01020304 little-endian = [04, 03, 02, 01]
        let body = [
            0, 0, 0, 0, // header
            0x04, 0x03, 0x02, 0x01, // id = 0x01020304
        ];
        assert_eq!(parse_request_entity_update(&body), vec![0x01020304]);
    }
}
