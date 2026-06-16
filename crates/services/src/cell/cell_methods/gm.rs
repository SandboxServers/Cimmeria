//! SGWGmPlayer own CellMethods — the gm*/debug command tail that appends at
//! flattened cell-method index 109+ (#473 / CAT-N-04).
//!
//! SGWGmPlayer declares `<Parent>SGWPlayer</Parent>` with empty
//! `<Implements>`, so its own `<Exposed/>` CellMethods land AFTER SGWPlayer's
//! own block (0-108) and the inherited indices do not renumber. Every index
//! in this range is a GM/debug command by construction, so the whole tail is
//! authorization-gated upstream in
//! [`crate::cell::dispatch::gm_gate`] (`requires_gm` returns `true` for any
//! `index >= 109`). By the time this dispatch runs, the caller is already
//! confirmed `access_level >= GameMaster` — these handlers do NOT re-check.
//!
//! Only a verified subset is implemented; every other 109+ index falls
//! through the router's auth-gated `warn!` arm (harmless — the call was
//! already authorized, it just has no handler yet). Each implemented index is
//! pinned to a constant and the constant is asserted against the
//! document-order count of `<Exposed/>` methods in `SGWGmPlayer.def`
//! (`tests::gm_indices_match_def_document_order`).

use cimmeria_entity::inventory::INV_MAIN;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::read_wstring;

// ── Flattened SGWGmPlayer cell-method indices ────────────────────────────────
//
// Counted from SGWGmPlayer.def in document order, starting at 109 for the
// first own `<Exposed/>` method (gmMissionAssign). The full derivation is in
// the bigworld-engine-advisor memo and re-asserted in the tests below; the
// def line references let a reviewer re-count without re-running the script.

/// `gmGiveItem(WSTRING DesignId, INT32 Quantity)` — def line 185. Count: the
/// 25th exposed method (109 + 24). Gives the item to the GM's own inventory.
pub const GM_GIVE_ITEM: u16 = 133;

/// `gmGotoXYZ(FLOAT aX, FLOAT aY, FLOAT aZ)` — def line 348. Count: the 55th
/// exposed method (109 + 54). Teleports the GM to the coordinate in-space.
pub const GM_GOTO_XYZ: u16 = 163;

/// `gmKillTarget(INT64 TargetId)` — def line 482. Count: the 82nd exposed
/// method (109 + 81). Kills an NPC (refuses player targets).
pub const GM_KILL_TARGET: u16 = 190;

/// Upper bound on a single `gmGiveItem` grant. The audit required the chat-
/// command path clamp quantity; the native path applies the same rule so a
/// fat-fingered (or replayed) `gmGiveItem("x", 2000000000)` can't blow up the
/// inventory / DB. 1000 is generous for any legitimate GM use.
const GM_GIVE_ITEM_MAX_QTY: i32 = 1000;

/// Dispatch an SGWGmPlayer own cell method (flattened index >= 109).
///
/// Returns `true` if the index was handled, `false` if it's an unimplemented
/// 109+ index (so the router falls through to its already-authorized
/// `warn!` arm). The caller is guaranteed GM by the dispatch-layer gate.
pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        GM_GIVE_ITEM => {
            handle_gm_give_item(entity_id, args, tx, space_mgr).await;
            true
        }
        GM_GOTO_XYZ => {
            handle_gm_goto_xyz(entity_id, args, tx, space_mgr).await;
            true
        }
        GM_KILL_TARGET => {
            handle_gm_kill_target(entity_id, args, tx, space_mgr).await;
            true
        }
        // Any other 109+ index is an unimplemented (but authorized) gm*
        // method — let the router fall through to its warn arm.
        _ => false,
    }
}

/// `gmGiveItem(WSTRING DesignId, INT32 Quantity)` — grant the item to the
/// calling GM's own inventory.
///
/// `DesignId` is a WSTRING in the def. In the original GM console it can be a
/// numeric design id or an internal item name; we only resolve the numeric
/// form here (the name→type_id table isn't wired into the cell). A non-numeric
/// id is rejected with a warn rather than guessed. Quantity is clamped to
/// `[1, GM_GIVE_ITEM_MAX_QTY]`.
async fn handle_gm_give_item(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let (design_id_str, consumed) = match read_wstring(args, 0) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(entity_id, error = %e, "gmGiveItem: malformed DesignId WSTRING");
            return;
        }
    };
    if args.len() < consumed + 4 {
        tracing::warn!(
            entity_id,
            args_len = args.len(),
            "gmGiveItem: truncated args (missing INT32 Quantity)"
        );
        return;
    }
    let quantity = i32::from_le_bytes([
        args[consumed],
        args[consumed + 1],
        args[consumed + 2],
        args[consumed + 3],
    ]);

    // DesignId resolution: numeric form only. SGW design ids are positive
    // integers; reject anything that isn't a positive i32 rather than grant a
    // bogus item_id.
    let type_id = match design_id_str.trim().parse::<i32>() {
        Ok(id) if id > 0 => id,
        _ => {
            tracing::warn!(
                entity_id,
                design_id = %design_id_str,
                "gmGiveItem: DesignId is not a positive numeric design id — \
                 internal-name resolution is not wired in the cell; rejecting"
            );
            return;
        }
    };

    // Clamp quantity. Reject < 1 outright (a zero/negative grant is a no-op
    // at best, a stack-underflow footgun at worst).
    if quantity < 1 {
        tracing::warn!(entity_id, quantity, "gmGiveItem: quantity < 1 rejected");
        return;
    }
    let count = quantity.min(GM_GIVE_ITEM_MAX_QTY);

    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(pid) => pid,
        None => {
            tracing::warn!(
                entity_id,
                "gmGiveItem: caller has no player_id (not a player entity)"
            );
            return;
        }
    };

    tracing::info!(
        entity_id,
        player_id,
        type_id,
        count,
        requested_qty = quantity,
        "gmGiveItem: granting item to GM"
    );

    // Reuse the canonical grant primitive — base persists to sgw_inventory and
    // emits onUpdateItem. Container defaults to INV_Main; the base side
    // re-homes weapons/ammo to the bandolier via the item_containers cache.
    let _ = tx
        .send(CellToBaseMsg::GrantItem {
            entity_id,
            player_id,
            item_id: type_id,
            container_id: INV_MAIN,
            count,
        })
        .await;
}

