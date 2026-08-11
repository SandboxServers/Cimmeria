//! Chat message distribution for the CellService.
//!
//! Handles spatial chat channels (say, emote, yell) by broadcasting
//! `onPlayerCommunication` to witnesses in the sender's Area of Interest.
//!
//! Reference: `python/cell/SGWPlayer.py:processPlayerCommunication()`

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::console;
use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// ── Channel IDs (from python/Atrea/enums.py EChannel) ─────────────────────

/// Local say channel — spatial, nearby players.
pub const CHAN_SAY: u8 = 0;
/// Emote channel — spatial, nearby players.
pub const CHAN_EMOTE: u8 = 1;
/// Yell channel — spatial, wider range.
pub const CHAN_YELL: u8 = 2;
/// Team channel — group members only.
pub const CHAN_TEAM: u8 = 3;
/// Squad channel — squad members only.
pub const CHAN_SQUAD: u8 = 4;
/// Command channel — guild/command members.
pub const CHAN_COMMAND: u8 = 5;
/// Officer channel — guild officers.
pub const CHAN_OFFICER: u8 = 6;
/// Server channel — system broadcasts only.
pub const CHAN_SERVER: u8 = 7;
/// GM-feedback channel. The client only registers the channels in the base's
/// `DEFAULT_CHAT_CHANNELS` (say/emote/yell/team/squad/command/server=7/tell=9);
/// there is **no** dedicated feedback channel, and an *unregistered* channel
/// (e.g. 8) falls back to the client's red unknown-channel splash popup. So GM
/// feedback rides the registered `tell` channel (9) — the same channel the
/// base's inline welcome message uses (`world_entry_chat::CHAN_TELL`).
pub const CHAN_FEEDBACK: u8 = 9;
/// Tell channel — direct player-to-player (handled by BaseApp, not here).
pub const CHAN_TELL: u8 = 9;
/// Splash screen channel.
pub const CHAN_SPLASH: u8 = 10;

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
        CHAN_SQUAD => {
            broadcast_to_squad(entity_id, speaker_name, speaker_flags, text, tx, space_mgr).await;
        }
        _ => {
            tracing::debug!(
                entity_id,
                channel,
                "Chat channel not handled by CellService"
            );
        }
    }
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

    // Collect witness IDs (clone to avoid borrow conflicts)
    let witnesses: Vec<u32> = entity.witnesses.iter().map(|eid| eid.0 as u32).collect();

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

/// Broadcast a chat message to all squad members of the sender.
///
/// Sends `onPlayerCommunication` to every squad member entity_id (including
/// the sender, so they see their own message echoed with squad formatting).
/// If the sender is not in a squad the message is silently dropped.
async fn broadcast_to_squad(
    sender_id: u32,
    speaker_name: &str,
    speaker_flags: u8,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let org_id = match space_mgr.squads.squad_of(sender_id) {
        Some(id) => id,
        None => {
            tracing::debug!(sender_id, "Squad chat: sender not in a squad");
            return;
        }
    };

    let member_ids: Vec<u32> = match space_mgr.squads.get_squad(org_id) {
        Some(s) => s.member_entity_ids(),
        None => return,
    };

    let args = serialize_on_player_communication(speaker_name, speaker_flags, CHAN_SQUAD, text);

    tracing::debug!(
        sender_id,
        org_id,
        member_count = member_ids.len(),
        "Routing squad chat"
    );

    for member_id in member_ids {
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id: member_id,
                method_index: ON_PLAYER_COMMUNICATION,
                args: args.clone(),
            })
            .await;
    }
}

