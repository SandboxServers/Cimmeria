//! Wire-packet decode stream — every inbound and outbound Mercury
//! method goes through this module on its way to SigNoz, with the
//! method-name lookup and (when registered) the per-method schema
//! decoder attached.
//!
//! # Why this exists
//!
//! `mercury.packet` events (in [`crate::mercury` instrumentation]) give
//! per-UDP-datagram metadata: direction, peer, length. That's enough
//! to answer "did the network move bytes?" but not "what was IN the
//! bundle?" — which is the question every wire-format bug starts with.
//!
//! This module adds the layer above: per-message-in-bundle capture
//! with `msg_id`, human-readable `msg_name`, raw args hex (truncated),
//! and a structured `decoded` JSON field when a schema decoder is
//! registered for the method.
//!
//! # Three tiers of fidelity
//!
//! 1. **Always-on**: every message logs `msg_id` + `msg_name` + `args_len`
//!    + `args_hex` (first 256 bytes). Answers "what method was called?"
//! 2. **Schema-decoded** (when a decoder is registered): adds a
//!    structured `decoded` JSON field with named fields. Answers "what
//!    was inside the method call?"
//! 3. **Full coverage** (Phase 2 follow-up): every method index in the
//!    protocol has a decoder. Right now we have ~10 priority decoders
//!    hand-written; the rest fall through to `args_hex` only.
//!
//! # Capture points
//!
//! * **Inbound** — [`log_inbound`] is called from the bundle scanner
//!   in `crates/services/src/base/connect_loop/encrypted/mod.rs` after
//!   `read_client_message_payload` extracts each message's payload.
//! * **Outbound** — [`log_outbound_entity_method`] is called from the
//!   `CellToBaseMsg::EntityMethodCall` and `WitnessEntityMethod` recv
//!   sites in `crates/services/src/base/world_entry/cell_dispatch/`.
//!
//! # SigNoz targets
//!
//! Inbound events use `target: "wire.in"`. Outbound use `target:
//! "wire.out"`. Both ship at info level. The OTLP filter in
//! `crates/server/src/main.rs` whitelists both targets explicitly
//! because they don't match the `cimmeria_services` prefix rule.

use std::net::SocketAddr;

pub mod client_names;
pub mod decoders;

/// Maximum bytes of `args_hex` to record per event. Caps the per-event
/// size at ~512 hex chars + structured fields → ~1 KB/event. Larger
/// payloads (resource fragments, char list bursts) get truncated; the
/// `args_len` field is the source of truth for actual size.
const HEX_DUMP_CAP: usize = 256;

/// Record an inbound message from a client bundle.
///
/// Called from the bundle scanner once per message in a decrypted
/// packet body. Most messages are entity-method calls; system messages
/// like `AUTHENTICATE` (0x01) and `ENABLE_ENTITIES` (0x08) pass
/// through this helper too.
pub fn log_inbound(peer: SocketAddr, msg_id: u8, payload: &[u8]) {
    let msg_name = client_names::inbound_msg_name(msg_id);
    let args_len = payload.len();
    let args_hex = hex_truncate(payload);
    let decoded = decoders::decode_inbound(msg_id, payload);
    let decoded_str = decoded.as_ref().map(serde_json::Value::to_string);

    tracing::info!(
        target: "wire.in",
        msg_id,
        msg_name,
        args_len,
        truncated = args_len > HEX_DUMP_CAP,
        args_hex = %args_hex,
        decoded = decoded_str.as_deref().unwrap_or(""),
        peer = %peer,
        "wire_inbound"
    );
}

/// Record an outbound entity-method call (the cell-to-base bridge).
///
/// `witness_id` is the player who will receive the message; for
/// self-targeted methods this equals `target_entity_id`. For AoI
/// fanout (`WitnessEntityMethod`), the witness is the observer and
/// the entity_id is the entity whose method is being broadcast.
pub fn log_outbound_entity_method(
    witness_id: u32,
    target_entity_id: u32,
    method_index: u16,
    args: &[u8],
) {
    let msg_name = client_names::outbound_method_name(method_index);
    let args_len = args.len();
    let args_hex = hex_truncate(args);
    let decoded = decoders::decode_outbound(method_index, args);
    let decoded_str = decoded.as_ref().map(serde_json::Value::to_string);

    tracing::info!(
        target: "wire.out",
        method_index,
        msg_name,
        args_len,
        truncated = args_len > HEX_DUMP_CAP,
        args_hex = %args_hex,
        decoded = decoded_str.as_deref().unwrap_or(""),
        witness_id,
        target_entity_id,
        "wire_outbound"
    );
}

/// Hex-encode the first [`HEX_DUMP_CAP`] bytes of `bytes`. Inline
/// encoder — adding the `hex` crate dep just for this would be
/// disproportionate to the use.
fn hex_truncate(bytes: &[u8]) -> String {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let len = bytes.len().min(HEX_DUMP_CAP);
    let mut out = String::with_capacity(len * 2);
    for &b in &bytes[..len] {
        out.push(HEX_CHARS[(b >> 4) as usize] as char);
        out.push(HEX_CHARS[(b & 0x0F) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_truncate_caps_long_payloads() {
        let bytes = vec![0xAB; 1024];
        let hex = hex_truncate(&bytes);
        assert_eq!(
            hex.len(),
            HEX_DUMP_CAP * 2,
            "should cap at HEX_DUMP_CAP bytes"
        );
    }

    #[test]
    fn hex_truncate_short_payload_complete() {
        let bytes = vec![0x12, 0x34, 0x56];
        assert_eq!(hex_truncate(&bytes), "123456");
    }

    #[test]
    fn hex_truncate_empty_is_empty() {
        assert_eq!(hex_truncate(&[]), "");
    }
}
