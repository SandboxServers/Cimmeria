//! Dialog action handlers: display, add/remove dialog set, add dialog (via
//! entity template).

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `Action::DisplayDialog` and `Action::StartDialog` — both render a dialog
/// to the player. They take different field names but the same id semantics,
/// merged into one handler.
///
/// The wire `EntityId` field of `onDialogDisplay` is the key the client
/// uses for `LookupEntityListenerEntry` to bind the dialog portrait actor
/// and the per-screen speaker entity (see
/// `docs/reverse-engineering/findings/dialog-portrait-lookup.md`).
/// Resolution order, from most to least direct:
///
/// 1. Chain `params["target_entity_id"]` — present when the chain was
///    fired from an `InteractTag` / `InteractTemplate` trigger
///    (`fire_interact_*` stamps it into the context).
/// 2. Player's `last_interaction_target` — the per-player pin set by
///    `handle_interact`. Covers follow-up chains (e.g. an
///    `OnDialogChoice` trigger that fires another `display_dialog` on
///    the same NPC) where the trigger event itself carries no NPC.
/// 3. **Monologue fallback** — if the dialog is in
///    `space_mgr.monologue_dialog_ids` (every screen has
///    `speaker_id = 0`, i.e. player-narration / inner-thought), bind
///    the player as the wire EntityId. The client's per-screen lookup
///    of `speaker_id = 0` naturally falls back to the player's name and
///    the portrait shows the player — exactly the intended render for
///    "the player is talking to themselves." Without this branch,
///    monologue dialogs (~42% of authored screens) silently never
///    display.
/// 4. Abort with a `warn` — the dialog is NPC-shaped but no NPC could
///    be resolved. Binding the player here would blank the NPC portrait
///    and substitute the player's name for every NPC line — the bug
///    that motivated the original gate. Bail loud so the chain author
///    can fix the missing target_entity_id wire-up.
#[tracing::instrument(
    name = "dialog.display",
    level = "info",
    skip_all,
    fields(entity_id, dialog_id, chain_id, npc_entity_id = tracing::field::Empty, monologue = tracing::field::Empty)
)]
pub(super) async fn display(
    dialog_id: i32,
    entity_id: u32,
    chain_id: i64,
    params: &std::collections::HashMap<String, serde_json::Value>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let npc_entity_id = params
        .get("target_entity_id")
        .and_then(|v| v.as_u64())
        .map(|v| v as i32)
        .or_else(|| {
            space_mgr
                .get_entity(entity_id)
                .and_then(|p| p.last_interaction_target)
                .map(|id| id as i32)
        });

    let npc_entity_id = match npc_entity_id {
        Some(id) => id,
        None => {
            // Monologue fallback — see doc on this fn for rationale.
            if space_mgr.monologue_dialog_ids.contains(&dialog_id) {
                tracing::Span::current().record("monologue", true);
                tracing::info!(
                    entity_id,
                    dialog_id,
                    chain_id,
                    "DisplayDialog: no NPC resolved, dialog is player-monologue \
                     (all screens speaker_id=0) — binding player as context entity"
                );
                entity_id as i32
            } else {
                tracing::warn!(
                    entity_id,
                    dialog_id,
                    chain_id,
                    "DisplayDialog: no NPC entity id in chain params or last_interaction_target -- \
                     cannot send onDialogDisplay (would bind player as speaker and blank portrait)"
                );
                return;
            }
        }
    };

    tracing::Span::current().record("npc_entity_id", npc_entity_id);
    tracing::info!(
        entity_id,
        dialog_id,
        npc_entity_id,
        chain_id,
        "Content: displaying dialog"
    );
    crate::cell::interactions::send_dialog_display(entity_id, npc_entity_id, dialog_id, tx).await;
}

/// `Action::AddDialogSet` — register a dialog set on the player's
/// `available_interactions` for the given template slot, and push an
/// InteractionType update for any matching NPC already in AoI.
#[tracing::instrument(
    name = "dialog.add_set",
    level = "info",
    skip_all,
    fields(entity_id, dialog_set_id, slot, chain_id)
)]
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
            "dialog_set_maps cache miss for add_dialog_set"
        );
    }
}

/// `Action::RemoveDialogSet` — drop the entry from
/// `available_interactions[slot]` and push an InteractionType update to
/// every entity sharing the template (with the per-entity base flags
/// merged in).
#[tracing::instrument(
    name = "dialog.remove_set",
    level = "info",
    skip_all,
    fields(entity_id, dialog_set_id, slot, chain_id)
)]
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
                // dropped interaction-type push leaves the
                // NPC with stale flags on the client (showing/hiding the
                // wrong interaction prompt).
                tracing::warn!(
                    entity_id,
                    target_id,
                    dialog_set_id,
                    slot,
                    chain_id,
                    phase = "remove",
                    "RemoveDialogSet: cell→base interaction-type send failed -- NPC interaction prompt stale: {e}"
                );
            }
        }
    }
}

