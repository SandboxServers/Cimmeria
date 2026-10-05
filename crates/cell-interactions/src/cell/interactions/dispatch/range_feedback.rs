//! What a player sees when an `interact` is refused for range (DA-F5).
//!
//! The client sends `interact` on a right-click from any distance and shows
//! nothing of its own when the server refuses it, so before this a too-far
//! click on most NPCs was a silent drop: the Debug Area run (DA-06) clicked
//! "Jay Test Abilities" from 7.7 m and nothing happened. Every button press
//! gets visible feedback on the first press, so every too-far refusal now
//! sends one `CHAN_FEEDBACK` line. A Banker and an organization registrar
//! keep their own lines and telemetry (bank-vault BV-02, ORG-05).
//!
//! Only `TooFar` answers. A missing target or one in another space is not
//! something a real click produces (the client cannot see across spaces),
//! so it stays a logged drop rather than a line that would confirm an id.

use tokio::sync::mpsc;

use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::{interact_range, InteractRangeFail, MAX_INTERACT_DISTANCE};

/// The line for a too-far `interact`, naming the target when it has a name.
pub(crate) fn too_far_line(target_name: Option<&str>) -> String {
    match target_name {
        Some(name) if !name.is_empty() => {
            format!("You are too far away from {name}. Move closer to interact.")
        }
        _ => "You are too far away. Move closer to interact.".to_string(),
    }
}

/// Answer an `interact` the range gate refused: the Banker's or registrar's
/// own line when the target is one, else the generic too-far line. Returns
/// whether a line was sent.
pub async fn reject_interact_out_of_range(
    entity_id: u32,
    target_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    if super::super::bank::reject_banker_out_of_range(entity_id, target_entity_id, tx, space_mgr)
        .await
    {
        return true;
    }
    if super::super::org_registrar::reject_registrar_out_of_range(
        entity_id,
        target_entity_id,
        tx,
        space_mgr,
    )
    .await
    {
        return true;
    }
    let Err(InteractRangeFail::TooFar { dist }) =
        interact_range(entity_id, target_entity_id, space_mgr)
    else {
        return false;
    };

    let target_name = space_mgr
        .get_entity(target_entity_id)
        .and_then(|t| t.npc_name.clone());
    let text = too_far_line(target_name.as_deref());
    let who = space_mgr.player_identity(entity_id);
    tracing::info!(
        event = "interaction.out_of_range",
        entity_id,
        entity_name = who.player_name,
        player_id = who.player_id,
        player_name = who.player_name,
        target_entity_id,
        target_entity_name = space_mgr.entity_label(target_entity_id),
        dist,
        max = MAX_INTERACT_DISTANCE,
        reason = "too_far",
        "interact refused for range; the player is told to move closer"
    );
    let msg = CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: ON_PLAYER_COMMUNICATION,
        args: serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, &text),
    };
    if tx.send(msg).await.is_err() {
        tracing::warn!(
            event = "interaction.feedback_send_failed",
            entity_id,
            entity_name = who.player_name,
            player_id = who.player_id,
            player_name = who.player_name,
            target_entity_id,
            target_entity_name = space_mgr.entity_label(target_entity_id),
            reason = "cell_to_base_closed",
            "out-of-range interact feedback could not be queued; the player sees nothing"
        );
        return false;
    }
    true
}