/// Serialize `onPlayerCommunication(Speaker, SpeakerFlags, Channel, Text)` args.
///
/// Wire format:
/// - Speaker: WSTRING (u32 char_count + N×2B UTF-16LE)
/// - SpeakerFlags: UINT8
/// - Channel: UINT8
/// - Text: WSTRING (u32 char_count + N×2B UTF-16LE)
fn serialize_on_player_communication(
    speaker: &str,
    speaker_flags: u8,
    channel: u8,
    text: &str,
) -> Vec<u8> {
    let speaker_utf16: Vec<u16> = speaker.encode_utf16().collect();
    let text_utf16: Vec<u16> = text.encode_utf16().collect();

    let capacity = 4 + speaker_utf16.len() * 2 + 1 + 1 + 4 + text_utf16.len() * 2;
    let mut args = Vec::with_capacity(capacity);

    // Speaker: WSTRING
    args.extend_from_slice(&(speaker_utf16.len() as u32).to_le_bytes());
    for &ch in &speaker_utf16 {
        args.extend_from_slice(&ch.to_le_bytes());
    }

    // SpeakerFlags: UINT8
    args.push(speaker_flags);

    // Channel: UINT8
    args.push(channel);

    // Text: WSTRING
    args.extend_from_slice(&(text_utf16.len() as u32).to_le_bytes());
    for &ch in &text_utf16 {
        args.extend_from_slice(&ch.to_le_bytes());
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_on_player_communication_basic() {
        let args = serialize_on_player_communication("Bob", 0, CHAN_SAY, "Hello");

        let mut offset = 0;
        // Speaker: "Bob" = 3 UTF-16 chars
        let speaker_len = u32::from_le_bytes([args[0], args[1], args[2], args[3]]);
        assert_eq!(speaker_len, 3);
        offset += 4 + 3 * 2; // 4 + 6 = 10

        // SpeakerFlags
        assert_eq!(args[offset], 0);
        offset += 1;

        // Channel
        assert_eq!(args[offset], CHAN_SAY);
        offset += 1;

        // Text: "Hello" = 5 UTF-16 chars
        let text_len = u32::from_le_bytes([
            args[offset],
            args[offset + 1],
            args[offset + 2],
            args[offset + 3],
        ]);
        assert_eq!(text_len, 5);
        offset += 4 + 5 * 2; // 4 + 10 = 14

        assert_eq!(args.len(), offset);
    }

    #[test]
    fn serialize_on_player_communication_empty_text() {
        let args = serialize_on_player_communication("A", 0x02, CHAN_EMOTE, "");

        // Speaker "A": 4 + 2 = 6 bytes
        // Flags: 1 byte
        // Channel: 1 byte
        // Text "": 4 + 0 = 4 bytes
        assert_eq!(args.len(), 6 + 1 + 1 + 4);

        // Check flags
        assert_eq!(args[6], 0x02);
        // Check channel
        assert_eq!(args[7], CHAN_EMOTE);
        // Check empty text
        let text_len = u32::from_le_bytes([args[8], args[9], args[10], args[11]]);
        assert_eq!(text_len, 0);
    }

    #[tokio::test]
    async fn broadcast_to_nonexistent_entity_is_noop() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let engine = ChainEngine::new();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        handle_chat_message(999, "Bob", 0, CHAN_SAY, "Hello", &tx, &mut mgr, &engine).await;

        // No messages should be sent
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn broadcast_say_to_witnesses() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
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

    /// A GM's `.`-command is consumed by the console and never broadcast to
    /// witnesses (never appears in others' chat).
    #[tokio::test]
    async fn gm_dot_command_is_intercepted_not_broadcast() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
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
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
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

    #[tokio::test]
    async fn non_cell_channel_ignored() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let engine = ChainEngine::new();

        // Tell channel should not be handled by CellService
        handle_chat_message(1, "Bob", 0, CHAN_TELL, "Hi", &tx, &mut mgr, &engine).await;
        assert!(rx.try_recv().is_err());
    }

    /// CHAN_SQUAD chat is delivered only to squad members, not AoI witnesses.
    #[tokio::test]
    async fn squad_chat_reaches_only_squad_members() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();

        // Entity 1 = Alice (squad member), entity 2 = Bob (squad member),
        // entity 3 = Carol (bystander, NOT in squad).
        for eid in [1u32, 2, 3] {
            mgr.create_entity(eid, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
                .unwrap();
            mgr.connect_entity(eid);
        }

        // Form a squad: Alice invites Bob.
        let rid = mgr
            .squads
            .record_invite(1, "Alice".into(), 2, 0, "Squad".into());
        mgr.squads
            .accept_invite(rid, 2, 200, "Bob".into(), 100)
            .unwrap();

        let engine = ChainEngine::new();
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);

        // Alice sends squad chat.
        handle_chat_message(
            1,
            "Alice",
            0,
            CHAN_SQUAD,
            "Squad only!",
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let mut recipients: Vec<u32> = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                ..
            } = msg
            {
                assert_eq!(method_index, ON_PLAYER_COMMUNICATION, "correct method");
                recipients.push(entity_id);
            }
        }

        // Both squad members (1 and 2) get the message; Carol (3) does not.
        assert!(
            recipients.contains(&1),
            "Alice must receive her own squad chat"
        );
        assert!(recipients.contains(&2), "Bob must receive squad chat");
        assert!(
            !recipients.contains(&3),
            "Carol must NOT receive squad chat"
        );
    }

    /// CHAN_SQUAD chat from a non-member is silently dropped.
    #[tokio::test]
    async fn squad_chat_from_non_member_is_dropped() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);

        let engine = ChainEngine::new();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        // Entity 1 is not in any squad.
        handle_chat_message(
            1,
            "Loner",
            0,
            CHAN_SQUAD,
            "nobody here",
            &tx,
            &mut mgr,
            &engine,
        )
        .await;
        assert!(
            rx.try_recv().is_err(),
            "no messages sent when not in a squad"
        );
    }
}