/// `Action::AddDialog` — like `AddDialogSet` but the slot comes from the
/// action's `entity_template` field instead of a separate `slot` field.
/// Skips with a warning when `entity_template` is `None`.
#[tracing::instrument(
    name = "dialog.add",
    level = "info",
    skip_all,
    fields(entity_id, dialog_set_id, entity_template = ?entity_template, chain_id),
)]
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
        tracing::warn!(dialog_set_id, "dialog_set_maps cache miss for add_dialog");
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
                // same shape as remove_dialog_set —
                // dropped interaction-type push leaves the NPC with
                // stale flags.
                tracing::warn!(
                    entity_id,
                    target_id,
                    dialog_id = entry.dialog_id,
                    slot,
                    phase = label,
                    "interaction-type send failed -- NPC prompt stale: {e}"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{make_space_manager, LogCapture};
    use std::collections::HashMap;
    use tracing::Level;

    fn empty_params() -> HashMap<String, serde_json::Value> {
        HashMap::new()
    }

    /// Chain `params["target_entity_id"]` is the most direct source — it's
    /// stamped by `fire_interact_*` for chains fired off an interact. The
    /// wire `EntityId` of `onDialogDisplay` must match that, not the
    /// player's id.
    #[tokio::test]
    async fn display_uses_target_entity_id_from_chain_params() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

        const NPC_ID: u32 = 0xABCD;
        let mut params = empty_params();
        params.insert("target_entity_id".into(), serde_json::json!(NPC_ID as u64));

        let (tx, mut rx) = mpsc::channel(4);
        display(
            /* dialog_id */ 4001, /* entity_id */ 1, /* chain_id */ 99, &params,
            &tx, &mgr,
        )
        .await;

        let msg = rx.try_recv().expect("must emit onDialogDisplay");
        match msg {
            CellToBaseMsg::EntityMethodCall { args, .. } => {
                let wire_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_eq!(
                    wire_entity_id as u32, NPC_ID,
                    "params.target_entity_id must win over any fallback; got {wire_entity_id}, expected {NPC_ID}"
                );
            }
            other => panic!("expected EntityMethodCall, got {other:?}"),
        }
    }

    /// When the chain didn't stamp `target_entity_id` (e.g. an
    /// `OnDialogChoice`-triggered follow-up dialog), fall back to the
    /// player's `last_interaction_target` pin.
    #[tokio::test]
    async fn display_falls_back_to_last_interaction_target() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        const NPC_ID: u32 = 0xBEEF;
        if let Some(p) = mgr.get_entity_mut(1) {
            p.last_interaction_target = Some(NPC_ID);
        }

        let params = empty_params();
        let (tx, mut rx) = mpsc::channel(4);
        display(2299, 1, 1021, &params, &tx, &mgr).await;

        let msg = rx.try_recv().expect("must emit onDialogDisplay");
        match msg {
            CellToBaseMsg::EntityMethodCall { args, .. } => {
                let wire_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_eq!(wire_entity_id as u32, NPC_ID);
            }
            other => panic!("expected EntityMethodCall, got {other:?}"),
        }
    }

    /// With neither `target_entity_id` in params nor a
    /// `last_interaction_target` pin, the handler must abort with a warn
    /// rather than emit a wire frame that binds the player as the
    /// speaker. The warn level is load-bearing — operators rely on it
    /// to correlate "dialog never opened" with a chain that wasn't
    /// fired off an interact path.
    #[tokio::test]
    async fn display_aborts_with_warn_when_no_npc_id_available() {
        let capture = LogCapture::install();
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        // last_interaction_target intentionally None.

        let params = empty_params();
        let (tx, mut rx) = mpsc::channel(4);
        display(4001, 1, 9999, &params, &tx, &mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "must not emit onDialogDisplay when no NPC id can be resolved"
        );
        assert!(
            capture
                .find_message(Level::WARN, "DisplayDialog: no NPC entity id")
                .is_some(),
            "abort must surface a WARN — silent return masks the chain-author bug"
        );
    }

    /// When no NPC resolves and the dialog is in the monologue cache,
    /// the wire EntityId must be the player's own id (not bail). The
    /// per-screen `speaker_id = 0` lookup on the client falls back to
    /// the player's name, rendering as inner thought.
    #[tokio::test]
    async fn display_binds_player_when_no_npc_and_dialog_is_monologue() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        // Cache dialog 2982 as a known monologue.
        mgr.monologue_dialog_ids.insert(2982);

        let params = empty_params();
        let (tx, mut rx) = mpsc::channel(4);
        display(2982, 1, 1001, &params, &tx, &mgr).await;

        let msg = rx.try_recv().expect(
            "monologue dialog must emit onDialogDisplay even with no NPC \
             resolved — reverting the monologue branch in display() fails here",
        );
        match msg {
            CellToBaseMsg::EntityMethodCall { args, .. } => {
                let wire_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_eq!(
                    wire_entity_id, 1,
                    "monologue must bind the player's own entity id as the wire EntityId"
                );
            }
            other => panic!("expected EntityMethodCall, got {other:?}"),
        }
    }

    /// Companion guard: a dialog NOT in the monologue cache still
    /// bails when no NPC resolves. Pins that the fallback is gated on
    /// the cache, not an "accept anything" hole — an NPC dialog whose
    /// chain context was lost must still warn, because binding the
    /// player there would blank the NPC portrait and substitute the
    /// player's name for every screen.
    #[tokio::test]
    async fn display_still_aborts_for_npc_dialog_not_in_monologue_cache() {
        let capture = LogCapture::install();
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        // monologue_dialog_ids intentionally empty for dialog 4001 (NPC
        // dialog — Future Col Marsh).

        let params = empty_params();
        let (tx, mut rx) = mpsc::channel(4);
        display(4001, 1, 9999, &params, &tx, &mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "non-monologue dialog with no NPC must still bail"
        );
        assert!(
            capture
                .find_message(Level::WARN, "DisplayDialog: no NPC entity id")
                .is_some(),
            "non-monologue bail must still surface the WARN"
        );
    }
}
