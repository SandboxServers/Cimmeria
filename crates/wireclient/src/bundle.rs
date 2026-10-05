//! Server → client bundle decoder.
//!
//! A reassembled Mercury bundle (the `Bytes` [`crate::session::GameSession`]
//! hands back from `recv_bundles`) is a back-to-back sequence of messages
//! with **two** framing families, mirroring the client → server decoder in
//! `crates/base/src/base/connect_loop/encrypted/mod.rs::read_client_message_payload`:
//!
//! - **Static base messages** (`0x00..=0x37`, plus `0xFF`): each has a fixed
//!   framing rule — either a `CONSTANT_LENGTH` payload with no prefix, or a
//!   `WORD_LENGTH` payload prefixed by a `u16` byte count. The table below
//!   (`STATIC_MSG_FORMAT`) is a direct Rust port of `SERVER_MSG_FORMAT` in
//!   `tools/pcap_dissect.py`, which is itself pinned against the live
//!   `InterfaceElementVec` — see that file's header comment for the Ghidra
//!   provenance.
//! - **Entity method calls** (`0x80..=0xFE`): always `WORD_LENGTH`-framed,
//!   and the payload always opens with a 4-byte `entity_id` (direct
//!   encoding) or a 4-byte `entity_id` + 1-byte `sub_index` (the `0xBD`
//!   extended-encoding marker, for method indices at or past the target's
//!   idbase). See `cimmeria_services::mercury::append_entity_method` for the
//!   producer side this mirrors.
//!
//! This decoder does not attempt semantic decode of every message body —
//! only enough structure (msg_id, entity_id, class_id, method sub-index) for
//! end-to-end tests to assert "did entity X's create/appearance/leave reach
//! this witness", which is the wire-level question `docs/architecture/wireclient.md`
//! Phase 3 (full behavior-trace decode) is scoped to solve generally. This
//! module is the minimal slice Phase 1.5/2 needs.

use bytes::Bytes;

/// One decoded message from a reassembled server→client bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S2CMessage {
    /// Raw wire msg_id byte.
    pub msg_id: u8,
    /// For entity-bearing messages (`CREATE_ENTITY`, `CREATE_BASE_PLAYER`,
    /// entity methods, avatar updates, `leaveAoI`, `entityInvisible`): the
    /// entity this message targets.
    pub entity_id: Option<u32>,
    /// `class_id` byte — present only on `CREATE_ENTITY` (0x09) and
    /// `CREATE_BASE_PLAYER` (0x05).
    pub class_id: Option<u8>,
    /// Decoded method index for an entity-method call (`0x80..=0xFE`).
    /// Direct encoding: `msg_id & 0x7F`. Extended (`0xBD`) encoding:
    /// `sub_index as u16 + idbase`, using [`cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER`]
    /// as the assumed idbase — correct for every player-ghost cascade
    /// method and wrong only for an NPC witness target, which is out of
    /// scope for the player-visibility harness this module serves.
    pub method_index: Option<u16>,
    /// The message's own payload, exactly as framed on the wire (no
    /// entity_id/sub_index stripped) — kept for callers that want to
    /// decode further (e.g. reading the wstring name out of
    /// `ON_BEING_NAME_UPDATE`).
    pub payload: Bytes,
}

impl S2CMessage {
    /// Convenience: true when this is `CREATE_ENTITY` (0x09) or
    /// `CREATE_BASE_PLAYER` (0x05) — the two messages that introduce a new
    /// entity to the client.
    pub fn is_create(&self) -> bool {
        matches!(self.msg_id, 0x05 | 0x09)
    }

    /// Convenience: true when this is `leaveAoI` (0x0C) or
    /// `entityInvisible` (0x0B) — the messages that remove/hide an entity.
    pub fn is_leave_or_hide(&self) -> bool {
        matches!(self.msg_id, 0x0B..=0x0C)
    }

    /// True for any message in the `UPDATE_AVATAR` family (0x10..=0x2F) or
    /// `detailedPosition` (0x30) — the position/orientation broadcast
    /// families. See the `aoi-witness-broadcast` agent's domain notes: NPCs
    /// use `detailedPosition`, player-controlled entities use
    /// `UPDATE_AVATAR`.
    pub fn is_position_update(&self) -> bool {
        matches!(self.msg_id, 0x10..=0x30)
    }
}

