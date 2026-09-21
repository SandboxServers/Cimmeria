//! `Action::NpcBark` — a non-modal companion line in the chat window.
//!
//! # Why this is not a dialog
//!
//! The 2009 client's dialog module has no non-modal path. Its lowest
//! screen type (`DUIST_None`, 0) is registered to the modal Blurb window
//! under a "TEMP HACK" comment, and every other type registers the same
//! modal `DialogWin`. A companion combat line ("Let's move out!") sent as
//! a dialog therefore stops the player dead in a window they must close —
//! the opposite of a bark.
//!
//! The one non-modal text route the client honours today is the chat
//! message the server already drives successfully:
//! `onPlayerCommunication(Speaker, SpeakerFlags, Channel, Text)`
//! (`entities/defs/interfaces/Communicator.def`), client method 28,
//! rendered by `ChatWindow.lua`'s `ChatMod.onMessageReceived`. This
//! module reuses the chat broadcaster's own serializer rather than
//! copying it, so a bark is byte-identical to a line that is known to
//! render.
//!
//! This deliberately does **not** route through `Action::SystemMessage`.
//! That stub's wire format is unknown, and the previous attempt to serve
//! it through method 28 garbled chat with empty-speaker `[] says` lines.
//!
//! # Delivery
//!
//! One `EntityMethodCall` addressed to the **triggering player only** —
//! not the say-chat witness fan-out, and not the sender-echo the chat
//! path adds. A bark is per-player mission feedback; fanning it out
//! would speak another player's escort line into a stranger's chat
//! window in a shared world.

use tokio::sync::mpsc;

use crate::cell::chat::serialize_on_player_communication;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::ON_PLAYER_COMMUNICATION;

/// `ESpeakerFlags::SPEAKER_None` (`entities/defs/enumerations.xml`). A
/// bark is neither a GM line nor a DND auto-reply, so the bitfield is
/// empty.
const SPEAKER_NONE: u8 = 0;

