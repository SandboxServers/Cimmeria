//! Chat message distribution for the CellService.
//!
//! Handles spatial chat channels (say, emote, yell) by broadcasting
//! `onPlayerCommunication` to witnesses in the sender's Area of Interest.
//!
//! Reference: `python/cell/SGWPlayer.py:processPlayerCommunication()`

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use crate::cell::console;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

// ── Channel IDs and the onPlayerCommunication serializer ──────────────────
//
// The `EChannel` ids and `serialize_on_player_communication` are wire
// contract and live in `cimmeria-wire`; re-exported at their old paths.

pub use cimmeria_wire::cell::chat::{
    CHAN_COMMAND, CHAN_EMOTE, CHAN_FEEDBACK, CHAN_OFFICER, CHAN_SAY, CHAN_SERVER, CHAN_SPLASH,
    CHAN_SQUAD, CHAN_TEAM, CHAN_TELL, CHAN_YELL,
};

pub(crate) use cimmeria_wire::cell::chat::serialize_on_player_communication;

// ── onPlayerCommunication client method index ──────────────────────────────

/// Communicator interface ClientMethod: onPlayerCommunication
/// Flat index 28 in SGWPlayer ClientMethods.
const ON_PLAYER_COMMUNICATION: u16 = 28;

// ── Chat distribution ──────────────────────────────────────────────────────

/// Handle a chat message from a player entity.
///
/// For spatial channels (say, emote, yell), broadcasts to all witnesses
/// of the sender's entity. Each witness receives `onPlayerCommunication`.
///
/// Reference: `python/cell/SGWPlayer.py:processPlayerCommunication()`
/// - say/emote/yell: broadcast to witnesses
/// - Client does NOT echo say (channel 0) — server must send it back
/// - Client DOES echo emote/yell — but Python sends it anyway (no harm)
/// `text_len` is recorded but the message body itself is intentionally
/// excluded from the span — chat content is user-private and shouldn't
/// land in the SigNoz log/trace store. Operators get "who sent how
/// many bytes on which channel" without sniffing message bodies.
#[tracing::instrument(
    name = "chat.send",
    level = "info",
    skip_all,
    fields(entity_id, channel, text_len = text.len()),
)]
pub async fn handle_chat_message(
    entity_id: u32,
    speaker_name: &str,
    speaker_flags: u8,
    channel: u8,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    // GM `.`-console interception. The 2009 client forwards `.`-prefixed
    // input as an ordinary CHAN_SAY chat message rather than eating it (unlike
    // `/`, which the client consumes). When the sender is a GM, we consume the
    // line as a dev/authoring console command and never broadcast it to other
    // players; a non-GM's `.`-text falls through to normal chat. Auth is on the
    // server-side `access_level`, never a client-asserted byte.
    if channel == CHAN_SAY && text.starts_with('.') {
        let access_level = space_mgr
            .get_entity(entity_id)
            .map_or(0, |e| e.access_level);
        if console::is_gm(access_level) {
            console::handle_console_command(entity_id, text, tx, space_mgr, engine).await;
            return;
        }
        // Non-GM `.`-text is ordinary chat — fall through to broadcast.
    }

    match channel {
        CHAN_SAY | CHAN_EMOTE | CHAN_YELL => {
            broadcast_to_witnesses(
                entity_id,
                speaker_name,
                speaker_flags,
                channel,
                text,
                tx,
                space_mgr,
            )
            .await;
        }
        CHAN_SERVER => {
            // `python/base/Chat.py::ChatChannelManager.__init__` creates the
            // "server" channel with `CHANNEL_FLAG_DisallowPlayerMessages` --
            // it is a system-broadcast channel (server -> all players), never
            // a player -> player one. The legacy `sendPlayerMessage` rejected
            // this with only a server-side `warn()` and no client-visible
            // reply, which is exactly the silent-drop this tester tripped
            // over ("i dont see my own message sent to the server channel").
            // Per the "every button press gets visible feedback" project
            // rule, tell the sender why instead of dropping it on the floor.
            tracing::debug!(
                entity_id,
                channel,
                "Chat: player attempted to speak on the system-only server channel"
            );
            send_channel_feedback(
                entity_id,
                "The server channel is for system messages only -- players cannot post here.",
                tx,
            )
            .await;
        }
        _ => {
            // Every other registered channel (team/squad/command/officer/
            // tell) was never distributed on the cell in the legacy server
            // either: `python/cell/SGWPlayer.py::processPlayerCommunication`
            // only special-cases say/emote/yell and falls through to
            // `self.onError("Speaking on channel %d is not supported yet!")`
            // for anything else, which itself is a client-visible
            // `onPlayerCommunication` reply, not a silent drop. Match that
            // shape here instead of only logging.
            tracing::debug!(
                entity_id,
                channel,
                "Chat channel not handled by CellService -- feedback sent to sender"
            );
            send_channel_feedback(
                entity_id,
                &format!("Speaking on channel {channel} is not supported yet!"),
                tx,
            )
            .await;
        }
    }
}

