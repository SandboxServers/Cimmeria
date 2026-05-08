//! Single entity-method-call packet builder — used by CellService to push
//! an `onTimerUpdate`, `onEffectResults`, etc. to one client.

use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_ACKS};

use crate::mercury::{append_entity_method, encrypt_packet, REPLY_FLAGS};

/// Build and encrypt a single server→client entity method call.
///
/// Used by CellService to send entity method calls (e.g., `onTimerUpdate`,
/// `onEffectResults`) to a specific client.
pub fn build_entity_method_packet(
    key: &[u8; 32],
    seq: u32,
    acks: &[u32],
    entity_id: u32,
    method_index: u16,
    args: &[u8],
) -> Vec<u8> {
    let mut body = Vec::with_capacity(32 + args.len());
    append_entity_method(&mut body, method_index, entity_id, args);
    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq), acks, None);
    encrypt_packet(&plaintext, key)
}
