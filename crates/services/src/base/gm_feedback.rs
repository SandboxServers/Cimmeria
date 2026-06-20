//! Base-side GM feedback line: serialize + single-recipient
//! `onPlayerCommunication` send to the GM's own client.
//!
//! This is the *definitive* (post-commit) counterpart to the cell-side
//! `gm::feedback::send_gm_feedback`. The base-round-trip GM commands
//! (give/crafting/spawn) used to feed back *optimistically* from the cell with
//! a "requested" line, which lied whenever the base dropped the DB write. The
//! handlers in this layer call this helper only after the write actually
//! commits, so the GM sees confirmation that reflects reality ("trust, but
//! verify").
//!
//! The `onPlayerCommunication` payload serializer is duplicated from the cell
//! copy (`gm::feedback::serialize_on_player_communication`, which is private)
//! — there's precedent for duplicating small wire serializers across the
//! cell/base split rather than promoting them to a shared crate.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// GM-feedback chat channel. The 2009 client only registers the channels in the
/// base's `DEFAULT_CHAT_CHANNELS` (say/emote/yell/team/squad/command/server=7/
/// tell=9); there is **no** dedicated feedback channel. Sending on an
/// *unregistered* channel — the old value `8` — makes the client fall back to
/// its **red unknown-channel splash popup**. So GM feedback rides the registered
/// `tell` channel (`9`), the same one the inline welcome message uses. Mirrors
/// the cell-side feedback channel.
const CHAN_FEEDBACK: u8 = 9;

/// Serialize `onPlayerCommunication(Speaker, SpeakerFlags, Channel, Text)`.
///
/// Wire shape (byte-identical to the cell-side serializer):
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

    args.extend_from_slice(&(speaker_utf16.len() as u32).to_le_bytes());
    for &ch in &speaker_utf16 {
        args.extend_from_slice(&ch.to_le_bytes());
    }
    args.push(speaker_flags);
    args.push(channel);
    args.extend_from_slice(&(text_utf16.len() as u32).to_le_bytes());
    for &ch in &text_utf16 {
        args.extend_from_slice(&ch.to_le_bytes());
    }
    args
}

/// Send a single definitive GM-feedback line to the entity's own client.
///
/// Speaker is `"SYSTEM"`, flags `0`, channel `CHAN_FEEDBACK`. Mirrors how
/// `progression::handle_grant_cash` sends a client packet:
/// `send_to_witness_reliable(...)` wrapping
/// `build_player_entity_method_packet(..., ON_PLAYER_COMMUNICATION, &payload)`.
///
/// If the entity has no connected client (e.g. it disconnected between the
/// write committing and this send), `send_to_witness_reliable` no-ops and warns
/// internally — the DB write already committed, so dropping the feedback line
/// is harmless.
pub(crate) async fn send_gm_feedback_to_client(
    entity_id: u32,
    text: &str,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let payload = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_PLAYER_COMMUNICATION,
                &payload,
                version,
            )
        },
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feedback wire shape: speaker "SYSTEM" (6 UTF-16 chars), flags 0,
    /// channel 9 (CHAN_feedback), then the text WSTRING. A drift here means
    /// the GM's client renders the feedback on the wrong channel or not at
    /// all. Byte-identical to the cell-side serializer's pinned shape.
    #[test]
    fn feedback_wire_shape_is_system_on_feedback_channel() {
        let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, "hello");
        // Speaker "SYSTEM" = 6 UTF-16 chars.
        assert_eq!(u32::from_le_bytes(args[0..4].try_into().unwrap()), 6);
        let flags_off = 4 + 6 * 2;
        assert_eq!(args[flags_off], 0, "speaker flags must be 0");
        assert_eq!(
            args[flags_off + 1],
            CHAN_FEEDBACK,
            "channel must be feedback (9), not server (8)"
        );
        assert_eq!(CHAN_FEEDBACK, 9, "feedback must use CHAN_feedback=9");
        // Text WSTRING char count follows speaker + flags + channel.
        let text_len_off = flags_off + 2;
        assert_eq!(
            u32::from_le_bytes(args[text_len_off..text_len_off + 4].try_into().unwrap()),
            5,
            "\"hello\" = 5 chars"
        );
    }
}
