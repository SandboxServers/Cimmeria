//! Inbound (client → server) message decoders — Phase 1 priority set.
//!
//! Only handles the system messages with fixed wire formats. Entity-
//! method calls (msg_id 0xC0+) carry args sized by their WORD_LENGTH
//! prefix; the bundle scanner reads those and passes the payload here
//! by their effective method index. Most rich decoding for those will
//! land in Phase 2.

use serde_json::{json, Value};

use super::primitives::Cursor;

pub fn decode(msg_id: u8, payload: &[u8]) -> Option<Value> {
    match msg_id {
        0x03 => decode_avatar_update_explicit(payload),
        // SGWPlayer client method 70 — `onActiveSlotUpdate` is OUTBOUND,
        // but the wire-symmetric inbound `requestActiveSlotChange` is
        // a cell-method call (msg_id encoded via extended dispatch at
        // 0xBD/subbyte). It arrives via `dispatch_cell_method`, not
        // through the system-message slot. Same shape applies to
        // `useAbility`.
        //
        // System-message decoders only.
        _ => None,
    }
}

/// 0x03 `avatarUpdateExplicit` — client movement update. Wire:
/// `INT32 spaceId, INT32 vehicleId, VECTOR3 pos, VECTOR3 vel, INT8x3
/// dir, UINT8 flags, UINT8x3 cells, UINT8 updateId` = 40 bytes.
fn decode_avatar_update_explicit(payload: &[u8]) -> Option<Value> {
    if payload.len() < 40 {
        return None;
    }
    let mut c = Cursor::new(payload);
    let space_id = c.i32_le()?;
    let vehicle_id = c.i32_le()?;
    let pos = [c.f32_le()?, c.f32_le()?, c.f32_le()?];
    let vel = [c.f32_le()?, c.f32_le()?, c.f32_le()?];
    let dir = [c.i8()?, c.i8()?, c.i8()?];
    let flags = c.u8()?;
    let cells = [c.u8()?, c.u8()?, c.u8()?];
    let update_id = c.u8()?;
    Some(json!({
        "space_id": space_id,
        "vehicle_id": vehicle_id,
        "pos": pos,
        "vel": vel,
        "dir": dir,
        "flags": flags,
        "cells": cells,
        "update_id": update_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_update_explicit_decodes_position() {
        // space=1, vehicle=0, pos=(1.0, 2.0, 3.0), zeros for the rest
        let mut payload = Vec::new();
        payload.extend_from_slice(&1i32.to_le_bytes());
        payload.extend_from_slice(&0i32.to_le_bytes());
        payload.extend_from_slice(&1.0f32.to_le_bytes());
        payload.extend_from_slice(&2.0f32.to_le_bytes());
        payload.extend_from_slice(&3.0f32.to_le_bytes());
        payload.extend(std::iter::repeat_n(0u8, 40 - 20));

        let decoded = decode_avatar_update_explicit(&payload).unwrap();
        assert_eq!(decoded["space_id"], 1);
        assert_eq!(decoded["pos"][0], 1.0);
        assert_eq!(decoded["pos"][1], 2.0);
        assert_eq!(decoded["pos"][2], 3.0);
    }

    #[test]
    fn truncated_avatar_update_returns_none() {
        assert!(decode_avatar_update_explicit(&[0u8; 20]).is_none());
    }
}
