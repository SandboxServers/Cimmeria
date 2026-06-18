//! Transport action handlers: ring transporter trigger, intra-space
//! teleport.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `Action::TriggerTransporter` — kick off the ring-transporter sequence
/// for the given region. Routes through `ring_transport::handle_interact`
/// which owns the state machine.
pub(super) async fn trigger_transporter(
    region_id: i32,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    tracing::info!(
        entity_id,
        region_id,
        chain_id,
        "Content: triggering transporter"
    );
    crate::cell::ring_transport::handle_interact(region_id, entity_id, tx, space_mgr, engine).await;
}

/// `Action::Teleport` — same-space chain teleport. Cross-space chain
/// teleport would need a path equivalent to GateTravel; deferred until
/// a chain actually demands it.
pub(super) async fn teleport(
    space_id: i32,
    position: [f32; 3],
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    tracing::info!(
        entity_id,
        space_id,
        ?position,
        chain_id,
        "Content: teleporting entity"
    );
    // The chain action's `space_id` is the destination space ID.
    // Cross-space chain teleport would need a path equivalent to
    // GateTravel; defer until a chain actually demands it.
    let current_space = space_mgr.get_entity(entity_id).map(|e| e.space_id.0);
    if Some(space_id) != current_space && space_id != 0 {
        tracing::warn!(
            entity_id, requested = space_id, current = ?current_space, chain_id,
            "Content: cross-space chain teleport not implemented — falling back to same-space move"
        );
    }
    // Capture the entity's prior position *before* the spatial-grid update
    // overwrites it — this becomes the `forcedPosition` prev-position
    // reference vector. Falling back to `position` (zero-distance interp) if
    // the entity is somehow missing keeps the camera-snap bug fixed even on
    // the unhappy path.
    let prev_pos = space_mgr
        .get_entity(entity_id)
        .map(|e| [e.position.x, e.position.y, e.position.z])
        .unwrap_or(position);
    // Same-world teleport: keep the spatial grid consistent here,
    // then route through TeleportPlayer for the authoritative
    // FORCED_POSITION snap + persist. The bare 116-only path the
    // previous version emitted does NOT move the avatar.
    // `update_entity_position` already writes `cell_entity.position`.
    space_mgr.update_entity_position(entity_id, position, [0, 0, 0], [0.0; 3]);
    // Authorized teleport: reseed the movement-validator clock (issue #478).
    space_mgr.note_authorized_teleport(entity_id);
    // SpaceId is i32 in the cell (matches DB type) but the wire
    // forced-position packet is u32 — space ids are always
    // non-negative, so the cast is a width-only conversion.
    let cell_space_id = space_mgr
        .get_entity(entity_id)
        .map(|e| e.space_id.0 as u32)
        .unwrap_or(space_id as u32);
    let _ = tx
        .send(CellToBaseMsg::TeleportPlayer {
            entity_id,
            space_id: cell_space_id,
            position,
            prev_pos,
        })
        .await;
}

/// `Action::CrossWorldTeleport` — direct cross-world hop bypassing the
/// ring-transport FSM. Same plumbing as the stargate dial path
/// (`cell::gate_travel::handle_dial_gate`): flush dirty bandolier ammo,
/// destroy the cell entity on this world, send `CellToBaseMsg::GateTravel`
/// with `destination_ring_id: None` so the base side does not emit
/// `BaseToCellMsg::AdvanceRingDestination` (there's no destination ring
/// FSM to advance — that's the whole point of using this action over
/// `TriggerTransporter`).
///
/// Used by mission 688's chain 1109 to deliver the player to Castle's
/// arrival platform: the canonical `GLB-RingTransporterBase` kismet
/// prefab actor doesn't exist on the Castle map (the platform there is
/// set-decorated geometry, never wired as a working transporter), so
/// firing the destination-side ring sequence has nothing to animate.
/// Skipping the ceremony entirely is cleaner than reusing a wrong-map
/// kismet that would play at the wrong location.
pub(super) async fn cross_world_teleport(
    world_name: String,
    position: [f32; 3],
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    tracing::info!(
        entity_id,
        world = %world_name,
        ?position,
        chain_id,
        "Content: cross-world teleporting entity"
    );

    // Flush any pending bandolier ammo writes before the cell entity is
    // destroyed — same ordering the ring's TeleportCrossWorld arm uses.
    // Skip silently if the entity is gone or has no player_id; nothing to flush.
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        if let Some(player_id) = entity.player_id {
            crate::cell::cell_methods::inventory::flush_dirty_bandolier_ammo(entity, player_id, tx)
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
            destination_ring_id: None,
        })
        .await
    {
        tracing::error!(
            entity_id, world = %world_name, ?position, chain_id, error = %e,
            "CrossWorldTeleport: cell→base GateTravel send failed -- player will be stuck on previous world"
        );
    }
}
