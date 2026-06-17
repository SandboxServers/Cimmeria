//! GM travel handlers — `gmGotoXYZ` (same-space snap), `gmGotoLocation`
//! (cross-world reload), and `gmDHD` (stargate dial).
//!
//! `gmGotoXYZ` is the verified same-space teleport (#518). `gmGotoLocation`
//! reuses the cross-world `GateTravel` path, and `gmDHD` reuses the canonical
//! [`crate::cell::gate_travel::handle_dial_gate`] primitive.

use tokio::sync::mpsc;

use crate::cell::gate_travel::handle_dial_gate;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::read_wstring;

/// `gmGotoXYZ(FLOAT aX, FLOAT aY, FLOAT aZ)` — teleport the calling GM to the
/// coordinate within their current space.
///
/// Mirrors the same-space content teleport: capture the prior position as the
/// forced-position reference vector, update the spatial grid, then route the
/// authoritative `FORCED_POSITION` snap through `TeleportPlayer`. Witnesses
/// pick up the move on the next AoI tick.
pub(super) async fn handle_goto_xyz(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(position) = read_xyz(args, 0) else {
        tracing::warn!(
            entity_id,
            args_len = args.len(),
            "gmGotoXYZ: truncated args (need 3×FLOAT = 12 bytes)"
        );
        return true;
    };

    // Reject non-finite coordinates — a NaN/inf snap corrupts the spatial grid
    // and the wire FORCED_POSITION. (A modified client / replayed packet could
    // supply garbage even though it's a GM channel.)
    if !position.iter().all(|c| c.is_finite()) {
        tracing::warn!(
            entity_id,
            ?position,
            "gmGotoXYZ: non-finite coordinate rejected"
        );
        return true;
    }

    // Resolve the caller's space + prior position. Missing entity → no-op.
    let (space_id, prev_pos) = match space_mgr.get_entity(entity_id) {
        Some(e) => (
            e.space_id.0 as u32,
            [e.position.x, e.position.y, e.position.z],
        ),
        None => {
            tracing::warn!(entity_id, "gmGotoXYZ: caller entity not found");
            return true;
        }
    };

    tracing::info!(entity_id, ?position, space_id, "gmGotoXYZ: teleporting GM");

    // Keep the spatial grid consistent first (writes cell_entity.position),
    // then send the authoritative snap.
    space_mgr.update_entity_position(entity_id, position, [0, 0, 0], [0.0; 3]);
    let _ = tx
        .send(CellToBaseMsg::TeleportPlayer {
            entity_id,
            space_id,
            position,
            prev_pos,
        })
        .await;
    true
}

/// `gmGotoLocation(WSTRING aWorldName, FLOAT aX, aY, aZ)` — cross-world
/// teleport the calling GM.
///
/// Unlike `gmGotoXYZ` (an in-space snap), this names a destination world and so
/// goes through the heavier `GateTravel` path: the cell tears the entity out of
/// its current space and the base re-runs the world-entry handshake in the
/// destination world. (If the GM passes their *current* world, this still does
/// a full reload — use `gmGotoXYZ` for a same-space hop.)
///
/// Caveat: the world name is GM-typed and not validated against the space table
/// here — an unknown world is the base side's concern. Since the entity is
/// destroyed before the `GateTravel` is sent (mirroring the stargate dial
/// path), a typo can strand the GM's session; this is GM-only and matches the
/// existing `handle_dial_gate` behavior.
pub(super) async fn handle_goto_location(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let (world_name, consumed) = match read_wstring(args, 0) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(entity_id, error = %e, "gmGotoLocation: malformed WorldName WSTRING");
            return true;
        }
    };
    let Some(position) = read_xyz(args, consumed) else {
        tracing::warn!(
            entity_id,
            args_len = args.len(),
            "gmGotoLocation: truncated args (need WSTRING + 3×FLOAT)"
        );
        return true;
    };
    if world_name.trim().is_empty() {
        tracing::warn!(entity_id, "gmGotoLocation: empty world name rejected");
        return true;
    }
    if !position.iter().all(|c| c.is_finite()) {
        tracing::warn!(
            entity_id,
            ?position,
            "gmGotoLocation: non-finite coordinate rejected"
        );
        return true;
    }
    if space_mgr.get_entity(entity_id).is_none() {
        tracing::warn!(entity_id, "gmGotoLocation: caller entity not found");
        return true;
    }

    tracing::info!(entity_id, %world_name, ?position, "gmGotoLocation: cross-world teleport via GateTravel");
    space_mgr.destroy_entity(entity_id);
    let _ = tx
        .send(CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name: world_name,
            position,
            rotation: [0.0; 3],
            destination_ring_id: None,
        })
        .await;
    true
}

/// `gmDHD(INT8 aGateAddress)` — dial a stargate by numeric address.
///
/// Address `0` is the def's "request the address list" sentinel; listing needs
/// a client feedback channel that isn't wired here yet, so a `0` address is a
/// no-op + warn. Any other address reuses [`handle_dial_gate`], which validates
/// the address against the cached stargate table and routes the cross-world
/// `GateTravel`.
pub(super) async fn handle_dhd(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let gate_addr = match args.first() {
        Some(&b) => b as i8, // INT8 (signed)
        None => {
            tracing::warn!(entity_id, "gmDHD: truncated args (need INT8 aGateAddress)");
            return true;
        }
    };
    if gate_addr <= 0 {
        tracing::warn!(
            entity_id,
            gate_addr,
            "gmDHD: address <= 0 (list request) unsupported — needs a client feedback channel"
        );
        return true;
    }
    tracing::info!(entity_id, gate_addr, "gmDHD: dialing stargate");
    // source address is unused by the primitive.
    handle_dial_gate(entity_id, i32::from(gate_addr), 0, tx, space_mgr).await;
    true
}

/// Read three consecutive little-endian `f32`s at `offset`, or `None` if the
/// slice is too short.
fn read_xyz(args: &[u8], offset: usize) -> Option<[f32; 3]> {
    let b = args.get(offset..offset + 12)?;
    Some([
        f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        f32::from_le_bytes([b[4], b[5], b[6], b[7]]),
        f32::from_le_bytes([b[8], b[9], b[10], b[11]]),
    ])
}
