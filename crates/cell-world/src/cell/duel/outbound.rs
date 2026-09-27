//! What the duel handlers send.
//!
//! Single-recipient `EntityMethodCall`s to one player's own client: the
//! feedback lines, the challenge prompt [143], the countdown timer
//! (`onTimerUpdate`, type 14), and `onDuelEntitiesSet` [151] /
//! `onDuelEntitiesClear` [153] (SS-D2). The one fan-out is the PvP flag,
//! which goes to the duelist's own client and to every witness of the
//! duelist ([`send_pvp_flag`]).
//!
//! 151 and 153 only ever name the two duelists' entity ids: AoI's own use of
//! 152 for interactable NPCs is untouched (SS-E1 D-Q5, audit A-41).

use tokio::sync::mpsc;

use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::being::ON_STATE_FIELD_UPDATE;
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use cimmeria_wire::cell::client_methods::duel::{
    build_duel_timer, build_on_duel_challenge, build_on_duel_entities_set, build_pvp_flag,
    ON_DUEL_CHALLENGE, ON_DUEL_ENTITIES_CLEAR, ON_DUEL_ENTITIES_SET, ON_ENTITY_PROPERTY,
    ON_TIMER_UPDATE,
};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

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

/// Queue a duel feedback line to player entity `entity_id`'s own client,
/// for callers outside the duel module (the auto-cycle tick). The other
/// duelist, if any, is logged on a send failure. `false` when not queued.
pub async fn send_player_line(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &SpaceManager,
    entity_id: u32,
    other_player_id: Option<i32>,
    text: &str,
) -> bool {
    let id = mgr.player_identity(entity_id);
    let to = Recipient {
        entity_id,
        account_id: id.account_id,
        player_id: id.player_id.unwrap_or_default(),
        other_player_id,
    };
    send_line(tx, to, text, None).await
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

/// Queue the countdown to the recipient's own client: `onTimerUpdate`
/// type 14 on their own entity, `total` seconds from now in the client's
/// game clock. The client counts it down as splash numbers
/// (`Event_UI_DuelTimerStart`, `Duel.lua`).
pub(super) async fn send_countdown(
    tx: &mpsc::Sender<CellToBaseMsg>,
    to: Recipient,
    total: f32,
    duel_id: DuelId,
) -> bool {
    let complete = cimmeria_wire::mercury::game_clock::game_time_secs() + total;
    let args = build_duel_timer(duel_id as i32, to.entity_id as i32, total, complete);
    send(tx, to, ON_TIMER_UPDATE, args, Some(duel_id)).await
}

/// Queue `onDuelEntitiesSet([a, b])` [151]: the two duelists' entity ids,
/// never anything else.
pub(super) async fn send_duel_entities_set(
    tx: &mpsc::Sender<CellToBaseMsg>,
    to: Recipient,
    duelists: [u32; 2],
    duel_id: DuelId,
) -> bool {
    let args = build_on_duel_entities_set(&[duelists[0] as i32, duelists[1] as i32]);
    send(tx, to, ON_DUEL_ENTITIES_SET, args, Some(duel_id)).await
}

/// Queue `onDuelEntitiesClear()` [153]: no arguments.
pub(super) async fn send_duel_entities_clear(
    tx: &mpsc::Sender<CellToBaseMsg>,
    to: Recipient,
    duel_id: DuelId,
) -> bool {
    send(tx, to, ON_DUEL_ENTITIES_CLEAR, Vec::new(), Some(duel_id)).await
}

/// Set the recipient's PvP flag to `flagged` on their own client and on the
/// client of every player who has them in AoI:
/// `onEntityProperty(GENERICPROPERTY_PvPFlag, 0 | 1)` on the duelist's
/// entity. Presentation only: the harm gate reads the registry
/// (D-SS23). A witness that arrives later gets the current value from the
/// AoI enter path (`space_manager::aoi`). Returns the witness count.
pub(super) async fn send_pvp_flag(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &SpaceManager,
    to: Recipient,
    flagged: bool,
    duel_id: DuelId,
) -> usize {
    let args = build_pvp_flag(flagged);
    send_to_self_and_witnesses(tx, mgr, to, ON_ENTITY_PROPERTY, args, duel_id).await
}

/// Send the recipient's new `state_field` (a `BSF_InCombat` flip) to their
/// own client and their witnesses, as the mob-combat path does
/// (`damage_apply`'s `send_entity_method_to_self_and_witnesses`).
pub(super) async fn send_state_field(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &SpaceManager,
    to: Recipient,
    state_field: u32,
    duel_id: DuelId,
) -> usize {
    let args = state_field.to_le_bytes().to_vec();
    send_to_self_and_witnesses(tx, mgr, to, ON_STATE_FIELD_UPDATE, args, duel_id).await
}

async fn send_to_self_and_witnesses(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &SpaceManager,
    to: Recipient,
    method_index: u16,
    args: Vec<u8>,
    duel_id: DuelId,
) -> usize {
    send(tx, to, method_index, args.clone(), Some(duel_id)).await;
    let witnesses = mgr.get_witnesses_of(to.entity_id);
    for &witness_id in &witnesses {
        if tx
            .send(CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id: to.entity_id,
                method_index,
                args: args.clone(),
                entity_is_player: true,
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
                witness_id,
                method_index,
                duel_id,
                reason = "cell_to_base_closed",
                "duel client method could not be queued to a witness"
            );
            break;
        }
    }
    witnesses.len()
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
