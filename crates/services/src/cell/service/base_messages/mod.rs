//! Dispatch handler for `BaseToCellMsg` variants — the per-message logic that
//! the cell loop runs on each inbound base message.
//!
//! Large handler bodies are extracted into submodules by variant family:
//! - [`player_init`] — `InitPlayerState` (mission/ability/bandolier restore)
//! - [`bandolier`] — `UpdateBandolierItem` + `SyncBandolierItems` (weapon display)

use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::super::content;
use super::super::messages::{BaseToCellMsg, CellToBaseMsg};
use super::super::space_manager::{ClientMoveOutcome, SpaceManager};
use super::super::{chat, dispatch, spawner};
use cimmeria_entity::movement_validation::MovementReject;

/// Stable metric/log label for a movement reject reason. Kept low-
/// cardinality (one token per layer) so the `movement_validation_rejects_total`
/// counter stays aggregatable.
fn movement_reject_label(reason: MovementReject) -> &'static str {
    match reason {
        MovementReject::OutOfBounds => "bounds",
        MovementReject::OffNavmesh => "navmesh",
        MovementReject::Teleport => "teleport",
    }
}

mod bandolier;
mod player_init;
mod request_entity_update;

#[cfg(test)]
mod tests;

/// 1-in-N sampling rate for player position updates. At the 10 Hz
/// client update rate, 10 = ~1 sample per second per active player —
/// enough to spot teleports / rubber-banding / stuck positions without
/// the per-frame noise. Bump up (e.g. 50) when the field is quiet
/// and movement is the least interesting signal.
const PLAYER_MOVE_LOG_SAMPLE: u64 = 10;

