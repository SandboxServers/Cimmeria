//! Effect → wire dispatcher.
//!
//! Consumes [`Effect`] values produced by the ring-transport state machine and
//! turns them into `CellToBaseMsg` sends + `SpaceManager` mutations. Also
//! contains the cross-link helpers (`mark_player_loaded`,
//! `try_advance_after_load`, `same_world_teleport`) — these all flow from
//! effect dispatch and need to share `Box<Pin<...>>`-ed recursion to handle
//! the synchronous `TeleportPlayer → mark_player_loaded → all_players_loaded`
//! chain that same-world teleports collapse into one tick.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::transporter::{Effect, State};
use super::wire_helpers::{
    send_destination_list, send_play_sequence, send_visible, update_state_flag, BSF_MOVEMENT_LOCK,
};
use crate::cell::content;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Dispatch a single [`Effect`] to the wire / world.
///
/// Returns a `Pin<Box<...>>` future to break the recursion cycle that
/// `Effect::TeleportPlayer` introduces: it triggers `mark_player_loaded` →
/// `try_advance_after_load` → more `dispatch_effects`. Without box-pinning,
/// rustc rejects the infinitely sized `Future`.
pub(super) fn dispatch_effect<'a>(
    effect: Effect,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
    engine: &'a ChainEngine,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(dispatch_effect_inner(effect, tx, space_mgr, engine))
}

async fn dispatch_effect_inner(
    effect: Effect,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    match effect {
        Effect::PlaySequence {
            entity_id,
            event_set_id,
            region_event,
        } => {
            send_play_sequence(entity_id, event_set_id, region_event, tx, space_mgr).await;
        }
        Effect::OnTeleportOut {
            entity_id,
            region_id,
            destination_id,
        } => {
            // No client wire call: this is a script-level event hook. The
            // current content engine has no `teleport_out` trigger registered,
            // so we just log. Add a TriggerType / loader entry if/when content
            // authoring needs it.
            tracing::debug!(
                entity_id,
                region_id,
                destination_id,
                "ring: onTeleportOut event (no chain trigger registered)"
            );
        }
        Effect::LockMovement { entity_id } => {
            update_state_flag(entity_id, BSF_MOVEMENT_LOCK, true, tx, space_mgr).await;
        }
        Effect::UnlockMovement { entity_id } => {
            update_state_flag(entity_id, BSF_MOVEMENT_LOCK, false, tx, space_mgr).await;
        }
        Effect::HidePlayer { entity_id } => {
            send_visible(entity_id, false, tx, space_mgr).await;
        }
        Effect::ShowPlayer { entity_id } => {
            send_visible(entity_id, true, tx, space_mgr).await;
        }
        Effect::TeleportPlayer {
            entity_id,
            position,
            world_name,
            destination_region_id,
        } => {
            // Resolve the authoritative space_id from the cell — bailing if the
            // entity is gone, because the alternative (synthesizing 0) would
            // dispatch a bogus snap and still mark the destination ring loaded.
            let space_id = match space_mgr.get_entity(entity_id).map(|e| e.space_id.0 as u32) {
                Some(s) => s,
                None => {
                    tracing::error!(
                        entity_id, destination_region_id,
                        "TeleportPlayer effect: entity missing from space — destination ring will time out in RemoteLoadWait"
                    );
                    return;
                }
            };
            if !same_world_teleport(entity_id, position, &world_name, space_id, tx, space_mgr).await
            {
                // Send failed — don't mark loaded; let the destination time out.
                return;
            }
            // Mark the player as "loaded" on the destination ring immediately —
            // Castle_CellBlock is non-instanced and same-world, so we don't
            // wait for a real `mapLoaded` round trip. Cross-world rings need a
            // different hook (BaseToCell `PlayerLoaded` after world re-entry);
            // see TeleportCrossWorld branch.
            mark_player_loaded(destination_region_id, entity_id, tx, space_mgr, engine).await;
        }
        Effect::TeleportCrossWorld {
            entity_id,
            position,
            world_name,
            destination_region_id,
        } => {
            // Cross-world ring transport — re-uses the gate-travel pipeline
            // (RESET_ENTITIES → ENABLE_ENTITIES → world entry replay) so the
            // client tears down its old-world entity view and re-loads on
            // the destination world cleanly. The destination ring's FSM
            // stays parked in `RemoteLoadWait` until base sends back
            // `BaseToCellMsg::AdvanceRingDestination` after `onClientReady`
            // — that's the deferred `mark_player_loaded` hook the issue's
            // Phase A design calls for.
            //
            // Order matters: flush bandolier ammo → destroy on this cell →
            // send GateTravel. The base side waits on the same client
            // socket so cell→base ordering is preserved naturally.
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                if let Some(player_id) = entity.player_id {
                    crate::cell::cell_methods::inventory::flush_dirty_bandolier_ammo(
                        entity, player_id, tx,
                    )
                    .await;
                }
            }
            space_mgr.destroy_entity(entity_id);

            if let Err(e) = tx
                .send(CellToBaseMsg::GateTravel {
                    entity_id,
                    target_world_name: world_name.clone(),
                    position,
                    rotation: [0.0, 0.0, 0.0],
                    destination_ring_id: Some(destination_region_id),
                })
                .await
            {
                // The destination ring is already in RemoteLoadWait at this
                // point. A failed send means base never tears down the
                // client view and the destination FSM will sit until a
                // future ring tick stalls. Log loudly; the player has to
                // relog to recover. No retry: the channel is gone.
                tracing::error!(
                    entity_id, %world_name, destination_region_id,
                    error = %e,
                    "TeleportCrossWorld: cell→base GateTravel send failed"
                );
            }
        }
        Effect::FireTeleportIn {
            entity_id,
            region_id,
        } => {
            let Some(player_id) = space_mgr.get_entity(entity_id).and_then(|e| e.player_id) else {
                tracing::error!(
                    entity_id, region_id,
                    "FireTeleportIn: entity has no player_id — refusing to fire chain (would miscredit arrival content)"
                );
                return;
            };
            content::fire_teleport_in(entity_id, player_id, region_id, engine, tx, space_mgr).await;
        }
        Effect::SendDestinationList {
            entity_id,
            source_region_id,
            destinations,
        } => {
            send_destination_list(entity_id, source_region_id, &destinations, tx, space_mgr).await;
        }
    }
}

