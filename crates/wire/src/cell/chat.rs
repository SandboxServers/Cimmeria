//! The `onPlayerCommunication` payload and the `EChannel` ids it carries.
//!
//! The chat handlers are `cimmeria_cell_console::cell::console::chat`,
//! which re-exports everything here; the `npc_bark` content action speaks through
//! the same serializer.
//!
//! Reference: `python/cell/SGWPlayer.py:processPlayerCommunication()`

// ── Channel IDs (`EChannel`, `entities/defs/enumerations.xml`) ─────────────
//
// The client compiles every built-in `UIChannel.*` id in as a literal equal
// to the enum value (D-ORG14, ORG-E1 Q5: `UIChannel.Server` is 8, `.Tell` 10),
// so these are wire constants, not server choices. They used to follow a
// drifted python copy (server 7, tell 9, splash 10); 7 is no `EChannel` at
// all, and the client's `ChatMod.ChannelMap[7]` is nil, so a line on 7 is a
// client Lua error. Every constant is pinned to the XML by
// `tests::chan_constants_match_enumerations_xml`.

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
/// Server channel — system broadcasts only. The client prints a line on it
/// in bright red **and** opens a modal "Server Message" prompt
/// (`ChatWindow.lua:160-162`), so only a broadcast meant to interrupt every
/// player belongs here (`/gmshout`, `.announce`), never routine feedback.
pub const CHAN_SERVER: u8 = 8;
/// Feedback channel — system lines to one player (GM feedback, refusals,
/// the login welcome). Rendered as an ordinary sky-blue line in the Info
/// tab, no popup. The legacy welcome (`cell/SGWPlayer.py:541`) used a
/// literal 9, which is this channel.
pub const CHAN_FEEDBACK: u8 = 9;
/// Tell channel — direct player-to-player, routed by the base
/// (`dispatch::tell`). The client's `/tell` sends this id and renders
/// incoming tells on it.
pub const CHAN_TELL: u8 = 10;
/// Splash channel — system splash text. No server path sends it yet.
pub const CHAN_SPLASH: u8 = 11;
/// The first user-created channel id ("Anything starting at CHAN_chat is a
/// user-created chat channel"). This server creates none.
pub const CHAN_CHAT: u8 = 12;

/// `ESpeakerFlags::SPEAKER_GM` (`entities/defs/enumerations.xml`): the
/// speaker is staff. Pinned to the def by
/// `tests::speaker_gm_matches_enumerations_xml`.
pub const SPEAKER_GM: u8 = 0x01;

/// The `onPlayerCommunication` args of a GM broadcast (`sendGMShout`, cell
/// method 222, and the `.announce` console command; D-SS16): the GM's name,
/// [`SPEAKER_GM`], and [`CHAN_SERVER`]. The space scope (cell) and the
/// global scope (base) both send these bytes, so the two cannot drift.
///
/// `CHAN_SERVER` is 8 (SS-C4, D-ORG14). The client shows a line on it in
/// red and opens its modal "Server Message" prompt, which is what a GM
/// shout is for. Before SS-C4 this rode 7, which the client has no
/// `ChannelMap` entry for, so the shout never displayed.
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

/// Serialize `onChatJoined(WSTRING ChannelName, UINT8 ChannelID)` args
/// (`Communicator.def`, client method 31): confirms the client joined a
/// **user** channel, never a built-in one (SS-C4, D-ORG14 -- the client
/// hardcodes 0-11 and needs no join for them).
///
/// `channel_id` here is the **display id**, `wire_id - CHAN_CHAT`
/// (`Constants.MIN_USER_CHANNEL` in the legacy `SGWPlayer.py::onChannelJoined`):
/// the client's `ChatMod.onChannelJoined` re-derives the wire channel as
/// `UIChannel.Chat + displayId` and treats every call as a user channel, so
/// sending the raw wire id here would double-offset it and both misfile
/// the channel and print the wrong id in the "You have joined channel"
/// line (`ChatWindow.lua:370-386`).
///
/// The client shows this call's own text as the "you joined" feedback
/// (`ChatWindow.lua`); the caller must not also send a separate feedback
/// line for a successful join, or the player sees the line twice.
///
/// Wire format: WSTRING (channel name) then one UINT8 (display id).
pub fn serialize_on_chat_joined(channel_name: &str, display_id: u8) -> Vec<u8> {
    let name_utf16: Vec<u16> = channel_name.encode_utf16().collect();
    let mut args = Vec::with_capacity(4 + name_utf16.len() * 2 + 1);
    args.extend_from_slice(&(name_utf16.len() as u32).to_le_bytes());
    for &ch in &name_utf16 {
        args.extend_from_slice(&ch.to_le_bytes());
    }
    args.push(display_id);
    args
}

