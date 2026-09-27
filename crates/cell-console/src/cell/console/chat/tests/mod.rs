//! Tests for the cell chat distribution, split by path: spatial broadcast,
//! the GM `.`-console interception, and the refusal feedback lines.

mod dot_command;
mod feedback;
mod spatial;

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
