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
/// Resolution order:
///
/// 1. **Monologue dialogs win outright.** If the dialog is in
///    `space_mgr.monologue_dialog_ids` (every screen has
///    `speaker_id = 0`, i.e. player-narration / inner-thought), bind the
///    player as the wire EntityId and ignore any NPC in scope. The
///    client's per-screen lookup of `speaker_id = 0` falls back to the
///    player's name and the portrait shows the player — exactly the
///    intended render for "the player is talking to themselves."
///
///    This check is **first**, not a fallback, and the ordering is
///    load-bearing. `last_interaction_target` is a sticky pin: it holds
///    the last NPC the player clicked and is never cleared, so for a
///    monologue fired after any interact (a minigame victory chain, a
///    follow-up `dialog_choice`) an NPC would always be in scope and
///    would be bound, portraying the NPC for lines the author wrote as
///    the player's own thoughts. `Castle.py` makes the same call
///    explicitly: its monologue displays pass `displayDialog(None, …)`.
/// 2. Chain `params["target_entity_id"]` — present when the chain was
///    fired from an `InteractTag` / `InteractTemplate` trigger
///    (`fire_interact_*` stamps it into the context).
/// 3. Player's `last_interaction_target` — the per-player pin, written by
///    the interact handler before the content-chain dispatch and again by
///    `interactions::handle_interact` on the non-chain path. Covers
///    follow-up chains (an `OnDialogChoice` trigger, a minigame victory
///    chain, the deferred-action drain) where the trigger event itself
///    carries no NPC.
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
    space_mgr: &mut SpaceManager,
) {
    // A monologue dialog binds the player regardless of what NPC happens
    // to be in scope — see the resolution order on this fn for why this
    // must come before the pin rather than after it.
    let npc_entity_id = if space_mgr.monologue_dialog_ids.contains(&dialog_id) {
        // `debug!`, not `info!`: the "Content: displaying dialog" line
        // below already reports every display at info, carrying the same
        // entity/dialog/chain plus the resolved npc_entity_id. This line
        // adds only the *reason* for that resolution, and the span's
        // `monologue` field records it for anyone filtering in SigNoz, so
        // an info-level copy is duplicate volume on a common path
        // (~42% of authored screens are monologue).
        tracing::Span::current().record("monologue", true);
        tracing::debug!(
            entity_id,
            dialog_id,
            chain_id,
            "DisplayDialog: dialog is player-monologue (all screens \
             speaker_id=0) — binding player as context entity"
        );
        entity_id as i32
    } else {
        let resolved = params
            .get("target_entity_id")
            .and_then(|v| v.as_u64())
            .map(|v| v as i32)
            .or_else(|| {
                space_mgr
                    .get_entity(entity_id)
                    .and_then(|p| p.last_interaction_target)
                    .map(|id| id as i32)
            });

        match resolved {
            Some(id) => id,
            None => {
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
    // Stuck-player detector: a dialog replaced within seconds cannot be read.
    let (replaced_dialog_id, ms_since_previous) =
        crate::cell::playtest_friction_watch::last_dialog(entity_id).unwrap_or((0, 0));
    tracing::debug!(
        target: "dialog.display",
        entity_id,
        dialog_id,
        chain_id,
        npc_entity_id,
        replaced_dialog_id,
        ms_since_previous,
        "dialog displayed -- replaced_dialog_id is the previous one shown to this player"
    );
    crate::cell::playtest_friction::dialog_shown(entity_id, dialog_id);
    crate::cell::player_journal::note(
        entity_id,
        crate::cell::player_journal::kinds::DIALOG,
        format!("dialog={dialog_id} chain={chain_id}"),
    );
    tracing::info!(
        entity_id,
        dialog_id,
        npc_entity_id,
        chain_id,
        "Content: displaying dialog"
    );
    crate::cell::interactions::send_dialog_display(
        entity_id,
        npc_entity_id,
        dialog_id,
        tx,
        space_mgr,
    )
    .await;
}

/// `Action::AddDialogSet` — register a dialog set on the player's
/// `available_interactions` for the given template slot, and push an
/// InteractionType update for any matching NPC already in AoI.
///
/// The bound row may be **interaction-only** (`dialog_id IS NULL`): it then
/// contributes its indicator bit to the pushed flags and nothing else, which is
/// exactly what `Castle.py`'s `addDialog(149, 3062)` does — raise the `!` over
/// Sgt. Gerschon while the dialog itself comes from an `interact_tag` chain.
/// The push is `SGWSpawnableEntity.InteractionType(UINT64 TypeId)`
/// (`entities/defs/SGWSpawnableEntity.def:114-116`), a lone flags bitfield, so
/// there is no dialog field to fill and no "absent dialog" sentinel to invent.
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
            dialog_id = ?entry.dialog_id,
            interaction_only = entry.dialog_id.is_none(),
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
            entry.dialog_id,
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

            let target_is_player = space_mgr.get_entity(target_id).is_some_and(|e| e.is_player);
            if let Err(e) = tx
                .send(CellToBaseMsg::WitnessEntityMethod {
                    witness_id: entity_id,
                    entity_id: target_id,
                    method_index: crate::mercury::method_idx::INTERACTION_TYPE,
                    args: (merged as u64).to_le_bytes().to_vec(),
                    entity_is_player: target_is_player,
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
            dialog_id = ?entry.dialog_id,
            interaction_only = entry.dialog_id.is_none(),
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
            entry.dialog_id,
            tx,
            space_mgr,
            "add_dialog",
        )
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
///
/// Both callers insert into `available_interactions[slot]` *before* calling
/// this, and the pushed value is the fold of that whole list over the NPC's
/// base flags — not just the entry that was added. Folding only the new entry
/// would clear every previously bound indicator on the same template until the
/// next AoI entry re-sent the full set, because `InteractionType` replaces the
/// client's bitfield rather than OR-ing into it. Two binds on one template is a
/// designed shape since CA02 (an interaction-only indicator alongside a topic
/// that carries a dialog), so this has to match the two places that already
/// fold: `remove_dialog_set` and the AoI re-send in
/// `space_manager/aoi.rs::compute_player_aoi`.
///
/// `dialog_id` is passed for the log line only; it never reaches the wire.
async fn send_interaction_update_if_visible(
    entity_id: u32,
    slot: i32,
    dialog_id: Option<i32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
    label: &str,
) {
    let player_flags = space_mgr
        .get_entity(entity_id)
        .and_then(|p| p.available_interactions.get(&slot))
        .map_or(0i64, |entries| {
            entries.iter().fold(0i64, |acc, &(_, _, f)| acc | f)
        });

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
            let merged = base_flags | player_flags;

            tracing::debug!(
                entity_id,
                target_id,
                dialog_id = ?dialog_id,
                base_flags,
                player_flags,
                merged,
                "Sending per-player InteractionType for {}",
                label
            );

            let target_is_player = space_mgr.get_entity(target_id).is_some_and(|e| e.is_player);
            if let Err(e) = tx
                .send(CellToBaseMsg::WitnessEntityMethod {
                    witness_id: entity_id,
                    entity_id: target_id,
                    method_index: crate::mercury::method_idx::INTERACTION_TYPE,
                    args: (merged as u64).to_le_bytes().to_vec(),
                    entity_is_player: target_is_player,
                })
                .await
            {
                // same shape as remove_dialog_set —
                // dropped interaction-type push leaves the NPC with
                // stale flags.
                tracing::warn!(
                    entity_id,
                    target_id,
                    dialog_id = ?dialog_id,
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
mod tests;
