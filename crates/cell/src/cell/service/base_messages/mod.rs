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
//! - [`respec`] — `AbilitiesReset` (trainer respec mirror + burst, AT-08)
//! - [`gm_abilities`] — `GmAbilitiesChanged` (GM give-all / reset mirror +
//!   burst, AB-N2)
//! - [`content_ability_grant`] — `ContentAbilitiesGranted` (the content
//!   `grant_ability` mirror: known set, branch credit, learned lines; CS-01a)
//! - [`inventory_events`] — `InventoryItemMoveApplied` / `InventoryItemRemoved` /
//!   `InventoryItemGranted` / `ItemUsed`
//! - `ItemUseConsumed` goes straight to `cell::content::apply_consumed_item`
//!   (the native consumable round trip)
//! - [`bandolier`] — `UpdateBandolierItem` + `SyncBandolierItems` (weapon display)
//! - [`minigame`] — `MinigameResult`
//! - [`gm_spawn`] — `GmSpawnNpcReady`, `GmSpawnNpcFailed`
//! - [`request_entity_update`] — `RequestEntityUpdate`
//! - `LootGrantRefused` goes straight to `cell::interactions` (the item goes
//!   back on its corpse)
//! - [`org`] — `Org` (organization traffic: the squad invite and kick)
//! - [`bank`] — `Bank` (the Team and Command vault grant, BV-07)
//! - `Duel` goes straight to `cell::duel::challenge` (SS-D1)
//! - [`ignore`] — `UpdateIgnoreList` (the Ignore set spatial chat reads, SS-C1)

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::super::messages::{BaseToCellMsg, CellToBaseMsg};
use super::super::space_manager::SpaceManager;
use super::super::{chat, dispatch, spawner};

