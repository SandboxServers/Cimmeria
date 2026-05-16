//! Dialog action handlers: display, add/remove dialog set, add dialog (via
//! entity template).

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `Action::DisplayDialog` and `Action::StartDialog` — both render a dialog
/// to the player. They take different field names but the same id semantics,
/// merged into one handler.
pub(super) async fn display(
    dialog_id: i32,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    tracing::info!(entity_id, dialog_id, chain_id, "Content: displaying dialog");
    crate::cell::interactions::send_dialog_display(entity_id, entity_id as i32, dialog_id, tx)
        .await;
}

/// `Action::AddDialogSet` — register a dialog set on the player's
/// `available_interactions` for the given template slot, and push an
/// InteractionType update for any matching NPC already in AoI.
pub(super) async fn add_dialog_set(
    dialog_set_id: i32,
    slot: i32,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
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
            slot,
            "dialog_set_maps cache miss for add_dialog_set"
        );
    }
}

/// `Action::RemoveDialogSet` — drop the entry from
/// `available_interactions[slot]` and push an InteractionType update to
/// every entity sharing the template (with the per-entity base flags
/// merged in).
pub(super) async fn remove_dialog_set(
    dialog_set_id: i32,
    slot: i32,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
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

            if let Err(e) = tx
                .send(CellToBaseMsg::WitnessEntityMethod {
                    witness_id: entity_id,
                    entity_id: target_id,
                    method_index: crate::mercury::method_idx::INTERACTION_TYPE,
                    args: (merged as u64).to_le_bytes().to_vec(),
                })
                .await
            {
                tracing::warn!(
                    entity_id,
                    target_id,
                    dialog_set_id,
                    chain_id,
                    phase = "remove",
                    "dialog InteractionType update send failed: {e}"
                );
            }
        }
    }
}

/// `Action::AddDialog` — like `AddDialogSet` but the slot comes from the
/// action's `entity_template` field instead of a separate `slot` field.
/// Skips with a warning when `entity_template` is `None`.
pub(super) async fn add_dialog(
    dialog_set_id: i32,
    entity_template: Option<i32>,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let slot = match entity_template {
        Some(tmpl) => tmpl,
        None => {
            tracing::warn!(
                entity_id,
                dialog_set_id,
                chain_id,
                "AddDialog: missing entity_template — skipping"
            );
            return;
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

        send_interaction_update_if_visible(entity_id, slot, &entry, tx, space_mgr, "add_dialog")
            .await;
    } else {
        tracing::warn!(
            dialog_set_id,
            slot,
            "dialog_set_maps cache miss for add_dialog"
        );
    }
}

/// Send per-player InteractionType update if the NPC is already in the
/// player's AoI.
///
/// Shared by `AddDialogSet` and `AddDialog` — both register a new dialog
/// entry and need to push the resulting flags to any sibling entity that
/// shares the template and is already witnessed by the player.
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

            if let Err(e) = tx
                .send(CellToBaseMsg::WitnessEntityMethod {
                    witness_id: entity_id,
                    entity_id: target_id,
                    method_index: crate::mercury::method_idx::INTERACTION_TYPE,
                    args: (merged as u64).to_le_bytes().to_vec(),
                })
                .await
            {
                tracing::warn!(
                    entity_id,
                    target_id,
                    dialog_id = entry.dialog_id,
                    chain_id,
                    phase = label,
                    "dialog InteractionType update send failed: {e}"
                );
            }
        } else {
            tracing::debug!(
                entity_id,
                target_id,
                "NPC not yet in player AoI — deferring InteractionType to AoI create"
            );
        }
    }
}
