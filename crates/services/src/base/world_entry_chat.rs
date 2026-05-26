//! Chat-channel registration + welcome message for `onClientReady`.
//!
//! Pure arg-builders for the entity-method packets fired in
//! [`super::world_entry_appearance::handle_on_client_ready`]. Split out
//! of `world_entry_appearance.rs` so the byte-level wire-format logic
//! has byte-exact unit tests without dragging in the file's async-handler
//! surface; the wire-emit side stays next to the rest of the
//! `onClientReady` finalisation.

use crate::mercury::write_wstring;

/// Default chat channels registered on every world-entry `onClientReady`.
///
/// Each entry is `(channel_name, channel_id)`. The id values come from
/// `python/common/Constants.py` and the channel set + ordering matches
/// `python/base/SGWPlayer.py onClientReady -> ChannelManager.playerLoggedIn`.
/// Pulled out as a module constant so it's covered by a byte-exact test
/// in [`tests`] and so a future configuration knob (per-archetype
/// channel sets, for example) has a single source to read from.
pub(crate) const DEFAULT_CHAT_CHANNELS: &[(&str, u8)] = &[
    ("say", 0),
    ("emote", 1),
    ("yell", 2),
    ("team", 3),
    ("squad", 4),
    ("command", 5),
    ("server", 7),
    ("tell", 9),
];

/// `onPlayerCommunication`'s "tell" channel id (matches `CHAN_TELL` in
/// `python/common/Constants.py` and `SGWPlayer.py:541`).
pub(crate) const CHAN_TELL: u8 = 9;

/// Build the `onChatJoined(WSTRING channelName, UINT8 channelID)` wire args.
///
/// Wire format: `[u32 char_count][UTF-16LE chars][u8 channel_id]`.
pub(crate) fn build_chat_joined_args(channel_name: &str, channel_id: u8) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8 + channel_name.len() * 2);
    write_wstring(&mut buf, channel_name);
    buf.push(channel_id);
    buf
}

