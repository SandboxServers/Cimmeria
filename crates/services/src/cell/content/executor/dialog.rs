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
                dialog_id = ?entry.dialog_id,
                base_flags,
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
                    dialog_id = ?entry.dialog_id,
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
            &tx, &mut mgr,
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
        display(2299, 1, 1021, &params, &tx, &mut mgr).await;

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
        display(4001, 1, 9999, &params, &tx, &mut mgr).await;

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
        display(2982, 1, 1001, &params, &tx, &mut mgr).await;

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

    /// **Precedence guard.** A monologue dialog must bind the player even
    /// when an NPC *is* resolvable — both from the sticky
    /// `last_interaction_target` pin and from an explicit
    /// `target_entity_id` chain param.
    ///
    /// `last_interaction_target` is never cleared, so once the player has
    /// clicked any NPC there is permanently an NPC in scope. If the
    /// monologue check were a fallback rather than the first branch, every
    /// monologue fired after an interact — a minigame victory chain, a
    /// follow-up `dialog_choice` — would portray that NPC speaking lines
    /// the author wrote as the player's inner thoughts. Castle mission
    /// 701's dialog 2575 ("You manage to free Capt. Copplemann...") is
    /// exactly this shape, and `Castle.py` passes an explicit
    /// `displayDialog(None, 2575)`.
    #[tokio::test]
    async fn monologue_binds_player_even_when_an_npc_is_resolvable() {
        const NPC_ID: u32 = 0xC0FFEE;
        const MONOLOGUE: i32 = 2575;

        // Case 1: NPC available via the sticky pin.
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        mgr.monologue_dialog_ids.insert(MONOLOGUE);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.last_interaction_target = Some(NPC_ID);
        }

        let (tx, mut rx) = mpsc::channel(4);
        display(MONOLOGUE, 1, 1234, &empty_params(), &tx, &mut mgr).await;

        match rx.try_recv().expect("monologue must still display") {
            CellToBaseMsg::EntityMethodCall { args, .. } => {
                let wire = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_eq!(
                    wire, 1,
                    "a monologue must bind the PLAYER, not the pinned NPC \
                     ({NPC_ID}) — reverting the monologue-first ordering \
                     fails here",
                );
            }
            other => panic!("expected EntityMethodCall, got {other:?}"),
        }

        // Case 2: NPC available via an explicit chain param, which is an
        // even stronger source than the pin.
        let mut params = empty_params();
        params.insert("target_entity_id".into(), serde_json::json!(NPC_ID as u64));
        display(MONOLOGUE, 1, 1234, &params, &tx, &mut mgr).await;

        match rx.try_recv().expect("monologue must still display") {
            CellToBaseMsg::EntityMethodCall { args, .. } => {
                let wire = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_eq!(
                    wire, 1,
                    "a monologue must outrank an explicit target_entity_id too",
                );
            }
            other => panic!("expected EntityMethodCall, got {other:?}"),
        }
    }

    /// Castle dialog_set_map 3062: `dialog_id IS NULL`, `interaction_flags =
    /// 16777216` (`INT_AStoryMissionActive`, the `!` over an NPC's head).
    /// `Castle.py` binds it on Sgt. Gerschon (template 149) with
    /// `addDialog(149, 3062)`.
    ///
    /// **Regression guard for defect B3.** Before CA02 the loader dropped
    /// NULL-dialog rows, so this bind was a `dialog_set_maps cache miss` warn
    /// and a no-op — the `!` never appeared. The guard asserts the three
    /// things that must hold for a flag-only bind:
    ///
    /// 1. the bind is recorded with `dialog_id: None` (not a substituted 0),
    /// 2. exactly one `InteractionType` push goes out, carrying the `!` bit
    ///    merged over the NPC's base flags,
    /// 3. no dialog is displayed — a flag-only row has nothing to show, and
    ///    Gerschon's dialog comes from a separate `interact_tag` chain.
    #[tokio::test]
    async fn add_dialog_set_with_null_dialog_pushes_flag_only() {
        use crate::cell::spawner::DialogSetMapEntry;
        use cimmeria_common::EntityId;

        const TEMPLATE_GERSCHON: i32 = 149;
        const SET_MAP_3062: i32 = 3062;
        /// `INT_AStoryMissionActive` — bit 24, the `!` indicator.
        const INT_A_STORY_MISSION_ACTIVE: i64 = 16_777_216;
        /// Arbitrary pre-existing base flag on the NPC, to prove the push
        /// merges rather than replaces.
        const NPC_BASE_FLAGS: i64 = 0x2;

        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        mgr.create_entity(2, "Agnos", [1.0; 3], [0.0; 3]).unwrap();
        mgr.dialog_set_maps.insert(
            SET_MAP_3062,
            DialogSetMapEntry {
                dialog_id: None,
                interaction_flags: INT_A_STORY_MISSION_ACTIVE,
            },
        );
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(42);
            p.witnesses.insert(EntityId(2));
        }
        if let Some(n) = mgr.get_entity_mut(2) {
            n.template_id = Some(TEMPLATE_GERSCHON);
            n.interaction_type_flags = NPC_BASE_FLAGS;
        }

        let (tx, mut rx) = mpsc::channel(8);
        add_dialog_set(
            SET_MAP_3062,
            TEMPLATE_GERSCHON,
            /* entity_id */ 1,
            /* chain_id */ 1201,
            &tx,
            &mut mgr,
        )
        .await;

        assert_eq!(
            mgr.get_entity(1)
                .and_then(|p| p.available_interactions.get(&TEMPLATE_GERSCHON))
                .map(Vec::as_slice),
            Some([(SET_MAP_3062, None, INT_A_STORY_MISSION_ACTIVE)].as_slice()),
            "the bind must be recorded with dialog_id None -- a substituted 0 \
             would make the click open an empty dialog"
        );

        let msg = rx.try_recv().expect(
            "flag-only bind must still push InteractionType -- with the loader \
             reverted to dropping NULL rows this is a cache miss and nothing is sent",
        );
        match msg {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                args,
                entity_is_player,
            } => {
                assert_eq!(witness_id, 1, "push is per-player, to the binding player");
                assert_eq!(entity_id, 2, "push targets the NPC, not the player");
                assert_eq!(method_index, crate::mercury::method_idx::INTERACTION_TYPE);
                assert!(!entity_is_player);
                assert_eq!(
                    args,
                    ((NPC_BASE_FLAGS | INT_A_STORY_MISSION_ACTIVE) as u64)
                        .to_le_bytes()
                        .to_vec(),
                    "payload is the merged flags as UINT64 LE -- the `!` bit OR'd \
                     over the NPC's base flags"
                );
            }
            other => panic!("expected WitnessEntityMethod, got {other:?}"),
        }

        assert!(
            rx.try_recv().is_err(),
            "a flag-only bind must emit the InteractionType push and nothing else -- \
             no onDialogDisplay, because the row has no dialog"
        );
    }

    /// Companion to the flag-only guard: a bind whose row *does* carry a
    /// dialog still behaves exactly as before. Pins that widening
    /// `dialog_id` to `Option` didn't change the with-dialog path, and that
    /// the dialog id stays off the wire in both cases — the pushed payload is
    /// the flags bitfield alone, per
    /// `entities/defs/SGWSpawnableEntity.def:114-116`.
    #[tokio::test]
    async fn add_dialog_set_with_dialog_pushes_same_flag_only_payload() {
        use crate::cell::spawner::DialogSetMapEntry;
        use cimmeria_common::EntityId;

        const TEMPLATE_GERSCHON: i32 = 149;
        const SET_MAP_3060: i32 = 3060;
        const DIALOG_2573: i32 = 2573;
        const FLAGS: i64 = 16_777_216;

        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        mgr.create_entity(2, "Agnos", [1.0; 3], [0.0; 3]).unwrap();
        mgr.dialog_set_maps.insert(
            SET_MAP_3060,
            DialogSetMapEntry {
                dialog_id: Some(DIALOG_2573),
                interaction_flags: FLAGS,
            },
        );
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(42);
            p.witnesses.insert(EntityId(2));
        }
        if let Some(n) = mgr.get_entity_mut(2) {
            n.template_id = Some(TEMPLATE_GERSCHON);
        }

        let (tx, mut rx) = mpsc::channel(8);
        add_dialog_set(SET_MAP_3060, TEMPLATE_GERSCHON, 1, 1201, &tx, &mut mgr).await;

        assert_eq!(
            mgr.get_entity(1)
                .and_then(|p| p.available_interactions.get(&TEMPLATE_GERSCHON))
                .map(Vec::as_slice),
            Some([(SET_MAP_3060, Some(DIALOG_2573), FLAGS)].as_slice())
        );

        match rx
            .try_recv()
            .expect("with-dialog bind must push InteractionType")
        {
            CellToBaseMsg::WitnessEntityMethod {
                method_index, args, ..
            } => {
                assert_eq!(method_index, crate::mercury::method_idx::INTERACTION_TYPE);
                assert_eq!(
                    args,
                    (FLAGS as u64).to_le_bytes().to_vec(),
                    "the dialog id is server-side state and must never appear in \
                     the InteractionType payload"
                );
            }
            other => panic!("expected WitnessEntityMethod, got {other:?}"),
        }
        assert!(rx.try_recv().is_err(), "exactly one push");
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
        display(4001, 1, 9999, &params, &tx, &mut mgr).await;

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
