//! Dialog interaction handlers: `dialogButtonChoice` (with the #479
//! open-dialog server-authority gate) and `initialResponse`.

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

    // Server-authority precondition (CAT-J-01 / #479): the dialog
    // must actually be open for this player. `open_dialog_id` is
    // pinned by `send_dialog_display` on every display path and
    // matched here on strict equality. Without this gate, a forged
    // `DialogButtonChoice` for any discovered `dialog_id` drives the
    // bound `OnDialogChoice` chain's actions (GrantXP / GrantItem /
    // AcceptMission / Teleport / …) with no precondition. Mirrors
    // python `SGWPlayer.dialogButtonChoice` rejecting a choice whose
    // id isn't in `displayedDialogs`. The pin is cleared on a valid
    // choice (one-shot — SGW sends exactly one choice per displayed
    // dialog_id), which also makes a replayed choice idempotent.
    let open_dialog_id = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.open_dialog_id);
    if open_dialog_id != Some(dialog_id) {
        tracing::warn!(
            entity_id,
            dialog_id,
            button_id,
            open_dialog_id = ?open_dialog_id,
            "dialogButtonChoice rejected -- no matching open dialog for this player \
             (forged/replayed choice or stale client state); chain not fired (#479)"
        );
        return;
    }
    // Clear the pin BEFORE firing the chain: a chain action may
    // open a follow-up dialog (`display_dialog`), whose
    // `send_dialog_display` re-arms the pin for the next choice.
    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
        player.open_dialog_id = None;
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
