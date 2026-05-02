//! Action execution — dispatches resolved content engine actions against the
//! game state (missions, items, dialogs, interactions, etc.).

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ResolvedActions;

use cimmeria_entity::missions::{MissionObjective, STATUS_ACTIVE};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Execute resolved actions from the content engine against the game state.
pub(super) async fn execute_actions(
    resolved: ResolvedActions,
    entity_id: u32,
    player_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) {
    for (chain_id, action) in resolved.actions {
        match action {
            Action::AcceptMission { mission_id } | Action::AdvanceMission { mission_id } => {
                tracing::info!(entity_id, mission_id, chain_id, "Content: accepting mission");
                if let Some(def) = space_mgr.mission_defs.get(&mission_id) {
                    let step_id = def.step_id;
                    let objectives: Vec<MissionObjective> = def.objectives.iter().map(|o| {
                        MissionObjective {
                            objective_id: o.objective_id,
                            status: STATUS_ACTIVE,
                            hidden: o.is_hidden,
                            optional: o.is_optional,
                        }
                    }).collect();
                    crate::cell::missions::accept_mission(
                        entity_id, mission_id, step_id, objectives, tx, space_mgr,
                    ).await;
                    if let Err(e) = tx.send(CellToBaseMsg::MissionUpdate {
                        player_id,
                        mission_id,
                        status: 1,
                        current_step_id: Some(step_id),
                        completed_step_ids: vec![],
                        completed_objective_ids: vec![],
                        active_objective_ids: vec![step_id],
                        failed_objective_ids: vec![],
                    }).await {
                        tracing::error!(
                            entity_id, player_id, mission_id, step_id,
                            chain_id, error = %e,
                            "MissionUpdate (accept) send to base failed -- mission progress not persisted"
                        );
                    }
                } else {
                    tracing::warn!(mission_id, chain_id, "No mission_defs entry — cannot accept mission");
                }
            }
            Action::CompleteMission { mission_id } => {
                tracing::info!(entity_id, mission_id, chain_id, "Content: completing mission");
                crate::cell::missions::complete_mission_direct(
                    entity_id, mission_id, tx, space_mgr,
                ).await;
                if let Err(e) = tx.send(CellToBaseMsg::MissionUpdate {
                    player_id,
                    mission_id,
                    status: 2,
                    current_step_id: None,
                    completed_step_ids: vec![],
                    completed_objective_ids: vec![],
                    active_objective_ids: vec![],
                    failed_objective_ids: vec![],
                }).await {
                    tracing::error!(
                        entity_id, player_id, mission_id, chain_id, error = %e,
                        "MissionUpdate (complete) send to base failed -- mission completion not persisted"
                    );
                }
            }
            Action::GrantItem { item_id, count, container_id } => {
                tracing::info!(entity_id, item_id, count, chain_id, "Content: granting item");
                let cid = container_id.filter(|&c| c > 0)
                    .unwrap_or_else(|| item_container(item_id, &space_mgr.item_containers));

                // If this is a weapon (bandolier), set ammo state on the entity.
                // Weapons start unloaded — the player must press R to reload.
                //
                // Stage C: insert a `BandolierItem` for the granted slot and seed
                // the AmmoSlot{N} stat to (0, 0, clip_size) so subsequent fire /
                // reload paths (which now read through `active_ammo()` and
                // `set_slot_ammo`) operate on a valid clamp range. We also send
                // an `onStatUpdate` so the client renders the empty mag for the
                // new weapon without waiting for the next fire.
                let mut ammo_stat_payload: Option<Vec<u8>> = None;
                if cid == 3 {
                    if let Some((clip, default_ammo_type)) = weapon_stats(item_id, &space_mgr.item_defs) {
                        if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                            // The weapon-grant chain doesn't tell us which slot
                            // the base will assign — content engine grants
                            // implicitly fill the active bandolier slot.
                            let slot_id = entity.active_bandolier_slot;
                            entity.bandolier_items.insert(slot_id, cimmeria_entity::cell_entity::BandolierItem {
                                item_id,
                                clip_size: clip,
                                default_ammo_type,
                                current_ammo: 0,
                                cur_ammo_type: default_ammo_type,
                            });
                            entity.bandolier_ammo_dirty.insert(slot_id);
                            let stat_id = cimmeria_entity::stats::AMMO_SLOT_1 + slot_id;
                            if let Some(stat) = entity.stats.get_mut(stat_id) {
                                stat.update(0, 0, clip);
                                let payload = entity.stats.serialize_dirty();
                                entity.stats.clear_dirty();
                                ammo_stat_payload = Some(payload);
                            }
                            tracing::info!(entity_id, item_id, slot_id, clip, "Weapon granted unloaded");
                        }
                    }
                }

                if let Err(e) = tx.send(CellToBaseMsg::GrantItem {
                    entity_id,
                    player_id,
                    item_id,
                    container_id: cid,
                    count,
                }).await {
                    tracing::error!(
                        entity_id, player_id, item_id, container_id = cid,
                        count, chain_id, error = %e,
                        "GrantItem send to base failed -- item not persisted to inventory"
                    );
                }

                if let Some(payload) = ammo_stat_payload {
                    if !payload.is_empty() {
                        crate::cell::abilities::send_entity_method(
                            entity_id, 20, payload, tx, space_mgr,
                        ).await;
                    }
                }
            }
            Action::DisplayDialog { dialog_id } | Action::StartDialog { dialog_set_id: dialog_id } => {
                tracing::info!(entity_id, dialog_id, chain_id, "Content: displaying dialog");
                crate::cell::interactions::send_dialog_display(entity_id, entity_id as i32, dialog_id, tx).await;
            }
            Action::PlaySequence { sequence_id } => {
                tracing::info!(entity_id, sequence_id, chain_id, "Content: playing sequence");
                let mut args = Vec::with_capacity(26);
                args.extend_from_slice(&sequence_id.to_le_bytes()); // KismetEventSetSeqID
                args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // SourceID
                args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // TargetID
                args.push(1);                                       // PrimaryTarget = true
                args.extend_from_slice(&0.0f32.to_le_bytes());     // ImpactTime
                args.extend_from_slice(&0u32.to_le_bytes());        // NameValuePairs count = 0
                args.push(0);                                       // ViewType = 0
                args.extend_from_slice(&0i32.to_le_bytes());        // InstanceId
                let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: 1, // onSequence (SGWSpawnableEntity)
                    args,
                }).await;
            }
            Action::AdvanceStep { mission_id, step_id } => {
                tracing::info!(entity_id, mission_id, step_id, chain_id, "Content: advancing step");
                crate::cell::missions::advance_step(entity_id, mission_id, step_id, tx, space_mgr).await;
                if let Err(e) = tx.send(CellToBaseMsg::MissionUpdate {
                    player_id,
                    mission_id,
                    status: 1,
                    current_step_id: Some(step_id),
                    completed_step_ids: vec![],
                    completed_objective_ids: vec![],
                    active_objective_ids: vec![step_id],
                    failed_objective_ids: vec![],
                }).await {
                    tracing::error!(
                        entity_id, player_id, mission_id, step_id,
                        chain_id, error = %e,
                        "MissionUpdate (advance step) send to base failed -- step progress not persisted"
                    );
                }
            }
            Action::AddDialogSet { dialog_set_id, slot, mission_id: _ } => {
                tracing::info!(entity_id, dialog_set_id, slot, chain_id, "Content: adding dialog set");

                if let Some(entry) = space_mgr.dialog_set_maps.get(&dialog_set_id).cloned() {
                    tracing::info!(
                        entity_id, dialog_set_id, slot,
                        dialog_id = entry.dialog_id,
                        interaction_flags = entry.interaction_flags,
                        "add_dialog_set: resolved dialog_set_map entry"
                    );

                    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
                        player.available_interactions
                            .entry(slot)
                            .or_default()
                            .push((dialog_set_id, entry.dialog_id, entry.interaction_flags));

                        tracing::info!(
                            entity_id, slot,
                            interactions_count = player.available_interactions.get(&slot).map_or(0, |v| v.len()),
                            "add_dialog_set: stored in available_interactions"
                        );
                    }

                    send_interaction_update_if_visible(entity_id, slot, &entry, tx, space_mgr, "add_dialog_set").await;
                } else {
                    tracing::warn!(dialog_set_id, "dialog_set_maps cache miss for add_dialog_set");
                }
            }
            Action::RemoveDialogSet { dialog_set_id, slot } => {
                tracing::info!(entity_id, dialog_set_id, slot, chain_id, "Content: removing dialog set");

                let removed_flags = if let Some(player) = space_mgr.get_entity_mut(entity_id) {
                    if let Some(entries) = player.available_interactions.get_mut(&slot) {
                        entries.retain(|&(dsm_id, _, _)| dsm_id != dialog_set_id);
                        if entries.is_empty() {
                            player.available_interactions.remove(&slot);
                        }
                    }
                    player.available_interactions.get(&slot)
                        .map(|entries| entries.iter().fold(0i64, |acc, &(_, _, flags)| acc | flags))
                } else {
                    None
                };

                // Update every entity sharing this template -- `.first()` would
                // arbitrarily pick one (HashMap iteration order is nondeterministic),
                // leaving sibling entities with stale interaction flags.
                for target_id in space_mgr.find_entities_by_template(entity_id, slot) {
                    let target_eid = cimmeria_common::EntityId(target_id as i32);
                    let in_witness_set = space_mgr.get_entity(entity_id)
                        .map_or(false, |p| p.witnesses.contains(&target_eid));

                    if in_witness_set {
                        let base_flags = space_mgr.get_entity(target_id)
                            .map(|e| e.interaction_type_flags).unwrap_or(0);
                        let merged = base_flags | removed_flags.unwrap_or(0);

                        let _ = tx.send(CellToBaseMsg::WitnessEntityMethod {
                            witness_id: entity_id,
                            entity_id: target_id,
                            method_index: crate::mercury::method_idx::INTERACTION_TYPE,
                            args: (merged as u64).to_le_bytes().to_vec(),
                        }).await;
                    }
                }
            }
            Action::RemoveItem { item_id, count } => {
                // TODO: Action::RemoveItem here gets a design id (type_id) from
                // the chain, but `RemoveInventoryItem` expects the inventory
                // instance id. Resolving that requires either a cell-side
                // instance↔design cache or a new "remove by type_id" base
                // handler. For the FindAmbernol use-item path we route
                // consumption through `UseInventoryItem` instead, which is
                // atomic on the base side, so this stub doesn't block that
                // mission. Revisit when chain-driven removals (turn-ins, etc.)
                // come up.
                tracing::warn!(
                    entity_id, item_id, count, chain_id,
                    "Content: RemoveItem stub — chain-driven item removal by design id not yet wired"
                );
            }
            Action::SetInteractionType { entity_tag, operation, mask } => {
                if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) {
                    let new_flags = if let Some(target) = space_mgr.get_entity_mut(target_id) {
                        let old = target.interaction_type_flags;
                        match operation.as_str() {
                            "add" | "|" => target.interaction_type_flags |= mask,
                            "remove" | "~" => target.interaction_type_flags &= !mask,
                            "set" => target.interaction_type_flags = mask,
                            _ => tracing::warn!(%operation, "Unknown interaction type operation"),
                        }
                        tracing::debug!(
                            entity_id, %entity_tag, target_id, %operation, mask,
                            old, new = target.interaction_type_flags, chain_id,
                            "Content: set interaction type"
                        );
                        Some(target.interaction_type_flags)
                    } else {
                        None
                    };

                    if let Some(flags) = new_flags {
                        let witnesses = space_mgr.get_witnesses_of(target_id);
                        for witness_id in witnesses {
                            let _ = tx.send(CellToBaseMsg::WitnessEntityMethod {
                                witness_id,
                                entity_id: target_id,
                                method_index: crate::mercury::method_idx::INTERACTION_TYPE,
                                args: (flags as u64).to_le_bytes().to_vec(),
                            }).await;
                        }
                    }
                } else {
                    tracing::debug!(entity_id, %entity_tag, chain_id, "Content: entity tag not found for SetInteractionType");
                }
            }
            Action::StartMinigame { minigame_type, on_victory_chains } => {
                tracing::info!(entity_id, %minigame_type, ?on_victory_chains, chain_id, "Content: starting minigame");
                let _ = tx.send(CellToBaseMsg::StartMinigame {
                    entity_id,
                    player_id,
                    game_name: minigame_type.clone(),
                    difficulty: 1, // TODO: parse from chain params when difficulty field is added
                    on_victory_chains: on_victory_chains.clone(),
                }).await;
            }
            Action::SetAggression { entity_tag, level: agg_level } => {
                if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) {
                    tracing::debug!(entity_id, %entity_tag, target_id, agg_level, chain_id, "Content: set aggression");
                    if let Some(target) = space_mgr.get_entity_mut(target_id) {
                        target.properties.insert(
                            "aggression".to_string(),
                            cimmeria_entity::base_entity::PropertyValue::Int32(agg_level),
                        );
                    }
                }
            }
            Action::DestroyTaggedEntity { entity_tag } => {
                if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) {
                    tracing::info!(entity_id, %entity_tag, target_id, chain_id, "Content: destroying tagged entity");
                    space_mgr.destroy_entity(target_id);
                } else {
                    tracing::debug!(entity_id, %entity_tag, chain_id, "Content: entity tag not found for DestroyTaggedEntity");
                }
            }
            Action::TriggerTransporter { region_id } => {
                tracing::info!(entity_id, region_id, chain_id, "Content: triggering transporter");
                crate::cell::ring_transport_runtime::handle_interact(
                    region_id, entity_id, tx, space_mgr, engine,
                ).await;
            }
            Action::Teleport { space_id, position } => {
                tracing::info!(
                    entity_id, space_id, ?position, chain_id,
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
                // Same-world teleport: keep the spatial grid consistent here,
                // then route through TeleportPlayer for the authoritative
                // FORCED_POSITION snap + persist. The bare 116-only path the
                // previous version emitted does NOT move the avatar.
                space_mgr.update_entity_position(entity_id, position, [0, 0, 0], [0.0; 3]);
                // SpaceId is i32 in the cell (matches DB type) but the wire
                // forced-position packet is u32 — space ids are always
                // non-negative, so the cast is a width-only conversion.
                let cell_space_id = space_mgr.get_entity(entity_id)
                    .map(|e| e.space_id.0 as u32)
                    .unwrap_or(space_id as u32);
                if let Some(e) = space_mgr.get_entity_mut(entity_id) {
                    e.position = cimmeria_common::Vector3::new(position[0], position[1], position[2]);
                }
                let _ = tx.send(CellToBaseMsg::TeleportPlayer {
                    entity_id,
                    space_id: cell_space_id,
                    position,
                }).await;
            }
            Action::SystemMessage { message_id } => {
                // TODO: Wire format for system messages is unknown. The previous
                // implementation incorrectly used onPlayerCommunication (method 28)
                // which caused garbled chat spam ("[] says") and client freezes.
                // Needs RE to find the correct client method for localized string
                // ID display (possibly onErrorCode or a UI-specific method).
                tracing::info!(entity_id, message_id, chain_id, "Content: system message (stub — correct wire format TBD)");
            }
            Action::AbandonMission { mission_id } => {
                tracing::info!(entity_id, mission_id, chain_id, "Content: abandoning mission");
                crate::cell::missions::abandon_mission(entity_id, mission_id, tx, space_mgr).await;
            }
            Action::IncrementCounter { counter_name, amount } => {
                tracing::debug!(entity_id, %counter_name, amount, chain_id, "Content: increment counter");
            }
            Action::ResetCounter { counter_name } => {
                tracing::debug!(entity_id, %counter_name, chain_id, "Content: reset counter");
            }
            Action::CompleteObjective { mission_id, objective_id } => {
                tracing::info!(entity_id, mission_id, objective_id, chain_id, "Content: complete objective");
                crate::cell::missions::complete_objective(entity_id, mission_id, objective_id, tx, space_mgr).await;
            }
            Action::SendMessage { channel, message } => {
                tracing::info!(entity_id, %channel, %message, chain_id, "Content: sending message");
            }
            Action::AddDialog { dialog_set_id, entity_template, mission_id: _ } => {
                let slot = match entity_template {
                    Some(tmpl) => tmpl,
                    None => {
                        tracing::warn!(entity_id, dialog_set_id, chain_id, "AddDialog: missing entity_template — skipping");
                        continue;
                    }
                };

                tracing::info!(entity_id, dialog_set_id, slot, chain_id, "Content: add dialog (via entity_template)");

                if let Some(entry) = space_mgr.dialog_set_maps.get(&dialog_set_id).cloned() {
                    tracing::info!(
                        entity_id, dialog_set_id, slot,
                        dialog_id = entry.dialog_id,
                        interaction_flags = entry.interaction_flags,
                        "add_dialog: resolved dialog_set_map entry"
                    );

                    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
                        player.available_interactions
                            .entry(slot)
                            .or_default()
                            .push((dialog_set_id, entry.dialog_id, entry.interaction_flags));
                    }

                    send_interaction_update_if_visible(entity_id, slot, &entry, tx, space_mgr, "add_dialog").await;
                } else {
                    tracing::warn!(dialog_set_id, "dialog_set_maps cache miss for add_dialog");
                }
            }
            Action::GenerateThreat { entity_tag, threat_level } => {
                // Generate threat on the NPC (found by tag) from the player.
                // If no entity_tag, the threat is on the player entity itself (ignored by combat).
                if let Some(tag) = &entity_tag {
                    if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, tag) {
                        tracing::info!(
                            entity_id, %tag, target_id, threat_level, chain_id,
                            "Content: generate threat on NPC from player"
                        );
                        crate::cell::combat::generate_threat(
                            space_mgr,
                            entity_id,  // attacker = the player
                            target_id,  // target = the NPC
                            threat_level as f32,
                        );
                    }
                } else {
                    tracing::debug!(entity_id, threat_level, chain_id, "Content: generate threat (no target tag, skipped)");
                }
            }
            Action::SetVisible { entity_tag, visible } => {
                if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) {
                    tracing::debug!(entity_id, %entity_tag, target_id, visible, chain_id, "Content: set visible");
                    let vis_byte: u8 = if visible { 1 } else { 0 };
                    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                        entity_id: target_id,
                        method_index: crate::mercury::method_idx::ON_VISIBLE,
                        args: vec![vis_byte],
                    }).await;
                }
            }
            Action::MoveWaypoint { entity_tag, destination, speed: _ } => {
                if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) {
                    tracing::debug!(entity_id, %entity_tag, target_id, ?destination, chain_id, "Content: move waypoint");
                    space_mgr.update_entity_position(
                        target_id,
                        destination,
                        [0, 0, 0],
                        [0.0; 3],
                    );
                }
            }
            Action::SetActiveSlot { bag_id, slot } => {
                tracing::info!(entity_id, bag_id, slot, chain_id, "Content: set active slot");
                // Send onActiveSlotUpdate(bagId, slotId) — slotId is 1-indexed on wire
                let mut args = Vec::with_capacity(8);
                args.extend_from_slice(&bag_id.to_le_bytes());
                args.extend_from_slice(&(slot + 1).to_le_bytes()); // 1-indexed
                let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: 70, // onActiveSlotUpdate (method_idx::ON_ACTIVE_SLOT_UPDATE)
                    args,
                }).await;
            }
            Action::TriggerChain { chain_id: target_chain_id } => {
                tracing::debug!(entity_id, target_chain_id, chain_id, "Content: trigger chain (caller must re-dispatch)");
            }
            other => {
                tracing::debug!(entity_id, chain_id, action = ?other, "Content: unhandled action");
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Determine the inventory container for an item from the DB-loaded map.
/// Falls back to INV_Main (1) if the item has no explicit container_sets entry.
pub(super) fn item_container(item_id: i32, item_containers: &std::collections::HashMap<i32, i32>) -> i32 {
    *item_containers.get(&item_id).unwrap_or(&1)
}

/// Return the clip size and default ammo type for a granted weapon item.
///
/// Reads from the `space_mgr.item_defs` cache loaded at startup from
/// `resources.items` (see `spawner::load_item_defs`). Returns `None` for
/// non-weapon items (clip_size IS NULL in DB) or when the cache wasn't
/// populated (e.g. tests without a DB pool) — callers skip the bandolier
/// seeding in that case, and the player can still receive the item normally.
fn weapon_stats(
    item_id: i32,
    item_defs: &std::collections::HashMap<i32, crate::cell::spawner::WeaponDef>,
) -> Option<(i32, i32)> {
    item_defs.get(&item_id).map(|d| (d.clip_size, d.default_ammo_type))
}

/// Send per-player InteractionType update if the NPC is already in the player's AoI.
///
/// Shared by `AddDialogSet` and `AddDialog` actions.
async fn send_interaction_update_if_visible(
    entity_id: u32,
    slot: i32,
    entry: &crate::cell::spawner::DialogSetMapEntry,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
    label: &str,
) {
    // Update every entity sharing this template instead of an arbitrary
    // first match -- spaces with multiple template-equal NPCs would otherwise
    // get a single nondeterministic update.
    for target_id in space_mgr.find_entities_by_template(entity_id, slot) {
        let target_eid = cimmeria_common::EntityId(target_id as i32);
        let in_witness_set = space_mgr.get_entity(entity_id)
            .map_or(false, |p| p.witnesses.contains(&target_eid));

        if in_witness_set {
            let base_flags = space_mgr.get_entity(target_id)
                .map(|e| e.interaction_type_flags).unwrap_or(0);
            let merged = base_flags | entry.interaction_flags;

            tracing::debug!(
                entity_id, target_id,
                dialog_id = entry.dialog_id, base_flags, merged,
                "Sending per-player InteractionType for {}", label
            );

            let _ = tx.send(CellToBaseMsg::WitnessEntityMethod {
                witness_id: entity_id,
                entity_id: target_id,
                method_index: crate::mercury::method_idx::INTERACTION_TYPE,
                args: (merged as u64).to_le_bytes().to_vec(),
            }).await;
        } else {
            tracing::debug!(
                entity_id, target_id,
                "NPC not yet in player AoI — deferring InteractionType to AoI create"
            );
        }
    }
}
