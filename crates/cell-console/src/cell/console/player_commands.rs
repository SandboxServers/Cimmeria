//! The `.`-console lines any player may type, GM or not.
//!
//! Everything else on the `.`-console is GM-only ([`super::dispatch`]). A
//! player command exists only where the client has no way to send the
//! request itself: today that is `.respeccraft`, because the client sends
//! `respecCrafting` (100) only from the Yes of a prompt the server has to
//! open first. A player command acts on the speaker only, never on a
//! target, and its line is consumed rather than broadcast.

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::{CellToBaseMsg, RespecCraftOpen};
use crate::cell::space_manager::SpaceManager;

/// The player commands, by the name typed after the `.`.
pub(super) const PLAYER_COMMANDS: &[&str] = &["respeccraft"];

/// Handle `text` if it is a player command. Returns `false` for any other
/// line, which then goes on to the GM console or to chat.
pub(crate) async fn handle_player_command(
    entity_id: u32,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    let mut parts = text.strip_prefix('.').unwrap_or(text).split_whitespace();
    let Some(name) = parts.next() else {
        return false;
    };
    if !PLAYER_COMMANDS.contains(&name) {
        return false;
    }
    if parts.next().is_some() {
        send_gm_feedback(entity_id, &format!(".{name} takes no arguments."), tx).await;
        return true;
    }
    respec_craft(entity_id, tx, space_mgr).await;
    true
}

/// `.respeccraft`: ask the base to open a crafting respec for the speaker.
/// The base decides whether there is anything to clear and answers with
/// the prompt or a line.
async fn respec_craft(entity_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &SpaceManager) {
    let Some(player_id) = space_mgr.get_entity(entity_id).and_then(|e| e.player_id) else {
        tracing::warn!(
            target: "crafting",
            event = "no_player",
            entity_id,
            method = "respeccraft",
            "respeccraft from an entity with no player_id; dropped"
        );
        send_gm_feedback(
            entity_id,
            "Crafting respec is unavailable right now. Nothing was changed.",
            tx,
        )
        .await;
        return;
    };
    let open = RespecCraftOpen {
        entity_id,
        player_id,
    };
    if let Err(e) = tx.send(CellToBaseMsg::RespecCraftOpen(open)).await {
        tracing::warn!(
            target: "crafting",
            event = "forward_failed",
            entity_id,
            player_id,
            method = "respeccraft",
            error = %e,
            "respeccraft could not be queued (base channel closed)"
        );
    }
}
