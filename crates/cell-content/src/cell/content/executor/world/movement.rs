//! `Action::MoveEntity` / `Action::MoveWaypoint`: reposition the acting
//! player or a tagged NPC.

use cimmeria_cell_world::cell::service::npc_ai;
use tokio::sync::mpsc;

use super::super::transport;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `Action::MoveEntity` — reposition either the acting player or a
/// tagged NPC. One seed verb, two very different mechanisms:
///
/// - **`use_player: true`** (seed rows 3007 / 3028, both with
///   `target_key` NULL) moves the *player* who fired the chain. That has
///   to go through [`transport::teleport`], which owns the spatial-grid
///   update, the `note_authorized_teleport` validator reseed, the
///   `CellToBaseMsg::TeleportPlayer` forced-position snap and the
///   prev-position anti-camera-snap. `update_entity_position` alone
///   moves the server's idea of the player and nothing the client sees.
/// - **`entity_tag`** (rows 3005 / 3009 / 3011) repositions an NPC, which
///   is exactly [`move_waypoint`]'s job.
///
/// `use_player` wins when both are set — the seed never does that, but
/// "move the player" is the more specific instruction.
///
/// The `world` param is a cross-world guard, not a destination selector.
/// All five seeded rows name the world they are already on, so it
/// normally resolves to the same-world path; a genuine mismatch on the
/// player path routes to [`transport::cross_world_teleport`] instead of
/// silently dropping the avatar at those coordinates on the wrong map.
/// A mismatch on the NPC path is refused: NPCs have no gate-travel
/// equivalent, and snapping one to coordinates in a world it isn't in
/// would place it somewhere arbitrary.
pub(in crate::cell::content::executor) async fn move_entity(
    entity_tag: Option<String>,
    destination: [f32; 3],
    world: Option<String>,
    use_player: Option<bool>,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if use_player == Some(true) {
        let current_world = space_mgr.get_entity_world_name(entity_id);
        // Only a *known* mismatch counts. An unresolvable current world
        // (entity already gone) falls through to the same-world path,
        // which fail-softs on the missing entity rather than firing a
        // gate travel for a player the cell can't see.
        let cross_world = match (world.as_deref(), current_world.as_deref()) {
            (Some(want), Some(cur)) => want != cur,
            _ => false,
        };
        if cross_world {
            // `world` is Some in this branch by construction.
            let target_world = world.unwrap_or_default();
            tracing::info!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                %target_world,
                ?destination,
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                "Content: move_entity crosses worlds -- routing through gate travel"
            );
            transport::cross_world_teleport(
                target_world,
                destination,
                entity_id,
                chain_id,
                tx,
                space_mgr,
            )
            .await;
            return;
        }
        // Same-world player move. `transport::teleport` treats its
        // `space_id` as the *destination* space and warns on a mismatch,
        // so pass the player's current space to keep it on the
        // same-space path. `0` is its "unspecified" sentinel and is what
        // a missing entity degrades to.
        let space_id = space_mgr
            .get_entity(entity_id)
            .map(|e| e.space_id.0)
            .unwrap_or(0);
        transport::teleport(space_id, destination, entity_id, chain_id, tx, space_mgr).await;
        return;
    }

    let Some(entity_tag) = entity_tag else {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            ?destination,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            "MoveEntity: row has neither use_player nor target_key -- nothing moved"
        );
        return;
    };

    let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) else {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            %entity_tag,
            ?destination,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            "MoveEntity: no entity matched tag in the source entity's space -- NPC reposition skipped"
        );
        return;
    };

    if let (Some(want), Some(cur)) = (
        world.as_deref(),
        space_mgr.get_entity_world_name(target_id).as_deref(),
    ) {
        if want != cur {
            let tn = space_mgr.entity_names(target_id);
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                %entity_tag,
                target_id,
                target_name = tn.entity_name,
                template_id = tn.template_id,
                template_name = tn.template_name,
                want_world = %want,
                current_world = %cur,
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                "MoveEntity: cross-world NPC move is not supported -- NPC reposition skipped"
            );
            return;
        }
    }

    move_waypoint(entity_tag, destination, entity_id, chain_id, tx, space_mgr).await;
}

/// `Action::MoveWaypoint` — snap the tagged entity to a new position.
/// No yaw/orientation change; chains call `update_position_preserving_facing`
/// directly.
///
/// The snap is broadcast to the entity's current witnesses immediately as a
/// per-witness `EntityMoved`, so a scripted reposition is visible on the
/// next frame rather than whenever the 100ms AoI tick next relays ghost
/// positions. Witnesses the move drops entirely still get their `LeftAoI`
/// from that tick, so a long-distance reposition needs no extra fan-out
/// here. That immediate fan-out deliberately duplicates the AoI tick's own
/// `EntityMoved` relay; harmless while NPC `UPDATE_AVATAR`/`EntityMoved`
/// remains unreliable and self-correcting, but it would amplify position
/// updates if that path ever becomes reliable for NPCs.
pub(in crate::cell::content::executor) async fn move_waypoint(
    entity_tag: String,
    destination: [f32; 3],
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) else {
        return;
    };
    let tn = space_mgr.entity_names(target_id);
    tracing::debug!(
        entity_id,
        entity_name = space_mgr.entity_names(entity_id).entity_name,
        %entity_tag,
        target_id,
        target_name = tn.entity_name,
        template_id = tn.template_id,
        template_name = tn.template_name,
        ?destination,
        chain_id,
        chain_name = cimmeria_names::book().chain(chain_id),
        "Content: move waypoint"
    );
    let Some(t) = space_mgr.get_entity(target_id) else {
        return;
    };
    let space_id = t.space_id.0 as u32;
    let direction = [t.direction.x, t.direction.y, t.direction.z];
    space_mgr.update_position_preserving_facing(target_id, destination, [0.0; 3]);
    space_mgr
        .npc_detectors
        .note_move_source(target_id, npc_ai::detectors::MoveSource::Content);
    // Authorized server move: reseed the movement-validator clock for
    // the moved entity (harmless for NPC targets — they never pass
    // through the client-position validator).
    space_mgr.note_authorized_teleport(target_id);

    // Broadcast the snap to current witnesses now instead of waiting for
    // the AoI tick's next pass, so a chain-driven reposition (escort
    // arrival, tutorial staging) does not hold stale on the client for up
    // to 100ms. Witness sets are last-tick snapshots, so a witness the
    // move left behind still gets the snap before its `LeftAoI`, and a
    // player newly in range gets a full `EnteredAoI` — the tick completes
    // the picture either way. A failed send only delays the relay by one
    // tick, but it is still an expectation seam, so log it.
    let witnesses = space_mgr.get_witnesses_of(target_id);
    for witness_id in witnesses {
        if let Err(e) = tx
            .send(CellToBaseMsg::EntityMoved {
                witness_id,
                entity_id: target_id,
                space_id,
                position: destination,
                direction,
                velocity: [0.0; 3],
                npc_moved_since_last: None,
            })
            .await
        {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                witness_id,
                witness_name = space_mgr.entity_names(witness_id).entity_name,
                target_id,
                target_name = tn.entity_name,
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                reason = "move_waypoint_send_failed",
                "MoveWaypoint: cell→base send failed -- witness holds the stale \
                 position until the next AoI tick relays it: {e}"
            );
        }
    }
}
