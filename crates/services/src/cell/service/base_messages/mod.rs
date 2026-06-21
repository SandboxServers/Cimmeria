//! Dispatch handler for `BaseToCellMsg` variants — the per-message logic that
//! the cell loop runs on each inbound base message.
//!
//! [`handle_base_message`] is a thin `match` that delegates each variant to a
//! handler in a submodule grouped by variant family:
//! - [`lifecycle`] — `CreateEntity` / `DestroyEntity` / `ConnectEntity` /
//!   `DisconnectEntity`
//! - [`movement`] — `EntityMove` (client-authoritative position update + snap-back)
//! - [`player_init`] — `InitPlayerState` (mission/ability/bandolier restore)
//! - [`ability_granted`] — `AbilityGranted` (hotbar refresh + trainer resend)
//! - [`inventory_events`] — `InventoryItemMoveApplied` / `InventoryItemRemoved` /
//!   `InventoryItemGranted` / `ItemUsed`
//! - [`bandolier`] — `UpdateBandolierItem` + `SyncBandolierItems` (weapon display)
//! - [`minigame`] — `MinigameResult`
//! - [`gm_spawn`] — `GmSpawnNpcReady`
//! - [`request_entity_update`] — `RequestEntityUpdate`

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::super::messages::{BaseToCellMsg, CellToBaseMsg};
use super::super::space_manager::SpaceManager;
use super::super::{chat, dispatch, spawner};

mod ability_granted;
mod bandolier;
mod gm_spawn;
mod inventory_events;
mod lifecycle;
mod minigame;
mod movement;
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
            lifecycle::handle_create_entity(
                entity_id,
                world_name,
                position,
                rotation,
                reply_tx,
                tx,
                space_mgr,
                spawn_records,
            )
            .await;
        }

        BaseToCellMsg::DestroyEntity { entity_id } => {
            lifecycle::handle_destroy_entity(entity_id, tx, space_mgr).await;
        }

        BaseToCellMsg::ConnectEntity { entity_id } => {
            lifecycle::handle_connect_entity(entity_id, tx, space_mgr).await;
        }

        BaseToCellMsg::DisconnectEntity { entity_id } => {
            lifecycle::handle_disconnect_entity(entity_id, tx, space_mgr).await;
        }

        BaseToCellMsg::EntityMove {
            entity_id,
            claimed_space_id,
            position,
            direction,
            velocity,
        } => {
            movement::handle_entity_move(
                entity_id,
                claimed_space_id,
                position,
                direction,
                velocity,
                tx,
                space_mgr,
            )
            .await;
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
            character_name,
        } => {
            // Cache the display name on the cell entity so cell-side seams (GM
            // `.`-console audit, mission/death/respawn Discord emits) can
            // attribute events to a name — the cell has no other source for it.
            // Set here rather than threaded through `handle_init_player_state`
            // to keep that function under the argument-count lint; the entity
            // already exists (created by the prior `ConnectEntity`).
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                entity.character_name = character_name;
            } else {
                // The entity should already exist (ConnectEntity precedes
                // InitPlayerState). If it doesn't, the name cache silently
                // fails and every cell-side emit for this player degrades to
                // `entity:<id>` — surface the ordering bug rather than hiding it.
                tracing::warn!(
                    entity_id,
                    "InitPlayerState: entity absent when caching character_name -- \
                     ConnectEntity ordering bug; cell-side emits will fall back to entity id"
                );
            }
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
            minigame::handle_minigame_result(
                entity_id,
                result_code,
                on_victory_chains,
                tx,
                space_mgr,
                engine,
            )
            .await;
        }

        BaseToCellMsg::UpdateIgnoreList {
            entity_id,
            ignore_names,
        } => {
            // Apply the player's ignore-name set to the cell entity. The AoI
            // filter (`compute_player_aoi`) and the chat belt-and-suspenders
            // check read this to exclude ignored player pairs. If the entity
            // is absent (message raced ahead of CreateEntity), drop silently
            // — the next UpdateIgnoreList (or login seed) re-applies it.
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                entity.ignore_names = ignore_names;
            } else {
                tracing::debug!(
                    entity_id,
                    "UpdateIgnoreList: entity absent -- ignore set dropped"
                );
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

        BaseToCellMsg::InventoryItemMoveApplied {
            entity_id,
            item_id,
            type_id,
            source_container_id,
            target_container_id,
            swapped_item_id,
        } => {
            inventory_events::handle_inventory_item_move_applied(
                entity_id,
                item_id,
                type_id,
                source_container_id,
                target_container_id,
                swapped_item_id,
                tx,
                space_mgr,
                engine,
            )
            .await;
        }

        BaseToCellMsg::InventoryItemRemoved {
            entity_id,
            item_id,
            source_container_id,
        } => {
            inventory_events::handle_inventory_item_removed(
                entity_id,
                item_id,
                source_container_id,
            );
        }

        BaseToCellMsg::InventoryItemGranted {
            entity_id,
            item_id,
            container_id,
            slot_id,
            quantity,
        } => {
            inventory_events::handle_inventory_item_granted(
                entity_id,
                item_id,
                container_id,
                slot_id,
                quantity,
            );
        }

        BaseToCellMsg::AbilityGranted {
            entity_id,
            ability_id,
            training_points_remaining,
        } => {
            ability_granted::handle_ability_granted(
                entity_id,
                ability_id,
                training_points_remaining,
                tx,
                space_mgr,
            )
            .await;
        }

        BaseToCellMsg::ItemUsed {
            entity_id,
            instance_id,
            type_id,
            target_id,
        } => {
            inventory_events::handle_item_used(
                entity_id,
                instance_id,
                type_id,
                target_id,
                tx,
                space_mgr,
                engine,
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
            gm_spawn::handle_gm_spawn_npc_ready(
                record,
                space_id,
                requester_entity_id,
                tx,
                space_mgr,
            )
            .await;
        }
    }
}
