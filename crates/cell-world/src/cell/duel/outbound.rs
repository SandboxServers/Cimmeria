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
use super::PlayerAt;

/// Who a duel send goes to, with the identity its failure log needs
/// (instrumentation discipline rule 5): the recipient's entity, account and
/// player, and the other duelist, if there is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Recipient {
    pub entity_id: u32,
    pub account_id: Option<u32>,
    pub player_id: i32,
    /// The other duelist's `player_id`, logged as `target_player_id`.
    pub other_player_id: Option<i32>,
}

impl Recipient {
    /// `player`, playing `player_id`, with `other` as the other duelist.
    pub(super) fn at(player: &PlayerAt, player_id: i32, other: Option<i32>) -> Self {
        Recipient {
            entity_id: player.entity_id,
            account_id: player.account_id,
            player_id,
            other_player_id: other,
        }
    }
}

/// Queue one `onPlayerCommunication("SYSTEM", 0, CHAN_FEEDBACK, text)` to
/// the recipient's own client. `false` when it could not be queued (already
/// logged as `duel.send_failed`).
pub(super) async fn send_line(
    tx: &mpsc::Sender<CellToBaseMsg>,
    to: Recipient,
    text: &str,
    duel_id: Option<DuelId>,
) -> bool {
    let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    send(tx, to, ON_PLAYER_COMMUNICATION, args, duel_id).await
}

/// Queue `onDuelChallenge(challenger, [])` [143] to the target: the client's
/// Yes/No prompt. The squad list is empty, squad duels being refused.
/// `false` when it could not be queued (already logged).
pub(super) async fn send_challenge_prompt(
    tx: &mpsc::Sender<CellToBaseMsg>,
    to: Recipient,
    challenger_entity_id: u32,
    duel_id: DuelId,
) -> bool {
    let args = build_on_duel_challenge(challenger_entity_id as i32, &[]);
    send(tx, to, ON_DUEL_CHALLENGE, args, Some(duel_id)).await
}

async fn send(
    tx: &mpsc::Sender<CellToBaseMsg>,
    to: Recipient,
    method_index: u16,
    args: Vec<u8>,
    duel_id: Option<DuelId>,
) -> bool {
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: to.entity_id,
            method_index,
            args,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            target: "duel",
            event = "duel.send_failed",
            account_id = to.account_id,
            player_id = to.player_id,
            entity_id = to.entity_id,
            target_player_id = to.other_player_id,
            method_index,
            duel_id,
            reason = "cell_to_base_closed",
            "duel client method could not be queued to the base"
        );
        return false;
    }
    true
}
