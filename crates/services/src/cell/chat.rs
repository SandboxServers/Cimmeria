//! Chat message distribution for the CellService.
//!
//! Handles spatial chat channels (say, emote, yell) by broadcasting
//! `onPlayerCommunication` to witnesses in the sender's Area of Interest.
//!
//! Reference: `python/cell/SGWPlayer.py:processPlayerCommunication()`

use tokio::sync::mpsc;

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
/// Feedback channel — system feedback.
pub const CHAN_FEEDBACK: u8 = 8;
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
pub async fn handle_chat_message(
    entity_id: u32,
    speaker_name: &str,
    speaker_flags: u8,
    channel: u8,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    match channel {
        CHAN_SAY | CHAN_EMOTE | CHAN_YELL => {
            broadcast_to_witnesses(entity_id, speaker_name, speaker_flags, channel, text, tx, space_mgr).await;
        }
        _ => {
            tracing::debug!(entity_id, channel, "Chat channel not handled by CellService");
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
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id: witness_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args: args.clone(),
        }).await;
    }

    // Also send to the sender themselves (client needs server echo for say channel,
    // and sending for all spatial channels is harmless)
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id: sender_id,
        method_index: ON_PLAYER_COMMUNICATION,
        args,
    }).await;
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
        let text_len = u32::from_le_bytes([args[offset], args[offset+1], args[offset+2], args[offset+3]]);
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
        let mgr = super::super::space_manager::SpaceManager::new(1);
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        handle_chat_message(999, "Bob", 0, CHAN_SAY, "Hello", &tx, &mgr).await;

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
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();
        mgr.create_entity(2, "Agnos", [15.0, 0.0, 15.0], [0.0; 3]).unwrap();
        mgr.connect_entity(1);
        mgr.connect_entity(2);

        // Manually add witness relationships (normally done by AoI tick)
        if let Some(e) = mgr.get_entity_mut(1) {
            e.witnesses.insert(cimmeria_common::EntityId(2));
        }

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);

        handle_chat_message(1, "Alice", 0, CHAN_SAY, "Hello world", &tx, &mgr).await;

        // Should get 2 messages: one for witness (entity 2) + one for sender (entity 1)
        let mut msgs = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            msgs.push(msg);
        }
        assert_eq!(msgs.len(), 2);

        // Check the first is to witness entity 2
        match &msgs[0] {
            CellToBaseMsg::EntityMethodCall { entity_id, method_index, .. } => {
                assert_eq!(*entity_id, 2);
                assert_eq!(*method_index, ON_PLAYER_COMMUNICATION);
            }
            _ => panic!("Expected EntityMethodCall"),
        }

        // Check the second is to sender entity 1
        match &msgs[1] {
            CellToBaseMsg::EntityMethodCall { entity_id, method_index, .. } => {
                assert_eq!(*entity_id, 1);
                assert_eq!(*method_index, ON_PLAYER_COMMUNICATION);
            }
            _ => panic!("Expected EntityMethodCall"),
        }
    }

    #[tokio::test]
    async fn non_cell_channel_ignored() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        // Tell channel should not be handled by CellService
        handle_chat_message(1, "Bob", 0, CHAN_TELL, "Hi", &tx, &mgr).await;
        assert!(rx.try_recv().is_err());
    }
}