mod ability_granted;
mod bandolier;
mod bank;
mod content_ability_grant;
mod gm_abilities;
mod gm_spawn;
mod ignore;
mod init_grant_merge;
mod inventory_events;
mod lab_console;
mod lab_query;
pub(in crate::cell::service) mod lifecycle;
mod minigame;
mod movement;
mod org;
mod passive_sync;
pub(crate) mod player_init;
mod request_entity_update;
mod respec;

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
            destination_space_id,
            account_id,
            player_id,
            account_name,
            player_name,
            reply_tx,
        } => {
            let is_player = player_id.is_some();
            lifecycle::handle_create_entity(
                entity_id,
                world_name,
                position,
                rotation,
                destination_space_id,
                lifecycle::BirthIdentity {
                    account_id,
                    player_id,
                    account_name,
                    player_name,
                },
                reply_tx,
                tx,
                space_mgr,
                spawn_records,
            )
            .await;
            // TG-DB-12: mission changes wait until `InitPlayerState` says
            // whether the saved missions loaded.
            if is_player {
                crate::cell::missions::state_lock::mark_load_pending(entity_id, space_mgr);
            }
        }

        BaseToCellMsg::DestroyEntity { entity_id } => {
            lifecycle::handle_destroy_entity(entity_id, tx, space_mgr).await;
        }

        BaseToCellMsg::ConnectEntity { entity_id } => {
            lifecycle::handle_connect_entity(entity_id, tx, space_mgr).await;
        }

        BaseToCellMsg::LetterTurnInSession { entity_id, session } => {
            if space_mgr.get_entity_world_name(entity_id).as_deref() != Some("Harset_CmdCenter") {
                return;
            }
            let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
                return;
            };
            if !entity.is_player || entity.player_id != Some(session.player_id) {
                return;
            }
            if let Some(current) = entity
                .extensions
                .get::<cimmeria_wire::cell::messages::LetterTurnInSession>()
            {
                let incoming = (session.generation, session.epoch);
                let stored = (current.generation, current.epoch);
                if incoming < stored || (incoming == stored && session.token != current.token) {
                    return;
                }
            }
            entity.extensions.insert(session);
        }

        BaseToCellMsg::LetterTurnInResult(result) => {
            crate::cell::content::apply_letter_turn_in_result(result, tx, space_mgr, engine).await;
        }

        BaseToCellMsg::DisconnectEntity {
            entity_id,
            reply_tx,
        } => {
            lifecycle::handle_disconnect_entity(entity_id, tx, space_mgr).await;
            // Defensible silent send: the base may have dropped the
            // receiver already (a caller that never returns the id to a
            // free list, e.g. `gate_travel::abandon_unspaced_session`, has
            // nothing to gate on it) -- see the negative-logging
            // convention's oneshot-reply exception.
            let _ = reply_tx.send(());
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
            packet_seq,
        } => {
            dispatch::dispatch_cell_method(
                entity_id,
                method_index,
                &args,
                tx,
                space_mgr,
                engine,
                packet_seq,
            )
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

        msg @ BaseToCellMsg::InitPlayerState { .. } => {
            player_init::dispatch::handle(msg, tx, space_mgr, engine).await;
        }

        // TG-DB-12: the saved missions were unreadable at world entry. The
        // lock arrives before `InitPlayerState`, so no `player_loaded` chain
        // runs against the empty tracker unlocked.
        BaseToCellMsg::MissionStateUnknown { entity_id } => {
            crate::cell::missions::state_lock::mark_state_unknown(entity_id, space_mgr);
        }

        BaseToCellMsg::SavedMissionsRecovered {
            entity_id,
            saved_missions,
        } => {
            let restored =
                player_init::mission_restore::build_restored_missions(&saved_missions, space_mgr);
            if crate::cell::missions::state_lock::recover_state(entity_id, restored, tx, space_mgr)
                .await
            {
                // Replay the `player_loaded` chains the lock dropped.
                crate::cell::content::replay_deferred_player_loaded(
                    entity_id, engine, tx, space_mgr,
                )
                .await;
            }
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

        BaseToCellMsg::BroadcastToWitnesses {
            entity_id,
            method_index,
            args,
        } => {
            let witness_count = crate::cell::abilities::send_entity_method_to_witnesses(
                entity_id,
                method_index,
                args,
                tx,
                space_mgr,
            )
            .await;
            // TG-AOI-02: the base-built appearance (equip, holster) reached
            // `witness_count` observers. Zero means nobody saw the change.
            let who = space_mgr.player_identity(entity_id);
            tracing::debug!(
                target: "aoi.appearance",
                event = "appearance_rebroadcast",
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                method_index,
                method_name = cimmeria_wire::names::entity_client_method(
                    who.player_id.is_some(),
                    method_index
                ),
                witness_count,
                "BroadcastToWitnesses: entity method rebroadcast to witnesses"
            );
        }

        BaseToCellMsg::LabConsoleExec {
            entity_id,
            line,
            reply_tx,
        } => {
            lab_console::handle_lab_console_exec(entity_id, line, reply_tx, tx, space_mgr, engine)
                .await;
        }

        BaseToCellMsg::LabQuery { query, reply_tx } => {
            // Read-only: pass an immutable borrow so the handler cannot mutate
            // simulation state.
            lab_query::handle_lab_query(query, reply_tx, space_mgr);
        }

        BaseToCellMsg::MinigameResult {
            entity_id,
            player_id,
            result_code,
            on_victory_chains,
        } => {
            minigame::handle_minigame_result(
                entity_id,
                player_id,
                result_code,
                on_victory_chains,
                tx,
                space_mgr,
                engine,
            )
            .await;
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
                space_mgr,
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
                space_mgr,
            );
        }

        BaseToCellMsg::AbilityGranted {
            entity_id,
            ability_id,
            training_points,
            tree_points_spent,
        } => {
            ability_granted::handle_ability_granted(
                ability_granted::Granted {
                    entity_id,
                    ability_id,
                    training_points,
                    tree_points_spent,
                },
                tx,
                space_mgr,
            )
            .await;
        }

        BaseToCellMsg::ProgressionChanged {
            entity_id,
            level,
            training_points,
        } => {
            ability_granted::handle_progression_changed(
                entity_id,
                level,
                training_points,
                space_mgr,
            );
        }

        BaseToCellMsg::TrainingPointsGranted {
            entity_id,
            training_points,
        } => {
            ability_granted::handle_training_points_granted(
                entity_id,
                training_points,
                tx,
                space_mgr,
            )
            .await;
        }

        BaseToCellMsg::GmAbilityGranted {
            entity_id,
            player_id,
            ability_id,
        } => {
            ability_granted::handle_gm_ability_granted(
                entity_id, player_id, ability_id, tx, space_mgr,
            )
            .await;
        }

        BaseToCellMsg::AbilitiesReset {
            entity_id,
            player_id,
            outcome,
        } => {
            respec::handle_abilities_reset(entity_id, player_id, outcome, tx, space_mgr).await;
        }

        BaseToCellMsg::GmAbilitiesChanged(changed) => {
            gm_abilities::handle_gm_abilities_changed(changed, tx, space_mgr).await;
        }

        BaseToCellMsg::ContentAbilitiesGranted(granted) => {
            content_ability_grant::handle_content_abilities_granted(granted, tx, space_mgr).await;
        }

        BaseToCellMsg::TutorialRecorded(recorded) => {
            crate::cell::content::apply_tutorial_recorded(recorded, tx, space_mgr).await;
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

        // The base consumed a native consumable's unit: apply its ability
        // (`cell::content::consumable_use`).
        BaseToCellMsg::ItemUseConsumed(consumed) => {
            crate::cell::content::apply_consumed_item(consumed, tx, space_mgr).await;
        }

        // The special-ammo reserve round trip (ammo campaign AM-02).
        BaseToCellMsg::AmmoReserve(answer) => {
            cimmeria_cell_combat::cell::cell_methods::player::world::reload_reserve::handle_reserve_answer(
                answer, tx, space_mgr,
            )
            .await;
        }

        BaseToCellMsg::RequestEntityUpdate {
            witness_id,
            entity_ids,
        } => {
            request_entity_update::handle(witness_id, entity_ids, space_mgr).await;
        }

        BaseToCellMsg::GmSpawnNpcReady {
            record,
            space_id,
            requester_entity_id,
            batch,
        } => {
            gm_spawn::handle_gm_spawn_npc_ready(
                record,
                space_id,
                requester_entity_id,
                batch,
                tx,
                space_mgr,
            )
            .await;
        }

        BaseToCellMsg::GmSpawnNpcFailed {
            requester_entity_id,
            batch,
            template_id,
            reason,
        } => {
            gm_spawn::handle_gm_spawn_npc_failed(
                requester_entity_id,
                batch,
                template_id,
                reason,
                tx,
                space_mgr,
            )
            .await;
        }

        BaseToCellMsg::LootGrantRefused {
            entity_id,
            player_id,
            source,
            design_id,
            quantity,
            container_id,
            reason,
        } => {
            crate::cell::interactions::handle_loot_grant_refused(
                entity_id,
                player_id,
                source,
                design_id,
                quantity,
                container_id,
                reason,
                tx,
                space_mgr,
            )
            .await;
        }

        BaseToCellMsg::Org(org_msg) => org::handle(org_msg, tx, space_mgr).await,

        BaseToCellMsg::Bank(bank_msg) => bank::handle(bank_msg, tx, space_mgr).await,

        BaseToCellMsg::Duel(duel_msg) => {
            super::super::duel::challenge::handle(duel_msg, tx, space_mgr).await;
        }
        BaseToCellMsg::UpdateIgnoreList {
            entity_id,
            player_id,
            account_id,
            version,
            ignore_names,
        } => ignore::handle(
            entity_id,
            player_id,
            account_id,
            version,
            ignore_names,
            space_mgr,
        ),
    }
}
