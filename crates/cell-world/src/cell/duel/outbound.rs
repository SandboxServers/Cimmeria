//! What the duel handlers send: feedback lines and the challenge prompt.
//!
//! Both are single-recipient `EntityMethodCall`s to one player's own client.
//! Neither is `onDuelEntitiesSet` [151] or `Clear` [153], which the server
//! must not send before SS-D2 (D-SS25, audit A-41).

use tokio::sync::mpsc;

use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use cimmeria_wire::cell::client_methods::duel::{build_on_duel_challenge, ON_DUEL_CHALLENGE};

use crate::cell::messages::CellToBaseMsg;

use super::registry::DuelId;

/// Queue one `onPlayerCommunication("SYSTEM", 0, CHAN_FEEDBACK, text)` to
/// `entity_id`'s own client.
pub(super) async fn send_line(
    tx: &mpsc::Sender<CellToBaseMsg>,
    entity_id: u32,
    text: &str,
    duel_id: Option<DuelId>,
) {
    let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    send(tx, entity_id, ON_PLAYER_COMMUNICATION, args, duel_id).await;
}

/// Queue `onDuelChallenge(challenger, [])` [143] to the target: the client's
/// Yes/No prompt. The squad list is empty, squad duels being refused.
pub(super) async fn send_challenge_prompt(
    tx: &mpsc::Sender<CellToBaseMsg>,
    target_entity_id: u32,
    challenger_entity_id: u32,
    duel_id: DuelId,
) {
    let args = build_on_duel_challenge(challenger_entity_id as i32, &[]);
    send(tx, target_entity_id, ON_DUEL_CHALLENGE, args, Some(duel_id)).await;
}

async fn send(
    tx: &mpsc::Sender<CellToBaseMsg>,
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    duel_id: Option<DuelId>,
) {
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            target: "duel",
            event = "duel.send_failed",
            entity_id,
            method_index,
            duel_id,
            reason = "cell_to_base_closed",
            "duel client method could not be queued to the base"
        );
    }
}