/// Speak one `resources.dialog_screens` line to the triggering player.
///
/// Three refusals, each with a `warn!` carrying a stable `reason` per
/// [`docs/architecture/negative-logging-convention.md`]. All three send
/// nothing at all — a half-formed bark renders as visible garbage in the
/// chat window, which is worse than silence:
///
/// 1. **`screen_not_cached`** — no `dialog_screens` row for `screen_id`.
///    Either the seed row names a screen that does not exist, or the
///    startup cache failed to load.
/// 2. **`empty_text`** — the row exists but its text is blank. The client
///    would draw a speaker prefix with nothing after it.
/// 3. **`actor_not_player`** — the chain fired from an NPC (an
///    `entity_dead_tag` chain, say), so "the triggering player" is
///    undefined and method 28 would resolve to no client address. Same
///    shape as `spawn_entity`'s refusal of the same situation.
#[tracing::instrument(
    name = "content.npc_bark",
    level = "info",
    skip_all,
    fields(entity_id, screen_id, chain_id)
)]
pub(super) async fn npc_bark(
    screen_id: i32,
    speaker: &str,
    channel: u8,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let text = match space_mgr.dialog_screen_text.get(&screen_id) {
        Some(t) => t,
        None => {
            tracing::warn!(
                entity_id,
                screen_id,
                chain_id,
                reason = "screen_not_cached",
                "npc_bark: no resources.dialog_screens row for this screen_id -- \
                 nothing spoken (check the seed row, or whether the startup \
                 dialog screen text cache loaded)"
            );
            return;
        }
    };
    if text.trim().is_empty() {
        tracing::warn!(
            entity_id,
            screen_id,
            chain_id,
            reason = "empty_text",
            "npc_bark: dialog screen text is blank -- nothing spoken (a speaker \
             prefix with no line is visible garbage in the chat window)"
        );
        return;
    }

    if !space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player) {
        tracing::warn!(
            entity_id,
            screen_id,
            chain_id,
            reason = "actor_not_player",
            "npc_bark: the chain's acting entity is not a player -- a bark is \
             addressed to the triggering player's own client, and method 28 to \
             an NPC resolves to no client address"
        );
        return;
    }

    tracing::info!(
        entity_id,
        screen_id,
        chain_id,
        speaker,
        channel,
        text_len = text.chars().count(),
        "Content: npc bark"
    );
    crate::cell::player_journal::note(
        entity_id,
        crate::cell::player_journal::kinds::BARK,
        format!("screen={screen_id} chain={chain_id}"),
    );

    let args = serialize_on_player_communication(speaker, SPEAKER_NONE, channel, text);
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args,
        })
        .await
    {
        // Same shape as PlaySequence / SetActiveSlot: the cell→base drop
        // swallows the line and the player loses the cue with no other
        // signal that the chain fired.
        tracing::warn!(
            entity_id,
            screen_id,
            chain_id,
            "npc_bark: cell→base send failed -- the line will not reach the player: {e}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use tracing::Level;

    /// Screen 96351 of dialog 5019 — Col. Marsh's escort line. Real id
    /// and real text from `db/resources/Dialogs/Seed/dialog_screens.sql`,
    /// so the byte assertions below are the bytes a shipped bark emits.
    const MARSH_SCREEN: i32 = 96351;
    const MARSH_TEXT: &str = "Let's move out!";
    const MARSH_SPEAKER: &str = "Col. Marsh";

    const PLAYER_EID: u32 = 8101;
    const OTHER_PLAYER_EID: u32 = 8102;
    const NPC_EID: u32 = 8103;

    /// One-space manager with the Marsh line in the screen-text cache.
    fn make_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.dialog_screen_text
            .insert(MARSH_SCREEN, MARSH_TEXT.to_string());
        mgr
    }

    fn stage_player(mgr: &mut SpaceManager, eid: u32) {
        mgr.create_entity(eid, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .expect("Agnos startup space must accept the entity");
        mgr.get_entity_mut(eid)
            .expect("entity must exist immediately after create_entity")
            .is_player = true;
        mgr.connect_entity(eid);
    }

    fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u32, u16, Vec<u8>)> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } = msg
            {
                out.push((entity_id, method_index, args));
            }
        }
        out
    }

    /// Byte-exact `onPlayerCommunication` payload for a bark.
    ///
    /// Built literally rather than by calling the serializer under test —
    /// asserting `serialize(..) == serialize(..)` would pass with any
    /// field order or encoding. WSTRING is a `u32` UTF-16 code-unit count
    /// followed by that many little-endian `u16`s; the two `UINT8`s sit
    /// between the two strings, flags first.
    #[tokio::test]
    async fn bark_payload_is_speaker_flags0_chan_say_text() {
        let mut mgr = make_mgr();
        stage_player(&mut mgr, PLAYER_EID);
        let (tx, mut rx) = mpsc::channel(8);

        npc_bark(MARSH_SCREEN, MARSH_SPEAKER, 0, PLAYER_EID, 900, &tx, &mgr).await;

        let sends = drain(&mut rx);
        assert_eq!(sends.len(), 1, "a bark must emit exactly one method call");
        let (target, method_index, args) = &sends[0];
        assert_eq!(*target, PLAYER_EID);
        assert_eq!(
            *method_index, 28,
            "barks ride onPlayerCommunication, client method 28"
        );

        let mut expected: Vec<u8> = Vec::new();
        // Speaker: "Col. Marsh" = 10 UTF-16 code units.
        expected.extend_from_slice(&10u32.to_le_bytes());
        for u in MARSH_SPEAKER.encode_utf16() {
            expected.extend_from_slice(&u.to_le_bytes());
        }
        // SpeakerFlags = SPEAKER_None, then Channel = CHAN_say.
        expected.push(0);
        expected.push(0);
        // Text: "Let's move out!" = 15 UTF-16 code units.
        expected.extend_from_slice(&15u32.to_le_bytes());
        for u in MARSH_TEXT.encode_utf16() {
            expected.extend_from_slice(&u.to_le_bytes());
        }

        assert_eq!(
            args, &expected,
            "bark payload must be byte-identical to the chat path's \
             onPlayerCommunication(Speaker, SpeakerFlags, Channel, Text)"
        );
        // Spelled out so a reordering that happens to keep the length
        // still fails on the offsets a reader can check by hand.
        assert_eq!(args.len(), 4 + 20 + 1 + 1 + 4 + 30);
        assert_eq!(args[24], 0, "SpeakerFlags byte must be 0 (SPEAKER_None)");
        assert_eq!(args[25], 0, "Channel byte must be 0 (CHAN_say)");
    }

    /// A bark reaches the triggering player and nobody else — not the
    /// say-chat witness fan-out, and not a sender echo. Another player
    /// standing in the same space, witnessing the first, must receive
    /// nothing.
    #[tokio::test]
    async fn bark_targets_only_the_triggering_player() {
        let mut mgr = make_mgr();
        stage_player(&mut mgr, PLAYER_EID);
        stage_player(&mut mgr, OTHER_PLAYER_EID);
        mgr.get_entity_mut(PLAYER_EID)
            .unwrap()
            .witnesses
            .insert(cimmeria_common::EntityId(OTHER_PLAYER_EID as i32));

        let (tx, mut rx) = mpsc::channel(8);
        npc_bark(MARSH_SCREEN, MARSH_SPEAKER, 0, PLAYER_EID, 900, &tx, &mgr).await;

        let targets: Vec<u32> = drain(&mut rx).into_iter().map(|(t, _, _)| t).collect();
        assert_eq!(
            targets,
            vec![PLAYER_EID],
            "a bark is per-player mission feedback; fanning it to witnesses \
             would speak one player's escort line into a stranger's chat window"
        );
    }

    /// An unknown `screen_id` warns with `reason = screen_not_cached`
    /// and sends nothing.
    #[tokio::test]
    async fn unknown_screen_id_warns_and_sends_nothing() {
        let mut mgr = make_mgr();
        stage_player(&mut mgr, PLAYER_EID);
        let (tx, mut rx) = mpsc::channel(8);

        let capture = LogCapture::install();
        npc_bark(424242, MARSH_SPEAKER, 0, PLAYER_EID, 900, &tx, &mgr).await;

        assert!(
            capture
                .find_event(Level::WARN, "npc_bark", "screen_not_cached")
                .is_some(),
            "an unresolvable screen_id must warn with reason=screen_not_cached"
        );
        assert!(
            drain(&mut rx).is_empty(),
            "an unresolvable screen_id must send nothing at all"
        );
    }

    /// A cached-but-blank line warns with `reason = empty_text` and sends
    /// nothing: the client would otherwise draw a speaker prefix with no
    /// line after it.
    #[tokio::test]
    async fn blank_text_warns_and_sends_nothing() {
        let mut mgr = make_mgr();
        stage_player(&mut mgr, PLAYER_EID);
        mgr.dialog_screen_text.insert(777, "   ".to_string());
        let (tx, mut rx) = mpsc::channel(8);

        let capture = LogCapture::install();
        npc_bark(777, MARSH_SPEAKER, 0, PLAYER_EID, 900, &tx, &mgr).await;

        assert!(
            capture
                .find_event(Level::WARN, "npc_bark", "empty_text")
                .is_some(),
            "a blank line must warn with reason=empty_text"
        );
        assert!(drain(&mut rx).is_empty(), "a blank line must send nothing");
    }

    /// A chain fired by an NPC has no "triggering player", and method 28
    /// addressed to an NPC resolves to no client. Warn, send nothing.
    #[tokio::test]
    async fn npc_actor_warns_and_sends_nothing() {
        let mut mgr = make_mgr();
        mgr.create_entity(NPC_EID, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .expect("Agnos startup space must accept the NPC");
        // `create_entity` leaves `is_player = false`; the NPC is never
        // connected.
        let (tx, mut rx) = mpsc::channel(8);

        let capture = LogCapture::install();
        npc_bark(MARSH_SCREEN, MARSH_SPEAKER, 0, NPC_EID, 900, &tx, &mgr).await;

        assert!(
            capture
                .find_event(Level::WARN, "npc_bark", "actor_not_player")
                .is_some(),
            "an NPC actor must warn with reason=actor_not_player"
        );
        assert!(
            drain(&mut rx).is_empty(),
            "an NPC actor must not produce an undeliverable method call"
        );
    }
}
