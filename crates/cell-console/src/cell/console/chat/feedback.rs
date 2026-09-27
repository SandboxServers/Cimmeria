//! Single-recipient feedback lines for chat the cell refuses to distribute.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;

use super::{serialize_on_player_communication, CHAN_FEEDBACK, ON_PLAYER_COMMUNICATION};

/// Send a single-recipient system feedback line on `CHAN_FEEDBACK` (9) --
/// the same "SYSTEM" speaker shape used by the GM `.`-console
/// (`console::gm::feedback::send_gm_feedback`), so an ordinary (non-GM)
/// player's rejected chat message gets the same treatment: an ordinary line
/// in the client's Info tab, never the modal "Server Message" prompt the
/// client opens for the server channel (8).
pub(super) async fn send_channel_feedback(
    entity_id: u32,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args,
        })
        .await;
}
