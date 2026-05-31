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
use super::super::space_manager::SpaceManager;
use super::super::{chat, dispatch, spawner};

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
            system_options,
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
            // We delegate the "is this newly-unlocked a prereq for B?"
            // decision to `try_open_trainer` itself — it recomputes every
            // `trainable` flag from current state (known set, level,
            // prereqs). Calling it whenever the player has a trainer pinned
            // is idempotent and matches Python's "always-resend on grant"
            // shape. `try_open_trainer` short-circuits to `false` when the
            // pinned target isn't a trainer template, so the only NPCs
            // that trigger a resend here are real trainers.
            //
            // `last_interaction_target` is set by `handle_interact` and
            // not cleared on trainer close. Trade-off: if a player opens
            // a trainer, closes it, then earns an ability some other way
            // (chain `Action::GrantAbility` from a quest turn-in), we'd
            // emit a spurious `onTrainerOpen`. The client tolerates an
            // unsolicited `onTrainerOpen` when the trainer window isn't
            // visible (UEvent_UI_TrainerOpen handler just shows the
            // panel), so this is harmless. See issue #55 deep dive Item B.
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
    }
}
