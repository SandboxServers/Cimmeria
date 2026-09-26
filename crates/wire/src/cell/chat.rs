//! The `onPlayerCommunication` payload and the `EChannel` ids it carries.
//!
//! The chat handlers stay in `cimmeria_services::cell::chat`, which
//! re-exports everything here; the `npc_bark` content action speaks through
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

/// Serialize `onPlayerCommunication(Speaker, SpeakerFlags, Channel, Text)` args.
///
/// Wire format:
/// - Speaker: WSTRING (u32 char_count + N×2B UTF-16LE)
/// - SpeakerFlags: UINT8
/// - Channel: UINT8
/// - Text: WSTRING (u32 char_count + N×2B UTF-16LE)
///
/// One builder for the chat broadcaster and the `npc_bark` content action
/// (`cimmeria_services::cell::content::executor::bark`), so a bark speaks
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
}
