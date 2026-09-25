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
    destination_space_id: Option<u32>,
    account_id: Option<u32>,
    player_id: Option<i32>,
    reply_tx: tokio::sync::oneshot::Sender<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    spawn_records: &[spawner::SpawnRecord],
) {
    tracing::debug!(
        entity_id,
        account_id,
        player_id,
        %world_name,
        ?position,
        ?destination_space_id,
        "CreateEntity"
    );

    // An explicit destination instance (GM cross-instance transfer, see
    // `crate::cell::space_transfer`) is re-validated HERE, not trusted from
    // the message: the cell loop keeps running between the transfer's
    // validation and this message arriving, and an instanced space is
    // destroyed the moment its last player leaves. A stale or foreign id
    // degrades to by-world-name resolution — landing in the wrong instance
    // of the right world is recoverable, landing in no space is not.
    let joined_instance =
        destination_space_id.and_then(|sid| match space_mgr.world_name_for_space(sid) {
            Some(w) if w == world_name => Some(sid),
            Some(other) => {
                tracing::warn!(
                    entity_id, requested_space_id = sid, %world_name, actual_world = %other,
                    "CreateEntity: requested destination instance belongs to another world — \
                     falling back to by-world-name resolution"
                );
                None
            }
            None => {
                tracing::warn!(
                    entity_id, requested_space_id = sid, %world_name,
                    "CreateEntity: requested destination instance is no longer loaded \
                     (last player left mid-transfer) — falling back to by-world-name resolution"
                );
                None
            }
        });

    // Joining an existing instance must NOT re-announce the space or re-spawn
    // its NPCs — both already happened when that instance was created.
    // For instanced worlds, every *fresh* CreateEntity gets a new space with
    // its own NPCs. For non-instanced worlds, the space already exists from
    // startup and NPCs were spawned then.
    let is_instanced = joined_instance.is_none() && space_mgr.is_world_instanced(&world_name);

    let created = match joined_instance {
        Some(sid) => space_mgr.create_entity_in_space(entity_id, sid, position, rotation),
        None => space_mgr.create_entity(entity_id, &world_name, position, rotation),
    };

    match created {
        Ok(space_id) => {
            // Identity-stamp the entity the moment it exists, before any
            // other handler can observe it. `InitPlayerState` re-asserts the
            // same pair later, but that only arrives after `onClientReady` —
            // stamping here is what lets world-entry movement rejects and the
            // gate-travel destination entity carry the account. NPCs pass
            // `None`/`None` and are left UNKNOWN.
            if account_id.is_some() || player_id.is_some() {
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    entity.account_id = account_id;
                    entity.player_id = player_id;
                } else {
                    // The create reported success, so the entity must be
                    // resolvable; if it isn't, every subsequent log for this
                    // session silently loses its identity fields.
                    tracing::warn!(
                        entity_id,
                        account_id,
                        player_id,
                        space_id,
                        reason = "identity_stamp_entity_missing",
                        "CreateEntity: entity absent immediately after a successful create -- \
                         session logs will carry no account_id/player_id until InitPlayerState"
                    );
                }
            }

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
            // KNOWN GAP: `reply_tx` is `Sender<u32>` with no failure channel,
            // so dropping it here makes the base side fall back to the
            // hardcoded `resolve_space_id_fallback` table and build a
            // world-entry packet for a space this entity is NOT in. Cross-world
            // GM transfer (`crate::cell::space_transfer`) validates the world
            // and the destination instance BEFORE teardown precisely so this
            // arm stays unreachable for that path; widening the oneshot to a
            // `Result` is the real fix and is tracked for the gate-travel owner.
            tracing::error!(
                entity_id, account_id, player_id, %world_name, ?destination_space_id,
                "Failed to create entity: {e} — entity is in NO space; \
                 base will fall back to a hardcoded space id"
            );
        }
    }
}

/// Handle `BaseToCellMsg::DestroyEntity`.
pub(super) async fn handle_destroy_entity(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Resolve identity BEFORE the teardown below removes the entity —
    // afterwards `player_identity` can only return UNKNOWN.
    let id = space_mgr.player_identity(entity_id);
    tracing::debug!(
        entity_id,
        account_id = id.account_id,
        player_id = id.player_id,
        "DestroyEntity"
    );
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
    flush_and_destroy(entity_id, tx, space_mgr).await;
}

