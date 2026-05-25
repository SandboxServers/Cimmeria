//! Dispatch handler for `BaseToCellMsg` variants — the per-message logic that
//! the cell loop runs on each inbound base message.
//!
//! Large handler bodies are extracted into submodules by variant family:
//! - [`player_init`] — `InitPlayerState` (mission/ability/bandolier restore)
//! - [`bandolier`] — `UpdateBandolierItem` + `SyncBandolierItems` (weapon display)

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::super::content;
use super::super::messages::{BaseToCellMsg, CellToBaseMsg};
use super::super::space_manager::SpaceManager;
use super::super::{chat, dispatch, spawner};

mod bandolier;
mod player_init;
mod request_entity_update;

#[cfg(test)]
mod tests;

/// Handle a single message from BaseApp.
pub(super) async fn handle_base_message(
    msg: BaseToCellMsg,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
    spawn_records: &[spawner::SpawnRecord],
) {
    match msg {
        BaseToCellMsg::CreateEntity {
            entity_id,
            world_name,
            position,
            rotation,
            reply_tx,
        } => {
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

        BaseToCellMsg::DestroyEntity { entity_id } => {
            tracing::debug!(entity_id, "DestroyEntity");
            // Stage D: flush any pending bandolier ammo writes before tearing
            // down the entity. Logout is a hard boundary — anything still in
            // `bandolier_ammo_dirty` after this is lost.
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                if let Some(player_id) = entity.player_id {
                    super::super::cell_methods::inventory::flush_dirty_bandolier_ammo(
                        entity, player_id, tx,
                    )
                    .await;
                }
            }
            space_mgr.destroy_entity(entity_id);
        }

        BaseToCellMsg::ConnectEntity { entity_id } => {
            tracing::debug!(entity_id, "ConnectEntity (player)");
            space_mgr.connect_entity(entity_id);
        }

        BaseToCellMsg::DisconnectEntity { entity_id } => {
            tracing::debug!(entity_id, "DisconnectEntity");
            // Flush dirty bandolier ammo BEFORE space_mgr.disconnect_entity,
            // which internally calls destroy_entity. Without this, the entity
            // is gone by the time DestroyEntity arrives next and its flush
            // is a silent no-op — that's why per-slot ammo and the loaded
            // state never persisted across a logoff.
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                if let Some(player_id) = entity.player_id {
                    super::super::cell_methods::inventory::flush_dirty_bandolier_ammo(
                        entity, player_id, tx,
                    )
                    .await;
                }
            }
            space_mgr.disconnect_entity(entity_id, tx).await;
        }

        BaseToCellMsg::EntityMove {
            entity_id,
            position,
            direction,
            velocity,
        } => {
            tracing::trace!(entity_id, ?position, "EntityMove");
            space_mgr.update_entity_position(entity_id, position, direction, velocity);
        }

        BaseToCellMsg::CellMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            dispatch::dispatch_cell_method(entity_id, method_index, &args, tx, space_mgr, engine)
                .await;
        }

        BaseToCellMsg::ChatMessage {
            entity_id,
            speaker_name,
            speaker_flags,
            channel,
            text,
        } => {
            chat::handle_chat_message(
                entity_id,
                &speaker_name,
                speaker_flags,
                channel,
                &text,
                tx,
                space_mgr,
            )
            .await;
        }

        BaseToCellMsg::InitPlayerState {
            entity_id,
            player_id,
            world_name,
            archetype_id,
            saved_missions,
            abilities,
            active_bandolier_slot,
            bandolier_items,
        } => {
            player_init::handle_init_player_state(
                entity_id,
                player_id,
                world_name,
                archetype_id,
                saved_missions,
                abilities,
                active_bandolier_slot,
                bandolier_items,
                tx,
                space_mgr,
                engine,
            )
            .await;
        }

        BaseToCellMsg::AdvanceRingDestination {
            entity_id,
            region_id,
        } => {
            // Cross-world ring transport: the destination ring has been
            // sitting in `RemoteLoadWait` since the source ring's
            // `Effect::TeleportCrossWorld` fired. Now that the player has
            // finished loading on this world (base-side `onClientReady`
            // ack), advance the destination FSM by recording the load —
            // `mark_player_loaded` (called inside
            // `handle_remote_player_loaded`) triggers the same
            // all-players-loaded / remote-warmup / cooldown chain the
            // same-world path runs synchronously after
            // `Effect::TeleportPlayer`.
            super::super::ring_transport::handle_remote_player_loaded(
                region_id, entity_id, tx, space_mgr, engine,
            )
            .await;
        }

        BaseToCellMsg::ReloadContentEngine => {}

        BaseToCellMsg::MinigameResult {
            entity_id,
            result_code,
            on_victory_chains,
        } => {
            tracing::info!(entity_id, result_code, chains = ?on_victory_chains, "Minigame result");
            if result_code == 1 {
                // Victory — fire on_victory_chains through the content engine
                let player_id = space_mgr
                    .get_entity(entity_id)
                    .and_then(|e| e.player_id)
                    .unwrap_or(0);
                for chain_id in &on_victory_chains {
                    content::fire_chain_by_id(
                        *chain_id, entity_id, player_id, engine, tx, space_mgr,
                    )
                    .await;
                }
            }
        }

        BaseToCellMsg::UpdateBandolierItem {
            entity_id,
            slot_id,
            item,
            make_active,
        } => {
            bandolier::handle_update_bandolier_item(
                entity_id,
                slot_id,
                item,
                make_active,
                tx,
                space_mgr,
            )
            .await;
        }

        BaseToCellMsg::SyncBandolierItems {
            entity_id,
            active_bandolier_slot,
            bandolier_items,
        } => {
            bandolier::handle_sync_bandolier_items(
                entity_id,
                active_bandolier_slot,
                bandolier_items,
                tx,
                space_mgr,
            )
            .await;
        }

        // Bandolier state is re-synced via SyncBandolierItems; this handler also
        // fires the `OnItemEquipped` content event when an item lands in the
        // bandolier from a non-bandolier container, so quest chains keyed on
        // `item_equipped::<type_id>` can advance (mission 622 pistol, mission
        // 641 P90).
        BaseToCellMsg::InventoryItemMoveApplied {
            entity_id,
            item_id,
            type_id,
            source_container_id,
            target_container_id,
            swapped_item_id,
        } => {
            tracing::debug!(entity_id, item_id, type_id, source = source_container_id, target = target_container_id, swapped_item_id = ?swapped_item_id, "Item moved in inventory");

            const INV_BANDOLIER: i32 = 3;
            if target_container_id == INV_BANDOLIER && source_container_id != INV_BANDOLIER {
                let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
                    Some(pid) => pid,
                    None => {
                        tracing::warn!(
                            entity_id,
                            type_id,
                            "InventoryItemMoveApplied: entity has no player_id — equip event dropped"
                        );
                        return;
                    }
                };
                content::fire_item_equipped(entity_id, player_id, type_id, engine, tx, space_mgr)
                    .await;
            }
        }

        BaseToCellMsg::InventoryItemRemoved {
            entity_id,
            item_id,
            source_container_id,
        } => {
            tracing::debug!(
                entity_id,
                item_id,
                source = source_container_id,
                "Item removed from inventory"
            );
        }

        BaseToCellMsg::InventoryItemGranted {
            entity_id,
            item_id,
            container_id,
            slot_id,
            quantity,
        } => {
            tracing::debug!(
                entity_id,
                item_id,
                container_id,
                slot_id,
                quantity,
                "Item granted to player"
            );
        }

        BaseToCellMsg::ItemUsed {
            entity_id,
            instance_id,
            type_id,
            target_id,
        } => {
            // Base verified ownership and forwarded the use event. Fire
            // `OnItemUse` so any chain conditioned on `item_use::<type_id>`
            // can run. The chain decides whether to consume (via
            // `Action::RemoveItem`) — base does NOT consume before this
            // message, the historical comment about a "consumption tx"
            // pre-dated the chain-decides-consumption design.
            let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
                Some(pid) => pid,
                None => {
                    tracing::warn!(
                        entity_id,
                        type_id,
                        "ItemUsed: entity has no player_id — content event dropped"
                    );
                    return;
                }
            };
            tracing::debug!(
                entity_id,
                player_id,
                instance_id,
                type_id,
                target_id,
                "ItemUsed: firing OnItemUse"
            );
            content::fire_item_use(
                entity_id,
                player_id,
                instance_id,
                type_id,
                engine,
                tx,
                space_mgr,
            )
            .await;
        }

        BaseToCellMsg::RequestEntityUpdate {
            witness_id,
            entity_ids,
        } => {
            request_entity_update::handle(witness_id, entity_ids, tx, space_mgr).await;
        }
    }
}
