//! GM feedback line: serialize + single-recipient `onPlayerCommunication` send.
//!
//! The native query commands (`gmUsers`, `testLOS`, and the SHOW/LIST family
//! in the ADAPT roadmap) produce text that must reach **one** GM client, not
//! the AoI. This is the single-recipient delivery path: an `EntityMethodCall`
//! to the GM's own client carrying `onPlayerCommunication` on the
//! `CHAN_FEEDBACK` chat channel. It is cell-local — no base round-trip beyond
//! the normal client-method relay.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::mercury::method_idx::ON_PLAYER_COMMUNICATION;

/// GM-feedback chat channel. The 2009 client only registers the channels in the
/// base's `DEFAULT_CHAT_CHANNELS` (say/emote/yell/team/squad/command/server=7/
/// tell=9); there is **no** dedicated feedback channel. Sending on an
/// *unregistered* channel — the old value `8` — makes the client fall back to
/// its **red unknown-channel splash popup**. So GM feedback rides the registered
/// `tell` channel (`9`), the same one the base's inline welcome message uses; it
/// renders as a normal, non-error, non-popup line.
const CHAN_FEEDBACK: u8 = 9;

/// Serialize `onPlayerCommunication(Speaker, SpeakerFlags, Channel, Text)`.
///
/// Wire shape (byte-identical to the chat broadcaster's serializer):
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

/// Send a single feedback line to the GM only (no witness fan-out).
///
/// Speaker is `"SYSTEM"`, flags `0`, channel `CHAN_FEEDBACK`.
pub(crate) async fn send_gm_feedback(
    caller_entity_id: u32,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: caller_entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args,
        })
        .await
    {
        tracing::warn!(
            caller_entity_id,
            error = %e,
            "GM feedback send to base failed — GM won't see the result line"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feedback wire shape: speaker "SYSTEM" (6 UTF-16 chars), flags 0,
    /// channel 9 (CHAN_feedback), then the text WSTRING. A drift here means the
    /// GM's client renders the feedback on the wrong channel or not at all.
    #[tokio::test]
    async fn feedback_wire_shape_is_system_on_feedback_channel() {
        let (tx, mut rx) = mpsc::channel(4);
        send_gm_feedback(7, "hello", &tx).await;
        let msg = rx
            .try_recv()
            .expect("feedback must emit an EntityMethodCall");
        let args = match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => {
                assert_eq!(entity_id, 7);
                assert_eq!(method_index, ON_PLAYER_COMMUNICATION);
                args
            }
            other => panic!("expected EntityMethodCall, got {other:?}"),
        };
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
