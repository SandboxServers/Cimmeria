//! GM travel handlers — `gmGotoXYZ` (same-space snap), `gmGotoLocation`
//! (cross-world reload), and `gmDHD` (stargate dial).
//!
//! `gmGotoXYZ` is the verified same-space teleport. `gmGotoLocation`
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
/// The `GateTravel` enqueue is confirmed **before** the local teardown: if the
/// channel is closed the entity is left in place rather than removed with no
/// transfer. The world name is GM-typed and not validated against the space
/// table here — an unknown world is the base side's concern (this is GM-only).
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
    // Enqueue the transfer first; only tear the entity out of the space once the
    // send is confirmed. A closed channel must not leave the GM removed locally
    // with no transfer in flight.
    if tx
        .send(CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name: world_name,
            position,
            rotation: [0.0; 3],
            destination_ring_id: None,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            entity_id,
            "gmGotoLocation: GateTravel enqueue failed; entity left in place"
        );
        return true;
    }
    space_mgr.destroy_entity(entity_id);
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

/// `gmGoto(WSTRING aNameOrID)` — teleport the calling GM to the target entity's
/// position. Numeric-id form only (name→id resolution isn't wired in the cell);
/// same-space only (cross-space requires `gmGotoLocation`).
pub(super) async fn handle_goto(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(target_eid) = parse_target_id(entity_id, args, "gmGoto") else {
        return true;
    };
    let caller_space = space_mgr.get_entity(entity_id).map(|e| e.space_id.0);
    let dest = match space_mgr.get_entity(target_eid) {
        Some(t) if Some(t.space_id.0) == caller_space => [t.position.x, t.position.y, t.position.z],
        Some(_) => {
            tracing::warn!(
                entity_id,
                target_eid,
                "gmGoto: target in a different space — refused"
            );
            return true;
        }
        None => {
            tracing::warn!(entity_id, target_eid, "gmGoto: target not found");
            return true;
        }
    };
    let (space_id, prev_pos) = match space_mgr.get_entity(entity_id) {
        Some(e) => (
            e.space_id.0 as u32,
            [e.position.x, e.position.y, e.position.z],
        ),
        None => return true,
    };
    tracing::info!(
        entity_id,
        target_eid,
        ?dest,
        "gmGoto: teleporting GM to target"
    );
    space_mgr.update_entity_position(entity_id, dest, [0, 0, 0], [0.0; 3]);
    let _ = tx
        .send(CellToBaseMsg::TeleportPlayer {
            entity_id,
            space_id,
            position: dest,
            prev_pos,
        })
        .await;
    true
}

/// `gmSummon(WSTRING aNameOrID)` — move the target entity to the caller's
/// position. A player target is snapped via `TeleportPlayer`; an NPC via the
/// spatial-grid update (witnesses pick it up on the next AoI tick). Numeric-id
/// form, same-space only.
pub(super) async fn handle_summon(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(target_eid) = parse_target_id(entity_id, args, "gmSummon") else {
        return true;
    };
    if target_eid == entity_id {
        tracing::warn!(entity_id, "gmSummon: cannot summon yourself");
        return true;
    }
    let (caller_space, caller_pos) = match space_mgr.get_entity(entity_id) {
        Some(e) => (e.space_id.0, [e.position.x, e.position.y, e.position.z]),
        None => return true,
    };
    let (is_player, target_prev) = match space_mgr.get_entity(target_eid) {
        Some(t) if t.space_id.0 == caller_space => {
            (t.is_player, [t.position.x, t.position.y, t.position.z])
        }
        Some(_) => {
            tracing::warn!(
                entity_id,
                target_eid,
                "gmSummon: target in a different space — refused"
            );
            return true;
        }
        None => {
            tracing::warn!(entity_id, target_eid, "gmSummon: target not found");
            return true;
        }
    };
    tracing::info!(
        entity_id,
        target_eid,
        is_player,
        "gmSummon: moving target to caller"
    );
    space_mgr.update_entity_position(target_eid, caller_pos, [0, 0, 0], [0.0; 3]);
    if is_player {
        // Players need the authoritative forced-position snap to their client.
        let _ = tx
            .send(CellToBaseMsg::TeleportPlayer {
                entity_id: target_eid,
                space_id: caller_space as u32,
                position: caller_pos,
                prev_pos: target_prev,
            })
            .await;
    }
    true
}

/// Parse a leading `WSTRING aNameOrID` as a positive numeric entity id.
/// Returns `None` (after a warn) on malformed/non-numeric input.
fn parse_target_id(entity_id: u32, args: &[u8], cmd: &str) -> Option<u32> {
    let name_or_id = match read_wstring(args, 0) {
        Ok((s, _)) => s,
        Err(e) => {
            tracing::warn!(entity_id, error = %e, cmd, "GM goto/summon: malformed NameOrID WSTRING");
            return None;
        }
    };
    match name_or_id.trim().parse::<u32>() {
        Ok(id) if id > 0 => Some(id),
        _ => {
            tracing::warn!(
                entity_id,
                name_or_id = %name_or_id,
                cmd,
                "GM goto/summon: NameOrID is not a positive numeric id — name resolution not wired in the cell"
            );
            None
        }
    }
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
