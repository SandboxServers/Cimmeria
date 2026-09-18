//! `handle_initial_response` — the `initialResponse` cell method: recover the
//! NPC pinned by the preceding `handle_interact`, find the matching dialog in
//! the player's available interactions, and fire `onDialogDisplay` +
//! `fire_dialog_open`.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::dialog::send_dialog_display;

/// Handle initial interaction response: find a matching dialog for the given
/// `interaction_set_map_id` in the player's available interactions and display it.
///
/// Called when the client sends an `initialResponse` cell method, typically
/// after clicking an NPC whose InteractionType was set by a content chain.
#[tracing::instrument(
    name = "dialog.initial_response",
    level = "info",
    skip_all,
    fields(entity_id, interaction_set_map_id)
)]
pub async fn handle_initial_response(
    entity_id: u32,
    interaction_set_map_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Search all per-player available_interactions for a matching dialog_set_map_id.
    // The outer Option is "was the set map bound at all"; the inner one is
    // "does that bind carry a dialog" — an interaction-only bind
    // (`dialog_set_maps.dialog_id IS NULL`) is bound but has nothing to
    // display, and the two cases get different logs below.
    let matched: Option<Option<i32>> = space_mgr.get_entity(entity_id).and_then(|p| {
        for entries in p.available_interactions.values() {
            for &(dsm_id, dialog_id, _) in entries {
                if dsm_id == interaction_set_map_id {
                    return Some(dialog_id);
                }
            }
        }
        None
    });

    if let Some(None) = matched {
        // Interaction-only bind: the row exists to raise an indicator bit, not
        // to open a dialog. Sending `onDialogDisplay` with a fabricated id
        // (0 or otherwise) would open an empty dialog on the client, so bail.
        tracing::info!(
            entity_id,
            interaction_set_map_id,
            "handle_initial_response: bound set map is interaction-only (NULL dialog) -- \
             nothing to display"
        );
        return;
    }

    if let Some(Some(dialog_id)) = matched {
        // Resolve player_id only after we know we have a dialog to fire.
        // Falling back to 0 here would attribute the resulting content-engine
        // side effects (mission progress, chain triggers) to a non-existent
        // player. Mirrors the existing protection in `send_store_open`.
        let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
            Some(id) => id,
            None => {
                tracing::warn!(
                    entity_id,
                    interaction_set_map_id,
                    dialog_id,
                    "handle_initial_response: missing player_id; aborting dialog open"
                );
                return;
            }
        };
        // Wire `EntityId` of `onDialogDisplay` must be the NPC the player
        // is talking to — the client passes it through
        // `LookupEntityListenerEntry` to bind the dialog portrait actor.
        // `last_interaction_target` was set by the preceding `handle_interact`
        // (the client only sends `interactionSetMapId` here, so the NPC ID
        // has to come from the per-player pin). Falling back to `entity_id`
        // (the player) makes the dialog speak as the player and blanks the
        // portrait — same shape as python's `SGWPlayer.initialResponse`
        // returning early when `lastInteractionTarget` is unset.
        let npc_entity_id = match space_mgr
            .get_entity(entity_id)
            .and_then(|p| p.last_interaction_target)
        {
            Some(id) => id as i32,
            None => {
                tracing::warn!(
                    entity_id,
                    interaction_set_map_id,
                    dialog_id,
                    "handle_initial_response: no last_interaction_target on player; \
                     aborting dialog open -- portrait would render blank against the player"
                );
                return;
            }
        };
        tracing::info!(
            entity_id,
            interaction_set_map_id,
            dialog_id,
            npc_entity_id,
            "handle_initial_response: found dialog, sending onDialogDisplay"
        );
        send_dialog_display(entity_id, npc_entity_id, dialog_id, tx, space_mgr).await;
        crate::cell::content::fire_dialog_open(
            entity_id, player_id, dialog_id, engine, tx, space_mgr,
        )
        .await;
    } else {
        tracing::debug!(
            entity_id,
            interaction_set_map_id,
            "handle_initial_response: no matching dialog_set_map_id in available_interactions"
        );
    }
}