/// Process-wide counter for player-move sampling. Atomic so multi-cell
/// (future) doesn't need refactoring; single-cell (today) is just an
/// inc + modulo.
static PLAYER_MOVE_COUNTER: AtomicU64 = AtomicU64::new(0);

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
            // Cancel any open trade BEFORE the rest of the teardown: the
            // surviving partner needs an onTradeResults(Cancelled) +
            // their own trade state cleared, otherwise their session
            // becomes a stranded ghost. Python relied on BigWorld GC for
            // this; Rust has to do it explicitly (deep dive gap).
            super::super::cell_methods::player::trade::cancel_trade_on_disconnect(
                entity_id, tx, space_mgr,
            )
            .await;
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

        BaseToCellMsg::DisconnectEntity { entity_id } => {
            tracing::debug!(entity_id, "DisconnectEntity");
            // Same as DestroyEntity: tear down any open trade with
            // Cancelled before the entity is removed. The disconnect
            // path doesn't reach the DestroyEntity arm directly (it
            // calls `space_mgr.disconnect_entity` which internally calls
            // destroy_entity), so this hook lives in both places — a
            // disconnect mid-trade has to notify the surviving partner.
            super::super::cell_methods::player::trade::cancel_trade_on_disconnect(
                entity_id, tx, space_mgr,
            )
            .await;
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
            claimed_space_id,
            position,
            direction,
            velocity,
        } => {
            tracing::trace!(entity_id, ?position, "EntityMove");
            // Server↔client space divergence. The write below is
            // server-authoritative (it uses the cell's own `entity_space`
            // binding, never `claimed_space_id`), so a mismatch cannot
            // corrupt the spatial grid; it is warn-only and exists to make
            // gate-travel / instance-reset races observable. A claimed id
            // of 0 is the pre-confirmation sentinel the client sends
            // before its space is bound — skip it to avoid benign startup
            // noise. Only a *known* binding that differs is a real
            // divergence: when the entity has no binding (`None`) the
            // packet is a stale post-disconnect leftover that the apply
            // below drops as `EntityMissing`, not a space mismatch.
            if let Some(actual_space_id) = space_mgr.get_entity_space_id(entity_id) {
                if claimed_space_id != 0 && actual_space_id != claimed_space_id {
                    tracing::warn!(
                        target: "movement.validation",
                        entity_id,
                        claimed_space_id,
                        actual_space_id,
                        reason = "space_mismatch",
                        "movement.space_mismatch: client claims a different space than \
                         the server binding (warn-only — write uses the server binding)"
                    );
                    cimmeria_observability::counter!(
                        "movement_validation_warns_total",
                        "reason" => "space_mismatch",
                    );
                }
            }
            // 1-in-N sampled debug log on the canonical player-move
            // target. Player movement is high volume (~10 Hz per
            // active player) and rarely the bug source, so sampling
            // gives operators "this player is alive and moving"
            // confirmation without flooding the log stream.
            let sample = PLAYER_MOVE_COUNTER.fetch_add(1, Ordering::Relaxed);
            if sample.is_multiple_of(PLAYER_MOVE_LOG_SAMPLE) {
                tracing::debug!(
                    target: "movement.player",
                    event = "position_update",
                    entity_id,
                    x = position[0],
                    y = position[1],
                    z = position[2],
                    vx = velocity[0],
                    vy = velocity[1],
                    vz = velocity[2],
                    sample_index = sample,
                    "player position update (sampled)"
                );
            }
            // Client-authoritative position updates go through the
            // movement validator. Server-authoritative paths (ring
            // transport, respawn, content teleport, NPC movement) call
            // `update_entity_position` directly and bypass validation —
            // they are the source of truth for those entities and
            // already snap via `BASEMSG_FORCED_POSITION` where needed.
            let outcome =
                space_mgr.apply_client_position_update(entity_id, position, direction, velocity);
            match outcome {
                ClientMoveOutcome::Accepted { .. } => {}
                ClientMoveOutcome::EntityMissing => {
                    // Stale inbound after destroy / disconnect.
                    // Matches the legacy silent-drop shape of
                    // `update_entity_position`; surface as debug so a
                    // future deluge here is queryable but doesn't
                    // alarm by default.
                    tracing::debug!(
                        target: "movement.validation",
                        entity_id,
                        reason = "entity_missing",
                        "EntityMove dropped: entity not in any space (likely post-disconnect)"
                    );
                }
                ClientMoveOutcome::Rejected {
                    reason,
                    last_valid,
                    space_id,
                    bounds,
                } => {
                    // Negative-log per docs/architecture/negative-logging-convention.md.
                    // `reason` carries the validation layer that fired
                    // (`bounds` | `navmesh` | `teleport`); `bounds_min`/
                    // `bounds_max` let an operator confirm which AABB the
                    // proposed position was tested against without grepping.
                    let reason_label = movement_reject_label(reason);
                    tracing::warn!(
                        target: "movement.validation",
                        entity_id,
                        space_id,
                        client_x = position[0],
                        client_y = position[1],
                        client_z = position[2],
                        last_valid_x = last_valid[0],
                        last_valid_y = last_valid[1],
                        last_valid_z = last_valid[2],
                        bounds_min_x = bounds.min[0],
                        bounds_min_y = bounds.min[1],
                        bounds_min_z = bounds.min[2],
                        bounds_max_x = bounds.max[0],
                        bounds_max_y = bounds.max[1],
                        bounds_max_z = bounds.max[2],
                        reason = reason_label,
                        reject = ?reason,
                        "movement.validation_reject: client position rejected by the \
                         {reason_label} layer — snapping back to last valid via FORCED_POSITION"
                    );
                    // One low-cardinality `reason` label per layer so the
                    // reject rate stays aggregatable without per-entity tags.
                    cimmeria_observability::counter!(
                        "movement_validation_rejects_total",
                        "reason" => reason_label,
                    );
                    // Snap the offending client back. The cell entity's
                    // position was NOT advanced — the next AoI tick
                    // (100 ms) rebroadcasts the last-valid position to
                    // witnesses, so witnesses never see the rejected
                    // coordinates. TeleportPlayer routes through
                    // `handle_teleport_player` which emits
                    // `BASEMSG_FORCED_POSITION` to the owner; the
                    // existing teleport bundle is the right primitive.
                    if let Err(e) = tx
                        .send(CellToBaseMsg::TeleportPlayer {
                            entity_id,
                            space_id,
                            position: last_valid,
                            prev_pos: last_valid,
                        })
                        .await
                    {
                        tracing::warn!(
                            entity_id,
                            space_id,
                            error = %e,
                            reason = "snap_back_send_failed",
                            "movement.bounds_violation: snap-back \
                             TeleportPlayer send to base failed — \
                             client will continue desynced"
                        );
                    }
                }
            }
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
                engine,
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
            system_options,
            state_field,
            access_level,
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
                system_options,
                state_field,
                access_level,
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

        BaseToCellMsg::AbilityGranted {
            entity_id,
            ability_id,
            training_points_remaining,
        } => {
            // Base persisted + debited; mirror onto the cell entity and
            // refresh the client hotbar via the shared helper.
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                entity.abilities.add_ability(ability_id);
            }
            tracing::info!(
                target: "abilities",
                event = "granted",
                entity_id,
                ability_id,
                training_points_remaining,
                "AbilityGranted: cell mirrored + hotbar refresh"
            );
            player_init::send_known_abilities_update(entity_id, tx, space_mgr).await;

            // Python parity (`AbilityTrainer.onTrainAbility:128`): if the
            // newly-learned ability is a prerequisite for another offered
            // ability, OR the player just ran out of training points, the
            // trainer list should refresh so the client's UI updates the
            // greyed-out state. Without this, the player sees a stale list
            // with the dependent ability still greyed out until they close
            // and re-open the trainer.
            //
            // **Contract — "resend on ANY grant while pinned":** this fires
            // for every `AbilityGranted` while `last_interaction_target` is
            // set, regardless of whether the granted ability is in the
            // trainer's offered list. We delegate the "is this newly-unlocked
            // a prereq for B?" decision to `try_open_trainer` itself, which
            // recomputes every `trainable` flag from current state (known
            // set, level, prereqs). This matches Python's
            // `AbilityTrainer.onTrainAbility` behavior: it re-fires
            // `onTrainerOpen` unconditionally after a successful train RPC.
            // `try_open_trainer` short-circuits to `false` when the pinned
            // target isn't a trainer template, so non-trainer NPCs pinned
            // as `last_interaction_target` (vendors, lootables, dialog NPCs)
            // never trigger a resend.
            //
            // `last_interaction_target` is set by `handle_interact` and
            // not cleared on trainer close. Trade-off: if a player opens
            // a trainer, closes it, then earns an ability some other way
            // (chain `Action::GrantAbility` from a quest turn-in), we'd
            // emit a spurious `onTrainerOpen`. The client tolerates an
            // unsolicited `onTrainerOpen` when the trainer window isn't
            // visible (UEvent_UI_TrainerOpen handler just shows the panel),
            // so this is harmless.
            let trainer_entity_id = space_mgr
                .get_entity(entity_id)
                .and_then(|p| p.last_interaction_target);
            if let Some(target) = trainer_entity_id {
                let is_trainer = space_mgr
                    .get_entity(target)
                    .and_then(|t| t.template_id)
                    .is_some_and(|tid| space_mgr.template_trainer_lists.contains_key(&tid));
                if is_trainer {
                    tracing::debug!(
                        target: "abilities",
                        event = "trainer_resend",
                        entity_id,
                        ability_id,
                        trainer_entity_id = target,
                        training_points_remaining,
                        "AbilityGranted: re-sending onTrainerOpen to refresh trainable flags"
                    );
                    let _ = crate::cell::interactions::try_open_trainer(
                        entity_id, target, tx, space_mgr,
                    )
                    .await;
                }
            }
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

        BaseToCellMsg::GmSpawnNpcReady {
            record,
            space_id,
            requester_entity_id,
        } => {
            // Base resolved the gmSpawnByCmd template into a SpawnRecord —
            // allocate an NPC id and drop it into the target space. AoI fanout
            // handles client visibility on the next tick, so there's no extra
            // send here (same as DB-seeded NPC spawns).
            let id = space_mgr.allocate_npc_id();
            match space_mgr.spawn_npc_from_record_in_space(id, &record, space_id) {
                // `record` is a Box<SpawnRecord>; `&record` coerces to
                // `&SpawnRecord` via auto-deref at the call site.
                Ok(placed_space) => {
                    tracing::info!(
                        npc_entity_id = id,
                        template_id = record.template_id,
                        template_name = %record.template_name,
                        space_id = placed_space,
                        x = record.x,
                        y = record.y,
                        z = record.z,
                        "GmSpawnNpcReady: NPC spawned"
                    );
                    // Definitive success line: the spawn actually took, so the
                    // GM gets confirmation with the real new NPC id. This is
                    // the cell-side completion of the gmSpawnByCmd round-trip
                    // (the cell-side "requested" optimism was removed).
                    crate::cell::cell_methods::gm::feedback::send_gm_feedback(
                        requester_entity_id,
                        &format!(
                            "gmSpawnByCmd: spawned npc {id} (template {})",
                            record.template_id
                        ),
                        tx,
                    )
                    .await;
                }
                Err(e) => {
                    tracing::warn!(
                        npc_entity_id = id,
                        template_id = record.template_id,
                        space_id,
                        "GmSpawnNpcReady: spawn failed: {e}"
                    );
                    crate::cell::cell_methods::gm::feedback::send_gm_feedback(
                        requester_entity_id,
                        &format!(
                            "gmSpawnByCmd: spawn failed for template {}",
                            record.template_id
                        ),
                        tx,
                    )
                    .await;
                }
            }
        }
    }
}
