//! Action execution — dispatches resolved content engine actions against the
//! game state (missions, items, dialogs, interactions, etc.).

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ResolvedActions;

use cimmeria_entity::missions::{MissionObjective, STATUS_ACTIVE};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Look up a mission's current `repeats` count from the cell-side player
/// instance. Returns 0 when the entity or mission isn't tracked — the cell
/// is the authoritative source for this value, so a missing entry means
/// "no completions yet" and 0 is the correct default.
fn mission_repeats(space_mgr: &SpaceManager, entity_id: u32, mission_id: i32) -> i32 {
    space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.missions.get_mission(mission_id))
        .map(|m| m.repeats)
        .unwrap_or(0)
}

/// Execute resolved actions from the content engine against the game state.
pub(super) async fn execute_actions(
    resolved: ResolvedActions,
    entity_id: u32,
    player_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) {
    // Destructure so the inner loop can move `actions` while later
    // action branches still read trigger-time `params`. `Action::RemoveItem`
    // looks up `instance_id` here to consume the exact stack the
    // player clicked on `useItem`.
    let ResolvedActions { actions, params } = resolved;
    for (chain_id, action) in actions {
        match action {
            Action::AcceptMission { mission_id } | Action::AdvanceMission { mission_id } => {
                tracing::info!(
                    entity_id,
                    mission_id,
                    chain_id,
                    "Content: accepting mission"
                );
                if let Some(def) = space_mgr.mission_defs.get(&mission_id) {
                    let step_id = def.step_id;
                    let objectives: Vec<MissionObjective> = def
                        .objectives
                        .iter()
                        .map(|o| MissionObjective {
                            objective_id: o.objective_id,
                            status: STATUS_ACTIVE,
                            hidden: o.is_hidden,
                            optional: o.is_optional,
                        })
                        .collect();
                    crate::cell::missions::accept_mission(
                        entity_id, mission_id, step_id, objectives, tx, space_mgr,
                    )
                    .await;
                    // Read repeats AFTER the helper runs — for a re-accept of
                    // a previously-completed repeatable mission, the count
                    // restored from DB is what should round-trip back, not 0.
                    let repeats = mission_repeats(space_mgr, entity_id, mission_id);
                    if let Err(e) = tx
                        .send(CellToBaseMsg::MissionUpdate {
                            player_id,
                            mission_id,
                            status: 1,
                            current_step_id: Some(step_id),
                            completed_step_ids: vec![],
                            completed_objective_ids: vec![],
                            active_objective_ids: vec![step_id],
                            failed_objective_ids: vec![],
                            repeats,
                        })
                        .await
                    {
                        tracing::error!(
                            entity_id, player_id, mission_id, step_id,
                            chain_id, error = %e,
                            "MissionUpdate (accept) send to base failed -- mission progress not persisted"
                        );
                    }
                } else {
                    tracing::warn!(
                        mission_id,
                        chain_id,
                        "No mission_defs entry — cannot accept mission"
                    );
                }
            }
            Action::CompleteMission { mission_id } => {
                tracing::info!(
                    entity_id,
                    mission_id,
                    chain_id,
                    "Content: completing mission"
                );
                crate::cell::missions::complete_mission_direct(
                    entity_id, mission_id, tx, space_mgr,
                )
                .await;
                // Read repeats AFTER complete_mission_direct so we capture
                // the post-bump value (`MissionInstance::complete` increments).
                let repeats = mission_repeats(space_mgr, entity_id, mission_id);
                if let Err(e) = tx
                    .send(CellToBaseMsg::MissionUpdate {
                        player_id,
                        mission_id,
                        status: 2,
                        current_step_id: None,
                        completed_step_ids: vec![],
                        completed_objective_ids: vec![],
                        active_objective_ids: vec![],
                        failed_objective_ids: vec![],
                        repeats,
                    })
                    .await
                {
                    tracing::error!(
                        entity_id, player_id, mission_id, chain_id, error = %e,
                        "MissionUpdate (complete) send to base failed -- mission completion not persisted"
                    );
                }
            }
            Action::GrantItem {
                item_id,
                count,
                container_id,
            } => {
                tracing::info!(
                    entity_id,
                    item_id,
                    count,
                    chain_id,
                    "Content: granting item"
                );
                let cid = container_id
                    .filter(|&c| c > 0)
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
                    if let Some((clip, default_ammo_type)) =
                        weapon_stats(item_id, &space_mgr.item_defs)
                    {
                        if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                            // The weapon-grant chain doesn't tell us which slot
                            // the base will assign — content engine grants
                            // implicitly fill the active bandolier slot.
                            let slot_id = entity.active_bandolier_slot;
                            entity.bandolier_items.insert(
                                slot_id,
                                cimmeria_entity::cell_entity::BandolierItem {
                                    item_id,
                                    clip_size: clip,
                                    default_ammo_type,
                                    current_ammo: 0,
                                    cur_ammo_type: default_ammo_type,
                                },
                            );
                            entity.bandolier_ammo_dirty.insert(slot_id);
                            let stat_id = cimmeria_entity::stats::AMMO_SLOT_1 + slot_id;
                            if let Some(stat) = entity.stats.get_mut(stat_id) {
                                stat.update(0, 0, clip);
                                let payload = entity.stats.serialize_dirty();
                                entity.stats.clear_dirty();
                                ammo_stat_payload = Some(payload);
                            }
                            tracing::info!(
                                entity_id,
                                item_id,
                                slot_id,
                                clip,
                                "Weapon granted unloaded"
                            );
                        }
                    }
                }

                if let Err(e) = tx
                    .send(CellToBaseMsg::GrantItem {
                        entity_id,
                        player_id,
                        item_id,
                        container_id: cid,
                        count,
                    })
                    .await
                {
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
                        )
                        .await;
                    }
                }
            }
            Action::DisplayDialog { dialog_id }
            | Action::StartDialog {
                dialog_set_id: dialog_id,
            } => {
                tracing::info!(entity_id, dialog_id, chain_id, "Content: displaying dialog");
                crate::cell::interactions::send_dialog_display(
                    entity_id,
                    entity_id as i32,
                    dialog_id,
                    tx,
                )
                .await;
            }
            Action::PlaySequence { sequence_id } => {
                tracing::info!(
                    entity_id,
                    sequence_id,
                    chain_id,
                    "Content: playing sequence"
                );
                let mut args = Vec::with_capacity(26);
                args.extend_from_slice(&sequence_id.to_le_bytes()); // KismetEventSetSeqID
                args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // SourceID
                args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // TargetID
                args.push(1); // PrimaryTarget = true
                args.extend_from_slice(&0.0f32.to_le_bytes()); // ImpactTime
                args.extend_from_slice(&0u32.to_le_bytes()); // NameValuePairs count = 0
                args.push(0); // ViewType = 0
                args.extend_from_slice(&0i32.to_le_bytes()); // InstanceId
                let _ = tx
                    .send(CellToBaseMsg::EntityMethodCall {
                        entity_id,
                        method_index: 1, // onSequence (SGWSpawnableEntity)
                        args,
                    })
                    .await;
            }
            Action::AdvanceStep {
                mission_id,
                step_id,
            } => {
                tracing::info!(
                    entity_id,
                    mission_id,
                    step_id,
                    chain_id,
                    "Content: advancing step"
                );
                crate::cell::missions::advance_step(entity_id, mission_id, step_id, tx, space_mgr)
                    .await;
                let repeats = mission_repeats(space_mgr, entity_id, mission_id);
                if let Err(e) = tx
                    .send(CellToBaseMsg::MissionUpdate {
                        player_id,
                        mission_id,
                        status: 1,
                        current_step_id: Some(step_id),
                        completed_step_ids: vec![],
                        completed_objective_ids: vec![],
                        active_objective_ids: vec![step_id],
                        failed_objective_ids: vec![],
                        repeats,
                    })
                    .await
                {
                    tracing::error!(
                        entity_id, player_id, mission_id, step_id,
                        chain_id, error = %e,
                        "MissionUpdate (advance step) send to base failed -- step progress not persisted"
                    );
                }
            }
            Action::AddDialogSet {
                dialog_set_id,
                slot,
                mission_id: _,
            } => {
                tracing::info!(
                    entity_id,
                    dialog_set_id,
                    slot,
                    chain_id,
                    "Content: adding dialog set"
                );

                if let Some(entry) = space_mgr.dialog_set_maps.get(&dialog_set_id).cloned() {
                    tracing::info!(
                        entity_id,
                        dialog_set_id,
                        slot,
                        dialog_id = entry.dialog_id,
                        interaction_flags = entry.interaction_flags,
                        "add_dialog_set: resolved dialog_set_map entry"
                    );

                    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
                        player
                            .available_interactions
                            .entry(slot)
                            .or_default()
                            .push((dialog_set_id, entry.dialog_id, entry.interaction_flags));

                        tracing::info!(
                            entity_id,
                            slot,
                            interactions_count = player
                                .available_interactions
                                .get(&slot)
                                .map_or(0, |v| v.len()),
                            "add_dialog_set: stored in available_interactions"
                        );
                    }

                    send_interaction_update_if_visible(
                        entity_id,
                        slot,
                        &entry,
                        tx,
                        space_mgr,
                        "add_dialog_set",
                    )
                    .await;
                } else {
                    tracing::warn!(
                        dialog_set_id,
                        "dialog_set_maps cache miss for add_dialog_set"
                    );
                }
            }
            Action::RemoveDialogSet {
                dialog_set_id,
                slot,
            } => {
                tracing::info!(
                    entity_id,
                    dialog_set_id,
                    slot,
                    chain_id,
                    "Content: removing dialog set"
                );

                let removed_flags = if let Some(player) = space_mgr.get_entity_mut(entity_id) {
                    if let Some(entries) = player.available_interactions.get_mut(&slot) {
                        entries.retain(|&(dsm_id, _, _)| dsm_id != dialog_set_id);
                        if entries.is_empty() {
                            player.available_interactions.remove(&slot);
                        }
                    }
                    player
                        .available_interactions
                        .get(&slot)
                        .map(|entries| entries.iter().fold(0i64, |acc, &(_, _, flags)| acc | flags))
                } else {
                    None
                };

                // Update every entity sharing this template -- `.first()` would
                // arbitrarily pick one (HashMap iteration order is nondeterministic),
                // leaving sibling entities with stale interaction flags.
                for target_id in space_mgr.find_entities_by_template(entity_id, slot) {
                    let target_eid = cimmeria_common::EntityId(target_id as i32);
                    let in_witness_set = space_mgr
                        .get_entity(entity_id)
                        .is_some_and(|p| p.witnesses.contains(&target_eid));

                    if in_witness_set {
                        let base_flags = space_mgr
                            .get_entity(target_id)
                            .map(|e| e.interaction_type_flags)
                            .unwrap_or(0);
                        let merged = base_flags | removed_flags.unwrap_or(0);

                        let _ = tx
                            .send(CellToBaseMsg::WitnessEntityMethod {
                                witness_id: entity_id,
                                entity_id: target_id,
                                method_index: crate::mercury::method_idx::INTERACTION_TYPE,
                                args: (merged as u64).to_le_bytes().to_vec(),
                            })
                            .await;
                    }
                }
            }
            Action::RemoveItem { item_id, count } => {
                // Prefer the originating inventory `instance_id` if the
                // chain context carries one (set by `fire_item_use` —
                // the OnItemUse dispatch path). This is the difference
                // between "consume the slappack the player clicked" and
                // "consume the leftmost slappack of that type": when a
                // player has two stacks of the same item and clicks the
                // second one, the first stack must NOT be silently
                // consumed instead.
                //
                // For chains fired by other paths (mission events,
                // ambernol vial consumption via `enter_region`, etc.)
                // there's no instance_id in context, so we fall back to
                // the by-type resolution that picks the player's first
                // matching instance. Both paths converge on the same
                // wire-update sequence on the base side.
                let instance_id = params
                    .get("instance_id")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32)
                    .filter(|&v| v != 0);

                let send_result =
                    match instance_id {
                        Some(instance) => {
                            tracing::info!(
                            entity_id, player_id, instance, type_id = item_id, count, chain_id,
                            "Content: RemoveItem → RemoveInventoryItem (by instance from context)"
                        );
                            tx.send(CellToBaseMsg::RemoveInventoryItem {
                                entity_id,
                                player_id,
                                item_id: instance,
                                quantity: count,
                            })
                            .await
                        }
                        None => {
                            tracing::info!(
                                entity_id,
                                player_id,
                                type_id = item_id,
                                count,
                                chain_id,
                                "Content: RemoveItem → RemoveInventoryItemByType"
                            );
                            tx.send(CellToBaseMsg::RemoveInventoryItemByType {
                                entity_id,
                                player_id,
                                type_id: item_id,
                                count,
                            })
                            .await
                        }
                    };

                if let Err(e) = send_result {
                    // Saturated/closed channel — the consume silently
                    // skips otherwise. Surface it loudly so missions
                    // that depend on the removal (e.g., FindAmbernol
                    // chain 1034 consumes the vial) don't silently
                    // strand the player with the item still in their
                    // bag while the chain reports completion.
                    tracing::error!(
                        entity_id, player_id, type_id = item_id, count, chain_id,
                        error = %e,
                        "Content: RemoveItem cell→base channel send failed — item NOT removed"
                    );
                }
            }
            Action::ChangeStat {
                stat_id,
                min,
                max,
                set_to_max,
                amount,
                use_ammo_stat,
            } => {
                // `use_ammo_stat=true` (or the legacy `stat_id=-1` sentinel
                // chain 2011 uses) means "resolve to the entity's active
                // ammo slot stat at apply time". That bandolier-aware
                // resolution is its own piece of work — implementing it
                // here would also need to track which slot the action
                // targets across grant / equip / reload cycles. Warn so
                // the silent no-op is visible in production logs (the
                // calling chain almost certainly expects the side
                // effect); flip back to info/debug when the resolution
                // path lands.
                if use_ammo_stat == Some(true) || stat_id < 0 {
                    tracing::warn!(
                        entity_id,
                        chain_id,
                        stat_id,
                        ?use_ammo_stat,
                        "Content: ChangeStat use_ammo_stat / negative stat_id \
                         is not yet implemented; skipping (deliberate stub)"
                    );
                    continue;
                }

                // Apply bounds first so `set_to_max` and `amount` see the
                // adjusted range, then `set_to_max`, then `amount` (the
                // delta path consumables use). All stat mutations bump the
                // dirty flag; `serialize_dirty` collects them into one
                // `onStatUpdate` payload.
                let payload = match space_mgr.get_entity_mut(entity_id) {
                    Some(entity) => match entity.stats.get_mut(stat_id) {
                        Some(stat) => {
                            if let Some(new_min) = min {
                                stat.set_min(new_min);
                            }
                            if let Some(new_max) = max {
                                stat.set_max(new_max);
                            }
                            if set_to_max == Some(true) {
                                stat.set_current(stat.max);
                            }
                            if let Some(delta) = amount {
                                stat.change(delta);
                            }
                            tracing::info!(
                                entity_id,
                                stat_id,
                                ?min,
                                ?max,
                                ?set_to_max,
                                ?amount,
                                cur = stat.cur,
                                max = stat.max,
                                chain_id,
                                "Content: ChangeStat applied"
                            );
                            let p = entity.stats.serialize_dirty();
                            entity.stats.clear_dirty();
                            p
                        }
                        None => {
                            tracing::warn!(
                                entity_id,
                                stat_id,
                                chain_id,
                                "Content: ChangeStat target stat not found"
                            );
                            Vec::new()
                        }
                    },
                    None => {
                        tracing::warn!(
                            entity_id,
                            chain_id,
                            "Content: ChangeStat source entity not found"
                        );
                        Vec::new()
                    }
                };

                if !payload.is_empty() {
                    if let Err(e) = tx
                        .send(CellToBaseMsg::EntityMethodCall {
                            entity_id,
                            method_index: crate::mercury::method_idx::ON_STAT_UPDATE,
                            args: payload,
                        })
                        .await
                    {
                        tracing::error!(
                            entity_id, chain_id, error = %e,
                            "Content: ChangeStat onStatUpdate send failed"
                        );
                    }
                }
            }
            Action::SetInteractionType {
                entity_tag,
                operation,
                mask,
            } => {
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
                            let _ = tx
                                .send(CellToBaseMsg::WitnessEntityMethod {
                                    witness_id,
                                    entity_id: target_id,
                                    method_index: crate::mercury::method_idx::INTERACTION_TYPE,
                                    args: (flags as u64).to_le_bytes().to_vec(),
                                })
                                .await;
                        }
                    }
                } else {
                    tracing::debug!(entity_id, %entity_tag, chain_id, "Content: entity tag not found for SetInteractionType");
                }
            }
            Action::StartMinigame {
                minigame_type,
                on_victory_chains,
            } => {
                tracing::info!(entity_id, %minigame_type, ?on_victory_chains, chain_id, "Content: starting minigame");
                let _ = tx
                    .send(CellToBaseMsg::StartMinigame {
                        entity_id,
                        player_id,
                        game_name: minigame_type.clone(),
                        difficulty: 1, // TODO: parse from chain params when difficulty field is added
                        on_victory_chains: on_victory_chains.clone(),
                    })
                    .await;
            }
            Action::SetAggression {
                entity_tag,
                level: agg_level,
            } => {
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
                tracing::info!(
                    entity_id,
                    region_id,
                    chain_id,
                    "Content: triggering transporter"
                );
                crate::cell::ring_transport::handle_interact(
                    region_id, entity_id, tx, space_mgr, engine,
                )
                .await;
            }
            Action::Teleport { space_id, position } => {
                tracing::info!(
                    entity_id,
                    space_id,
                    ?position,
                    chain_id,
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
                // `update_entity_position` already writes `cell_entity.position`.
                space_mgr.update_entity_position(entity_id, position, [0, 0, 0], [0.0; 3]);
                // SpaceId is i32 in the cell (matches DB type) but the wire
                // forced-position packet is u32 — space ids are always
                // non-negative, so the cast is a width-only conversion.
                let cell_space_id = space_mgr
                    .get_entity(entity_id)
                    .map(|e| e.space_id.0 as u32)
                    .unwrap_or(space_id as u32);
                let _ = tx
                    .send(CellToBaseMsg::TeleportPlayer {
                        entity_id,
                        space_id: cell_space_id,
                        position,
                    })
                    .await;
            }
            Action::SystemMessage { message_id } => {
                // TODO: Wire format for system messages is unknown. The previous
                // implementation incorrectly used onPlayerCommunication (method 28)
                // which caused garbled chat spam ("[] says") and client freezes.
                // Needs RE to find the correct client method for localized string
                // ID display (possibly onErrorCode or a UI-specific method).
                tracing::info!(
                    entity_id,
                    message_id,
                    chain_id,
                    "Content: system message (stub — correct wire format TBD)"
                );
            }
            Action::AbandonMission { mission_id } => {
                tracing::info!(
                    entity_id,
                    mission_id,
                    chain_id,
                    "Content: abandoning mission"
                );
                crate::cell::missions::abandon_mission(entity_id, mission_id, tx, space_mgr).await;
            }
            Action::IncrementCounter {
                counter_name,
                amount,
            } => {
                // Counter values live on the cell entity and are read by
                // `Condition::Counter` via `populate_mission_context`
                // writing `counter_<name>` into the chain context.
                //
                // Resolve/execute ordering gotcha: chains are resolved
                // (conditions evaluated against ctx) before any actions
                // run, so a sibling completion chain on the same
                // trigger event reads the PRE-increment counter value.
                // For "kill N targets to complete" the completion
                // condition must be `counter >= N - 1` so it fires on
                // the kill that brings the counter to N. Documented at
                // each call site that uses this pattern (e.g.,
                // castle_cellblock_chains.sql chain 1087 / 1094).
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    let entry = entity.counters.entry(counter_name.clone()).or_insert(0);
                    *entry = entry.saturating_add(amount);
                    tracing::debug!(
                        entity_id, player_id, %counter_name, amount,
                        new_value = *entry, chain_id,
                        "Content: incremented counter"
                    );
                } else {
                    tracing::warn!(
                        entity_id, %counter_name, chain_id,
                        "Content: increment_counter source entity missing — counter not updated"
                    );
                }
            }
            Action::ResetCounter { counter_name } => {
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    let removed = entity.counters.remove(&counter_name);
                    tracing::debug!(
                        entity_id, player_id, %counter_name, ?removed, chain_id,
                        "Content: reset counter"
                    );
                }
            }
            Action::CompleteObjective {
                mission_id,
                objective_id,
            } => {
                tracing::info!(
                    entity_id,
                    mission_id,
                    objective_id,
                    chain_id,
                    "Content: complete objective"
                );
                crate::cell::missions::complete_objective(
                    entity_id,
                    mission_id,
                    objective_id,
                    tx,
                    space_mgr,
                )
                .await;
            }
            Action::SendMessage { channel, message } => {
                tracing::info!(entity_id, %channel, %message, chain_id, "Content: sending message");
            }
            Action::AddDialog {
                dialog_set_id,
                entity_template,
                mission_id: _,
            } => {
                let slot = match entity_template {
                    Some(tmpl) => tmpl,
                    None => {
                        tracing::warn!(
                            entity_id,
                            dialog_set_id,
                            chain_id,
                            "AddDialog: missing entity_template — skipping"
                        );
                        continue;
                    }
                };

                tracing::info!(
                    entity_id,
                    dialog_set_id,
                    slot,
                    chain_id,
                    "Content: add dialog (via entity_template)"
                );

                if let Some(entry) = space_mgr.dialog_set_maps.get(&dialog_set_id).cloned() {
                    tracing::info!(
                        entity_id,
                        dialog_set_id,
                        slot,
                        dialog_id = entry.dialog_id,
                        interaction_flags = entry.interaction_flags,
                        "add_dialog: resolved dialog_set_map entry"
                    );

                    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
                        player
                            .available_interactions
                            .entry(slot)
                            .or_default()
                            .push((dialog_set_id, entry.dialog_id, entry.interaction_flags));
                    }

                    send_interaction_update_if_visible(
                        entity_id,
                        slot,
                        &entry,
                        tx,
                        space_mgr,
                        "add_dialog",
                    )
                    .await;
                } else {
                    tracing::warn!(dialog_set_id, "dialog_set_maps cache miss for add_dialog");
                }
            }
            Action::GenerateThreat {
                entity_tag,
                threat_level,
            } => {
                // Generate threat on the NPC (found by tag) from the player.
                // If no entity_tag, the threat is on the player entity itself (ignored by combat).
                if let Some(tag) = &entity_tag {
                    if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, tag) {
                        tracing::info!(
                            entity_id, %tag, target_id, threat_level, chain_id,
                            "Content: generate threat on NPC from player"
                        );
                        if let Some(new_state) = crate::cell::combat::generate_threat(
                            space_mgr,
                            entity_id, // attacker = the player
                            target_id, // target = the NPC
                            threat_level as f32,
                        ) {
                            // Player just entered combat — broadcast state
                            // change so the client flips in-combat HUD.
                            crate::cell::abilities::send_entity_method(
                                entity_id,
                                crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                                new_state.to_le_bytes().to_vec(),
                                tx,
                                space_mgr,
                            )
                            .await;
                        }
                    }
                } else {
                    tracing::debug!(
                        entity_id,
                        threat_level,
                        chain_id,
                        "Content: generate threat (no target tag, skipped)"
                    );
                }
            }
            Action::SetVisible {
                entity_tag,
                visible,
            } => {
                if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) {
                    tracing::debug!(entity_id, %entity_tag, target_id, visible, chain_id, "Content: set visible");
                    let vis_byte: u8 = if visible { 1 } else { 0 };
                    let _ = tx
                        .send(CellToBaseMsg::EntityMethodCall {
                            entity_id: target_id,
                            method_index: crate::mercury::method_idx::ON_VISIBLE,
                            args: vec![vis_byte],
                        })
                        .await;
                }
            }
            Action::MoveWaypoint {
                entity_tag,
                destination,
                speed: _,
            } => {
                if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) {
                    tracing::debug!(entity_id, %entity_tag, target_id, ?destination, chain_id, "Content: move waypoint");
                    space_mgr.update_entity_position(target_id, destination, [0, 0, 0], [0.0; 3]);
                }
            }
            Action::SetActiveSlot { bag_id, slot } => {
                tracing::info!(
                    entity_id,
                    bag_id,
                    slot,
                    chain_id,
                    "Content: set active slot"
                );
                // Send onActiveSlotUpdate(bagId, slotId) — slotId is 1-indexed on wire
                let mut args = Vec::with_capacity(8);
                args.extend_from_slice(&bag_id.to_le_bytes());
                args.extend_from_slice(&(slot + 1).to_le_bytes()); // 1-indexed
                let _ = tx
                    .send(CellToBaseMsg::EntityMethodCall {
                        entity_id,
                        method_index: 70, // onActiveSlotUpdate (method_idx::ON_ACTIVE_SLOT_UPDATE)
                        args,
                    })
                    .await;
            }
            Action::TriggerChain {
                chain_id: target_chain_id,
            } => {
                tracing::debug!(
                    entity_id,
                    target_chain_id,
                    chain_id,
                    "Content: trigger chain (caller must re-dispatch)"
                );
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
pub(super) fn item_container(
    item_id: i32,
    item_containers: &std::collections::HashMap<i32, i32>,
) -> i32 {
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
    item_defs
        .get(&item_id)
        .map(|d| (d.clip_size, d.default_ammo_type))
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
        let in_witness_set = space_mgr
            .get_entity(entity_id)
            .is_some_and(|p| p.witnesses.contains(&target_eid));

        if in_witness_set {
            let base_flags = space_mgr
                .get_entity(target_id)
                .map(|e| e.interaction_type_flags)
                .unwrap_or(0);
            let merged = base_flags | entry.interaction_flags;

            tracing::debug!(
                entity_id,
                target_id,
                dialog_id = entry.dialog_id,
                base_flags,
                merged,
                "Sending per-player InteractionType for {}",
                label
            );

            let _ = tx
                .send(CellToBaseMsg::WitnessEntityMethod {
                    witness_id: entity_id,
                    entity_id: target_id,
                    method_index: crate::mercury::method_idx::INTERACTION_TYPE,
                    args: (merged as u64).to_le_bytes().to_vec(),
                })
                .await;
        } else {
            tracing::debug!(
                entity_id,
                target_id,
                "NPC not yet in player AoI — deferring InteractionType to AoI create"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};

    fn make_space_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr
    }

    /// Build a connected player at id=1 with HEALTH at `cur/max`, dirty
    /// flags cleared so a single `serialize_dirty` reads only what
    /// `Action::ChangeStat` writes. Used by the heal-action tests below.
    fn make_player_with_health(mgr: &mut SpaceManager, cur: i32, max: i32) {
        use cimmeria_entity::stats::HEALTH;
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(42);
            if let Some(h) = e.stats.get_mut(HEALTH) {
                h.update(0, cur, max);
            }
            e.stats.clear_dirty();
        }
        mgr.connect_entity(1);
    }

    /// `change_stat { amount: +500 }` is the canonical heal-on-use shape
    /// (Health Slappack TC1, chain 4001). Three things must hold: HP
    /// advances by exactly the delta when room is available, the change
    /// is broadcast as a single onStatUpdate carrying the new HEALTH
    /// value, and the entity's dirty state is drained so a follow-up
    /// serialize doesn't re-emit the same stat.
    #[tokio::test]
    async fn change_stat_amount_advances_health_and_emits_on_stat_update() {
        use cimmeria_entity::stats::HEALTH;

        let mut mgr = make_space_mgr();
        make_player_with_health(&mut mgr, 200, 1000);

        let (tx, mut rx) = mpsc::channel(8);
        let engine = ChainEngine::new();
        let resolved = ResolvedActions {
            params: std::collections::HashMap::new(),
            actions: vec![(
                4001,
                Action::ChangeStat {
                    stat_id: HEALTH,
                    min: None,
                    max: None,
                    use_ammo_stat: None,
                    set_to_max: None,
                    amount: Some(500),
                },
            )],
        };

        execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

        // Entity state.
        let entity = mgr.get_entity(1).unwrap();
        assert_eq!(
            entity.stats.get(HEALTH).unwrap().cur,
            700,
            "HEALTH.cur must advance by +500"
        );
        assert!(
            !entity.stats.has_dirty(),
            "executor must clear dirty after sending — otherwise the next \
             serialize would re-emit the same stat"
        );

        // Wire frame.
        let msg = rx.try_recv().expect("expected onStatUpdate");
        match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => {
                assert_eq!(entity_id, 1);
                assert_eq!(method_index, crate::mercury::method_idx::ON_STAT_UPDATE);
                let count = u32::from_le_bytes([args[0], args[1], args[2], args[3]]) as usize;
                let mut found = false;
                for i in 0..count {
                    let off = 4 + i * 16;
                    let stat_id = i32::from_le_bytes([
                        args[off],
                        args[off + 1],
                        args[off + 2],
                        args[off + 3],
                    ]);
                    if stat_id == HEALTH {
                        let cur = i32::from_le_bytes([
                            args[off + 8],
                            args[off + 9],
                            args[off + 10],
                            args[off + 11],
                        ]);
                        assert_eq!(cur, 700, "wire payload carries the post-heal HEALTH.cur");
                        found = true;
                    }
                }
                assert!(found, "onStatUpdate payload must include HEALTH");
            }
            other => panic!("expected EntityMethodCall, got {other:?}"),
        }

        assert!(rx.try_recv().is_err(), "no further messages expected");
    }

    /// Heal must clamp at `max` — a slappack on a near-full bar can't
    /// push the wire payload to `cur > max`. The clamp is `Stat::change`'s
    /// responsibility; this guard pins that the executor doesn't bypass
    /// it (e.g., by writing `cur` directly).
    #[tokio::test]
    async fn change_stat_amount_clamps_to_max() {
        use cimmeria_entity::stats::HEALTH;

        let mut mgr = make_space_mgr();
        make_player_with_health(&mut mgr, 950, 1000);

        let (tx, _rx) = mpsc::channel(8);
        let engine = ChainEngine::new();
        let resolved = ResolvedActions {
            params: std::collections::HashMap::new(),
            actions: vec![(
                4001,
                Action::ChangeStat {
                    stat_id: HEALTH,
                    min: None,
                    max: None,
                    use_ammo_stat: None,
                    set_to_max: None,
                    amount: Some(500),
                },
            )],
        };

        execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

        let h = mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap();
        assert_eq!(
            h.cur, 1000,
            "heal clamps to max even on +500 over a 50-room bar"
        );
        assert!(h.cur <= h.max, "wire invariant cur <= max preserved");
    }

    /// Negative `amount` damages the stat. The same code path serves
    /// debuff/poison-style chains; if the executor ever silently ignored
    /// negative deltas (e.g., `cur += amount.max(0)`) this guard fails.
    #[tokio::test]
    async fn change_stat_negative_amount_damages_stat() {
        use cimmeria_entity::stats::HEALTH;

        let mut mgr = make_space_mgr();
        make_player_with_health(&mut mgr, 800, 1000);

        let (tx, _rx) = mpsc::channel(8);
        let engine = ChainEngine::new();
        let resolved = ResolvedActions {
            params: std::collections::HashMap::new(),
            actions: vec![(
                4001,
                Action::ChangeStat {
                    stat_id: HEALTH,
                    min: None,
                    max: None,
                    use_ammo_stat: None,
                    set_to_max: None,
                    amount: Some(-300),
                },
            )],
        };

        execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

        assert_eq!(
            mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur,
            500,
            "negative amount must subtract from cur"
        );
    }

    /// `set_to_max: true` snaps `cur` to `max` regardless of the prior
    /// value. Pairs with the legacy reload-effect chain (effect_id-driven,
    /// `set_to_max=true` on the ammo stat) so the bounds-modifying path
    /// stays exercised alongside the new `amount` path.
    #[tokio::test]
    async fn change_stat_set_to_max_snaps_current_to_max() {
        use cimmeria_entity::stats::HEALTH;

        let mut mgr = make_space_mgr();
        make_player_with_health(&mut mgr, 100, 1000);

        let (tx, _rx) = mpsc::channel(8);
        let engine = ChainEngine::new();
        let resolved = ResolvedActions {
            params: std::collections::HashMap::new(),
            actions: vec![(
                4001,
                Action::ChangeStat {
                    stat_id: HEALTH,
                    min: None,
                    max: None,
                    use_ammo_stat: None,
                    set_to_max: Some(true),
                    amount: None,
                },
            )],
        };

        execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

        let h = mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap();
        assert_eq!(h.cur, 1000, "set_to_max snaps cur to max");
        assert_eq!(h.max, 1000, "max unchanged");
    }

    /// `use_ammo_stat=true` (and its legacy `stat_id=-1` sentinel form,
    /// used by the seeded Reload effect chain 2011) must skip cleanly
    /// rather than warn-and-no-op on a missing stat lookup. Pin the
    /// no-side-effects shape: HP unchanged, no wire message sent. When
    /// active-ammo-slot resolution lands, this test should be replaced
    /// with one that asserts the resolved ammo stat actually mutates.
    #[tokio::test]
    async fn change_stat_with_use_ammo_stat_skips_cleanly() {
        use cimmeria_entity::stats::HEALTH;

        let mut mgr = make_space_mgr();
        make_player_with_health(&mut mgr, 500, 1000);

        let (tx, mut rx) = mpsc::channel(8);
        let engine = ChainEngine::new();
        let resolved = ResolvedActions {
            params: std::collections::HashMap::new(),
            actions: vec![
                (
                    2011,
                    Action::ChangeStat {
                        stat_id: -1, // legacy sentinel for "ammo stat"
                        min: None,
                        max: None,
                        use_ammo_stat: Some(true),
                        set_to_max: Some(true),
                        amount: None,
                    },
                ),
                (
                    2011,
                    Action::ChangeStat {
                        stat_id: -1, // negative-only path also skips
                        min: None,
                        max: None,
                        use_ammo_stat: None,
                        set_to_max: None,
                        amount: Some(50),
                    },
                ),
            ],
        };

        execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

        let h = mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap();
        assert_eq!(
            h.cur, 500,
            "HEALTH untouched — ammo-stat path is unimplemented"
        );
        assert!(
            rx.try_recv().is_err(),
            "no onStatUpdate must fire when the ammo-stat path skips"
        );
    }

    /// Regression for #95: `Action::RemoveItem` must route through the new
    /// `RemoveInventoryItemByType` cell→base RPC, not the silently-ignored
    /// stub it used to be. Locks in the chain-driven removal path that
    /// chain 1034 (FindAmbernol consume) depends on.
    #[tokio::test]
    async fn remove_item_action_emits_remove_inventory_by_type() {
        let mut mgr = make_space_mgr();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();

        let (tx, mut rx) = mpsc::channel(8);
        let engine = ChainEngine::new();
        let resolved = ResolvedActions {
            params: std::collections::HashMap::new(),
            actions: vec![(
                1034,
                Action::RemoveItem {
                    item_id: 19,
                    count: 1,
                },
            )],
        };

        execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

        let msg = rx.try_recv().expect("expected RemoveInventoryItemByType");
        match msg {
            CellToBaseMsg::RemoveInventoryItemByType {
                entity_id,
                player_id,
                type_id,
                count,
            } => {
                assert_eq!(entity_id, 1);
                assert_eq!(player_id, 42);
                assert_eq!(type_id, 19);
                assert_eq!(count, 1);
            }
            other => panic!("expected RemoveInventoryItemByType, got {:?}", other),
        }
    }
}
