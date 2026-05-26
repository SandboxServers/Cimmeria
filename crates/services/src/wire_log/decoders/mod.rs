//! Per-method schema decoders.
//!
//! # The contract
//!
//! Each decoder takes raw arg bytes and returns a [`serde_json::Value`]
//! (typically `Value::Object`) with named fields per the method's wire
//! schema. Decoders are **best-effort**: a parse failure returns
//! `None` and the wire-log capture falls back to `args_hex` only —
//! never panics, never loses the event.
//!
//! # Phase 1 coverage (this PR)
//!
//! Ten priority decoders covering the methods that immediately matter
//! for active debugging:
//!
//! * Outbound: `BeingAppearance`, `onSequence`, `onDialogDisplay`,
//!   `onEffectResults`, `onStateFieldUpdate`, `onStatUpdate`,
//!   `onTimerUpdate`
//! * Inbound: `avatarUpdateExplicit`, `requestActiveSlotChange`,
//!   `useAbility`
//!
//! # Phase 2 plan (deferred)
//!
//! Remaining ~240 method decoders. The right approach is a small
//! codegen tool that parses the canonical schema strings in
//! `docs/protocol/client-method-dispatch-table.md` (and the
//! `.def` files under `entities/defs/`) and emits decoders. Hand-
//! writing all 240 would take ~40 hours; codegen is ~6-hour build
//! plus per-method tuning. Tracked as a separate work stream.

mod generated;
mod inbound;
mod outbound;
mod primitives;

/// Decode an outbound entity-method payload. Returns `None` when no
/// decoder is registered for the method index or the parse fails.
///
/// Lookup order:
///   1. Hand-written decoders in [`outbound`] — overrides codegen for
///      methods where we want richer semantic fields (BSF bits on
///      onStateFieldUpdate, etc.).
///   2. Codegen decoders in [`generated`] — covers ~120 methods with
///      pure-primitive schemas, parsed from the dispatch table doc.
///   3. `None` — caller falls back to the always-on `args_hex` field.
pub fn decode_outbound(method_index: u16, args: &[u8]) -> Option<serde_json::Value> {
    outbound::decode(method_index, args).or_else(|| generated::decode_generated(method_index, args))
}

/// Decode an inbound bundle message payload. Returns `None` when no
/// decoder is registered for the message id or the parse fails.
pub fn decode_inbound(msg_id: u8, payload: &[u8]) -> Option<serde_json::Value> {
    inbound::decode(msg_id, payload)
}
