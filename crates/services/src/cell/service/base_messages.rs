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
            archetype_id,
            saved_missions,
            abilities,
            active_bandolier_slot,
            bandolier_items,
        } => {
            tracing::debug!(entity_id, player_id, archetype_id, %world_name, saved_count = saved_missions.len(), ability_count = abilities.len(), "InitPlayerState");
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                entity.player_id = Some(player_id);
                entity.archetype_id = Some(archetype_id);

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
            // Display the newly-equipped weapon and arm the OOC holster
            // timer so it re-holsters after `OOC_HOLSTER_DELAY`. Mirrors
            // `python/cell/SGWPlayer.py:onItemEquipped` which fires
            // `playSequence(Item_Equip)` whenever an item lands in the
            // bandolier. Gated on `make_active` so only the *active*
            // weapon's equip plays the animation (non-active slots are
            // wire-invisible anyway).
            //
            // In-combat equip: weapon is already drawn,
            // `combat_exit_at` stays None (set by `exit_player_combat`
            // when the fight ends). We skip the timer arming here to
            // avoid stamping a timer while threatened_mobs is non-empty,
            // which the holster scan doesn't (yet) gate on combat state.
            let (play_equip_anim, drew_weapon, was_in_combat, entity_state, anim_path) =
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    entity.bandolier_items.insert(slot_id, item);
                    let is_player = entity.is_player;
                    let player_id = entity.player_id;
                    // Fire the equip-display animation when the inserted
                    // slot is (or just became) the player's *active*
                    // slot. Three cases land here:
                    //
                    // 1. `make_active=true` — base's bandolier_slot
                    //    UPDATE flipped to the new slot; we own
                    //    `active_bandolier_slot` from here.
                    // 2. `make_active=false` BUT `slot_id` already
                    //    equals `active_bandolier_slot` — this is the
                    //    initial-grant case (PR #338 follow-up): the
                    //    base's SQL doesn't flip bandolier_slot when
                    //    the INSERT already lands at the player's
                    //    default selection (the NOT EXISTS guard finds
                    //    the row we just inserted), so
                    //    `bandolier_became_active` is false even
                    //    though the new weapon IS the active one.
                    //    Without this branch the first weapon a
                    //    player ever gets never plays its equip
                    //    animation or attaches its mesh.
                    // 3. `make_active=false` AND `slot_id` !=
                    //    `active_bandolier_slot` — the player got a
                    //    second/third weapon into a non-active slot.
                    //    Mesh stays wherever the active slot is. Skip.
                    let slot_is_active = make_active || entity.active_bandolier_slot == slot_id;
                    let path = if make_active {
                        "make_active=true"
                    } else if slot_is_active {
                        "initial-grant (slot matches active)"
                    } else {
                        "non-active slot — skipping"
                    };
                    if slot_is_active {
                        entity.active_bandolier_slot = slot_id;
                        let in_combat = !entity.threatened_mobs.is_empty();
                        let drew = if !in_combat {
                            entity.set_weapon_holstered(false);
                            entity.combat_exit_at = Some(std::time::Instant::now());
                            // Cancel any in-flight holster Phase 2 — the
                            // player just (re-)equipped, the mesh should
                            // STAY attached; a stale Phase 2 would yank it
                            // away mid-equip.
                            entity.holster_animation_complete_at = None;
                            true
                        } else {
                            false
                        };
                        (true, drew, in_combat, (is_player, player_id), path)
                    } else {
                        (false, false, false, (is_player, player_id), path)
                    }
                } else {
                    (false, false, false, (false, None), "entity missing")
                };
            tracing::info!(
                entity_id,
                slot_id,
                make_active,
                play_equip_anim,
                drew_weapon,
                was_in_combat,
                is_player = entity_state.0,
                player_id = ?entity_state.1,
                anim_path,
                "UpdateBandolierItem: equip-display decision"
            );
            if play_equip_anim {
                // Appearance refresh first so the weapon mesh is socket-
                // attached when the `Item_Equip` animation plays. Same
                // ordering principle as combat-enter (PR #338).
                super::super::abilities::request_appearance_refresh(entity_id, tx, space_mgr).await;
                super::super::cell_methods::player::world::fire_item_sequence(
                    entity_id,
                    super::super::spawner::EVENT_ITEM_EQUIP,
                    tx,
                    space_mgr,
                )
                .await;
            }
        }

        BaseToCellMsg::SyncBandolierItems {
            entity_id,
            active_bandolier_slot,
            bandolier_items,
        } => {
            // Detect "weapon manually equipped into active slot" — when
            // a player drags a weapon from main inventory into their
            // bandolier slot, base persists the move and dispatches
            // `SyncBandolierItems` with the full new bandolier set. We
            // need to fire the equip-display chain (draw weapon, mesh
            // attach, Item_Equip animation, OOC re-holster timer) when
            // the active slot just gained a weapon it didn't have
            // before. Compare prev vs new item_id in the active slot.
            //
            // (PR #338 follow-up — initial player equip goes through
            // `moveInventoryItem`, not `grantItem`. The earlier
            // `UpdateBandolierItem` fix only covered chain-engine
            // grants; the player-driven drag-to-equip case lands here.)
            // Detect "weapon manually equipped into active slot" — when
            // a player drags a weapon from main inventory into their
            // bandolier slot, base persists the move and dispatches
            // `SyncBandolierItems` with the full new bandolier set. We
            // need to fire the equip-display chain (draw weapon, mesh
            // attach, Item_Equip animation, OOC re-holster timer) when
            // the active slot just gained a weapon it didn't have
            // before. Compare prev vs new item_id in the active slot.
            //
            // (PR #338 follow-up — initial player equip goes through
            // `moveInventoryItem`, not `grantItem`. The earlier
            // `UpdateBandolierItem` fix only covered chain-engine
            // grants; the player-driven drag-to-equip case lands here.)
            let (prev_active_item_id, new_active_item_id) =
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    let prev = entity
                        .bandolier_items
                        .get(&active_bandolier_slot)
                        .map(|i| i.item_id);
                    entity.active_bandolier_slot = active_bandolier_slot;
                    entity.bandolier_items = bandolier_items.into_iter().collect();
                    let new = entity
                        .bandolier_items
                        .get(&active_bandolier_slot)
                        .map(|i| i.item_id);

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
                    (prev, new)
                } else {
                    (None, None)
                };

            // Borrow released — push the dirty stats out.
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                let payload = entity.stats.serialize_dirty();
                entity.stats.clear_dirty();
                if !payload.is_empty() {
                    super::super::abilities::send_entity_method(
                        entity_id, 20, payload, tx, space_mgr,
                    )
                    .await;
                }
            }

            // Re-borrow to make the equip-display decision.
            let active_slot_gained_weapon =
                new_active_item_id.is_some() && new_active_item_id != prev_active_item_id;
            let (play_equip_anim, drew_weapon, was_in_combat, entity_state, anim_path) =
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    let is_player = entity.is_player;
                    let player_id = entity.player_id;
                    let in_combat = !entity.threatened_mobs.is_empty();
                    let path = if !active_slot_gained_weapon {
                        if new_active_item_id.is_none() {
                            "active slot is empty — skip"
                        } else {
                            "active slot unchanged — skip"
                        }
                    } else if in_combat {
                        "in combat — skip OOC timer arming"
                    } else {
                        "active slot gained weapon (OOC) — draw + animate"
                    };
                    if active_slot_gained_weapon && !in_combat {
                        entity.set_weapon_holstered(false);
                        entity.combat_exit_at = Some(std::time::Instant::now());
                        entity.holster_animation_complete_at = None;
                        (true, true, false, (is_player, player_id), path)
                    } else {
                        (false, false, in_combat, (is_player, player_id), path)
                    }
                } else {
                    (false, false, false, (false, None), "entity missing")
                };
            tracing::info!(
                entity_id,
                active_bandolier_slot,
                play_equip_anim,
                drew_weapon,
                was_in_combat,
                is_player = entity_state.0,
                player_id = ?entity_state.1,
                anim_path,
                "SyncBandolierItems: equip-display decision"
            );
            if play_equip_anim {
                super::super::abilities::request_appearance_refresh(entity_id, tx, space_mgr).await;
                super::super::cell_methods::player::world::fire_item_sequence(
                    entity_id,
                    super::super::spawner::EVENT_ITEM_EQUIP,
                    tx,
                    space_mgr,
                )
                .await;
            }
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use cimmeria_entity::cell_entity::BandolierItem;

    #[tokio::test]
    async fn destroy_entity_flushes_dirty_bandolier_and_destroys_entity() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.bandolier_items.insert(
                0,
                BandolierItem {
                    item_id: 10,
                    clip_size: 30,
                    default_ammo_type: 1,
                    current_ammo: 17,
                    cur_ammo_type: 1,
                },
            );
            e.bandolier_ammo_dirty.insert(0);
        }

        let (tx, mut rx) = mpsc::channel(8);
        let engine = ChainEngine::new();

        handle_base_message(
            BaseToCellMsg::DestroyEntity { entity_id: 1 },
            &tx,
            &mut mgr,
            &engine,
            &[],
        )
        .await;

        // A BandolierAmmoUpdate must be sent while handling destroy.
        let mut got_flush = false;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::BandolierAmmoUpdate { player_id, .. } = msg {
                assert_eq!(player_id, 100);
                got_flush = true;
            }
        }
        assert!(
            got_flush,
            "DestroyEntity must flush dirty bandolier ammo before tearing down"
        );
        assert!(
            mgr.get_entity(1).is_none(),
            "entity must be destroyed after flush"
        );
    }

    #[tokio::test]
    async fn entity_move_updates_position_in_space_manager() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();

        let (tx, _rx) = mpsc::channel(8);
        let engine = ChainEngine::new();

        handle_base_message(
            BaseToCellMsg::EntityMove {
                entity_id: 1,
                position: [10.0, 20.0, 30.0],
                direction: [0, 0, 0],
                velocity: [1.0, 2.0, 3.0],
            },
            &tx,
            &mut mgr,
            &engine,
            &[],
        )
        .await;

        let entity = mgr.get_entity(1).unwrap();
        assert_eq!(entity.position.x, 10.0);
        assert_eq!(entity.position.y, 20.0);
        assert_eq!(entity.position.z, 30.0);
        assert_eq!(entity.velocity, [1.0, 2.0, 3.0]);
    }

    /// `InventoryItemMoveApplied` with target=bandolier (3) and source≠3
    /// fires the `OnItemEquipped` content event. Pin the dispatch path
    /// end-to-end by registering a chain that reacts with
    /// `IncrementCounter` and asserting the entity's counter moved.
    #[tokio::test]
    async fn item_move_applied_into_bandolier_fires_equip_event() {
        use cimmeria_content_engine::actions::Action;
        use cimmeria_content_engine::chain::Chain;
        use cimmeria_content_engine::triggers::Trigger;

        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
        }

        let mut engine = ChainEngine::new();
        engine.register_chain(Chain {
            id: 9100,
            name: "test: bandolier-equip → bump".to_string(),
            enabled: true,
            trigger: Trigger::OnItemEquipped { item_id: Some(55) },
            conditions: vec![],
            actions: vec![Action::IncrementCounter {
                counter_name: "test_bandolier_equip".to_string(),
                amount: 1,
            }],
            priority: 0,
        });

        let (tx, _rx) = mpsc::channel(16);
        handle_base_message(
            BaseToCellMsg::InventoryItemMoveApplied {
                entity_id: 1,
                item_id: 0xABCD,
                type_id: 55,
                source_container_id: 1, // backpack
                target_container_id: 3, // bandolier
                swapped_item_id: None,
            },
            &tx,
            &mut mgr,
            &engine,
            &[],
        )
        .await;

        let entity = mgr.get_entity(1).expect("entity must still exist");
        assert_eq!(
            entity.counters.get("test_bandolier_equip"),
            Some(&1),
            "move into bandolier (target=3, source≠3) must fire OnItemEquipped",
        );
    }

    /// A move WITHIN the bandolier (source=3, target=3 — the player
    /// reordering their bandolier slots) must NOT fire `OnItemEquipped`.
    /// Without this guard, every drag between bandolier slots would
    /// re-fire equip chains and re-grant whatever they grant.
    #[tokio::test]
    async fn item_move_within_bandolier_does_not_fire_equip_event() {
        use cimmeria_content_engine::actions::Action;
        use cimmeria_content_engine::chain::Chain;
        use cimmeria_content_engine::triggers::Trigger;

        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
        }

        let mut engine = ChainEngine::new();
        engine.register_chain(Chain {
            id: 9101,
            name: "test: any equip → bump".to_string(),
            enabled: true,
            trigger: Trigger::OnItemEquipped { item_id: None },
            conditions: vec![],
            actions: vec![Action::IncrementCounter {
                counter_name: "test_within_bandolier".to_string(),
                amount: 1,
            }],
            priority: 0,
        });

        let (tx, _rx) = mpsc::channel(16);
        handle_base_message(
            BaseToCellMsg::InventoryItemMoveApplied {
                entity_id: 1,
                item_id: 0xABCD,
                type_id: 55,
                source_container_id: 3, // bandolier → bandolier
                target_container_id: 3,
                swapped_item_id: None,
            },
            &tx,
            &mut mgr,
            &engine,
            &[],
        )
        .await;

        let entity = mgr.get_entity(1).expect("entity must still exist");
        assert!(
            !entity.counters.contains_key("test_within_bandolier"),
            "bandolier-internal move must not fire OnItemEquipped; got {:?}",
            entity.counters,
        );
    }

    /// A move OUT of the bandolier (source=3, target=1) must not fire
    /// `OnItemEquipped` either — that's an unequip, not an equip.
    #[tokio::test]
    async fn item_move_out_of_bandolier_does_not_fire_equip_event() {
        use cimmeria_content_engine::actions::Action;
        use cimmeria_content_engine::chain::Chain;
        use cimmeria_content_engine::triggers::Trigger;

        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
        }

        let mut engine = ChainEngine::new();
        engine.register_chain(Chain {
            id: 9102,
            name: "test: any equip → bump".to_string(),
            enabled: true,
            trigger: Trigger::OnItemEquipped { item_id: None },
            conditions: vec![],
            actions: vec![Action::IncrementCounter {
                counter_name: "test_unequip_path".to_string(),
                amount: 1,
            }],
            priority: 0,
        });

        let (tx, _rx) = mpsc::channel(16);
        handle_base_message(
            BaseToCellMsg::InventoryItemMoveApplied {
                entity_id: 1,
                item_id: 0xABCD,
                type_id: 55,
                source_container_id: 3, // bandolier → backpack
                target_container_id: 1,
                swapped_item_id: None,
            },
            &tx,
            &mut mgr,
            &engine,
            &[],
        )
        .await;

        let entity = mgr.get_entity(1).expect("entity must still exist");
        assert!(
            !entity.counters.contains_key("test_unequip_path"),
            "unequip (source=3, target=1) must not fire OnItemEquipped",
        );
    }

    #[tokio::test]
    async fn update_bandolier_item_inserts_slot_and_sets_active() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();

        let item = BandolierItem {
            item_id: 42,
            clip_size: 25,
            default_ammo_type: 2,
            current_ammo: 25,
            cur_ammo_type: 2,
        };

        let (tx, _rx) = mpsc::channel(8);
        let engine = ChainEngine::new();

        handle_base_message(
            BaseToCellMsg::UpdateBandolierItem {
                entity_id: 1,
                slot_id: 2,
                item,
                make_active: true,
            },
            &tx,
            &mut mgr,
            &engine,
            &[],
        )
        .await;

        let entity = mgr.get_entity(1).unwrap();
        assert_eq!(
            entity.active_bandolier_slot, 2,
            "active_bandolier_slot must update"
        );
        assert!(
            entity.bandolier_items.contains_key(&2),
            "bandolier_items must contain the new slot"
        );
    }

    /// `UpdateBandolierItem` with `make_active=true` on an OOC player
    /// must draw the weapon, stamp the OOC re-holster timer, and
    /// dispatch a `RefreshAppearance` (mesh attach for the equip
    /// animation) + an `ON_SEQUENCE` (Item_Equip animation).
    ///
    /// Bug shape this catches: a refactor that drops the draw-on-equip
    /// hook leaves the player picking up a weapon that stays invisible
    /// until the next combat enter — the symptom that drove this fix
    /// (the player's mental model is "I just equipped this, I should
    /// see it").
    #[tokio::test]
    async fn update_bandolier_item_draws_weapon_and_arms_holster_timer_when_active() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.archetype_id = Some(1);
            e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
            e.weapon_holstered = true; // OOC + holstered (post-grace)
            e.combat_exit_at = None;
        }
        mgr.connect_entity(1);
        // Seed the Item_Equip sequence so `fire_item_sequence` can resolve.
        // Soldier (archetype 1) → event set 804 → seq 1872 (Item_Equip).
        mgr.sequence_map
            .insert((804, crate::cell::spawner::EVENT_ITEM_EQUIP), 1872);

        let item = BandolierItem {
            item_id: 42,
            clip_size: 25,
            default_ammo_type: 2,
            current_ammo: 25,
            cur_ammo_type: 2,
        };

        let (tx, mut rx) = mpsc::channel(16);
        let engine = ChainEngine::new();

        handle_base_message(
            BaseToCellMsg::UpdateBandolierItem {
                entity_id: 1,
                slot_id: 0,
                item,
                make_active: true,
            },
            &tx,
            &mut mgr,
            &engine,
            &[],
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert!(
            !e.weapon_holstered,
            "equip on OOC player must draw the weapon so it's visible",
        );
        assert!(
            e.combat_exit_at.is_some(),
            "OOC holster timer must be armed so the weapon re-holsters \
             after OOC_HOLSTER_DELAY (matches the post-combat behavior — \
             player sees the weapon for a few seconds then it goes away)",
        );

        let mut saw_refresh = false;
        let mut saw_sequence = false;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::RefreshAppearance {
                    holstered: false, ..
                } => saw_refresh = true,
                CellToBaseMsg::EntityMethodCall { method_index, .. }
                    if method_index
                        == crate::cell::client_methods::spawnable_entity::ON_SEQUENCE =>
                {
                    saw_sequence = true;
                }
                _ => {}
            }
        }
        assert!(
            saw_refresh,
            "equip must dispatch RefreshAppearance(holstered=false) so the \
             client attaches the weapon mesh before the equip animation",
        );
        assert!(
            saw_sequence,
            "equip must fire Item_Equip onSequence so the client plays the \
             equip animation. Without it, the weapon just teleports into the \
             hand with no animation.",
        );
    }

    /// `SyncBandolierItems` is the message base dispatches after a
    /// player-driven `moveInventoryItem` lands a weapon in a bandolier
    /// slot (the drag-from-backpack-to-bandolier flow — distinct from
    /// the chain-engine `grantItem` path that goes through
    /// `UpdateBandolierItem`). When the active slot just gained a
    /// weapon, the cell must fire the equip-display chain: draw the
    /// weapon, dispatch `RefreshAppearance(holstered=false)` for the
    /// mesh attach, fire `Item_Equip` for the unholster animation, and
    /// arm the OOC re-holster timer.
    ///
    /// Bug shape this catches: a refactor that drops the
    /// prev-vs-new comparison in the SyncBandolierItems handler
    /// regresses to "weapon equip into bandolier doesn't unholster" —
    /// the symptom that drove this fix.
    #[tokio::test]
    async fn sync_bandolier_items_active_slot_gained_weapon_draws_and_animates() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.archetype_id = Some(1);
            e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
            e.weapon_holstered = true; // before the equip
            e.active_bandolier_slot = 0;
            // Active slot starts empty — player hasn't equipped a weapon yet.
            e.bandolier_items.clear();
        }
        mgr.connect_entity(1);
        mgr.sequence_map
            .insert((804, crate::cell::spawner::EVENT_ITEM_EQUIP), 1872);

        let item = BandolierItem {
            item_id: 55,
            clip_size: 15,
            default_ammo_type: 2,
            current_ammo: 0,
            cur_ammo_type: 2,
        };

        let (tx, mut rx) = mpsc::channel(16);
        let engine = ChainEngine::new();

        handle_base_message(
            BaseToCellMsg::SyncBandolierItems {
                entity_id: 1,
                active_bandolier_slot: 0,
                bandolier_items: vec![(0, item)],
            },
            &tx,
            &mut mgr,
            &engine,
            &[],
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert!(
            !e.weapon_holstered,
            "player-driven equip into bandolier must draw the weapon — \
             this is the playtest symptom: 'on equip is when it needs to unholster'",
        );
        assert!(
            e.combat_exit_at.is_some(),
            "OOC re-holster timer must arm so the weapon goes away after \
             OOC_HOLSTER_DELAY, matching the grant path's behavior",
        );

        let mut saw_refresh = false;
        let mut saw_sequence = false;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::RefreshAppearance {
                    holstered: false, ..
                } => saw_refresh = true,
                CellToBaseMsg::EntityMethodCall { method_index, .. }
                    if method_index
                        == crate::cell::client_methods::spawnable_entity::ON_SEQUENCE =>
                {
                    saw_sequence = true;
                }
                _ => {}
            }
        }
        assert!(
            saw_refresh,
            "equip into bandolier must dispatch RefreshAppearance — \
             without it the weapon mesh never attaches on the client model",
        );
        assert!(
            saw_sequence,
            "equip into bandolier must fire Item_Equip — without it the \
             weapon teleports into the hand with no unholster animation",
        );
    }

    /// `SyncBandolierItems` when the active slot is UNCHANGED (same
    /// item_id as before) must NOT re-fire the equip-display chain.
    /// This catches the "post-vendor-buy resync re-equips the weapon
    /// you already had" regression — base resyncs the bandolier after
    /// any inventory change, and we don't want a stash-slot grant to
    /// retrigger the active weapon's equip animation.
    #[tokio::test]
    async fn sync_bandolier_items_active_slot_unchanged_does_not_re_animate() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.weapon_holstered = false; // already drawn — mid-fight or post-equip grace
            e.active_bandolier_slot = 0;
            e.bandolier_items.insert(
                0,
                BandolierItem {
                    item_id: 55,
                    clip_size: 15,
                    default_ammo_type: 2,
                    current_ammo: 7,
                    cur_ammo_type: 2,
                },
            );
            // Mock that combat_exit_at was stamped a while ago — we
            // want to verify the resync DOESN'T reset it (which would
            // extend the OOC grace window mid-game).
            e.combat_exit_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(5));
        }
        mgr.connect_entity(1);

        let prior_combat_exit_at = mgr.get_entity(1).unwrap().combat_exit_at;

        let (tx, mut rx) = mpsc::channel(16);
        let engine = ChainEngine::new();

        // Resync with same active item — picks up new stash slot.
        handle_base_message(
            BaseToCellMsg::SyncBandolierItems {
                entity_id: 1,
                active_bandolier_slot: 0,
                bandolier_items: vec![
                    (
                        0,
                        BandolierItem {
                            item_id: 55,
                            clip_size: 15,
                            default_ammo_type: 2,
                            current_ammo: 7,
                            cur_ammo_type: 2,
                        },
                    ),
                    (
                        2,
                        BandolierItem {
                            item_id: 99,
                            clip_size: 30,
                            default_ammo_type: 1,
                            current_ammo: 30,
                            cur_ammo_type: 1,
                        },
                    ),
                ],
            },
            &tx,
            &mut mgr,
            &engine,
            &[],
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert_eq!(
            e.combat_exit_at, prior_combat_exit_at,
            "resync that doesn't change the active slot's item must NOT \
             restamp combat_exit_at — that would extend the OOC grace \
             window every time the player gets ANY new stash item",
        );

        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::RefreshAppearance { .. } => {
                    panic!(
                        "unchanged-active-slot resync must NOT dispatch \
                         RefreshAppearance — broadcasting on every stash \
                         update is wire spam",
                    );
                }
                CellToBaseMsg::EntityMethodCall { method_index, .. }
                    if method_index
                        == crate::cell::client_methods::spawnable_entity::ON_SEQUENCE =>
                {
                    panic!("unchanged-active-slot resync must NOT re-fire Item_Equip");
                }
                _ => {}
            }
        }
    }

    /// `UpdateBandolierItem` with `make_active=false` but a slot that
    /// MATCHES the entity's already-active slot must STILL draw the
    /// weapon. This is the initial-grant case: base's
    /// `handle_grant_item` SQL only flips `bandolier_slot` when the
    /// previous selection points at a vacant slot — but the INSERT
    /// happens BEFORE the UPDATE, so for an initial grant where
    /// `next_slot == p.bandolier_slot == 0`, the NOT EXISTS check
    /// finds the just-inserted row and the UPDATE is skipped,
    /// leaving `bandolier_became_active=false`. The new weapon IS
    /// the active one even though base couldn't say so, and the
    /// equip animation MUST fire — otherwise the player's first
    /// weapon never appears.
    ///
    /// Bug shape this catches: a refactor that ties the equip-display
    /// gate strictly to `make_active=true` regresses initial weapon
    /// grants back to invisible-on-equip.
    #[tokio::test]
    async fn update_bandolier_item_make_active_false_but_slot_matches_active_still_draws() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.archetype_id = Some(1);
            e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
            e.weapon_holstered = true;
            // Player's default active slot is 0 (fresh character).
            e.active_bandolier_slot = 0;
        }
        mgr.connect_entity(1);
        mgr.sequence_map
            .insert((804, crate::cell::spawner::EVENT_ITEM_EQUIP), 1872);

        let item = BandolierItem {
            item_id: 42,
            clip_size: 25,
            default_ammo_type: 2,
            current_ammo: 25,
            cur_ammo_type: 2,
        };

        let (tx, mut rx) = mpsc::channel(16);
        let engine = ChainEngine::new();

        // Initial weapon grant where base's bandolier_slot UPDATE
        // skipped (next_slot == p.bandolier_slot == 0, NOT EXISTS
        // finds the new INSERT). So `make_active=false`, even though
        // this IS the active slot from the entity's perspective.
        handle_base_message(
            BaseToCellMsg::UpdateBandolierItem {
                entity_id: 1,
                slot_id: 0,
                item,
                make_active: false,
            },
            &tx,
            &mut mgr,
            &engine,
            &[],
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert!(
            !e.weapon_holstered,
            "initial grant into the already-active slot must draw the weapon — \
             without this the first weapon a player ever receives stays \
             invisible (the bug from PR #338 playtest)",
        );
        assert!(
            e.combat_exit_at.is_some(),
            "OOC holster timer must arm so the weapon re-holsters after \
             OOC_HOLSTER_DELAY, just like the `make_active=true` path",
        );

        let mut saw_refresh = false;
        let mut saw_sequence = false;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::RefreshAppearance {
                    holstered: false, ..
                } => saw_refresh = true,
                CellToBaseMsg::EntityMethodCall { method_index, .. }
                    if method_index
                        == crate::cell::client_methods::spawnable_entity::ON_SEQUENCE =>
                {
                    saw_sequence = true;
                }
                _ => {}
            }
        }
        assert!(
            saw_refresh,
            "initial grant must dispatch RefreshAppearance — the base's \
             refresh_player_appearance is skipped on container_id=3 to \
             avoid racing this cell-side broadcast, so this IS the broadcast \
             that attaches the weapon mesh",
        );
        assert!(
            saw_sequence,
            "initial grant must fire Item_Equip — same animation as a \
             slot-swap into the active slot",
        );
    }

    /// `UpdateBandolierItem` with `make_active=false` AND a slot
    /// different from the entity's active slot must NOT draw the
    /// weapon. The player still has their existing active weapon —
    /// this is a non-active stash slot grant.
    ///
    /// Bug shape this catches: a refactor that drops the `slot_is_active`
    /// gate and always fires equip-display would yank the visible
    /// weapon mid-fight when a quest reward lands in a stash slot.
    #[tokio::test]
    async fn update_bandolier_item_make_active_false_into_non_active_slot_does_not_draw() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.weapon_holstered = true;
            // Player's active slot is 0 (their existing pistol).
            e.active_bandolier_slot = 0;
        }
        mgr.connect_entity(1);

        let item = BandolierItem {
            item_id: 99,
            clip_size: 30,
            default_ammo_type: 1,
            current_ammo: 30,
            cur_ammo_type: 1,
        };

        let (tx, mut rx) = mpsc::channel(16);
        let engine = ChainEngine::new();

        // Quest reward lands in slot 2, not the active slot 0.
        handle_base_message(
            BaseToCellMsg::UpdateBandolierItem {
                entity_id: 1,
                slot_id: 2,
                item,
                make_active: false,
            },
            &tx,
            &mut mgr,
            &engine,
            &[],
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert_eq!(
            e.active_bandolier_slot, 0,
            "stash-slot grant must NOT touch active_bandolier_slot",
        );
        assert!(
            e.weapon_holstered,
            "stash-slot grant must NOT draw the weapon — player still has \
             their existing active weapon (which is holstered here)",
        );
        assert!(
            e.combat_exit_at.is_none(),
            "stash-slot grant must NOT arm the OOC holster timer",
        );

        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::RefreshAppearance { .. } => {
                    panic!(
                        "stash-slot grant must NOT dispatch RefreshAppearance — \
                         the active slot's appearance is unchanged",
                    );
                }
                CellToBaseMsg::EntityMethodCall { method_index, .. }
                    if method_index
                        == crate::cell::client_methods::spawnable_entity::ON_SEQUENCE =>
                {
                    panic!("stash-slot grant must NOT fire Item_Equip");
                }
                _ => {}
            }
        }
    }

    /// `UpdateBandolierItem` while in combat must NOT arm the OOC
    /// holster timer — `combat_exit_at` is supposed to be stamped by
    /// `exit_player_combat`, and stamping it here would let the holster
    /// fire mid-combat (the holster scan doesn't gate on
    /// `threatened_mobs.is_empty()` today).
    #[tokio::test]
    async fn update_bandolier_item_in_combat_does_not_arm_holster_timer() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.archetype_id = Some(1);
            e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
            e.weapon_holstered = false; // in combat, already drawn
            e.combat_exit_at = None;
            e.threatened_mobs.insert(999); // pretend there's an aggro
        }
        mgr.connect_entity(1);

        let item = BandolierItem {
            item_id: 42,
            clip_size: 25,
            default_ammo_type: 2,
            current_ammo: 25,
            cur_ammo_type: 2,
        };

        let (tx, _rx) = mpsc::channel(16);
        let engine = ChainEngine::new();

        handle_base_message(
            BaseToCellMsg::UpdateBandolierItem {
                entity_id: 1,
                slot_id: 0,
                item,
                make_active: true,
            },
            &tx,
            &mut mgr,
            &engine,
            &[],
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert!(
            e.combat_exit_at.is_none(),
            "in-combat equip must NOT stamp combat_exit_at — the timer \
             would otherwise fire mid-combat and holster the weapon while \
             the player is still fighting. `exit_player_combat` stamps \
             this on combat exit naturally.",
        );
    }

    /// `BaseToCellMsg::AdvanceRingDestination` is the cross-world ring
    /// transport's deferred-load callback. After the source ring's
    /// `Effect::TeleportCrossWorld` fires, the destination ring's FSM
    /// sits in `RemoteLoadWait` until base sends this message back
    /// (after the destination world's `onClientReady`). The handler
    /// must forward to `ring_transport::handle_remote_player_loaded`
    /// without crashing when the destination ring isn't loaded — the
    /// integration-shaped fail-soft path that lets a ring not pre-loaded
    /// in this cell instance be a quiet no-op rather than a panic.
    #[tokio::test]
    async fn advance_ring_destination_forwards_without_panic_when_ring_absent() {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        // Player is on Castle but no ring transporter region 34 is
        // loaded — handle_remote_player_loaded must short-circuit
        // gracefully rather than panic on the missing region lookup.
        mgr.create_entity(2, "Castle", [466.365, 70.397, 991.466], [0.0; 3])
            .unwrap();

        let (tx, _rx) = mpsc::channel(8);
        let engine = ChainEngine::new();

        handle_base_message(
            BaseToCellMsg::AdvanceRingDestination {
                entity_id: 2,
                region_id: 34,
            },
            &tx,
            &mut mgr,
            &engine,
            &[],
        )
        .await;

        // The entity must still exist — AdvanceRingDestination doesn't
        // tear anything down on its own; it just records a load on the
        // destination ring's FSM (which is a no-op when the ring isn't
        // loaded). Pinning post-state-equality guards against a future
        // refactor that accidentally couples the dispatcher to entity
        // teardown.
        assert!(
            mgr.get_entity(2).is_some(),
            "AdvanceRingDestination must not destroy the recipient entity"
        );
    }
}
