//! `gmGotoLocation` — cross-world teleport of the calling GM via `GateTravel`.

use cimmeria_cell_world::cell::plugin::EntityHookPoint;
use tokio::sync::mpsc;

use super::super::feedback::send_gm_feedback;
use super::read_xyz;
use crate::cell::console::travel::named_destination::{entry_suffix, no_entry_point_line};
use crate::cell::console::travel::world_entry_point::world_entry_point;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::read_wstring;

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
pub(in crate::cell::console::gm) async fn handle_goto_location(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // The calling GM's Rule 5 identity, carried on every row below.
    let who = space_mgr.player_identity(entity_id);
    let (world_name, consumed) = match read_wstring(args, 0) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                error = %e,
                "gmGotoLocation: malformed WorldName WSTRING",
            );
            send_gm_feedback(entity_id, "gmGotoLocation: malformed WorldName", tx).await;
            return true;
        }
    };
    let Some(position) = read_xyz(args, consumed) else {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            args_len = args.len(),
            "gmGotoLocation: truncated args (need WSTRING + 3×FLOAT)"
        );
        send_gm_feedback(
            entity_id,
            "gmGotoLocation: need WorldName + 3 FLOAT coordinates",
            tx,
        )
        .await;
        return true;
    };
    if world_name.trim().is_empty() {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            "gmGotoLocation: empty world name rejected"
        );
        send_gm_feedback(entity_id, "gmGotoLocation: empty world name rejected", tx).await;
        return true;
    }
    if !position.iter().all(|c| c.is_finite()) {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            ?position,
            "gmGotoLocation: non-finite coordinate rejected"
        );
        send_gm_feedback(
            entity_id,
            "gmGotoLocation: non-finite coordinate rejected",
            tx,
        )
        .await;
        return true;
    }
    if space_mgr.get_entity(entity_id).is_none() {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            "gmGotoLocation: caller entity not found"
        );
        send_gm_feedback(entity_id, "gmGotoLocation: caller entity not found", tx).await;
        return true;
    }

    // `(0, 0, 0)` means "the world's entry point" — the same one
    // `.gotolocation <world>` uses. The wire signature always carries three
    // floats, so this sentinel is the only way to say "no coordinates" from
    // `/gmgotolocation <world>` (typed as `<world> 0 0 0`, or zero-filled by
    // the client). The origin is never a deliberate destination — the same
    // reading `respawner_fallback::is_unauthored` gives it. Deliberate
    // deviation: owner request 2026-09-27. An unknown world keeps the old
    // path, where the base side refuses it.
    let mut position = position;
    let mut entry_source = None;
    if position == [0.0; 3] {
        if let Some(canonical) = space_mgr.canonical_world_name(&world_name) {
            match world_entry_point(space_mgr, canonical) {
                Some((entry, source)) => {
                    position = entry;
                    entry_source = Some(source);
                }
                None => {
                    tracing::warn!(
                        entity_id,
                        entity_name = space_mgr.entity_label(entity_id),
                        account_id = who.account_id,
                        account_name = who.account_name,
                        player_id = who.player_id,
                        player_name = who.player_name,
                        world = %world_name,
                        reason = "no_entry_point",
                        "gmGotoLocation: origin requested and the world has no entry point",
                    );
                    send_gm_feedback(
                        entity_id,
                        &format!("gmGotoLocation: {}", no_entry_point_line(canonical)),
                        tx,
                    )
                    .await;
                    return true;
                }
            }
        }
    }

    tracing::info!(
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        world = %world_name,
        ?position,
        entry_source = ?entry_source,
        "gmGotoLocation: cross-world teleport via GateTravel",
    );
    // Feedback wording is captured before the move because `world_name` is moved
    // into the GateTravel message below.
    let feedback = format!(
        "gmGotoLocation: teleporting to {} ({}, {}, {}){}",
        world_name,
        position[0],
        position[1],
        position[2],
        entry_suffix(entry_source)
    );
    // SS-D3, #962: the travel hook. The duel plugin ends the traveller's duel
    // (`EDUEL_DEFEAT_Teleport`) here; `every_travel_site_fires_the_travel_hook`.
    space_mgr
        .fire_entity_hook(EntityHookPoint::BeforeTravelSend, entity_id, tx)
        .await;
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
            // Native gmGotoLocation has no instance selector; the base side
            // resolves the destination by world name.
            destination_space_id: None,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            "gmGotoLocation: GateTravel enqueue failed; entity left in place"
        );
        send_gm_feedback(entity_id, "gmGotoLocation: transfer enqueue failed", tx).await;
        return true;
    }
    // Pets stay behind (D-PT01); gone before the GM's destroy.
    cimmeria_cell_world::cell::pets::on_owner_left(
        entity_id,
        cimmeria_cell_world::cell::pets::PetDespawnReason::OwnerLeftSpace,
        cimmeria_cell_world::cell::pets::OwnerPath::GmTravel,
        tx,
        space_mgr,
    )
    .await;
    space_mgr.destroy_entity(entity_id);
    send_gm_feedback(entity_id, &feedback, tx).await;
    true
}
