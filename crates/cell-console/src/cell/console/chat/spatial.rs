//! Spatial chat (say, emote, yell): `onPlayerCommunication` to the
//! speaker's player witnesses and an echo to the speaker.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::{
    serialize_on_player_communication, CHAN_SAY, CHAT_LOG_TARGET, ON_PLAYER_COMMUNICATION,
};

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

    // No early return when nobody is in range: a lone emote/yell speaker
    // still needs the echo below (the client does not show those locally).
    // A lone `say` speaker gets nothing from here -- see the echo comment
    // below -- which matches the legacy server exactly.
    if witnesses.is_empty() {
        tracing::trace!(target: CHAT_LOG_TARGET, sender_id, channel, "Chat: no witnesses");
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
    // One row per withheld witness, so SigNoz names both players (rule 5).
    for &wid in &ignored_by {
        tracing::debug!(
            target: CHAT_LOG_TARGET,
            event = "chat.spatial_ignored",
            entity_id = sender_id,
            player_id = entity.player_id,
            account_id = entity.account_id,
            target_entity_id = wid,
            target_player_id = space_mgr.get_entity(wid).and_then(|w| w.player_id),
            target_account_id = space_mgr.get_entity(wid).and_then(|w| w.account_id),
            channel,
            reason = "witness_ignores_speaker",
            "spatial chat line withheld from a witness who ignores the speaker"
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

    // Echo to the sender themselves -- but NEVER for `say`. The original
    // server comment (`deprecated/python/cell/SGWPlayer.py:1841-1843`,
    // `processPlayerCommunication`) reads:
    //
    //   # The client only echoes back messages in CHAN_say for some reason
    //   # For all other channels we need to notify the client as well
    //   if channelId != Atrea.enums.CHAN_say:
    //       self.client.onPlayerCommunication(speaker, speakerFlags, channelId, message)
    //
    // i.e. the 2009 client shows its OWN `say` line locally (natively, not
    // through the ChatWindow.lua `MessageReceived` path -- grep of the
    // client's ChatWindow.lua turns up no local-echo call from
    // `onTextAccepted`/`processTextCommand`, so this is client-native
    // behaviour, not scripted) and never needed a server round trip for it.
    // `emote` and `yell` are NOT locally echoed, so the legacy server sent
    // the echo for those two channels only.
    //
    // A prior pass here (SS-C1, `92cdeddaa`) inverted this on the belief
    // "the client does not echo say locally" and sent the echo
    // unconditionally, doubling the speaker's own `say` line in the
    // Info tab (client-native echo + server echo). See
    // `docs/reverse-engineering/findings/chat-speaker-echo.md`.
    if channel != CHAN_SAY {
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id: sender_id,
                method_index: ON_PLAYER_COMMUNICATION,
                args,
            })
            .await;
    }
}
