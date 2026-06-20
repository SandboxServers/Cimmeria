//! GM world/entity-op handlers — `gmKillTarget`, `gmDespawnByCmd` / `despawnMob`,
//! `gmRespawn`, and `gmSetTarget`. Each reuses a canonical cell primitive
//! (`abilities::gm_kill_npc`, `SpaceManager::destroy_entity`, combat
//! `handle_respawn`, and the `current_target_id` write + `onTargetUpdate`
//! fanout).

use tokio::sync::mpsc;

use super::feedback::send_gm_feedback;
use super::{read_i32, read_i64};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::ON_TARGET_UPDATE;
use crate::mercury::read_wstring;

/// `gmKillTarget(INT64 TargetId)` — kill an NPC.
///
/// Safety: REFUSE player targets (player death goes
/// through the PvP/respawn path, not a GM one-shot), resolve only entities in
/// the GM's own space, and run the kill through the canonical death sequence
/// ([`crate::cell::abilities::gm_kill_npc`]) so loot, threat fanout, and the
/// dead-state flip land in protocol order.
pub(super) async fn handle_kill_target(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let target_i64 = match read_i64(args, 0) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                "gmKillTarget: truncated args (need INT64 = 8 bytes)"
            );
            send_gm_feedback(entity_id, "gmKillTarget: missing INT64 TargetId", tx).await;
            return true;
        }
    };
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
            send_gm_feedback(entity_id, "gmKillTarget: target id out of range", tx).await;
            return true;
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
            send_gm_feedback(
                entity_id,
                &format!("gmKillTarget: target {target_eid} not found"),
                tx,
            )
            .await;
            return true;
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
        send_gm_feedback(
            entity_id,
            &format!("gmKillTarget: target {target_eid} is in a different space"),
            tx,
        )
        .await;
        return true;
    }
    if target.is_player {
        tracing::warn!(
            entity_id,
            target_eid,
            "gmKillTarget: target is a player — refused (GM kill is NPC-only)"
        );
        send_gm_feedback(
            entity_id,
            &format!("gmKillTarget: target {target_eid} is a player (NPC-only)"),
            tx,
        )
        .await;
        return true;
    }

    tracing::info!(entity_id, target_eid, "gmKillTarget: killing NPC");
    let killed = crate::cell::abilities::gm_kill_npc(target_eid, entity_id, tx, space_mgr).await;
    if killed {
        send_gm_feedback(
            entity_id,
            &format!("gmKillTarget: killed npc {target_eid}"),
            tx,
        )
        .await;
    } else {
        // gm_kill_npc fails closed on already-dead / re-resolved-as-player —
        // surface it so the GM knows the command no-op'd.
        tracing::warn!(
            entity_id,
            target_eid,
            "gmKillTarget: kill not applied (target already dead or not an NPC)"
        );
        send_gm_feedback(
            entity_id,
            &format!(
                "gmKillTarget: kill not applied (npc {target_eid} already dead or not an NPC)"
            ),
            tx,
        )
        .await;
    }
    true
}

/// `gmDespawnByCmd(INT32 TargetID)` / `despawnMob(INT32 entityID)` — remove an
/// NPC from the space.
///
/// Refuses player targets (a GM should never `destroy_entity` a live player —
/// that path is respawn/disconnect, not despawn) and only reaches entities in
/// the GM's own space. `destroy_entity` removes the NPC from the spatial grid;
/// AoI cleanup propagates the removal to witnesses on the next tick (no extra
/// base notification needed, mirroring the gate-travel teardown).
pub(super) async fn handle_despawn(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let target_i32 = match read_i32(args, 0) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                "gmDespawn: truncated args (need INT32 TargetID)"
            );
            send_gm_feedback(entity_id, "gmDespawn: missing INT32 TargetID", tx).await;
            return true;
        }
    };
    let target_eid = match u32::try_from(target_i32) {
        Ok(id) if id != 0 => id,
        _ => {
            tracing::warn!(entity_id, target_i32, "gmDespawn: invalid target id");
            send_gm_feedback(entity_id, "gmDespawn: invalid target id", tx).await;
            return true;
        }
    };

    let caller_space = space_mgr.get_entity(entity_id).map(|e| e.space_id.0);
    match space_mgr.get_entity(target_eid) {
        Some(t) if Some(t.space_id.0) != caller_space => {
            tracing::warn!(
                entity_id,
                target_eid,
                target_space = t.space_id.0,
                caller_space = ?caller_space,
                "gmDespawn: target in a different space — refused"
            );
            send_gm_feedback(
                entity_id,
                &format!("gmDespawn: target {target_eid} is in a different space"),
                tx,
            )
            .await;
            return true;
        }
        Some(t) if t.is_player => {
            tracing::warn!(
                entity_id,
                target_eid,
                "gmDespawn: target is a player — refused (NPC-only)"
            );
            send_gm_feedback(
                entity_id,
                &format!("gmDespawn: target {target_eid} is a player (NPC-only)"),
                tx,
            )
            .await;
            return true;
        }
        Some(_) => {}
        None => {
            tracing::warn!(entity_id, target_eid, "gmDespawn: target not found");
            send_gm_feedback(
                entity_id,
                &format!("gmDespawn: target {target_eid} not found"),
                tx,
            )
            .await;
            return true;
        }
    }

    tracing::info!(entity_id, target_eid, "gmDespawn: despawning NPC");
    space_mgr.destroy_entity(target_eid);
    send_gm_feedback(
        entity_id,
        &format!("gmDespawn: despawned npc {target_eid}"),
        tx,
    )
    .await;
    true
}