/// Apply a list of effects sequentially.
pub(super) async fn dispatch_effects(
    effects: Vec<Effect>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    for effect in effects {
        dispatch_effect(effect, tx, space_mgr, engine).await;
    }
}

/// Mark a player as loaded on the destination ring and, if the destination is
/// ready, fire its `all_players_loaded` transition.
pub(super) async fn mark_player_loaded(
    dst_region_id: i32,
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let recorded = match space_mgr.ring_transporters.get_mut(dst_region_id) {
        Some(dst) => {
            // `player_loaded` is idempotent on the same eid.
            let _ = dst.player_loaded(entity_id);
            true
        }
        None => false,
    };
    // Python clears `destinationRingId` here once the destination ring has
    // taken responsibility for the player. Only clear after a successful
    // load record — otherwise we'd lose the routing pointer while the
    // player is still in transit.
    if recorded {
        if let Some(player) = space_mgr.get_entity_mut(entity_id) {
            player.destination_ring_id = None;
        }
    }
    try_advance_after_load(dst_region_id, tx, space_mgr, engine).await;
}

/// Check if the destination has all players loaded; if so, trigger the
/// `all_players_loaded` transition. Handles both the "real" path (sequence +
/// timer) and the fast path (no players → skip straight to cooldown).
pub(super) async fn try_advance_after_load(
    dst_region_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let now = Instant::now();
    let (do_fire, do_fast_path) = match space_mgr.ring_transporters.get(dst_region_id) {
        Some(t) => {
            let ready = t.state == State::RemoteLoadWait
                && t.players_loaded.len() as u32 == t.num_remote_players;
            let empty = ready && t.players_loaded.is_empty();
            (ready, empty)
        }
        None => (false, false),
    };
    if !do_fire {
        return;
    }
    let effects = match space_mgr.ring_transporters.get_mut(dst_region_id) {
        Some(t) => t.all_players_loaded(now),
        None => return,
    };
    dispatch_effects(effects, tx, space_mgr, engine).await;

    // Python: if no one loaded, immediately call __remoteWarmupTimerExpired,
    // and from there __cooldownTimerExpired (since players_loaded is empty
    // there too).
    if do_fast_path {
        let effects = match space_mgr.ring_transporters.get_mut(dst_region_id) {
            Some(t) => t.remote_warmup_timer_expired(now),
            None => return,
        };
        dispatch_effects(effects, tx, space_mgr, engine).await;
        let effects = match space_mgr.ring_transporters.get_mut(dst_region_id) {
            Some(t) => t.cooldown_timer_expired(),
            None => return,
        };
        dispatch_effects(effects, tx, space_mgr, engine).await;
    }
}

/// Returns `true` if the cell→base hand-off succeeded. The caller should
/// gate `mark_player_loaded` on this — a failed send means the base never
/// snapped the avatar and the destination ring shouldn't pretend the player
/// arrived.
async fn same_world_teleport(
    entity_id: u32,
    position: [f32; 3],
    _world_name: &str,
    space_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // Capture the entity's prior position before the spatial-grid update
    // overwrites it — becomes the `forcedPosition` prev-position reference.
    // Falling back to `position` (zero-distance interp) on a missing entity
    // still avoids the camera-through-floor glitch.
    let prev_pos = space_mgr
        .get_entity(entity_id)
        .map(|e| [e.position.x, e.position.y, e.position.z])
        .unwrap_or(position);
    // Server-side: keep the spatial grid + entity position consistent so AoI
    // ticks broadcast the new position to other witnesses (build_avatar_update).
    // `update_entity_position` already writes `cell_entity.position`.
    space_mgr.update_entity_position(entity_id, position, [0, 0, 0], [0.0; 3]);
    // Authorized teleport: reseed the movement-validator clock so the
    // first post-ring client packet isn't measured against the pre-ring
    // sample (which would log a spurious speed warning).
    space_mgr.note_authorized_teleport(entity_id);

    // Client-side: hand off to the base. The base sends `forcedPosition` (0x31)
    // to authoritatively snap the player's own avatar, then `onPlayerTeleport`
    // (method 116) for streaming-load coordination. The bare 116-only path the
    // previous version used does NOT move the avatar — see SGWPlayer.def comment
    // and the `handle_teleport_player` handler.
    if let Err(e) = tx
        .send(CellToBaseMsg::TeleportPlayer {
            entity_id,
            space_id,
            position,
            prev_pos,
        })
        .await
    {
        tracing::error!(entity_id, space_id, ?position, error = %e,
            "TeleportPlayer: cell→base channel send failed");
        return false;
    }
    true
}
