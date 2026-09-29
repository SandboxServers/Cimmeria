//! Test-only decoding of the packets the crafting code sends to a player.
//!
//! The test sessions use the all-zero key of
//! `test_default_connected_client_state`, so a sent packet decrypts to
//! `[flags(1)][body][seq(4)]`, and the body is one entity-method call:
//! `[0x80 | index][len(2)][entity(4)][args]` below the player's idbase, or
//! `[0xBD][len(2)][entity(4)][index - idbase][args]` at or above it.

use cimmeria_mercury::channel_bundle::{EXTENDED_ENCODING_MARKER, IDBASE_SGW_PLAYER};
use cimmeria_mercury::encryption::MercuryEncryption;

/// One decoded entity-method call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MethodCall {
    pub method: u16,
    pub entity_id: u32,
    pub args: Vec<u8>,
}

pub(crate) fn decode(packet: &[u8]) -> MethodCall {
    let enc = MercuryEncryption::from_session_key([0u8; 32]);
    let pt = enc.decrypt(packet).expect("decrypt test packet");
    let body = &pt[1..pt.len() - 4];
    let entity_id = u32::from_le_bytes(body[3..7].try_into().unwrap());
    if body[0] == EXTENDED_ENCODING_MARKER {
        MethodCall {
            method: u16::from(IDBASE_SGW_PLAYER) + u16::from(body[7]),
            entity_id,
            args: body[8..].to_vec(),
        }
    } else {
        MethodCall {
            method: u16::from(body[0] & 0x7F),
            entity_id,
            args: body[7..].to_vec(),
        }
    }
}

pub(crate) fn decode_all(packets: &[Vec<u8>]) -> Vec<MethodCall> {
    packets.iter().map(|p| decode(p)).collect()
}

/// The text of a `CHAN_FEEDBACK` `onPlayerCommunication` call.
pub(crate) fn feedback_text(call: &MethodCall) -> String {
    let args = &call.args;
    let speaker_len = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
    let mut offset = 4 + speaker_len * 2 + 2;
    let text_len = u32::from_le_bytes(args[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    let units: Vec<u16> = (0..text_len)
        .map(|i| u16::from_le_bytes(args[offset + 2 * i..offset + 2 * i + 2].try_into().unwrap()))
        .collect();
    String::from_utf16_lossy(&units)
}

/// `(instance id, stack size, container, wire slot)` per `onUpdateItem`
/// entry, in wire order. An `InvItem` entry is `id, dbid, stack, slot,
/// container` (i32 each), `bound` (u8), `durability` (i32), the ammo-type
/// `ARRAY<INT32>`, then `curAmmoType` and `charges` (i32 each).
pub(crate) fn update_item_rows(call: &MethodCall) -> Vec<(i32, i32, i32, i32)> {
    let args = &call.args;
    let i32_at = |o: usize| i32::from_le_bytes(args[o..o + 4].try_into().unwrap());
    let count = i32_at(0) as usize;
    let mut rows = Vec::with_capacity(count);
    let mut offset = 4;
    for _ in 0..count {
        rows.push((
            i32_at(offset),
            i32_at(offset + 8),
            i32_at(offset + 16),
            i32_at(offset + 12),
        ));
        let ammo = i32_at(offset + 25) as usize;
        offset += 37 + 4 * ammo;
    }
    assert_eq!(offset, args.len(), "onUpdateItem arguments fully consumed");
    rows
}

/// The ids in an `onRemoveItem(ARRAY<INT32>)` call.
pub(crate) fn remove_item_ids(call: &MethodCall) -> Vec<i32> {
    let count = u32::from_le_bytes(call.args[0..4].try_into().unwrap()) as usize;
    assert_eq!(call.args.len(), 4 + 4 * count);
    (0..count)
        .map(|k| i32::from_le_bytes(call.args[4 + 4 * k..8 + 4 * k].try_into().unwrap()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mercury::{build_player_entity_method_packet, method_idx};
    use cimmeria_mercury::encryption::EncryptionVersion;

    fn build(method: u16, args: &[u8]) -> Vec<u8> {
        build_player_entity_method_packet(
            &[0u8; 32],
            3,
            &[],
            0x0A0B_0C0D,
            method,
            args,
            EncryptionVersion::V1,
        )
    }

    /// Below the player's idbase the index rides the message id.
    #[test]
    fn decodes_a_direct_encoded_call() {
        let call = decode(&build(12, &[1, 2, 3]));
        assert_eq!(
            call,
            MethodCall {
                method: 12,
                entity_id: 0x0A0B_0C0D,
                args: vec![1, 2, 3]
            }
        );
    }

    /// At or above it the index rides the sub-index byte.
    #[test]
    fn decodes_an_extended_encoded_call() {
        let call = decode(&build(method_idx::ON_UPDATE_DISCIPLINE, &[9, 8]));
        assert_eq!(call.method, method_idx::ON_UPDATE_DISCIPLINE);
        assert_eq!(call.entity_id, 0x0A0B_0C0D);
        assert_eq!(call.args, vec![9, 8]);
    }
}