/// `gmRespawn()` — respawn the calling GM.
///
/// Reuses the combat respawn primitive with `respawner_id = 0`, which makes
/// `resolve_respawn_target` fall back to the player's current-world respawner
/// (or the Castle default). Drives the full HEALTH/FOCUS reset + state-flag
/// clear + pawn re-anchor.
pub(super) async fn handle_respawn_cmd(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    if space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.player_id)
        .is_none()
    {
        tracing::warn!(
            entity_id,
            "gmRespawn: caller has no player_id (respawn is player-only)"
        );
        send_gm_feedback(entity_id, "gmRespawn: caller is not a player", tx).await;
        return true;
    }
    tracing::info!(entity_id, "gmRespawn: respawning GM");
    crate::cell::cell_methods::player::combat::handle_respawn(entity_id, 0, tx, space_mgr).await;
    send_gm_feedback(entity_id, "gmRespawn: respawned", tx).await;
    true
}

/// `gmSetTarget(WSTRING aNameOrID)` — set the calling GM's current target.
///
/// Numeric-id form only (name→id resolution isn't wired into the cell). Mirrors
/// the inbound `setTargetID` handler: a positive id stores the target and a
/// `0` clears it ("deselect" sentinel), then `onTargetUpdate` is sent to the
/// owner and fanned out to AoI witnesses so peers see the selection.
pub(super) async fn handle_set_target(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let (name_or_id, _) = match read_wstring(args, 0) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(entity_id, error = %e, "gmSetTarget: malformed NameOrID WSTRING");
            send_gm_feedback(entity_id, "gmSetTarget: malformed NameOrID", tx).await;
            return true;
        }
    };
    let target_id = match name_or_id.trim().parse::<i32>() {
        Ok(id) if id >= 0 => id,
        _ => {
            tracing::warn!(
                entity_id,
                name_or_id = %name_or_id,
                "gmSetTarget: NameOrID is not a non-negative numeric id — \
                 name resolution is not wired in the cell; rejecting"
            );
            send_gm_feedback(
                entity_id,
                "gmSetTarget: NameOrID must be a non-negative numeric id",
                tx,
            )
            .await;
            return true;
        }
    };

    match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e.current_target_id = if target_id > 0 { Some(target_id) } else { None },
        None => {
            tracing::warn!(entity_id, "gmSetTarget: caller entity not found");
            send_gm_feedback(entity_id, "gmSetTarget: caller entity not found", tx).await;
            return true;
        }
    }
    tracing::info!(entity_id, target_id, "gmSetTarget: target set");

    // Notify the owner so the client enables auto-attack against the target.
    let reply = target_id.to_le_bytes().to_vec();
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_TARGET_UPDATE,
            args: reply,
        })
        .await;
    // Fan out to witnesses so peers see who the GM is targeting.
    for witness_id in space_mgr.get_witnesses_of(entity_id) {
        let _ = tx
            .send(CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index: ON_TARGET_UPDATE,
                args: target_id.to_le_bytes().to_vec(),
            })
            .await;
    }
    let feedback = if target_id > 0 {
        format!("gmSetTarget: target set to {target_id}")
    } else {
        "gmSetTarget: target cleared".to_string()
    };
    send_gm_feedback(entity_id, &feedback, tx).await;
    true
}
