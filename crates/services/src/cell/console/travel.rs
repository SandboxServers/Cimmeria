//! Player-administration travel console commands: `.gotoxyz`.
//!
//! Same-space authoritative teleport of the selected target, falling back to
//! the caller when nothing is selected — legacy `gotoXYZ`
//! (`deprecated/python/cell/commands/Player.py:368-382`): `entity = target or
//! player`, then `entity.teleportTo(Vector3(x, y, z), 0.0)`.
//!
//! Reuses the same mechanism the native `gmGotoXYZ`/`gmSummon` handlers
//! already use (`crates/services/src/cell/cell_methods/gm/travel.rs`):
//! `space_mgr.update_entity_position(...)` first (spatial grid +
//! `cell_entity.position`), then `space_mgr.note_authorized_teleport(...)`
//! (reseeds the movement-validator clock so the snap doesn't trip
//! speed-hack detection — called unconditionally for both players and NPCs,
//! matching `gmSummon`'s existing precedent; the validator's `move_clock` is
//! only ever consulted against *client* movement packets, which NPCs never
//! send, so the reseed is a harmless no-op for an NPC target).
//!
//! A player target additionally needs the authoritative `TeleportPlayer` snap
//! pushed to its own client (`BASEMSG_FORCED_POSITION`) — an NPC has no
//! client to push that to, but other players still see the moved NPC through
//! the normal AoI/witness broadcast once the spatial grid updates (see
//! `crates/services/src/cell/cell_methods/gm/tests/travel.rs`'s
//! `summoned_npc_is_broadcast_to_caller_witness` for the equivalent proof on
//! `gmSummon`). `TeleportPlayer` carries no GM-feedback field at all — this
//! module sends its own immediate feedback to the *caller* right after the
//! send is confirmed, exactly like the native handlers, addressed to
//! `caller_id` rather than always the moved entity (D03).

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `.gotoxyz <x> <y> <z>` — teleport the selected target (or the caller, if
/// nothing is selected) to the given coordinates within its current space.
///
/// `Target::None` with in-handler `target.unwrap_or(caller_id)` resolution —
/// unlike most typed dot commands, legacy `gotoXYZ` makes the target
/// optional with a caller fallback rather than requiring a selection.
pub(super) async fn goto_xyz(
    caller_id: u32,
    target: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // `parse_f32` already rejects non-finite (NaN/inf) values — matches the
    // native `gmGotoXYZ`'s separate finite check via the console's shared
    // parse layer instead of duplicating it here.
    let Some(x) = super::parse_f32(caller_id, args, 0, "x", tx).await else {
        return;
    };
    let Some(y) = super::parse_f32(caller_id, args, 1, "y", tx).await else {
        return;
    };
    let Some(z) = super::parse_f32(caller_id, args, 2, "z", tx).await else {
        return;
    };
    let position = [x, y, z];

    let entity = target.unwrap_or(caller_id);
    let Some(e) = space_mgr.get_entity(entity) else {
        send_gm_feedback(caller_id, "gotoxyz: entity not found", tx).await;
        return;
    };
    let space_id = e.space_id.0 as u32;
    let prev_pos = [e.position.x, e.position.y, e.position.z];
    let is_player = e.is_player;

    tracing::info!(
        caller_id,
        entity,
        ?position,
        space_id,
        is_player,
        "GM .gotoxyz: moving entity"
    );

    // Keep the spatial grid consistent first (writes cell_entity.position and
    // the AoI-relevant grid index — witnesses pick this up on the next AoI
    // tick regardless of player/NPC), then send the authoritative snap for a
    // player target only.
    space_mgr.update_entity_position(entity, position, [0, 0, 0], [0.0; 3]);
    space_mgr.note_authorized_teleport(entity);

    if is_player {
        if let Err(err) = tx
            .send(CellToBaseMsg::TeleportPlayer {
                entity_id: entity,
                space_id,
                position,
                prev_pos,
            })
            .await
        {
            tracing::warn!(
                caller_id,
                entity,
                error = %err,
                "gotoxyz: base channel closed, snap not sent"
            );
            return; // don't claim a snap that never sent.
        }
    }

    send_gm_feedback(
        caller_id,
        &format!("gotoxyz: moved entity {entity} to ({x}, {y}, {z})"),
        tx,
    )
    .await;
}
