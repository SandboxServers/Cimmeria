//! Dialog interaction handlers: `dialogButtonChoice` (with the #479
//! offered-dialog server-authority gate) and `initialResponse`.

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

/// Handle `dialogButtonChoice(dialog_id, button_id)`. Args are the raw
/// 8-byte LE payload; the caller passes the wire bytes through unchanged.
pub(super) async fn handle_dialog_button_choice(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    if args.len() < 8 {
        return;
    }
    let dialog_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    let button_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
    tracing::info!(entity_id, dialog_id, button_id, "dialogButtonChoice");

    // Server-authority precondition (CAT-J-01 / #479): the dialog must
    // have been offered to THIS player. `send_dialog_display` records
    // every display in `offered_dialog_ids`; the take below removes it.
    // Without this gate, a forged `DialogButtonChoice` for any discovered
    // `dialog_id` drives the bound `OnDialogChoice` chain's actions
    // (GrantXP / GrantItem / AcceptMission / Teleport / …) with no
    // precondition. Mirrors python `SGWPlayer.dialogButtonChoice`
    // rejecting a choice whose id isn't in `displayedDialogs`.
    //
    // The take happens BEFORE the chain fires, for two reasons: it makes
    // the choice one-shot (a replay finds the id gone and is rejected),
    // and a chain action may display a follow-up dialog whose
    // `send_dialog_display` must be free to record its own id.
    //
    // A SET rather than a single pin because the client holds two active
    // dialogs and evicts the older one, whose zero-button close arrives
    // after the replacement was displayed (DU-08 / client contract F13).
    // A single pin rejected that close and silently dropped the evicted
    // dialog's chain.
    let offered = space_mgr
        .get_entity_mut(entity_id)
        .is_some_and(|e| e.take_offered_dialog(dialog_id));
    if !offered {
        let offered_dialog_ids = space_mgr
            .get_entity(entity_id)
            .map(|e| e.offered_dialogs())
            .unwrap_or_default();
        tracing::warn!(
            entity_id,
            dialog_id,
            button_id,
            ?offered_dialog_ids,
            "dialogButtonChoice rejected -- dialog was never offered to this player \
             (forged/replayed choice or stale client state); chain not fired (#479)"
        );
        return;
    }

    let player_id = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.player_id)
        .unwrap_or(0);
    crate::cell::content::fire_dialog_choice(
        entity_id, player_id, dialog_id, button_id, engine, tx, space_mgr,
    )
    .await;
}

/// Handle `initialResponse(interaction_set_map_id)`.
pub(super) async fn handle_initial_response(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    if args.len() >= 4 {
        let interaction_set_map_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
        tracing::info!(entity_id, interaction_set_map_id, "initialResponse");

        crate::cell::interactions::handle_initial_response(
            entity_id,
            interaction_set_map_id,
            engine,
            tx,
            space_mgr,
        )
        .await;
    } else {
        tracing::warn!(
            entity_id,
            args_len = args.len(),
            "initialResponse: truncated args, dropping"
        );
    }
}
