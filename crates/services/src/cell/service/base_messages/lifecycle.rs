//! Entity-lifecycle `BaseToCellMsg` handlers: create / destroy / connect /
//! disconnect. These arms manage the cell-side entity's presence in a space —
//! allocation on create, AoI introduction on connect, and the trade-cancel +
//! bandolier-ammo-flush teardown that both the destroy and disconnect paths
//! share. Extracted from `base_messages/mod.rs` as a pure code move.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;

/// Handle `BaseToCellMsg::CreateEntity`.
pub(super) async fn handle_create_entity(
    entity_id: u32,
    world_name: String,
    position: [f32; 3],
    rotation: [f32; 3],
    reply_tx: tokio::sync::oneshot::Sender<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    spawn_records: &[spawner::SpawnRecord],
) {
    tracing::debug!(entity_id, %world_name, ?position, "CreateEntity");

    // For instanced worlds, every CreateEntity gets a new space with its
    // own NPCs. For non-instanced worlds, the space already exists from
    // startup and NPCs were spawned then.
    let is_instanced = space_mgr.is_world_instanced(&world_name);

    match space_mgr.create_entity(entity_id, &world_name, position, rotation) {
        Ok(space_id) => {
            if is_instanced {
                // Notify BaseApp about the new instanced space so it can
                // route entity messages to it
                let _ = tx
                    .send(CellToBaseMsg::SpaceData {
                        space_id,
                        world_name: world_name.clone(),
                    })
                    .await;

                let npc_count = spawner::spawn_instance_npcs_from_records(
                    spawn_records,
                    &world_name,
                    space_id,
                    space_mgr,
                );
                if npc_count > 0 {
                    tracing::info!(world = %world_name, space_id, npc_count, "Spawned instance NPCs");
                }
            }

            let _ = reply_tx.send(space_id);
            let _ = tx
                .send(CellToBaseMsg::EntityCreated {
                    entity_id,
                    space_id,
                    position,
                })
                .await;
        }
        Err(e) => {
            tracing::error!(entity_id, %world_name, "Failed to create entity: {e}");
        }
    }
}

/// Handle `BaseToCellMsg::DestroyEntity`.
pub(super) async fn handle_destroy_entity(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    tracing::debug!(entity_id, "DestroyEntity");
    // Cancel any open trade BEFORE the rest of the teardown: the
    // surviving partner needs an onTradeResults(Cancelled) +
    // their own trade state cleared, otherwise their session
    // becomes a stranded ghost. Python relied on BigWorld GC for
    // this; Rust has to do it explicitly (deep dive gap).
    crate::cell::cell_methods::player::trade::cancel_trade_on_disconnect(entity_id, tx, space_mgr)
        .await;
    // Stage D: flush any pending bandolier ammo writes before tearing
    // down the entity. Logout is a hard boundary — anything still in
    // `bandolier_ammo_dirty` after this is lost.
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        if let Some(player_id) = entity.player_id {
            crate::cell::cell_methods::inventory::flush_dirty_bandolier_ammo(entity, player_id, tx)
                .await;
        }
    }
    space_mgr.destroy_entity(entity_id);
}

/// Handle `BaseToCellMsg::ConnectEntity`.
pub(super) async fn handle_connect_entity(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    tracing::debug!(entity_id, "ConnectEntity (player)");
    space_mgr.connect_entity(entity_id);
    // Introduce the just-connected player to everything already in
    // range immediately, rather than waiting for the next AoI tick.
    // The cell loop's `select!` can run an AoI tick before this
    // `ConnectEntity` is processed; that tick skips the space because
    // the player isn't in `space.players` yet, so NPCs spawned during
    // instance creation (e.g. the Castle_CellBlock stasis-room corpses)
    // would otherwise stay un-introduced until a later tick or a relog.
    for event in space_mgr.compute_aoi_changes_for_player(entity_id) {
        if let Err(e) = tx.send(event).await {
            tracing::warn!(
                entity_id,
                error = %e,
                "ConnectEntity: AoI introduction send failed — \
                 player may see a delayed entity population"
            );
            break;
        }
    }
}

/// Handle `BaseToCellMsg::DisconnectEntity`.
pub(super) async fn handle_disconnect_entity(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    tracing::debug!(entity_id, "DisconnectEntity");
    // Same as DestroyEntity: tear down any open trade with
    // Cancelled before the entity is removed. The disconnect
    // path doesn't reach the DestroyEntity arm directly (it
    // calls `space_mgr.disconnect_entity` which internally calls
    // destroy_entity), so this hook lives in both places — a
    // disconnect mid-trade has to notify the surviving partner.
    crate::cell::cell_methods::player::trade::cancel_trade_on_disconnect(entity_id, tx, space_mgr)
        .await;
    // Flush dirty bandolier ammo BEFORE space_mgr.disconnect_entity,
    // which internally calls destroy_entity. Without this, the entity
    // is gone by the time DestroyEntity arrives next and its flush
    // is a silent no-op — that's why per-slot ammo and the loaded
    // state never persisted across a logoff.
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        if let Some(player_id) = entity.player_id {
            crate::cell::cell_methods::inventory::flush_dirty_bandolier_ammo(entity, player_id, tx)
                .await;
        }
    }
    space_mgr.disconnect_entity(entity_id, tx).await;
}
