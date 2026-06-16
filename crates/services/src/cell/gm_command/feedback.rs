//! GM feedback line: serialize + single-recipient `onPlayerCommunication` send.

use tokio::sync::mpsc;

use super::CellToBaseMsg;

/// `onPlayerCommunication` flat ClientMethod index for SGWPlayer (28).
/// Canonical constant lives in `crate::mercury::method_idx`; the chat
/// broadcaster (`cell::chat`) uses the same index. Feedback is addressed to a
/// single entity (the GM), not fanned out to witnesses.
const ON_PLAYER_COMMUNICATION: u16 = crate::mercury::method_idx::ON_PLAYER_COMMUNICATION;

/// Feedback chat channel id (`EChannel.CHAN_FEEDBACK = 8`, from
/// `python/Atrea/enums.py`). Mirrors [`crate::cell::chat::CHAN_FEEDBACK`].
const CHAN_FEEDBACK: u8 = 8;

/// Serialize `onPlayerCommunication(Speaker, SpeakerFlags, Channel, Text)`.
///
/// Byte-for-byte identical to `cell::chat::serialize_on_player_communication`:
/// - Speaker: WSTRING (u32 char_count + N×2B UTF-16LE)
/// - SpeakerFlags: UINT8
/// - Channel: UINT8
/// - Text: WSTRING (u32 char_count + N×2B UTF-16LE)
///
/// Duplicated here (rather than re-exported) because the chat module's copy is
/// private and the two callers serialize for different reasons (broadcast vs.
/// single-recipient feedback); the wire shape is pinned by tests in both
/// modules so a drift in one trips its own guard.
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
pub async fn send_gm_feedback(caller_entity_id: u32, text: &str, tx: &mpsc::Sender<CellToBaseMsg>) {
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
            "gm_command: feedback send to base failed — GM won't see the result line"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests_common::*;
    use super::*;

    /// Feedback is byte-identical to the chat module's serializer: speaker
    /// "SYSTEM", flags 0, channel 8 (CHAN_FEEDBACK), then the text WSTRING.
    #[tokio::test]
    async fn feedback_wire_shape_is_system_on_feedback_channel() {
        let (tx, mut rx) = mpsc::channel(4);
        send_gm_feedback(7, "hello", &tx).await;
        let msgs = drain(&mut rx);
        let args = match &msgs[0] {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => {
                assert_eq!(*entity_id, 7);
                assert_eq!(*method_index, ON_PLAYER_COMMUNICATION);
                args.clone()
            }
            _ => panic!("expected EntityMethodCall"),
        };
        // Speaker "SYSTEM" = 6 UTF-16 chars.
        assert_eq!(u32::from_le_bytes(args[0..4].try_into().unwrap()), 6);
        let flags_off = 4 + 6 * 2;
        assert_eq!(args[flags_off], 0, "speaker flags must be 0");
        assert_eq!(
            args[flags_off + 1],
            CHAN_FEEDBACK,
            "channel must be feedback (8)"
        );
        assert_eq!(decode_feedback_text(&args), "hello");
    }
}
