//! Apply a durable Frost letter result only to its originating cell entry.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::known_names;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::messages::{
    LetterTurnInOutcome, LetterTurnInRefusal, LetterTurnInResult, LetterTurnInSession,
};
use tokio::sync::mpsc;

use super::event_dispatch::fire_mission_completed;
use super::executor::letter_turn_in::PendingLetterTurnIn;
use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::missions::{
    announce_letter_turn_in, install_letter_turn_in, valid_letter_postimage,
};
use crate::cell::space_manager::{interact_range, SpaceManager};

fn matches_entry(
    result: &LetterTurnInResult,
    pending: &PendingLetterTurnIn,
    space_mgr: &SpaceManager,
) -> bool {
    let request = result.request;
    let accepted = pending.request;
    if request.entity_id != accepted.entity_id
        || request.player_id != accepted.player_id
        || request.attempt != accepted.attempt
        || request.token != accepted.token
        || request.generation != accepted.generation
        || request.epoch != accepted.epoch
    {
        return false;
    }
    let Some(player) = space_mgr.get_entity(request.entity_id) else {
        return false;
    };
    player.is_player
        && player.player_id == Some(request.player_id)
        && player
            .extensions
            .get::<LetterTurnInSession>()
            .is_some_and(|session| {
                session.player_id == request.player_id
                    && session.token == request.token
                    && session.generation == request.generation
                    && session.epoch == request.epoch
            })
        && space_mgr
            .get_entity_world_name(request.entity_id)
            .as_deref()
            == Some("Harset_CmdCenter")
        && space_mgr.get_entity_world_id(request.entity_id) == Some(68)
        && space_mgr
            .get_entity(pending.marsh_entity_id)
            .is_some_and(|marsh| {
                marsh.tag.as_deref() == Some("CmdCenter_Marsh") && marsh.template_id == Some(10)
            })
        && space_mgr.get_entity_space_id(request.entity_id)
            == space_mgr.get_entity_space_id(pending.marsh_entity_id)
}

fn still_at_marsh(pending: &PendingLetterTurnIn, space_mgr: &SpaceManager) -> bool {
    space_mgr
        .get_entity(pending.request.entity_id)
        .is_some_and(|player| player.last_interaction_target == Some(pending.marsh_entity_id))
        && interact_range(
            pending.request.entity_id,
            pending.marsh_entity_id,
            space_mgr,
        )
        .is_ok()
}

async fn remove_letter_binding(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let flags = if let Some(player) = space_mgr.get_entity_mut(entity_id) {
        if let Some(entries) = player.available_interactions.get_mut(&10) {
            entries.retain(|&(id, _, _)| id != 120101);
            if entries.is_empty() {
                player.available_interactions.remove(&10);
            }
        }
        player
            .available_interactions
            .get(&10)
            .map(|entries| entries.iter().fold(0i64, |sum, &(_, _, flags)| sum | flags))
    } else {
        None
    };
    for marsh_id in space_mgr.find_entities_by_template(entity_id, 10) {
        let visible = space_mgr.get_entity(entity_id).is_some_and(|player| {
            player
                .witnesses
                .contains(&cimmeria_common::EntityId(marsh_id as i32))
        });
        if !visible {
            continue;
        }
        let base_flags = space_mgr
            .get_entity(marsh_id)
            .map_or(0, |marsh| marsh.interaction_type_flags);
        let is_player = space_mgr
            .get_entity(marsh_id)
            .is_some_and(|target| target.is_player);
        if let Err(error) = tx
            .send(CellToBaseMsg::WitnessEntityMethod {
                witness_id: entity_id,
                entity_id: marsh_id,
                method_index: crate::mercury::method_idx::INTERACTION_TYPE,
                args: ((base_flags | flags.unwrap_or(0)) as u64)
                    .to_le_bytes()
                    .to_vec(),
                entity_is_player: is_player,
            })
            .await
        {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                marsh_id,
                marsh_name = space_mgr.entity_label(marsh_id),
                %error,
                "Frost letter interaction update not queued"
            );
        }
    }
}

