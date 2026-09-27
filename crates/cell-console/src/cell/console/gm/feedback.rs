//! GM feedback line: serialize + single-recipient `onPlayerCommunication` send.
//!
//! The native query commands (`gmUsers`, `testLOS`, and the SHOW/LIST family
//! in the ADAPT roadmap) produce text that must reach **one** GM client, not
//! the AoI. This is the single-recipient delivery path: an `EntityMethodCall`
//! to the GM's own client carrying `onPlayerCommunication` on the
//! `CHAN_FEEDBACK` chat channel. It is cell-local — no base round-trip beyond
//! the normal client-method relay.

use tokio::sync::mpsc;

use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use crate::cell::messages::CellToBaseMsg;
use crate::mercury::method_idx::ON_PLAYER_COMMUNICATION;

/// Send a single feedback line to the GM only (no witness fan-out).
///
/// Speaker is `"SYSTEM"`, flags `0`, channel `CHAN_FEEDBACK` (9), the
/// client's feedback channel: an ordinary Info-tab line. Not the server
/// channel (8), which the client shows as a modal "Server Message" prompt
/// (`ChatWindow.lua:160-162`); that prompt is what earlier comments here
/// called the "red unknown-channel splash popup".
pub async fn send_gm_feedback(caller_entity_id: u32, text: &str, tx: &mpsc::Sender<CellToBaseMsg>) {
    // The only record of what a `.`-command told the GM (`.location`'s
    // position, `.searchmission`'s hits, every rejection reason).
    tracing::debug!(
        target: "console.feedback",
        entity_id = caller_entity_id,
        text_len = text.chars().count(),
        text = %text.chars().take(400).collect::<String>(),
        "GM console feedback sent to client"
    );
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
            9,
            "channel must be CHAN_feedback (9), not server (8) or tell (10)"
        );
        // Text WSTRING char count follows speaker + flags + channel.
        let text_len_off = flags_off + 2;
        assert_eq!(
            u32::from_le_bytes(args[text_len_off..text_len_off + 4].try_into().unwrap()),
            5,
            "\"hello\" = 5 chars"
        );
    }
}
