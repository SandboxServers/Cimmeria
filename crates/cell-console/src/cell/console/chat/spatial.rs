//! Spatial chat (say, emote, yell): `onPlayerCommunication` to the
//! speaker's player witnesses and an echo to the speaker.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::{serialize_on_player_communication, CHAT_LOG_TARGET, ON_PLAYER_COMMUNICATION};

/// Broadcast a chat message to all witnesses of the sender entity.
///
/// Serializes `onPlayerCommunication(speaker, flags, channel, text)` args
/// and sends one `EntityMethodCall` per witness.
pub(super) async fn broadcast_to_witnesses(
    sender_id: u32,
    speaker_name: &str,
    speaker_flags: u8,
    channel: u8,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let entity = match space_mgr.get_entity(sender_id) {
        Some(e) => e,
        None => {
            tracing::warn!(target: CHAT_LOG_TARGET, sender_id, "Chat: sender entity not found");
            return;
        }
    };

    // `entity.witnesses` holds everything the SENDER currently sees — other
    // players AND NPCs in its own AoI (see
    // `SpaceManager::get_witnesses_of`'s doc comment: this field is "what
    // this player witnesses", the reverse direction from "who witnesses this
    // entity"). Routing chat to `entity_id: witness_id` dispatches an
    // `EntityMethodCall`/witness send keyed on the recipient's own client
    // address; an NPC id has no entry in `entity_to_addr` (NPCs have no
    // client), so every NPC in the sender's AoI produced a dropped-packet
    // WARN (`entity_to_addr_miss`) for a recipient that was never going to
    // receive anything anyway. Filter to players only — NPCs cannot receive
    // chat and must not be treated as broadcast targets.
    let witnesses: Vec<u32> = entity
        .witnesses
        .iter()
        .map(|eid| eid.0 as u32)
        .filter(|&wid| space_mgr.get_entity(wid).is_some_and(|e| e.is_player))
        .collect();

    // No early return when nobody is in range: the speaker's own echo below
    // must still go out. The client does not echo say (channel 0) locally, so
    // a lone speaker would otherwise see nothing for their first line.
    if witnesses.is_empty() {
        tracing::trace!(target: CHAT_LOG_TARGET, sender_id, "Chat: no witnesses; echo to the speaker only");
    }

    // D-SS15: a witness who ignores the speaker does not hear them. One
    // direction only (a witness the speaker ignores still hears the speaker),
    // and it filters the line, not the AoI. The speaker's own echo below is
    // unchanged, so being ignored is not revealed. Names compare
    // case-insensitively (the D-SS13 fold), so an entry stored as "bob"
    // matches the speaker "Bob".
    let folded_speaker = speaker_name.to_lowercase();
    let (witnesses, ignored_by): (Vec<u32>, Vec<u32>) = witnesses.into_iter().partition(|&wid| {
        !space_mgr.get_entity(wid).is_some_and(|w| {
            w.ignore_names
                .iter()
                .any(|n| n.to_lowercase() == folded_speaker)
        })
    });
    if !ignored_by.is_empty() {
        tracing::debug!(
            target: CHAT_LOG_TARGET,
            event = "chat.spatial_ignored",
            entity_id = sender_id,
            player_id = entity.player_id,
            account_id = entity.account_id,
            channel,
            skipped = ignored_by.len(),
            reason = "witness_ignores_speaker",
            "spatial chat withheld from witnesses who ignore the speaker"
        );
    }

    // Serialize onPlayerCommunication args once
    let args = serialize_on_player_communication(speaker_name, speaker_flags, channel, text);

    tracing::debug!(
        target: CHAT_LOG_TARGET,
        sender_id,
        channel,
        witness_count = witnesses.len(),
        speaker = speaker_name,
        "Broadcasting chat to witnesses"
    );

    // Send to each witness
    for witness_id in witnesses {
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id: witness_id,
                method_index: ON_PLAYER_COMMUNICATION,
                args: args.clone(),
            })
            .await;
    }

    // Also send to the sender themselves (client needs server echo for say channel,
    // and sending for all spatial channels is harmless)
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: sender_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args,
        })
        .await;
}