/// Consume one correlated result and continue only from A1's committed postimage.
pub async fn apply_letter_turn_in_result(
    result: LetterTurnInResult,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let entity_id = result.request.entity_id;
    let player_name = known_names::player_name(result.request.player_id);
    let Some(pending) = space_mgr
        .get_entity(entity_id)
        .and_then(|player| player.extensions.get::<PendingLetterTurnIn>())
        .copied()
    else {
        return;
    };
    if !matches_entry(&result, &pending, space_mgr) {
        tracing::debug!(
            entity_id,
            entity_name = player_name,
            player_id = result.request.player_id,
            player_name,
            reason = "stale_result",
            "Frost letter result dropped"
        );
        return;
    }
    match result.outcome {
        LetterTurnInOutcome::UnknownRetryable => {
            tracing::warn!(
                entity_id,
                entity_name = player_name,
                player_id = result.request.player_id,
                player_name,
                "Frost letter outcome uncertain; same attempt retained"
            );
        }
        LetterTurnInOutcome::Refused(reason) => {
            space_mgr
                .get_entity_mut(entity_id)
                .unwrap()
                .extensions
                .remove::<PendingLetterTurnIn>();
            tracing::info!(
                entity_id,
                entity_name = player_name,
                player_id = result.request.player_id,
                player_name,
                ?reason,
                "Frost letter turn-in refused"
            );
            let feedback = match reason {
                LetterTurnInRefusal::NoAccessibleLetter => {
                    "Frost's Letter is not in an accessible bag."
                }
                LetterTurnInRefusal::WrongMissionState => {
                    "Frost's Letter cannot be handed in at this mission step."
                }
                LetterTurnInRefusal::AlreadyCompleted => "Frost's Letter was already handed in.",
                LetterTurnInRefusal::InvalidCorrelation
                | LetterTurnInRefusal::NoPlayer
                | LetterTurnInRefusal::StateChanged => "Cannot hand in Frost's Letter right now.",
            };
            let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, feedback);
            if let Err(error) = tx.try_send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_PLAYER_COMMUNICATION,
                args,
            }) {
                tracing::warn!(
                    entity_id,
                    entity_name = player_name,
                    %error,
                    "Frost letter refusal feedback not queued"
                );
            }
        }
        LetterTurnInOutcome::Committed(committed)
        | LetterTurnInOutcome::AlreadyCommitted(committed) => {
            if !valid_letter_postimage(&committed.mission) {
                tracing::warn!(
                    entity_id,
                    entity_name = player_name,
                    player_id = result.request.player_id,
                    player_name,
                    reason = "invalid_postimage",
                    "Frost letter result retained for retry"
                );
                return;
            }
            let show_dialog = still_at_marsh(&pending, space_mgr);
            if !install_letter_turn_in(entity_id, &committed.mission, space_mgr) {
                tracing::warn!(
                    entity_id,
                    entity_name = player_name,
                    reason = "mission_tracker_locked",
                    "Frost letter postimage not installed"
                );
                return;
            }
            // Removing pending before the first await makes any duplicate or late
            // refusal inert even if a channel send yields during the continuation.
            space_mgr
                .get_entity_mut(entity_id)
                .unwrap()
                .extensions
                .remove::<PendingLetterTurnIn>();
            tracing::info!(
                entity_id,
                entity_name = player_name,
                player_id = result.request.player_id,
                player_name,
                item_id = committed.item_id,
                item_name = cimmeria_names::book().item(committed.item_id),
                stack_before = committed.stack_before,
                stack_after = committed.stack_after,
                "Frost letter persisted mission installed"
            );
            if show_dialog {
                crate::cell::interactions::send_dialog_display(
                    entity_id,
                    pending.marsh_entity_id as i32,
                    4576,
                    tx,
                    space_mgr,
                )
                .await;
            }
            remove_letter_binding(entity_id, tx, space_mgr).await;
            announce_letter_turn_in(entity_id, tx, space_mgr).await;
            fire_mission_completed(
                entity_id,
                result.request.player_id,
                1360,
                engine,
                tx,
                space_mgr,
            )
            .await;
        }
    }
}