/// `gmGotoXYZ(FLOAT aX, FLOAT aY, FLOAT aZ)` — teleport the calling GM to the
/// coordinate within their current space.
///
/// Mirrors the same-space content teleport
/// ([`crate::cell::content::executor`]): capture the prior position as the
/// forced-position reference vector, update the spatial grid, then route the
/// authoritative `FORCED_POSITION` snap through `TeleportPlayer`. Witnesses
/// pick up the move on the next AoI tick.
async fn handle_gm_goto_xyz(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if args.len() < 12 {
        tracing::warn!(
            entity_id,
            args_len = args.len(),
            "gmGotoXYZ: truncated args (need 3×FLOAT = 12 bytes)"
        );
        return;
    }
    let x = f32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    let y = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
    let z = f32::from_le_bytes([args[8], args[9], args[10], args[11]]);
    let position = [x, y, z];

    // Reject non-finite coordinates — a NaN/inf snap corrupts the spatial grid
    // and the wire FORCED_POSITION. (A modified client / replayed packet could
    // supply garbage even though it's a GM channel.)
    if !position.iter().all(|c| c.is_finite()) {
        tracing::warn!(
            entity_id,
            ?position,
            "gmGotoXYZ: non-finite coordinate rejected"
        );
        return;
    }

    // Resolve the caller's space + prior position. Missing entity → no-op.
    let (space_id, prev_pos) = match space_mgr.get_entity(entity_id) {
        Some(e) => (
            e.space_id.0 as u32,
            [e.position.x, e.position.y, e.position.z],
        ),
        None => {
            tracing::warn!(entity_id, "gmGotoXYZ: caller entity not found");
            return;
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
}

/// `gmKillTarget(INT64 TargetId)` — kill an NPC.
///
/// Safety per the CAT-N audit: REFUSE player targets (player death goes
/// through the PvP/respawn path, not a GM one-shot), resolve only entities in
/// the GM's own space, and run the kill through the canonical death sequence
/// ([`crate::cell::abilities::gm_kill_npc`]) so loot, threat fanout, and the
/// dead-state flip land in protocol order.
async fn handle_gm_kill_target(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if args.len() < 8 {
        tracing::warn!(
            entity_id,
            args_len = args.len(),
            "gmKillTarget: truncated args (need INT64 = 8 bytes)"
        );
        return;
    }
    let target_i64 = i64::from_le_bytes([
        args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7],
    ]);
    // Entity ids are u32 on the wire/grid; an out-of-range INT64 can't name a
    // real entity. Reject rather than truncate.
    let target_eid = match u32::try_from(target_i64) {
        Ok(id) => id,
        Err(_) => {
            tracing::warn!(
                entity_id,
                target_i64,
                "gmKillTarget: target id out of u32 range"
            );
            return;
        }
    };

    // Resolve the target and validate: must exist, be in the GM's space, and
    // be an NPC. Each rejection is its own warn so an operator can tell which
    // guard fired.
    let caller_space = space_mgr.get_entity(entity_id).map(|e| e.space_id.0);
    let target = match space_mgr.get_entity(target_eid) {
        Some(t) => t,
        None => {
            tracing::warn!(entity_id, target_eid, "gmKillTarget: target not found");
            return;
        }
    };
    if Some(target.space_id.0) != caller_space {
        tracing::warn!(
            entity_id,
            target_eid,
            target_space = target.space_id.0,
            caller_space = ?caller_space,
            "gmKillTarget: target is in a different space — refused"
        );
        return;
    }
    if target.is_player {
        tracing::warn!(
            entity_id,
            target_eid,
            "gmKillTarget: target is a player — refused (GM kill is NPC-only)"
        );
        return;
    }

    tracing::info!(entity_id, target_eid, "gmKillTarget: killing NPC");
    let killed = crate::cell::abilities::gm_kill_npc(target_eid, entity_id, tx, space_mgr).await;
    if !killed {
        // gm_kill_npc fails closed on already-dead / re-resolved-as-player —
        // surface it so the GM knows the command no-op'd.
        tracing::warn!(
            entity_id,
            target_eid,
            "gmKillTarget: kill not applied (target already dead or not an NPC)"
        );
    }
}

#[cfg(test)]
#[path = "gm_tests.rs"]
mod tests;
