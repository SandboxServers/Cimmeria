//! Resource-cache builders: cooked-data fragments and version-info responses.
//!
//! These power the ClientCache: the client requests resource categories
//! (CharDefs, item defs, etc.) and the server streams XML fragments back.

use cimmeria_mercury::packet::FLAG_HAS_ACKS;

use super::{encrypt_packet, BASEMSG_ON_VERSION_INFO, BASEMSG_RESOURCE_FRAGMENT, REPLY_FLAGS};

/// Build and encrypt a `BASEMSG_RESOURCE_FRAGMENT` (0x36).
///
/// Wire format (VARIABLE_LENGTH_MESSAGE):
/// ```text
/// [dataId: u16]     — increments per resource transfer
/// [chunkId: u8]     — 0, 1, 2, ... fragment sequence
/// [flags: u8]       — 0x41=first, 0x40=middle, 0x42=last, 0x43=first+last
/// [msgType: u8]     — 0 (MESSAGE_CacheData), only in FIRST fragment
/// [categoryId: u32] — e.g. 7 (char_creation), only in FIRST fragment
/// [elementId: u32]  — e.g. CharDefId (1-23), only in FIRST fragment
/// [xmlBody: bytes]  — raw UTF-8 XML chunk
/// ```
pub fn build_resource_fragment(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    data_id: u16,
    chunk_id: u8,
    frag_flags: u8,
    msg_type: Option<u8>,
    category_id: Option<u32>,
    element_id: Option<u32>,
    xml_chunk: &[u8],
) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    let mut payload = Vec::with_capacity(64 + xml_chunk.len());

    // Fixed header
    payload.extend_from_slice(&data_id.to_le_bytes());
    payload.push(chunk_id);
    payload.push(frag_flags);

    // First-fragment-only fields
    if let Some(mt) = msg_type {
        payload.push(mt);
    }
    if let Some(cat) = category_id {
        payload.extend_from_slice(&cat.to_le_bytes());
    }
    if let Some(elem) = element_id {
        payload.extend_from_slice(&elem.to_le_bytes());
    }

    // XML data
    payload.extend_from_slice(xml_chunk);

    let mut body = Vec::with_capacity(3 + payload.len());
    body.push(BASEMSG_RESOURCE_FRAGMENT);
    // WORD_LENGTH message: u16 length prefix (matches messages.cpp:373)
    body.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    body.extend_from_slice(&payload);

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt `onVersionInfo` (0x80) for a ClientCache method call.
///
/// Tells the client about the current version of a resource category.
///
/// `invalidate_all = true` makes the client drop and re-request the entire
/// category. `invalidate_all = false` + a non-empty `invalid_keys` slice
/// scopes the invalidation to just those element IDs — the client drops
/// only those entries from its local cache and sends `elementDataRequest`
/// for each, leaving the rest of its local PAK untouched. The client side
/// of this branch lives in `ServerConnection::onVersionInfo` and was
/// confirmed to parse `InvalidKeys` as a `PropertyList<long>` and per-key
/// invalidate via the cache element's destructor.
pub fn build_version_info(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    category_id: u32,
    version: u32,
    required_updates: u32,
    invalidate_all: bool,
    invalid_keys: &[u32],
    account_entity_id: u32,
) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    let mut payload = Vec::with_capacity(32 + invalid_keys.len() * 4);
    payload.extend_from_slice(&account_entity_id.to_le_bytes());
    payload.extend_from_slice(&category_id.to_le_bytes());
    payload.extend_from_slice(&version.to_le_bytes());
    payload.extend_from_slice(&required_updates.to_le_bytes());
    payload.push(if invalidate_all { 1 } else { 0 });
    // invalidKeys = ARRAY<u32> { count, entries... }
    payload.extend_from_slice(&(invalid_keys.len() as u32).to_le_bytes());
    for &k in invalid_keys {
        payload.extend_from_slice(&k.to_le_bytes());
    }

    let mut body = Vec::with_capacity(4 + payload.len());
    body.push(BASEMSG_ON_VERSION_INFO);
    body.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    body.extend_from_slice(&payload);

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}