/// Send a single-recipient system feedback line on `CHAN_FEEDBACK` -- the
/// same registered-channel/"SYSTEM" speaker shape used by the GM `.`-console
/// (`cell::cell_methods::gm::feedback::send_gm_feedback`), reimplemented here
/// so an ordinary (non-GM) player's rejected chat message gets the exact same
/// treatment: a real line on a channel their client already renders normally,
/// never the client's red unknown-channel splash popup that an unregistered
/// channel id would trigger.
async fn send_channel_feedback(entity_id: u32, text: &str, tx: &mpsc::Sender<CellToBaseMsg>) {
    let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args,
        })
        .await;
}

/// Broadcast a chat message to all witnesses of the sender entity.
///
/// Serializes `onPlayerCommunication(speaker, flags, channel, text)` args
/// and sends one `EntityMethodCall` per witness.
async fn broadcast_to_witnesses(
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
            tracing::warn!(sender_id, "Chat: sender entity not found");
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

    if witnesses.is_empty() {
        tracing::trace!(sender_id, "Chat: no witnesses to broadcast to");
        return;
    }

    // Serialize onPlayerCommunication args once
    let args = serialize_on_player_communication(speaker_name, speaker_flags, channel, text);

    tracing::debug!(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn broadcast_to_nonexistent_entity_is_noop() {
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let engine = ChainEngine::new();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        handle_chat_message(999, "Bob", 0, CHAN_SAY, "Hello", &tx, &mut mgr, &engine).await;

        // No messages should be sent
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn broadcast_say_to_witnesses() {
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();

        // Create two players near each other
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.create_entity(2, "Agnos", [15.0, 0.0, 15.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        mgr.connect_entity(2);

        // Manually add witness relationships (normally done by AoI tick)
        if let Some(e) = mgr.get_entity_mut(1) {
            e.witnesses.insert(cimmeria_common::EntityId(2));
        }

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let engine = ChainEngine::new();

        handle_chat_message(
            1,
            "Alice",
            0,
            CHAN_SAY,
            "Hello world",
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        // Should get 2 messages: one for witness (entity 2) + one for sender (entity 1)
        let mut msgs = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            msgs.push(msg);
        }
        assert_eq!(msgs.len(), 2);

        // Check the first is to witness entity 2
        match &msgs[0] {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                ..
            } => {
                assert_eq!(*entity_id, 2);
                assert_eq!(*method_index, ON_PLAYER_COMMUNICATION);
            }
            _ => panic!("Expected EntityMethodCall"),
        }

        // Check the second is to sender entity 1
        match &msgs[1] {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                ..
            } => {
                assert_eq!(*entity_id, 1);
                assert_eq!(*method_index, ON_PLAYER_COMMUNICATION);
            }
            _ => panic!("Expected EntityMethodCall"),
        }
    }

    /// Regression guard: an NPC in the sender's own AoI (`entity.witnesses`
    /// stores what the sender *sees*, not who sees the sender — see
    /// `SpaceManager::get_witnesses_of`) must never become a chat broadcast
    /// target. Before the fix, `broadcast_to_witnesses` dispatched an
    /// `EntityMethodCall { entity_id: <npc_id>, .. }` for every NPC in
    /// range; base then tried to resolve that NPC id through
    /// `entity_to_addr` (which only ever holds player entries) and logged
    /// `AoI reliable: no client addr for witness -- packet dropped`
    /// (`reason = entity_to_addr_miss`) once per NPC, per chat line, for
    /// every NPC near a talking player. Reverting the `is_player` filter in
    /// `broadcast_to_witnesses` makes this test fail by emitting an
    /// `EntityMethodCall` addressed to the NPC.
    #[tokio::test]
    async fn broadcast_say_skips_npc_witnesses() {
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();

        // One player (the speaker), one other player (a real witness), and
        // one NPC that happens to be in the speaker's AoI too.
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.create_entity(2, "Agnos", [15.0, 0.0, 15.0], [0.0; 3])
            .unwrap();
        mgr.create_entity(100008, "Agnos", [12.0, 0.0, 12.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        mgr.connect_entity(2);
        // Entity 100008 is never `connect_entity`'d, so it stays an NPC
        // (`is_player == false`) exactly like a Castle spawn-set mob.

        // Manually add witness relationships (normally done by AoI tick) --
        // the speaker sees both the other player AND the NPC.
        if let Some(e) = mgr.get_entity_mut(1) {
            e.witnesses.insert(cimmeria_common::EntityId(2));
            e.witnesses.insert(cimmeria_common::EntityId(100008));
        }

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let engine = ChainEngine::new();

        handle_chat_message(
            1,
            "Alice",
            0,
            CHAN_SAY,
            "Hello world",
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let mut msgs = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            msgs.push(msg);
        }

        // Exactly 2 messages: the real player witness (2) + the sender echo
        // (1). The NPC (100008) must never appear as an `entity_id`.
        assert_eq!(
            msgs.len(),
            2,
            "NPC witness must not receive its own EntityMethodCall: {msgs:?}"
        );
        for msg in &msgs {
            if let CellToBaseMsg::EntityMethodCall { entity_id, .. } = msg {
                assert_ne!(
                    *entity_id, 100008,
                    "chat must never target an NPC id -- it has no client and \
                     entity_to_addr resolution always misses for it"
                );
            }
        }
    }

    /// A GM's `.`-command is consumed by the console and never broadcast to
    /// witnesses (never appears in others' chat).
    #[tokio::test]
    async fn gm_dot_command_is_intercepted_not_broadcast() {
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.create_entity(2, "Agnos", [15.0, 0.0, 15.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        mgr.connect_entity(2);
        if let Some(e) = mgr.get_entity_mut(1) {
            e.access_level = 2; // GameMaster
            e.witnesses.insert(cimmeria_common::EntityId(2));
        }
        let engine = ChainEngine::new();
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);

        handle_chat_message(1, "Gm", 0, CHAN_SAY, ".players", &tx, &mut mgr, &engine).await;

        // Witness (entity 2) must receive NOTHING — the command was consumed.
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall { entity_id, .. } = msg {
                assert_ne!(entity_id, 2, "GM .-command must not broadcast to witnesses");
            }
        }
    }

    /// A non-GM's `.`-text is ordinary chat and DOES broadcast.
    #[tokio::test]
    async fn non_gm_dot_text_is_normal_chat() {
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.create_entity(2, "Agnos", [15.0, 0.0, 15.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        mgr.connect_entity(2);
        if let Some(e) = mgr.get_entity_mut(1) {
            // access_level stays 0 (Player)
            e.witnesses.insert(cimmeria_common::EntityId(2));
        }
        let engine = ChainEngine::new();
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);

        handle_chat_message(1, "Joe", 0, CHAN_SAY, ".hello", &tx, &mut mgr, &engine).await;

        // Witness (entity 2) should receive the chat broadcast.
        let mut witness_got_chat = false;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall { entity_id, .. } = msg {
                if entity_id == 2 {
                    witness_got_chat = true;
                }
            }
        }
        assert!(
            witness_got_chat,
            "non-GM .-text must broadcast as normal chat"
        );
    }

    /// Decode `onPlayerCommunication` args back to `(flags, channel, text)`.
    /// Test-only mirror of `serialize_on_player_communication`'s layout.
    fn decode_on_player_communication(args: &[u8]) -> (u8, u8, String) {
        let speaker_units = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
        let mut offset = 4 + speaker_units * 2;
        let flags = args[offset];
        offset += 1;
        let channel = args[offset];
        offset += 1;
        let text_units = u32::from_le_bytes(args[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        let text_bytes = &args[offset..offset + text_units * 2];
        let text: String = char::decode_utf16(
            text_bytes
                .chunks(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]])),
        )
        .map(|r| r.unwrap_or('?'))
        .collect();
        (flags, channel, text)
    }

    /// Unsupported channels (team/squad/command/officer/tell) are not
    /// distributed on the cell -- but unlike a silent drop, the sender must
    /// get a feedback line so a chat message never just vanishes (project
    /// rule: every button press gets visible feedback). No witness ever
    /// receives anything for these channels.
    #[tokio::test]
    async fn non_cell_channel_feeds_back_to_sender_only() {
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.create_entity(2, "Agnos", [15.0, 0.0, 15.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        mgr.connect_entity(2);
        if let Some(e) = mgr.get_entity_mut(1) {
            e.witnesses.insert(cimmeria_common::EntityId(2));
        }

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let engine = ChainEngine::new();

        // Tell channel is not distributed on the cell, but the sender must
        // still hear back -- not dead silence.
        handle_chat_message(1, "Bob", 0, CHAN_TELL, "Hi", &tx, &mut mgr, &engine).await;

        let msg = rx
            .try_recv()
            .expect("sender must receive a feedback line, not silence");
        let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        else {
            panic!("expected EntityMethodCall");
        };
        assert_eq!(entity_id, 1, "feedback must go to the sender only");
        assert_eq!(method_index, ON_PLAYER_COMMUNICATION);
        let (flags, channel, text) = decode_on_player_communication(&args);
        assert_eq!(flags, 0);
        assert_eq!(
            channel, CHAN_FEEDBACK,
            "feedback rides the registered feedback channel"
        );
        assert!(
            text.contains("not supported yet"),
            "feedback text must explain the channel is unsupported, got: {text}"
        );
        assert!(
            text.contains(&CHAN_TELL.to_string()),
            "feedback text must name the offending channel id, got: {text}"
        );

        // No second message -- in particular, the witness (entity 2) never
        // hears about an unsupported-channel attempt.
        assert!(
            rx.try_recv().is_err(),
            "witness must not receive anything for an unsupported channel"
        );
    }

    /// The server channel is system-broadcast-only
    /// (`CHANNEL_FLAG_DisallowPlayerMessages` in the legacy
    /// `ChatChannelManager`) -- a player's own chat send on it must never be
    /// silently dropped, and must never reach witnesses as if it were a
    /// normal broadcast.
    #[tokio::test]
    async fn server_channel_rejects_player_speech_with_feedback() {
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.create_entity(2, "Agnos", [15.0, 0.0, 15.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        mgr.connect_entity(2);
        if let Some(e) = mgr.get_entity_mut(1) {
            e.witnesses.insert(cimmeria_common::EntityId(2));
        }

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let engine = ChainEngine::new();

        handle_chat_message(
            1,
            "Bob",
            0,
            CHAN_SERVER,
            "is this thing on?",
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let msg = rx
            .try_recv()
            .expect("sender must receive a feedback line explaining the rejection");
        let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        else {
            panic!("expected EntityMethodCall");
        };
        assert_eq!(entity_id, 1, "feedback must go to the sender only");
        assert_eq!(method_index, ON_PLAYER_COMMUNICATION);

        let (flags, channel, text) = decode_on_player_communication(&args);
        assert_eq!(flags, 0);
        assert_eq!(
            channel, CHAN_FEEDBACK,
            "feedback rides the registered feedback channel"
        );
        assert!(
            text.contains("system messages only"),
            "feedback text must explain the server channel is system-only, got: {text}"
        );

        // No second message: the witness must never see a player message
        // that was rejected as system-only, nor a broadcast of it.
        assert!(
            rx.try_recv().is_err(),
            "witness must not receive anything for a rejected server-channel send"
        );
    }
}
