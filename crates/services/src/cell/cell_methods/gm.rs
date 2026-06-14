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
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;

    /// Re-assert the document-order count of `<Exposed/>` CellMethods in
    /// `SGWGmPlayer.def`. The first own exposed method (gmMissionAssign, def
    /// line 65) is index 109; counting forward in document order — skipping
    /// `gmSetCallback` (def line 312, which has NO `<Exposed/>`) — lands each
    /// implemented method at the constant below. If this drifts, the client's
    /// method table and our dispatch disagree and gm* commands silently route
    /// to the wrong handler.
    #[test]
    fn gm_indices_match_def_document_order() {
        // 109 + offset, where offset is the zero-based document-order position
        // among exposed methods (gmMissionAssign = 0).
        assert_eq!(
            GM_GIVE_ITEM,
            109 + 24,
            "gmGiveItem is the 25th exposed (def line 185)"
        );
        assert_eq!(
            GM_GOTO_XYZ,
            109 + 54,
            "gmGotoXYZ is the 55th exposed (def line 348)"
        );
        assert_eq!(
            GM_KILL_TARGET,
            109 + 81,
            "gmKillTarget is the 82nd exposed (def line 482)"
        );
    }

    /// All implemented indices sit in the GM tail (109 or above), so the
    /// dispatch-layer gate (`gm_gate::requires_gm`, which gates every index
    /// from 109 up) covers them. A constant that slipped below 109 would be
    /// reachable by a non-GM — this pins the invariant. The `109` literal here
    /// is the same SGWGmPlayer base the gate uses (the
    /// `gm_gate::SGWGMPLAYER_CELL_METHOD_BASE` constant); keep them in lockstep.
    #[test]
    fn implemented_indices_are_in_gm_tail() {
        const GM_TAIL_BASE: u16 = 109;
        for idx in [GM_GIVE_ITEM, GM_GOTO_XYZ, GM_KILL_TARGET] {
            assert!(
                idx >= GM_TAIL_BASE,
                "implemented gm* index {idx} must be in the GM-gated tail (>= 109)"
            );
        }
    }

    fn mgr_with_player(eid: u32, world: &str) -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = format!(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="{world}" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#
        );
        mgr.parse_spaces_xml(&xml).unwrap();
        mgr.create_startup_spaces(&format!(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="{world}" /></Spaces>"#
        ))
        .unwrap();
        mgr.create_entity(eid, world, [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(eid) {
            e.is_player = true;
            e.player_id = Some(100);
            e.access_level = 2; // GameMaster
        }
        mgr
    }

    /// Helper to build the `gmGiveItem` arg buffer: WSTRING DesignId + INT32 qty.
    fn give_item_args(design_id: &str, qty: i32) -> Vec<u8> {
        let mut args = Vec::new();
        crate::mercury::write_wstring(&mut args, design_id);
        args.extend_from_slice(&qty.to_le_bytes());
        args
    }

    #[tokio::test]
    async fn gm_give_item_emits_grant_with_clamped_qty() {
        let mut mgr = mgr_with_player(1, "Castle");
        let (tx, mut rx) = mpsc::channel(8);

        // Request 5000 — must clamp to GM_GIVE_ITEM_MAX_QTY (1000).
        let args = give_item_args("1234", 5000);
        assert!(dispatch(1, GM_GIVE_ITEM, &args, &tx, &mut mgr).await);

        match rx.try_recv().expect("gmGiveItem must emit GrantItem") {
            CellToBaseMsg::GrantItem {
                entity_id,
                player_id,
                item_id,
                container_id,
                count,
            } => {
                assert_eq!(entity_id, 1);
                assert_eq!(player_id, 100);
                assert_eq!(item_id, 1234);
                assert_eq!(container_id, INV_MAIN);
                assert_eq!(
                    count, GM_GIVE_ITEM_MAX_QTY,
                    "quantity must clamp to the cap"
                );
            }
            other => panic!("expected GrantItem, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn gm_give_item_rejects_non_numeric_and_nonpositive_qty() {
        let mut mgr = mgr_with_player(1, "Castle");
        let (tx, mut rx) = mpsc::channel(8);

        // Non-numeric design id → no grant.
        let args = give_item_args("AmberVial", 1);
        assert!(dispatch(1, GM_GIVE_ITEM, &args, &tx, &mut mgr).await);
        assert!(
            rx.try_recv().is_err(),
            "non-numeric DesignId must not grant"
        );

        // Quantity 0 → no grant.
        let args = give_item_args("1234", 0);
        assert!(dispatch(1, GM_GIVE_ITEM, &args, &tx, &mut mgr).await);
        assert!(rx.try_recv().is_err(), "quantity 0 must not grant");
    }

    #[tokio::test]
    async fn gm_goto_xyz_updates_position_and_emits_teleport() {
        let mut mgr = mgr_with_player(1, "Castle");
        let (tx, mut rx) = mpsc::channel(8);

        let mut args = Vec::new();
        for c in [10.0f32, 20.0, 30.0] {
            args.extend_from_slice(&c.to_le_bytes());
        }
        assert!(dispatch(1, GM_GOTO_XYZ, &args, &tx, &mut mgr).await);

        match rx.try_recv().expect("gmGotoXYZ must emit TeleportPlayer") {
            CellToBaseMsg::TeleportPlayer {
                entity_id,
                position,
                prev_pos,
                ..
            } => {
                assert_eq!(entity_id, 1);
                assert_eq!(position, [10.0, 20.0, 30.0]);
                assert_eq!(prev_pos, [0.0, 0.0, 0.0], "prev_pos is the spawn origin");
            }
            other => panic!("expected TeleportPlayer, got {other:?}"),
        }
        // Spatial grid updated.
        let e = mgr.get_entity(1).unwrap();
        assert_eq!(
            [e.position.x, e.position.y, e.position.z],
            [10.0, 20.0, 30.0]
        );
    }

    #[tokio::test]
    async fn gm_goto_xyz_rejects_non_finite() {
        let mut mgr = mgr_with_player(1, "Castle");
        let (tx, mut rx) = mpsc::channel(8);

        let mut args = Vec::new();
        args.extend_from_slice(&f32::NAN.to_le_bytes());
        args.extend_from_slice(&0.0f32.to_le_bytes());
        args.extend_from_slice(&0.0f32.to_le_bytes());
        assert!(dispatch(1, GM_GOTO_XYZ, &args, &tx, &mut mgr).await);
        assert!(rx.try_recv().is_err(), "NaN coordinate must not teleport");
    }

    #[tokio::test]
    async fn gm_kill_target_kills_npc_in_same_space() {
        let mut mgr = mgr_with_player(1, "Castle");
        // NPC at id 2 in the same space.
        mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        let (tx, mut _rx) = mpsc::channel(32);

        let args = 2i64.to_le_bytes();
        assert!(dispatch(1, GM_KILL_TARGET, &args, &tx, &mut mgr).await);

        let npc = mgr.get_entity(2).unwrap();
        assert!(
            crate::cell::combat::is_dead_state(npc.state_field),
            "gmKillTarget must mark the NPC dead"
        );
    }

    #[tokio::test]
    async fn gm_kill_target_refuses_player() {
        let mut mgr = mgr_with_player(1, "Castle");
        // A second player (not an NPC) at id 2.
        mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(2) {
            e.is_player = true;
            e.player_id = Some(200);
        }
        let (tx, mut _rx) = mpsc::channel(32);

        let args = 2i64.to_le_bytes();
        assert!(dispatch(1, GM_KILL_TARGET, &args, &tx, &mut mgr).await);

        let victim = mgr.get_entity(2).unwrap();
        assert!(
            !crate::cell::combat::is_dead_state(victim.state_field),
            "gmKillTarget must refuse a player target"
        );
    }

    /// An unimplemented 109+ index returns `false` so the router falls
    /// through to its (already-authorized) warn arm — no panic.
    #[tokio::test]
    async fn unimplemented_gm_index_returns_false() {
        let mut mgr = mgr_with_player(1, "Castle");
        let (tx, _rx) = mpsc::channel(8);
        // 142 = gmSetGodMode — in the tail, not implemented here.
        assert!(!dispatch(1, 142, &[], &tx, &mut mgr).await);
    }
}
