//! `gmGotoXYZ` — same-space snap teleport of the calling GM.

use cimmeria_cell_world::cell::plugin::EntityHookPoint;
use tokio::sync::mpsc;

use super::super::feedback::send_gm_feedback;
use super::super::forward_to_base;
use super::read_xyz;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `gmGotoXYZ(FLOAT aX, FLOAT aY, FLOAT aZ)` — teleport the calling GM to the
/// coordinate within their current space.
///
/// Mirrors the same-space content teleport: capture the prior position as the
/// forced-position reference vector, update the spatial grid, then route the
/// authoritative `FORCED_POSITION` snap through `TeleportPlayer`. Witnesses
/// pick up the move on the next AoI tick.
pub(in crate::cell::console::gm) async fn handle_goto_xyz(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(position) = read_xyz(args, 0) else {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            args_len = args.len(),
            "gmGotoXYZ: truncated args (need 3×FLOAT = 12 bytes)"
        );
        send_gm_feedback(entity_id, "gmGotoXYZ: need 3 FLOAT coordinates", tx).await;
        return true;
    };

    // Reject non-finite coordinates — a NaN/inf snap corrupts the spatial grid
    // and the wire FORCED_POSITION. (A modified client / replayed packet could
    // supply garbage even though it's a GM channel.)
    if !position.iter().all(|c| c.is_finite()) {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            ?position,
            "gmGotoXYZ: non-finite coordinate rejected"
        );
        send_gm_feedback(entity_id, "gmGotoXYZ: non-finite coordinate rejected", tx).await;
        return true;
    }

    // Resolve the caller's space + prior position. Missing entity → no-op.
    let (space_id, prev_pos) = match space_mgr.get_entity(entity_id) {
        Some(e) => (
            e.space_id.0 as u32,
            [e.position.x, e.position.y, e.position.z],
        ),
        None => {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                "gmGotoXYZ: caller entity not found"
            );
            send_gm_feedback(entity_id, "gmGotoXYZ: caller entity not found", tx).await;
            return true;
        }
    };

    tracing::info!(
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        ?position,
        space_id,
        world = space_mgr.world_name_for_space(space_id),
        "gmGotoXYZ: teleporting GM"
    );

    // Keep the spatial grid consistent first (writes cell_entity.position),
    // then send the authoritative snap. Position-only: `gmGotoXYZ` names a
    // destination, not a facing, so the GM keeps looking where they were.
    space_mgr.update_position_preserving_facing(entity_id, position, [0.0; 3]);
    space_mgr.note_authorized_teleport(entity_id); // reseed validator clock

    // SS-D3, #962: the travel hook. The duel plugin ends the traveller's duel
    // (`EDUEL_DEFEAT_Teleport`) here; `every_travel_site_fires_the_travel_hook`.
    space_mgr
        .fire_entity_hook(EntityHookPoint::BeforeTravelSend, entity_id, tx)
        .await;
    if !forward_to_base(
        tx,
        CellToBaseMsg::TeleportPlayer {
            entity_id,
            space_id,
            position,
            prev_pos,
        },
        "gmGotoXYZ",
    )
    .await
    {
        return true; // base channel closed — don't claim a snap that never sent.
    }
    // A pet out with the moved player comes along (pets PT-02).
    cimmeria_cell_world::cell::pets::on_owner_teleported(
        entity_id,
        cimmeria_cell_world::cell::pets::OwnerPath::GmTravel,
        tx,
        space_mgr,
    )
    .await;
    send_gm_feedback(
        entity_id,
        &format!(
            "gmGotoXYZ: teleported to ({}, {}, {})",
            position[0], position[1], position[2]
        ),
        tx,
    )
    .await;
    true
}
