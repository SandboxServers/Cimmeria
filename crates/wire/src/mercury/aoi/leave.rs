//! Leave-AoI packet builders — `ENTITY_INVISIBLE (0x0B)` alone for
//! ring-transport teleport-out fades, or paired with `LEAVE_AOI (0x0C)` for
//! a full client-side delete.

use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_ACKS};

use crate::mercury::{encrypt_packet, REPLY_FLAGS};

use super::{BASEMSG_ENTITY_INVISIBLE, BASEMSG_LEAVE_AOI};

/// Build and encrypt `ENTITY_INVISIBLE (0x0B)` *alone* — temporary visual
/// hide that keeps the entity in the client's AoI bookkeeping. Used for the
/// ring-transport teleport-out fade. Matches C++ `ClientHandler::leaveAoI(id,
/// deleteEntity=false)` from `client_handler.cpp:516-528`.
///
/// To re-show, send `onVisible(1)` (entity method index 8 with arg 0x01) —
/// see `client_handler.cpp::enterAoI`.
pub fn build_entity_invisible(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    entity_id: u32,
    version: EncryptionVersion,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(8);
    body.push(BASEMSG_ENTITY_INVISIBLE);
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.push(0xFF); // idAlias = no alias

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key, version)
}

/// Build and encrypt `ENTITY_INVISIBLE (0x0B)` + `LEAVE_AOI (0x0C)` for when
/// an entity leaves a witness's Area of Interest.
///
/// Matches C++ `ClientHandler::leaveAoI(id, deleteEntity=true)` from
/// `client_handler.cpp:516-539`.
pub fn build_entity_leave(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    entity_id: u32,
    version: EncryptionVersion,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(24);

    // ENTITY_INVISIBLE (0x0B, CONSTANT_LENGTH = 5)
    body.push(BASEMSG_ENTITY_INVISIBLE);
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.push(0xFF); // idAlias = no alias

    // LEAVE_AOI (0x0C, WORD_LENGTH)
    body.push(BASEMSG_LEAVE_AOI);
    body.extend_from_slice(&8u16.to_le_bytes()); // wordLength = 8
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes()); // cacheStamp = 0

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key, version)
}