/// Handle `BaseToCellMsg::ConnectEntity`.
pub(super) async fn handle_connect_entity(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    tracing::debug!(
        entity_id,
        account_id = id.account_id,
        player_id = id.player_id,
        "ConnectEntity (player)"
    );
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
                account_id = id.account_id,
                player_id = id.player_id,
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
    // Resolved before teardown — this is the last log line of the session
    // that can still name the account, so losing it here is what forced the
    // wall-clock correlation this convention replaces.
    let id = space_mgr.player_identity(entity_id);
    tracing::debug!(
        entity_id,
        account_id = id.account_id,
        player_id = id.player_id,
        "DisconnectEntity"
    );
    // Same as DestroyEntity: tear down any open trade with
    // Cancelled before the entity is removed. The disconnect
    // path doesn't reach the DestroyEntity arm directly (it
    // calls `space_mgr.disconnect_entity` which internally calls
    // destroy_entity), so this hook lives in both places — a
    // disconnect mid-trade has to notify the surviving partner.
    crate::cell::cell_methods::player::trade::cancel_trade_on_disconnect(entity_id, tx, space_mgr)
        .await;
    // Persist the last known world + position BEFORE the teardown below
    // removes the entity. This is the only write of `sgw_player.pos_*` on
    // the way out of a session: gate travel and the GM teleport write their
    // destination, nothing else does, so without this a returning character
    // spawns at the last gate arrival or the creation point rather than
    // where they logged out. Players only — NPCs have no row to update.
    persist_last_position(entity_id, tx, space_mgr).await;
    // Flush dirty bandolier ammo BEFORE space_mgr.disconnect_entity,
    // which internally calls destroy_entity. Without this, the entity
    // is gone by the time DestroyEntity arrives next and its flush
    // is a silent no-op — that's why per-slot ammo and the loaded
    // state never persisted across a logoff.
    flush_and_disconnect(entity_id, tx, space_mgr).await;
}

/// Flush dirty bandolier ammo, then destroy the entity.
///
/// The flush must happen before the entity is removed: once it is gone
/// `bandolier_ammo_dirty` can no longer be read and the writes are lost.
/// Extracted so the `DestroyEntity` handler and its regression guard share
/// one code path — a test that re-implements this order cannot catch a
/// handler that swaps it.
pub(in crate::cell::service) async fn flush_and_destroy(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    flush_bandolier_ammo_for_entity(entity_id, tx, space_mgr).await;
    space_mgr.destroy_entity(entity_id);
}

/// Flush dirty bandolier ammo, then disconnect the entity.
///
/// `SpaceManager::disconnect_entity` internally destroys the entity, so the
/// flush has to happen first or it becomes a silent no-op and per-slot ammo
/// never persists across a logoff.
pub(in crate::cell::service) async fn flush_and_disconnect(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    flush_bandolier_ammo_for_entity(entity_id, tx, space_mgr).await;
    space_mgr.disconnect_entity(entity_id, tx).await;
}

/// Flush any dirty bandolier ammo rows for a player-owned entity.
async fn flush_bandolier_ammo_for_entity(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        if let Some(player_id) = entity.player_id {
            crate::cell::cell_methods::inventory::flush_dirty_bandolier_ammo(entity, player_id, tx)
                .await;
        }
    }
}

/// Queue `CellToBaseMsg::PersistPosition` for a player entity that is about
/// to be torn down. No-op for NPCs and for entities that never received
/// their `player_id` (a session that dropped before character select).
async fn persist_last_position(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(entity) = space_mgr.get_entity(entity_id) else {
        return;
    };
    let (true, Some(player_id)) = (entity.is_player, entity.player_id) else {
        return;
    };
    let Some(world_name) = space_mgr.get_entity_world_name(entity_id) else {
        tracing::warn!(
            entity_id,
            player_id,
            reason = "entity_has_no_space",
            "DisconnectEntity: player entity is in no space — last position not persisted"
        );
        return;
    };
    let position = [entity.position.x, entity.position.y, entity.position.z];
    if let Err(e) = tx
        .send(CellToBaseMsg::PersistPosition {
            player_id,
            world_name: world_name.clone(),
            position,
        })
        .await
    {
        tracing::warn!(
            entity_id,
            player_id,
            world = %world_name,
            ?position,
            error = %e,
            "DisconnectEntity: PersistPosition send failed — the next login will use \
             the previously stored position"
        );
    }
}