/// Serialize `onChatLeft(WSTRING ChannelName)` args (`Communicator.def`,
/// client method 32): confirms the client left a **user** channel. Like
/// [`serialize_on_chat_joined`], the client shows its own "you left
/// channel" feedback from this call and removes the channel's tab
/// subscriptions (`ChatWindow.lua::onChannelLeft`); the caller must not
/// also send a feedback line for a successful leave.
///
/// Wire format: one WSTRING (channel name).
pub fn serialize_on_chat_left(channel_name: &str) -> Vec<u8> {
    let name_utf16: Vec<u16> = channel_name.encode_utf16().collect();
    let mut args = Vec::with_capacity(4 + name_utf16.len() * 2);
    args.extend_from_slice(&(name_utf16.len() as u32).to_le_bytes());
    for &ch in &name_utf16 {
        args.extend_from_slice(&ch.to_le_bytes());
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

    /// Byte-exact GM broadcast line (SS-C2, SS-C4): speaker "Gm" (2 units),
    /// SPEAKER_GM, `CHAN_server` = 8, text "Hi!" (3 units). A drift in the
    /// flag shows the shout as an ordinary player line; a drift in the
    /// channel byte (the old 7) makes the client call a nil `ChannelMap`
    /// entry and display nothing.
    #[test]
    fn gm_broadcast_bytes_are_exact() {
        let args = serialize_gm_broadcast("Gm", "Hi!");
        let expected: Vec<u8> = vec![
            0x02, 0x00, 0x00, 0x00, // speaker char count
            b'G', 0x00, b'm', 0x00, // "Gm" UTF-16LE
            0x01, // SPEAKER_GM
            0x08, // CHAN_server
            0x03, 0x00, 0x00, 0x00, // text char count
            b'H', 0x00, b'i', 0x00, b'!', 0x00, // "Hi!" UTF-16LE
        ];
        assert_eq!(args, expected);
    }

    /// `onChatJoined("chat", 3)`: WSTRING "chat" (4 units) then one UINT8
    /// display id. A drift here either misfiles the channel client-side
    /// (wrong id) or corrupts the next message in the bundle (wrong length).
    #[test]
    fn serialize_on_chat_joined_is_exact() {
        let args = serialize_on_chat_joined("chat", 3);
        let expected: Vec<u8> = vec![
            0x04, 0x00, 0x00, 0x00, // "chat" char count
            b'c', 0x00, b'h', 0x00, b'a', 0x00, b't', 0x00, // "chat" UTF-16LE
            0x03, // display id
        ];
        assert_eq!(args, expected);
    }

    #[test]
    fn serialize_on_chat_joined_empty_name() {
        assert_eq!(
            serialize_on_chat_joined("", 0),
            vec![0x00, 0x00, 0x00, 0x00, 0x00],
            "empty name still carries the length prefix and the id byte"
        );
    }

    /// `onChatLeft("chat")`: one WSTRING, no trailing byte -- unlike
    /// `onChatJoined`, there is no id to confuse this with.
    #[test]
    fn serialize_on_chat_left_is_exact() {
        let args = serialize_on_chat_left("chat");
        let expected: Vec<u8> = vec![
            0x04, 0x00, 0x00, 0x00, // "chat" char count
            b'c', 0x00, b'h', 0x00, b'a', 0x00, b't', 0x00, // "chat" UTF-16LE
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

    /// Read `EChannel` out of the def and return `(name, value)` pairs in
    /// file order. Parsed, not copied, so a constant and a test cannot
    /// drift together.
    fn echannel_tokens_from_xml() -> Vec<(String, u8)> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../entities/defs/enumerations.xml");
        let xml = std::fs::read_to_string(&path).expect("read enumerations.xml");
        let start = xml.find("<EChannel>").expect("EChannel block");
        let end = start + xml[start..].find("</EChannel>").expect("close tag");
        xml[start..end]
            .split("<Token>")
            .skip(1)
            .map(|t| {
                let name = &t[t.find("<Name>").unwrap() + 6..t.find("</Name>").unwrap()];
                let v = &t[t.find("<Value>").unwrap() + 7..t.find("</Value>").unwrap()];
                (name.trim().to_string(), v.trim().parse::<u8>().unwrap())
            })
            .collect()
    }

    /// Every `CHAN_*` constant equals its `EChannel` token (D-ORG14, SS-C4),
    /// and every token has a constant. The client hardcodes the same
    /// literals, so a constant that drifts from the XML sends a line the
    /// client files under another channel, or under none.
    #[test]
    fn chan_constants_match_enumerations_xml() {
        let rust: &[(&str, u8)] = &[
            ("CHAN_say", CHAN_SAY),
            ("CHAN_emote", CHAN_EMOTE),
            ("CHAN_yell", CHAN_YELL),
            ("CHAN_team", CHAN_TEAM),
            ("CHAN_squad", CHAN_SQUAD),
            ("CHAN_command", CHAN_COMMAND),
            ("CHAN_officer", CHAN_OFFICER),
            ("CHAN_server", CHAN_SERVER),
            ("CHAN_feedback", CHAN_FEEDBACK),
            ("CHAN_tell", CHAN_TELL),
            ("CHAN_splash", CHAN_SPLASH),
            ("CHAN_chat", CHAN_CHAT),
        ];
        let xml = echannel_tokens_from_xml();
        for (name, value) in &xml {
            let (_, ours) = rust
                .iter()
                .find(|(n, _)| n == name)
                .unwrap_or_else(|| panic!("EChannel token {name} has no CHAN_* constant"));
            assert_eq!(ours, value, "{name}: Rust constant vs enumerations.xml");
        }
        assert_eq!(
            rust.len(),
            xml.len(),
            "a CHAN_* constant names no EChannel token"
        );
    }

    /// No `CHAN_*` constant is 7: `EChannel` skips it, and the client has
    /// no `ChannelMap` entry for it (`ChatWindow.lua:1297-1312`), so a line
    /// on 7 is a client Lua error. The id used to be the server channel.
    #[test]
    fn no_chan_constant_is_seven() {
        for c in [
            CHAN_SAY,
            CHAN_EMOTE,
            CHAN_YELL,
            CHAN_TEAM,
            CHAN_SQUAD,
            CHAN_COMMAND,
            CHAN_OFFICER,
            CHAN_SERVER,
            CHAN_FEEDBACK,
            CHAN_TELL,
            CHAN_SPLASH,
            CHAN_CHAT,
        ] {
            assert_ne!(c, 7, "7 is not an EChannel id");
        }
        assert!(
            echannel_tokens_from_xml().iter().all(|(_, v)| *v != 7),
            "enumerations.xml still skips 7"
        );
    }

    /// The top-level, comma-separated arguments of the call whose `(` is at
    /// `open`, skipping string and char literals and line comments so a `,`
    /// or `)` inside them does not split an argument. `None` if the call
    /// does not close.
    fn call_args(src: &str, open: usize) -> Option<Vec<String>> {
        let bytes = src.as_bytes();
        let (mut depth, mut i, mut arg_start) = (0usize, open, open + 1);
        let mut args = Vec::new();
        while i < bytes.len() {
            match bytes[i] {
                b'"' => {
                    i += 1;
                    while i < bytes.len() && bytes[i] != b'"' {
                        i += if bytes[i] == b'\\' { 2 } else { 1 };
                    }
                }
                // A char literal (`'('`, `'\''`); a lifetime has no closing quote.
                b'\'' if bytes.get(i + 2) == Some(&b'\'') => i += 2,
                b'\'' if bytes.get(i + 1) == Some(&b'\\') => {
                    i += 2;
                    while i < bytes.len() && bytes[i] != b'\'' {
                        i += 1;
                    }
                }
                b'/' if bytes.get(i + 1) == Some(&b'/') => {
                    while i < bytes.len() && bytes[i] != b'\n' {
                        i += 1;
                    }
                }
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        args.push(src[arg_start..i].trim().to_string());
                        return Some(args);
                    }
                }
                b',' if depth == 1 => {
                    args.push(src[arg_start..i].trim().to_string());
                    arg_start = i + 1;
                }
                _ => {}
            }
            i += 1;
        }
        None
    }

    fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read_dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                rust_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// No server->client `onPlayerCommunication` names its channel with a
    /// number (SS-C4). Every call of the one serializer across the
    /// workspace passes a `CHAN_*` constant or a variable, never a literal,
    /// so the channel byte always comes from the constants
    /// `no_chan_constant_is_seven` and `chan_constants_match_enumerations_xml`
    /// pin. With both, nothing can send the old server id 7. The scan finds
    /// the serializer's call sites itself; the floor on their count keeps it
    /// from passing by finding none.
    #[test]
    fn no_player_communication_call_uses_a_literal_channel() {
        let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut files = Vec::new();
        rust_files(&crates, &mut files);
        let needle = "serialize_on_player_communication(";
        let mut calls = 0;
        let mut literal = Vec::new();
        let mut unparsed = Vec::new();
        for file in &files {
            let src = std::fs::read_to_string(file).expect("read source");
            for (at, _) in src.match_indices(needle) {
                let line_start = src[..at].rfind('\n').map_or(0, |n| n + 1);
                if src[..at].ends_with("fn ") || src[line_start..at].contains("//") {
                    continue; // the definition, or a mention in a comment
                }
                let Some(args) = call_args(&src, at + needle.len() - 1) else {
                    unparsed.push(format!("{}: byte {at}", file.display()));
                    continue;
                };
                if args.len() != 4 {
                    continue; // a `use` list or a doc mention, not a call
                }
                calls += 1;
                let channel = args[2].trim_end_matches("u8").trim();
                if !channel.is_empty()
                    && channel
                        .chars()
                        .all(|c| c.is_ascii_hexdigit() || c == 'x' || c == '_')
                    && channel.starts_with(|c: char| c.is_ascii_digit())
                {
                    literal.push(format!("{}: channel `{}`", file.display(), args[2]));
                }
            }
        }
        assert!(
            unparsed.is_empty(),
            "calls the scan could not parse: {unparsed:#?}"
        );
        assert!(
            calls >= 20,
            "found only {calls} serializer calls; the scan is broken"
        );
        assert!(
            literal.is_empty(),
            "onPlayerCommunication with a literal channel byte, use a CHAN_* constant:\n{}",
            literal.join("\n")
        );
    }
}
