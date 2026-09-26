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

#[cfg(test)]
mod tests {
    use super::*;

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
}
