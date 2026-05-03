//! Dispatch handler for `BaseToCellMsg` variants — the per-message logic that
//! the cell loop runs on each inbound base message.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::super::content;
use super::super::messages::{BaseToCellMsg, CellToBaseMsg};
use super::super::space_manager::SpaceManager;
use super::super::{chat, dispatch, spawner};

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
            saved_missions,
            abilities,
            active_bandolier_slot,
            bandolier_items,
        } => {
            tracing::debug!(entity_id, player_id, %world_name, saved_count = saved_missions.len(), ability_count = abilities.len(), "InitPlayerState");
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                entity.player_id = Some(player_id);

                // Register player's known abilities on the server-side entity
                for &ability_id in &abilities {
                    entity.abilities.add_ability(ability_id);
                }
                tracing::debug!(
                    entity_id,
                    count = abilities.len(),
                    "Registered player abilities on cell entity"
                );

                // Apply bandolier state to entity (Bug #2: restore persisted bandolier slot and items)
                entity.active_bandolier_slot = active_bandolier_slot;
                entity.bandolier_items = bandolier_items.into_iter().collect();
                tracing::debug!(
                    entity_id,
                    active_bandolier_slot,
                    bandolier_item_count = entity.bandolier_items.len(),
                    "Applied bandolier state to cell entity"
                );

                // Stage B: Seed each populated bandolier slot's AmmoSlot{N} stat
                // from its persisted current_ammo / clip_size. The default stat
                // tuple is (0,0,0), and `set_slot_ammo` clamps via the stat
                // bounds — without this seed, every later refill/decrement
                // would silently pin to 0. Clearing dirty avoids a duplicate
                // stat send (the initial mapLoaded uses serialize_all()).
                let slot_seed: Vec<(i32, i32, i32)> = entity
                    .bandolier_items
                    .iter()
                    .map(|(&slot, item)| (slot, item.current_ammo, item.clip_size))
                    .collect();
                for (slot_id, current, clip) in slot_seed {
                    let stat_id = cimmeria_entity::stats::AMMO_SLOT_1 + slot_id;
                    if let Some(stat) = entity.stats.get_mut(stat_id) {
                        stat.update(0, current, clip);
                        stat.clear_dirty();
                    }
                }

                // Restore saved missions BEFORE content engine fires, so that
                // chain conditions correctly see existing mission state and
                // don't re-trigger already-active or completed missions.
                for saved in &saved_missions {
                    use cimmeria_entity::missions::{
                        MissionInstance, MissionObjective, STATUS_ACTIVE, STATUS_COMPLETED,
                    };
                    let objectives: Vec<MissionObjective> = saved
                        .active_objective_ids
                        .iter()
                        .map(|&oid| {
                            let status = if saved.completed_objective_ids.contains(&oid) {
                                STATUS_COMPLETED
                            } else {
                                STATUS_ACTIVE
                            };
                            MissionObjective {
                                objective_id: oid,
                                status,
                                hidden: false,
                                optional: false,
                            }
                        })
                        .collect();

                    let mut mission = MissionInstance::new(
                        saved.mission_id,
                        saved.current_step_id.unwrap_or(0),
                        objectives,
                    );
                    mission.status = saved.status;
                    mission.completed_steps = saved.completed_step_ids.clone();
                    mission.completed_objectives = saved.completed_objective_ids.clone();
                    // Without this, `complete()` on a re-accepted repeatable
                    // mission post-relog would jump from 0 -> 1 instead of
                    // N -> N+1, defeating the numRepeats cap. (#118)
                    mission.repeats = saved.repeats;

                    entity.missions.add_mission(mission);
                    tracing::debug!(
                        entity_id,
                        mission_id = saved.mission_id,
                        status = saved.status,
                        "Restored saved mission"
                    );
                }
                entity.saved_missions_loaded = true;
            }

            // Send addClientHintedGenericRegion for each client-hinted region in
            // this world. Matches Python Space.playerEntered() → queryRegions():
            // clearClientHintedGenericRegions was already sent in mapLoaded body,
            // now register all regions so the client can fire triggerRegion events.
            {
                use super::super::space_manager::REGION_FLAG_CLIENT_HINTED;
                let world_regions: Vec<_> = space_mgr
                    .regions_for_world(&world_name)
                    .iter()
                    .filter(|r| r.flags & REGION_FLAG_CLIENT_HINTED != 0)
                    .map(|r| (r.runtime_id, r.height, r.radius, r.flags, r.points.clone()))
                    .collect();

                let region_count = world_regions.len();
                for (rid, height, radius, flags, points) in world_regions {
                    let mut args = Vec::with_capacity(16 + points.len() * 12);
                    args.extend_from_slice(&(rid as i32).to_le_bytes());
                    args.extend_from_slice(&height.to_le_bytes());
                    args.extend_from_slice(&radius.to_le_bytes());
                    args.extend_from_slice(&flags.to_le_bytes());
                    args.extend_from_slice(&(points.len() as u32).to_le_bytes()); // ARRAY count
                    for p in &points {
                        args.extend_from_slice(&p[0].to_le_bytes()); // x
                        args.extend_from_slice(&p[1].to_le_bytes()); // y
                        args.extend_from_slice(&p[2].to_le_bytes()); // z
                    }
                    let _ = tx
                        .send(CellToBaseMsg::EntityMethodCall {
                            entity_id,
                            method_index: 125, // addClientHintedGenericRegion
                            args,
                        })
                        .await;
                }
                if region_count > 0 {
                    tracing::info!(
                        entity_id, player_id, world = %world_name,
                        count = region_count, "Sent region registrations"
                    );
                }
            }

            content::fire_player_loaded(entity_id, player_id, &world_name, engine, tx, space_mgr)
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
                        *chain_id as i64,
                        entity_id,
                        player_id,
                        engine,
                        tx,
                        space_mgr,
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
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                entity.bandolier_items.insert(slot_id, item);
                if make_active {
                    entity.active_bandolier_slot = slot_id;
                }
            }
        }

        BaseToCellMsg::SyncBandolierItems {
            entity_id,
            active_bandolier_slot,
            bandolier_items,
        } => {
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                entity.active_bandolier_slot = active_bandolier_slot;
                entity.bandolier_items = bandolier_items.into_iter().collect();

                // Re-seed AmmoSlot{N} stats from the new bandolier set so the
                // client's bandolier UI reflects the actual ammo of any newly-
                // equipped weapon. Without this, post-vendor-buy or post-grant
                // bars show stale stats from the previous weapon.
                //
                // Slots that disappeared from `bandolier_items` get reset to
                // (0, 0, 0) so an empty slot's bar clears.
                let new_states: Vec<(i32, i32, i32)> = (0..5)
                    .map(|slot_id| {
                        let (cur, max) = entity
                            .bandolier_items
                            .get(&slot_id)
                            .map_or((0, 0), |item| (item.current_ammo, item.clip_size));
                        (slot_id, cur, max)
                    })
                    .collect();
                for (slot_id, cur, max) in new_states {
                    let stat_id = cimmeria_entity::stats::AMMO_SLOT_1 + slot_id;
                    if let Some(stat) = entity.stats.get_mut(stat_id) {
                        stat.update(0, cur, max);
                    }
                }
                // Push the dirty stats to the client immediately so the UI
                // updates without waiting for the next stat broadcast.
                let payload = entity.stats.serialize_dirty();
                entity.stats.clear_dirty();
                if !payload.is_empty() {
                    super::super::abilities::send_entity_method(
                        entity_id, 20, payload, tx, space_mgr,
                    )
                    .await;
                }
            }
        }

        // Bandolier state is re-synced via SyncBandolierItems; these handlers are
        // logging-only — base owns the inventory mutation, cell only learns about it.
        BaseToCellMsg::InventoryItemMoveApplied {
            entity_id,
            item_id,
            source_container_id,
            target_container_id,
            swapped_item_id,
        } => {
            tracing::debug!(entity_id, item_id, source = source_container_id, target = target_container_id, swapped_item_id = ?swapped_item_id, "Item moved in inventory");
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
            type_id,
            target_id,
        } => {
            // Base committed the consumption transaction; fire `OnItemUse` so
            // any chain conditioned on `item_use::<type_id>` can run. Mission
            // progression that gates on this only advances after the vial is
            // actually consumed — if base failed to consume, this event never
            // arrives.
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
                type_id,
                target_id,
                "ItemUsed: firing OnItemUse"
            );
            content::fire_item_use(entity_id, player_id, type_id, engine, tx, space_mgr).await;
        }
    }
}
