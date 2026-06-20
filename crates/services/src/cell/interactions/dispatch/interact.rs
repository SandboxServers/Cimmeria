//! `handle_interact` — the `interact(targetEntityId)` cell method: validate
//! the player + target, distance-check, pin the interaction target, then
//! dispatch by per-player available interaction or static interaction type.

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::NpcInteractionType;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::dialog::send_dialog_display;
use super::super::loot::send_loot_display;
use super::super::vendor::send_store_open;
use super::MAX_INTERACT_DISTANCE;

/// Handle `interact(targetEntityId)` cell method call.
///
/// Flow:
/// 1. Validate player and target entities exist
/// 2. Check distance (max 5.0 units)
/// 3. Look up target's interaction type
/// 4. Send appropriate client method response
///
/// Returns `Some(dialog_id)` if a dialog was opened (for content engine events).
#[tracing::instrument(
    name = "interaction.interact",
    level = "info",
    skip_all,
    fields(entity_id, target_entity_id, space_id = tracing::field::Empty)
)]
pub async fn handle_interact(
    entity_id: u32,
    target_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<i32> {
    // Validate player exists
    let (player_pos, player_space_id) = match space_mgr.get_entity(entity_id) {
        Some(e) => (e.position, e.space_id.0),
        None => {
            tracing::warn!(entity_id, "interact: player entity not found");
            return None;
        }
    };
    tracing::Span::current().record("space_id", player_space_id);

    // Validate target exists and get interaction data
    let (target_pos, interaction_type, npc_name, target_template_id) =
        match space_mgr.get_entity(target_entity_id) {
            Some(e) => (
                e.position,
                e.interaction_type.clone(),
                e.npc_name.clone().unwrap_or_default(),
                e.template_id,
            ),
            None => {
                tracing::info!(
                    entity_id,
                    target_entity_id,
                    "interact: target entity not found"
                );
                return None;
            }
        };

    tracing::info!(
        entity_id, target_entity_id, %npc_name,
        ?interaction_type, ?target_template_id,
        "interact: target resolved"
    );

    // Distance check
    let dist = player_pos.distance_squared_to(&target_pos).sqrt();
    if dist > MAX_INTERACT_DISTANCE {
        tracing::info!(entity_id, target_entity_id, dist, "interact: too far away");
        return None;
    }

    // Pin the interaction target on the player. Downstream dispatchers
    // that don't get the NPC entity ID on their own wire frame
    // (`handle_initial_response`, content-engine `display_dialog`
    // fired from a follow-up trigger like `OnDialogChoice`) read this
    // to fill the wire `EntityId` field of `onDialogDisplay`. The
    // dialog portrait widget binds its actor by that ID via
    // `LookupEntityListenerEntry`, so passing the player's ID there
    // (the prior bug) made the dialog speak as the player. Mirrors
    // python's `SGWPlayer.lastInteractionTarget` write in `interact()`.
    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
        player.last_interaction_target = Some(target_entity_id);
    }

    // Check per-player available interactions (from add_dialog_set content actions).
    // These take priority over static interaction_type.
    if let Some(tmpl_id) = target_template_id {
        let dialog_id = space_mgr
            .get_entity(entity_id)
            .and_then(|p| p.available_interactions.get(&tmpl_id))
            .and_then(|entries| entries.first())
            .map(|&(_, dialog_id, _)| dialog_id);

        if let Some(dialog_id) = dialog_id {
            tracing::info!(
                entity_id,
                target_entity_id,
                tmpl_id,
                dialog_id,
                "interact: per-player dialog set → onDialogDisplay"
            );
            send_dialog_display(entity_id, target_entity_id as i32, dialog_id, tx, space_mgr).await;
            return Some(dialog_id);
        } else {
            tracing::info!(
                entity_id,
                tmpl_id,
                "interact: no per-player interactions for template"
            );
        }
    }

    // Dispatch based on static interaction type
    match interaction_type {
        Some(NpcInteractionType::Dialog { dialog_id }) => {
            tracing::info!(
                entity_id,
                target_entity_id,
                dialog_id,
                "interact: static dialog → onDialogDisplay"
            );
            send_dialog_display(entity_id, target_entity_id as i32, dialog_id, tx, space_mgr).await;
            Some(dialog_id)
        }
        Some(NpcInteractionType::Vendor) => {
            tracing::info!(
                entity_id,
                target_entity_id,
                "interact: vendor → OpenVendorStore"
            );
            send_store_open(entity_id, target_entity_id, tx, space_mgr).await;
            None
        }
        Some(NpcInteractionType::Trainer { archetype_id }) => {
            // Deprecated routing arm. The canonical trainer path is the
            // `template_trainer_lists` lookup in
            // `cell_methods/player/interaction.rs::dispatch`, which calls
            // `interactions::try_open_trainer` BEFORE falling through to
            // `handle_interact`. By the time control reaches this arm,
            // `try_open_trainer` has already either opened the trainer
            // (returning `true`, in which case we don't get here) or
            // determined the NPC isn't a trainer (no `trainer_ability_list_id`
            // on the template). If the NPC has the `Trainer` tag but no
            // template-registered list, the only sane action is to log
            // and drop — emitting `onTrainerOpen` from here would bypass
            // the per-archetype offering, already-known, level, and
            // prereq filters that the canonical handler enforces.
            tracing::warn!(
                target: "abilities",
                event = "trainer_deprecated_routing_arm",
                entity_id,
                target_entity_id,
                archetype_id,
                reason = "deprecated_routing_arm",
                "interact: deprecated NpcInteractionType::Trainer arm hit — \
                 template_trainer_lists is the canonical path; set \
                 entity_templates.trainer_ability_list_id on this NPC's \
                 template instead of using the Trainer interaction tag"
            );
            None
        }
        Some(NpcInteractionType::Loot) => {
            tracing::info!(
                entity_id,
                target_entity_id,
                "interact: loot → onLootDisplay"
            );
            // Track which entity the player is looting (for lootItem calls)
            if let Some(player) = space_mgr.get_entity_mut(entity_id) {
                player.looting_entity = Some(target_entity_id);
            }
            send_loot_display(entity_id, target_entity_id as i32, 1, tx, space_mgr).await;
            None
        }
        None => {
            tracing::info!(
                entity_id,
                target_entity_id,
                "interact: target has no static interaction type"
            );
            None
        }
    }
}