/// Msg-framing rule for the static (`0x00..=0x37`, `0xFF`) range.
#[derive(Debug, Clone, Copy)]
enum Framing {
    /// Fixed-length payload of exactly this many bytes, no length prefix.
    Constant(usize),
    /// `u16`-LE length prefix, then that many payload bytes.
    Word,
}

/// Static-range framing, from the table `cimmeria_mercury` owns (it also
/// drives the fragmenter's header guard). Entity-method ids `0x80..=0xFE`
/// are handled separately below, so they map to `None` here.
fn static_framing(msg_id: u8) -> Option<Framing> {
    use cimmeria_mercury::packet::{server_message_framing, ServerMessageFraming};
    if (0x80..=0xFE).contains(&msg_id) {
        return None;
    }
    server_message_framing(msg_id).map(|f| match f {
        ServerMessageFraming::Constant(n) => Framing::Constant(n),
        ServerMessageFraming::Word => Framing::Word,
    })
}

/// Decode a reassembled server→client bundle into its constituent
/// messages. Stops (returning what it decoded so far) on any framing it
/// cannot account for — a truncated payload, an unrecognized static
/// msg_id in the `0x38..=0x7F` gap, or a WORD_LENGTH prefix that
/// overruns the remaining bytes. Callers that need "the bundle decoded
/// cleanly" should compare `decode_bundle(&body).len()` against an
/// expected count rather than trusting silent partial decode.
pub fn decode_bundle(body: &Bytes) -> Vec<S2CMessage> {
    let mut out = Vec::new();
    let mut offset = 0usize;

    while offset < body.len() {
        let msg_id = body[offset];
        let msg_start = offset;
        offset += 1;

        if let Some(framing) = static_framing(msg_id) {
            let payload = match framing {
                Framing::Constant(len) => {
                    if offset + len > body.len() {
                        break;
                    }
                    let p = body.slice(offset..offset + len);
                    offset += len;
                    p
                }
                Framing::Word => {
                    if offset + 2 > body.len() {
                        break;
                    }
                    let len = u16::from_le_bytes([body[offset], body[offset + 1]]) as usize;
                    offset += 2;
                    if offset + len > body.len() {
                        break;
                    }
                    let p = body.slice(offset..offset + len);
                    offset += len;
                    p
                }
            };

            let (entity_id, class_id) = match msg_id {
                // createEntity: [entity_id:4][idAlias:1][class_id:1][..]
                0x09 if payload.len() >= 6 => (
                    Some(u32::from_le_bytes([
                        payload[0], payload[1], payload[2], payload[3],
                    ])),
                    Some(payload[5]),
                ),
                // createBasePlayer: [entity_id:4][class_id:1][propCount:1]
                0x05 if payload.len() >= 5 => (
                    Some(u32::from_le_bytes([
                        payload[0], payload[1], payload[2], payload[3],
                    ])),
                    Some(payload[4]),
                ),
                // avatarUpdate family, detailedPosition: entity_id is the
                // first 4 bytes of every variant (`build_avatar_update`).
                0x10..=0x30 if payload.len() >= 4 => (
                    Some(u32::from_le_bytes([
                        payload[0], payload[1], payload[2], payload[3],
                    ])),
                    None,
                ),
                // updateEntity / leaveAoI / entityInvisible: entity_id
                // leads the payload in every producer that emits them.
                0x0A..=0x0C if payload.len() >= 4 => (
                    Some(u32::from_le_bytes([
                        payload[0], payload[1], payload[2], payload[3],
                    ])),
                    None,
                ),
                _ => (None, None),
            };

            out.push(S2CMessage {
                msg_id,
                entity_id,
                class_id,
                method_index: None,
                payload,
            });
            continue;
        }

        // Entity method call: 0x80..=0xFE, always WORD_LENGTH.
        if msg_id >= 0x80 {
            if offset + 2 > body.len() {
                break;
            }
            let len = u16::from_le_bytes([body[offset], body[offset + 1]]) as usize;
            offset += 2;
            if offset + len > body.len() {
                break;
            }
            let payload = body.slice(offset..offset + len);
            offset += len;

            let (entity_id, method_index) = if msg_id == 0xBD {
                // Extended encoding: [entity_id:4][sub_index:1][args...]
                if payload.len() >= 5 {
                    let eid = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                    let sub_index = payload[4] as u16;
                    (
                        Some(eid),
                        Some(
                            sub_index
                                + u16::from(cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER),
                        ),
                    )
                } else {
                    (None, None)
                }
            } else if payload.len() >= 4 {
                // Direct encoding: [entity_id:4][args...]
                let eid = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                (Some(eid), Some(u16::from(msg_id & 0x7F)))
            } else {
                (None, None)
            };

            out.push(S2CMessage {
                msg_id,
                entity_id,
                class_id: None,
                method_index,
                payload,
            });
            continue;
        }

        // Unrecognized static msg_id in the undefined 0x38..=0x7F gap (or
        // any id this decoder's table doesn't yet cover). Nothing tells us
        // its length, so stop rather than guess and desync the rest of the
        // bundle.
        tracing::warn!(
            // nt:id-only the id is one this decoder has no entry for, so it
            // has no name either; the wire crate is a dev-dependency here.
            msg_id = format_args!("{msg_id:#04x}"),
            offset = msg_start,
            "decode_bundle: unrecognized msg_id, stopping decode of this bundle"
        );
        break;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word_msg(msg_id: u8, payload: &[u8]) -> Vec<u8> {
        let mut v = vec![msg_id];
        v.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        v.extend_from_slice(payload);
        v
    }

    #[test]
    fn decodes_create_entity_with_class_id() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&42u32.to_le_bytes()); // entity_id
        payload.push(0xFF); // idAlias
        payload.push(0x03); // class_id = SGWGmPlayer
        payload.push(0x00);
        payload.push(0x00);
        let body = Bytes::from(word_msg(0x09, &payload));
        let msgs = decode_bundle(&body);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].msg_id, 0x09);
        assert_eq!(msgs[0].entity_id, Some(42));
        assert_eq!(msgs[0].class_id, Some(0x03));
        assert!(msgs[0].is_create());
    }

    #[test]
    fn decodes_create_base_player() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&99u32.to_le_bytes());
        payload.push(0x02); // SGWPlayer
        payload.push(0x00); // propertyCount
        let body = Bytes::from(word_msg(0x05, &payload));
        let msgs = decode_bundle(&body);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].entity_id, Some(99));
        assert_eq!(msgs[0].class_id, Some(0x02));
    }

    #[test]
    fn decodes_avatar_update_constant_length() {
        let mut body = vec![0x10u8]; // NoAliasFullPosYawPitchRoll
        body.extend_from_slice(&7u32.to_le_bytes()); // entity_id
        body.extend_from_slice(&[0u8; 21]); // remaining 25-4=21 bytes
        let msgs = decode_bundle(&Bytes::from(body));
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].entity_id, Some(7));
        assert!(msgs[0].is_position_update());
    }

    #[test]
    fn decodes_leave_aoi() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&7u32.to_le_bytes());
        let body = Bytes::from(word_msg(0x0C, &payload));
        let msgs = decode_bundle(&body);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].entity_id, Some(7));
        assert!(msgs[0].is_leave_or_hide());
    }

    #[test]
    fn decodes_entity_method_direct_encoding() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&123u32.to_le_bytes());
        payload.extend_from_slice(b"hi");
        let body = Bytes::from(word_msg(0x80 | 12, &payload)); // method 12
        let msgs = decode_bundle(&body);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].entity_id, Some(123));
        assert_eq!(msgs[0].method_index, Some(12));
    }

    #[test]
    fn decodes_entity_method_extended_encoding() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&123u32.to_le_bytes());
        payload.push(5); // sub_index -> method 61 + 5 = 66
        payload.extend_from_slice(b"hi");
        let body = Bytes::from(word_msg(0xBD, &payload));
        let msgs = decode_bundle(&body);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].entity_id, Some(123));
        assert_eq!(msgs[0].method_index, Some(66));
    }

    #[test]
    fn decodes_multiple_messages_in_one_bundle() {
        // A constant-length message (updateFrequencyNotification, 0x02)
        // followed by a word-length message (createEntity, 0x09) in the
        // same bundle -- exercises that the decoder correctly resumes at
        // the right offset across a framing-style change mid-bundle.
        let mut body = vec![0x02u8, 5]; // updateFrequencyNotification, freq=5
        let mut ce_payload = Vec::new();
        ce_payload.extend_from_slice(&1u32.to_le_bytes());
        ce_payload.push(0xFF);
        ce_payload.push(0x02);
        ce_payload.push(0);
        ce_payload.push(0);
        body.extend_from_slice(&word_msg(0x09, &ce_payload));
        let msgs = decode_bundle(&Bytes::from(body));
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].msg_id, 0x02);
        assert_eq!(msgs[1].msg_id, 0x09);
        assert_eq!(msgs[1].entity_id, Some(1));
    }
}
