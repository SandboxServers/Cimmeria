//! The `onPlayerCommunication` payload and the `EChannel` ids it carries.
//!
//! The chat handlers are `cimmeria_cell_console::cell::console::chat`,
//! which re-exports everything here; the `npc_bark` content action speaks through
//! the same serializer.
//!
//! Reference: `python/cell/SGWPlayer.py:processPlayerCommunication()`

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

/// `ESpeakerFlags::SPEAKER_GM` (`entities/defs/enumerations.xml`): the
/// speaker is staff. Pinned to the def by
/// `tests::speaker_gm_matches_enumerations_xml`.
pub const SPEAKER_GM: u8 = 0x01;

/// The `onPlayerCommunication` args of a GM broadcast (`sendGMShout`, cell
/// method 222, and the `.announce` console command; D-SS16): the GM's name,
/// [`SPEAKER_GM`], and [`CHAN_SERVER`]. The space scope (cell) and the
/// global scope (base) both send these bytes, so the two cannot drift.
///
/// The channel is whatever `CHAN_SERVER` holds; the organizations campaign
/// owns that constant (D-ORG14, D-SS17), and the broadcast follows it.
pub fn serialize_gm_broadcast(speaker: &str, text: &str) -> Vec<u8> {
    serialize_on_player_communication(speaker, SPEAKER_GM, CHAN_SERVER, text)
}

/// Serialize `onPlayerCommunication(Speaker, SpeakerFlags, Channel, Text)` args.
///
/// Wire format:
/// - Speaker: WSTRING (u32 char_count + N×2B UTF-16LE)
/// - SpeakerFlags: UINT8
/// - Channel: UINT8
/// - Text: WSTRING (u32 char_count + N×2B UTF-16LE)
///
/// One builder for the chat broadcaster and the `npc_bark` content action
/// (`cimmeria_cell_content::cell::content::executor::bark`), so a bark speaks
/// through the same bytes as chat. A bark is the only non-modal text route
/// the client actually honours, so it must be byte-identical to the
/// chat path that is known to render — a second copy of this serializer
/// is a second place for that to drift.
pub fn serialize_on_player_communication(
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

/// Serialize `onTellSent(WSTRING aTarget, WSTRING aText)` args
/// (`Communicator.def`, client method 30): the sender's confirmation that a
/// tell went out, naming the recipient.
///
/// Wire format: two WSTRINGs, each `u32` UTF-16 unit count then the units LE.
pub fn serialize_on_tell_sent(target: &str, text: &str) -> Vec<u8> {
    let mut args = Vec::with_capacity(8 + (target.len() + text.len()) * 2);
    for s in [target, text] {
        let units: Vec<u16> = s.encode_utf16().collect();
        args.extend_from_slice(&(units.len() as u32).to_le_bytes());
        for u in units {
            args.extend_from_slice(&u.to_le_bytes());
        }
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `onTellSent("Bo", "hi")`: `[02 00 00 00]['B' 'o'][02 00 00 00]['h' 'i']`.
    #[test]
    fn serialize_on_tell_sent_is_two_wstrings() {
        assert_eq!(
            serialize_on_tell_sent("Bo", "hi"),
            vec![
                0x02, 0x00, 0x00, 0x00, b'B', 0x00, b'o', 0x00, //
                0x02, 0x00, 0x00, 0x00, b'h', 0x00, b'i', 0x00,
            ]
        );
        assert_eq!(
            serialize_on_tell_sent("", ""),
            vec![0u8; 8],
            "empty strings still carry both length prefixes"
        );
    }

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

    /// Byte-exact GM broadcast line (SS-C2): speaker "Gm" (2 units),
    /// SPEAKER_GM, CHAN_SERVER, text "Hi!" (3 units). A drift in the flag
    /// or the channel byte shows the shout as an ordinary player line, or
    /// on a channel the client never joined.
    #[test]
    fn gm_broadcast_bytes_are_exact() {
        let args = serialize_gm_broadcast("Gm", "Hi!");
        let expected: Vec<u8> = vec![
            0x02,
            0x00,
            0x00,
            0x00, // speaker char count
            b'G',
            0x00,
            b'm',
            0x00, // "Gm" UTF-16LE
            0x01, // SPEAKER_GM
            CHAN_SERVER,
            0x03,
            0x00,
            0x00,
            0x00, // text char count
            b'H',
            0x00,
            b'i',
            0x00,
            b'!',
            0x00, // "Hi!" UTF-16LE
        ];
        assert_eq!(args, expected);
    }

    /// `SPEAKER_GM` is read from the def, not from a copy of itself.
    #[test]
    fn speaker_gm_matches_enumerations_xml() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../entities/defs/enumerations.xml");
        let xml = std::fs::read_to_string(&path).expect("read enumerations.xml");
        let start = xml.find("<ESpeakerFlags>").expect("ESpeakerFlags block");
        let end = start + xml[start..].find("</ESpeakerFlags>").expect("close tag");
        let block = &xml[start..end];
        let token = block
            .split("<Token>")
            .find(|t| t.contains("<Name>SPEAKER_GM</Name>"))
            .expect("SPEAKER_GM token");
        let v = &token[token.find("<Value>").unwrap() + 7..token.find("</Value>").unwrap()];
        assert_eq!(v.trim().parse::<u8>().unwrap(), SPEAKER_GM);
    }
}
