//! The welcome message for `onClientReady`, and why no chat channel is
//! registered there.
//!
//! Pure arg-builder for the entity-method packet fired in
//! `cimmeria_base_world_entry::base::world_entry_appearance::handle_on_client_ready`. Split out
//! of `world_entry_appearance.rs` so the byte-level wire-format logic
//! has byte-exact unit tests without dragging in the file's async-handler
//! surface; the wire-emit side stays next to the rest of the
//! `onClientReady` finalisation.
//!
//! **No `onChatJoined` at login (SS-C4, D-ORG14).** The client compiles every
//! built-in channel id (say 0 to splash 11) in as a literal equal to
//! `EChannel` (ORG-E1 Q5), so it needs no registration to send or show them.
//! It treats every `onChatJoined(name, id)` as a *user* channel: its
//! `ChatMod.onChannelJoined` files it under `UIChannel.Chat + id` and prints
//! "You have joined channel [id:name]" (`ChatWindow.lua:370-386`,
//! `ChatWindow.int:107`). The legacy server matched that: `onChatJoined`
//! only for ids of 12 and up, carrying the id minus 12
//! (`deprecated/python/base/SGWPlayer.py:162-163`), and `playerLoggedIn`
//! joined the server channel on the server side only (`Chat.py:183-190`).
//! Rust used to send eight built-in `onChatJoined`s, which made eight bogus
//! user channels. A user-channel feature, when one exists, sends
//! `onChatJoined` from its own join path.

use cimmeria_wire::cell::chat::CHAN_FEEDBACK;

use crate::mercury::write_wstring;

/// Build the `onPlayerCommunication(speaker, speakerFlags, channel, text)`
/// wire args for the post-`onClientReady` welcome message.
///
/// Wire format: `[wstring speaker][u8 speaker_flags=0][u8 channel][wstring text]`.
/// The text is `"Welcome to Stargate Worlds. Your player id is: {entity_id}."`
/// pinned by [`tests`]; matches the python reference at `SGWPlayer.py:541`,
/// which sends a literal channel 9. That is `CHAN_feedback`, an ordinary
/// line in the Info tab. It is not the server channel (8): the client opens
/// a modal "Server Message" prompt for every line on 8, which would greet
/// every login with a dialog. The byte has been 9 all along; before SS-C4 it
/// was named `CHAN_TELL`.
pub fn build_welcome_message_args(speaker: &str, entity_id: u32) -> Vec<u8> {
    let welcome = format!("Welcome to Stargate Worlds. Your player id is: {entity_id}.");
    let mut buf = Vec::with_capacity(16 + (speaker.len() + welcome.len()) * 2);
    write_wstring(&mut buf, speaker);
    buf.push(0u8); // SpeakerFlags
    buf.push(CHAN_FEEDBACK);
    write_wstring(&mut buf, &welcome);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `build_welcome_message_args` wire layout:
    /// `[wstring speaker][u8 0=flags][u8 9=CHAN_feedback][wstring text]`.
    /// Pin the full byte sequence so a regression that drops the
    /// flags byte, picks a different channel, or rewrites the welcome
    /// text fails here.
    #[test]
    fn build_welcome_message_args_emits_speaker_flags_chan_feedback_then_text() {
        let buf = build_welcome_message_args("S", 7);
        // Expected text: "Welcome to Stargate Worlds. Your player id is: 7."
        // (50 chars, all BMP, so 50 UTF-16 code units, 100 bytes).
        let mut expected: Vec<u8> = Vec::new();
        // speaker wstring "S"
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(&[b'S', 0]);
        expected.push(0u8); // SpeakerFlags
        expected.push(9u8); // CHAN_feedback, as the python's literal 9

        // text wstring
        let text = "Welcome to Stargate Worlds. Your player id is: 7.";
        let utf16: Vec<u16> = text.encode_utf16().collect();
        expected.extend_from_slice(&(utf16.len() as u32).to_le_bytes());
        for unit in utf16 {
            expected.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(buf, expected, "byte-exact wire layout for welcome");
    }

    /// Entity id substitution must actually thread into the text. A
    /// regression that hard-codes a placeholder (or formats the wrong
    /// variable) would leave the welcome message lying to the player —
    /// catch it by varying entity_id and asserting the text reflects it.
    #[test]
    fn build_welcome_message_args_includes_entity_id_in_text() {
        let buf = build_welcome_message_args("Tester", 12345);
        // Skip the speaker wstring + flags + channel bytes to reach text.
        let speaker_units = "Tester".encode_utf16().count();
        let text_offset = 4 + speaker_units * 2 + 1 + 1;
        let text_count = u32::from_le_bytes(
            buf[text_offset..text_offset + 4]
                .try_into()
                .expect("4 bytes"),
        ) as usize;
        let text_bytes = &buf[text_offset + 4..text_offset + 4 + text_count * 2];
        let text: String = char::decode_utf16(
            text_bytes
                .chunks(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]])),
        )
        .map(|r| r.unwrap_or('?'))
        .collect();
        assert_eq!(
            text, "Welcome to Stargate Worlds. Your player id is: 12345.",
            "entity_id must be substituted into the welcome message"
        );
    }

    /// Fallback speaker for the player_name=None branch in
    /// `handle_on_client_ready` must produce a valid welcome message,
    /// not panic or omit fields. The fallback string ("Server") is
    /// non-empty, so a future refactor that swaps the fallback for an
    /// empty string is distinguishable from the legitimate case.
    #[test]
    fn build_welcome_message_args_handles_fallback_speaker() {
        let buf = build_welcome_message_args("Server", 42);
        // Just verify the speaker wstring is well-formed and reachable.
        let speaker_count = u32::from_le_bytes(buf[0..4].try_into().expect("4 bytes")) as usize;
        assert_eq!(speaker_count, 6, "'Server' has 6 UTF-16 code units");
        let speaker_bytes = &buf[4..4 + speaker_count * 2];
        let speaker: String = char::decode_utf16(
            speaker_bytes
                .chunks(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]])),
        )
        .map(|r| r.unwrap_or('?'))
        .collect();
        assert_eq!(speaker, "Server");
        // SpeakerFlags + Channel bytes are at the fixed offset after the
        // speaker wstring.
        let after_speaker = 4 + speaker_count * 2;
        assert_eq!(buf[after_speaker], 0, "SpeakerFlags must be 0");
        assert_eq!(
            buf[after_speaker + 1],
            CHAN_FEEDBACK,
            "Channel must be CHAN_FEEDBACK"
        );
    }
}
