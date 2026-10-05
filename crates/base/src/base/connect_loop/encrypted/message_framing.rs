//! Inbound client-message framing: how long each message in a client
//! bundle is, and the one payload (`requestEntityUpdate`) the bundle
//! scanner decodes itself. Split out of the encrypted-datagram dispatch.

use super::super::{read_constant_payload, read_word_length_payload};

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
pub(super) fn read_client_message_payload<'a>(
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
pub(super) fn parse_request_entity_update(payload: &[u8]) -> Vec<u32> {
    if payload.len() < 4 {
        return Vec::new();
    }
    let entity_id = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    vec![entity_id]
}
