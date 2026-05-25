//! Single entity-method-call packet builder — used by CellService to push
//! an `onTimerUpdate`, `onEffectResults`, etc. to one client.

use cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER;
use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_ACKS};

use crate::mercury::{append_entity_method, encrypt_packet, REPLY_FLAGS};

/// Build and encrypt a single server→client entity method call.
///
/// `idbase` is the per-entity sub-slot threshold for `entity_id`'s class —
/// see [`cimmeria_mercury::channel_bundle::idbase_from_exposed_method_count`].
/// Pass [`cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER`] when the
/// target is the connected player; pass
/// [`cimmeria_mercury::channel_bundle::IDBASE_NPC_DEFAULT`] for any
/// non-player entity in the current SGW schema. For the common case where
/// the target is the connected player, prefer
/// [`build_player_entity_method_packet`].
pub fn build_entity_method_packet(
    key: &[u8; 32],
    seq: u32,
    acks: &[u32],
    entity_id: u32,
    method_index: u16,
    idbase: u8,
    args: &[u8],
) -> Vec<u8> {
    let mut body = Vec::with_capacity(32 + args.len());
    append_entity_method(&mut body, method_index, idbase, entity_id, args);
    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq), acks, None);
    encrypt_packet(&plaintext, key)
}

/// SGWPlayer-target convenience wrapper for [`build_entity_method_packet`].
/// Equivalent to passing `IDBASE_SGW_PLAYER` (`61`).
pub fn build_player_entity_method_packet(
    key: &[u8; 32],
    seq: u32,
    acks: &[u32],
    entity_id: u32,
    method_index: u16,
    args: &[u8],
) -> Vec<u8> {
    build_entity_method_packet(
        key,
        seq,
        acks,
        entity_id,
        method_index,
        IDBASE_SGW_PLAYER,
        args,
    )
}