/// Build the `onPlayerCommunication(speaker, speakerFlags, channel, text)`
/// wire args for the post-`onClientReady` welcome message.
///
/// Wire format: `[wstring speaker][u8 speaker_flags=0][u8 channel][wstring text]`.
/// The text is `"Welcome to Stargate Worlds. Your player id is: {entity_id}."`
/// pinned by [`tests`]; matches the python reference at `SGWPlayer.py:541`.
pub(crate) fn build_welcome_message_args(speaker: &str, entity_id: u32) -> Vec<u8> {
    let welcome = format!("Welcome to Stargate Worlds. Your player id is: {entity_id}.");
    let mut buf = Vec::with_capacity(16 + (speaker.len() + welcome.len()) * 2);
    write_wstring(&mut buf, speaker);
    buf.push(0u8); // SpeakerFlags
    buf.push(CHAN_TELL);
    write_wstring(&mut buf, &welcome);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `DEFAULT_CHAT_CHANNELS` must match the original python
    /// `SGWPlayer.py onClientReady -> ChannelManager.playerLoggedIn`
    /// channel set exactly — same names, same ids, same order. A
    /// regression that adds, drops, or reorders entries silently
    /// changes the wire-burst shape on every world entry; pinning
    /// the full slice catches it.
    #[test]
    fn default_chat_channels_matches_sgwplayer_channel_set() {
        let expected: &[(&str, u8)] = &[
            ("say", 0),
            ("emote", 1),
            ("yell", 2),
            ("team", 3),
            ("squad", 4),
            ("command", 5),
            ("server", 7),
            ("tell", 9),
        ];
        assert_eq!(
            DEFAULT_CHAT_CHANNELS, expected,
            "DEFAULT_CHAT_CHANNELS drifted from the SGWPlayer.py reference set",
        );
        // CHAN_TELL constant must agree with the "tell" entry in the slice;
        // the welcome message in `handle_on_client_ready` reads from
        // CHAN_TELL while the channel-join loop reads from the slice.
        let tell_entry = DEFAULT_CHAT_CHANNELS
            .iter()
            .find(|(name, _)| *name == "tell")
            .expect("'tell' channel must be in DEFAULT_CHAT_CHANNELS");
        assert_eq!(
            tell_entry.1, CHAN_TELL,
            "CHAN_TELL ({}) must equal the 'tell' entry's id ({}) in DEFAULT_CHAT_CHANNELS",
            CHAN_TELL, tell_entry.1
        );
    }

    /// `build_chat_joined_args` wire layout: `[wstring channel_name][u8
    /// channel_id]`. Byte-exact comparison so a regression that drops the
    /// length prefix, swaps to UTF-8, or appends the id before the
    /// wstring (matching the reverse-order python alias) fails here.
    #[test]
    fn build_chat_joined_args_emits_wstring_then_u8_id() {
        let buf = build_chat_joined_args("say", 0);
        let expected: &[u8] = &[
            3, 0, 0, 0, b's', 0, b'a', 0, b'y', 0, // wstring "say"
            0, // channel_id
        ];
        assert_eq!(buf, expected, "byte-exact wire layout for 'say'/0");

        let buf = build_chat_joined_args("tell", 9);
        let expected: &[u8] = &[
            4, 0, 0, 0, b't', 0, b'e', 0, b'l', 0, b'l', 0, // wstring "tell"
            9, // channel_id
        ];
        assert_eq!(buf, expected, "byte-exact wire layout for 'tell'/9");
    }

    /// All 8 channels in `DEFAULT_CHAT_CHANNELS` must produce well-formed
    /// `onChatJoined` args. Sweeping the actual slice catches a future
    /// channel addition that forgets to update `build_chat_joined_args`
    /// (e.g. introducing a wider id type).
    #[test]
    fn build_chat_joined_args_round_trips_for_all_default_channels() {
        for &(name, id) in DEFAULT_CHAT_CHANNELS {
            let buf = build_chat_joined_args(name, id);
            // Length prefix must equal name.chars().encode_utf16().count()
            // (matches `write_wstring` semantics, not `name.len()`).
            let utf16_units: u32 = name.encode_utf16().count() as u32;
            assert_eq!(
                &buf[0..4],
                &utf16_units.to_le_bytes(),
                "channel {name}: wstring length prefix must be UTF-16 code-unit count"
            );
            let payload_end = 4 + (utf16_units as usize) * 2;
            assert_eq!(
                buf.len(),
                payload_end + 1,
                "channel {name}: total length = 4 (count) + 2*units + 1 (id)",
            );
            assert_eq!(
                buf[payload_end], id,
                "channel {name}: id byte must follow the wstring",
            );
        }
    }

    /// `build_welcome_message_args` wire layout:
    /// `[wstring speaker][u8 0=flags][u8 9=CHAN_TELL][wstring text]`.
    /// Pin the full byte sequence so a regression that drops the
    /// flags byte, picks a different channel, or rewrites the welcome
    /// text fails here.
    #[test]
    fn build_welcome_message_args_emits_speaker_flags_chan_tell_then_text() {
        let buf = build_welcome_message_args("S", 7);
        // Expected text: "Welcome to Stargate Worlds. Your player id is: 7."
        // (50 chars, all BMP, so 50 UTF-16 code units, 100 bytes).
        let mut expected: Vec<u8> = Vec::new();
        // speaker wstring "S"
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(&[b'S', 0]);
        expected.push(0u8); // SpeakerFlags
        expected.push(9u8); // CHAN_TELL
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
                .chunks_exact(2)
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
    /// not panic or omit fields. The fallback string ("Server") is a
    /// real channel name in `DEFAULT_CHAT_CHANNELS`, so a future
    /// refactor that swaps the fallback for an empty string is
    /// distinguishable from the legitimate case.
    #[test]
    fn build_welcome_message_args_handles_fallback_speaker() {
        let buf = build_welcome_message_args("Server", 42);
        // Just verify the speaker wstring is well-formed and reachable.
        let speaker_count = u32::from_le_bytes(buf[0..4].try_into().expect("4 bytes")) as usize;
        assert_eq!(speaker_count, 6, "'Server' has 6 UTF-16 code units");
        let speaker_bytes = &buf[4..4 + speaker_count * 2];
        let speaker: String = char::decode_utf16(
            speaker_bytes
                .chunks_exact(2)
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
            CHAN_TELL,
            "Channel must be CHAN_TELL"
        );
    }
}
