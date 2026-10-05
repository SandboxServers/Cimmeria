//! What a player sees when an `interact` is refused for range (DA-F5).
//!
//! The client sends `interact` on a right-click from any distance and shows
//! nothing of its own when the server refuses it, so before this a too-far
//! click on most NPCs was a silent drop: the Debug Area run (DA-06) clicked
//! "Jay Test Abilities" from 7.7 m and nothing happened. Every button press
//! gets visible feedback on the first press, so a too-far refusal sends one
//! `CHAN_FEEDBACK` line. A Banker and an organization registrar keep their
//! own lines and telemetry (bank-vault BV-02, ORG-05).
//!
//! **Only for a target the player can see.** The line names the NPC, and NPC
//! ids are sequential, so answering every id in the space would let a
//! modified client sweep `interact(id)` and list every named NPC in a shared
//! world (a rare-spawn radar), and the generic line against a silent drop
//! would show which ids are players. A real click only names something on
//! screen, so the line goes out only when the target is in the player's
//! witness set or within its AoI radius; anything else stays the silent
//! logged drop. A missing target or one in another space never answers.
//!
//! **One line per player per [`RANGE_FEEDBACK_INTERVAL`].** Clicking while
//! walking up is ordinary play, and a hostile client could otherwise turn
//! each small inbound packet into a reliable chat line and log rows.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::{interact_range, InteractRangeFail, MAX_INTERACT_DISTANCE};

/// At most one too-far line per player per this interval.
pub(crate) const RANGE_FEEDBACK_INTERVAL: Duration = Duration::from_millis(1500);

/// The line for a too-far `interact`, naming the target when it has a name.
pub(crate) fn too_far_line(target_name: Option<&str>) -> String {
    match target_name {
        Some(name) if !name.is_empty() => {
            format!("You are too far away from {name}. Move closer to interact.")
        }
        _ => "You are too far away. Move closer to interact.".to_string(),
    }
}

/// Whether `entity_id` can see `target_entity_id`: the target is in its
/// witness set (the entities its client was introduced to), or within its
/// AoI radius of `dist` metres (a player's set can lag a tick behind).
fn caller_can_see(
    space_mgr: &SpaceManager,
    entity_id: u32,
    target_entity_id: u32,
    dist: f32,
) -> bool {
    space_mgr.get_entity(entity_id).is_some_and(|caller| {
        caller
            .get_witnesses()
            .iter()
            .any(|w| w.0 == target_entity_id as i32)
            || dist <= caller.aoi_radius
    })
}

/// Answer an `interact` the range gate refused: the Banker's or registrar's
/// own line when the target is one, else the generic too-far line. Only for
/// a target the player can see, and at most once per
/// [`RANGE_FEEDBACK_INTERVAL`]. Returns whether a line was sent.
pub async fn reject_interact_out_of_range(
    entity_id: u32,
    target_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Err(InteractRangeFail::TooFar { dist }) =
        interact_range(entity_id, target_entity_id, space_mgr)
    else {
        return false;
    };
    let who = space_mgr.player_identity(entity_id);
    if !caller_can_see(space_mgr, entity_id, target_entity_id, dist) {
        // No name on purpose: the target is one the player cannot see.
        tracing::debug!(
            event = "interaction.out_of_range_unseen",
            entity_id,
            entity_name = who.player_name,
            player_id = who.player_id,
            player_name = who.player_name,
            target_entity_id, // nt:id-only the target is out of the player's view; naming it in a player-attributed row is what the gate avoids
            dist,
            reason = "target_not_in_view",
            "too-far interact on a target the player cannot see; no feedback line (name oracle)"
        );
        return false;
    }
    let now = Instant::now();
    let last = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.last_range_feedback_at);
    if last.is_some_and(|t| now.duration_since(t) < RANGE_FEEDBACK_INTERVAL) {
        tracing::debug!(
            event = "interaction.out_of_range_throttled",
            entity_id,
            entity_name = who.player_name,
            player_id = who.player_id,
            player_name = who.player_name,
            target_entity_id,
            target_entity_name = space_mgr.entity_label(target_entity_id),
            dist,
            reason = "rate_limited",
            "too-far interact inside the feedback interval; line dropped"
        );
        return false;
    }

    let sent = send_line(entity_id, target_entity_id, dist, tx, space_mgr).await;
    if sent {
        if let Some(e) = space_mgr.get_entity_mut(entity_id) {
            e.last_range_feedback_at = Some(now);
        }
    }
    sent
}

/// The line itself: the Banker's, the registrar's, or the generic one.
async fn send_line(
    entity_id: u32,
    target_entity_id: u32,
    dist: f32,
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
