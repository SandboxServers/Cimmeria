//! What the owner sees when an owner-pet ability is refused or changes the
//! pet's state (pets PT-08).
//!
//! `onErrorCode` has no Lua consumer in the shipped client (AT-E1), so each
//! refusal also sends a `CHAN_FEEDBACK` chat line, which is what renders.
//! A toggle or a doom that succeeded gets a line too: the Ability window
//! shows no toggle state, so without it the second press of Holy Warrior
//! would look like the first (project rule: every press gets feedback).

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use super::super::super::super::messages::CellToBaseMsg;
use crate::cell::abilities::wire_ledger::{self, WireCtx};

/// `ERRORCODE_SYSTEM_Ability`, the only `EErrorCodeSystem` value.
const ERRORCODE_SYSTEM_ABILITY: u8 = 0;

/// `CONDITION_FEEDBACK_EffectMonikerOnEntity`: the pet already carries the
/// effect (To The Death pressed while it runs).
pub(super) const FEEDBACK_EFFECT_ON_ENTITY: u16 = 133;

/// The line for To The Death pressed while the pet is already doomed.
pub(super) const DOOMED_TEXT: &str = "Your pet is already fighting to the death.";

/// `onErrorCode(ERRORCODE_SYSTEM_Ability, ability_id, code)` and a
/// `CHAN_FEEDBACK` line to `owner`. Both self-only.
pub(super) async fn send_refusal(
    owner: u32,
    id: PlayerIdentity,
    ability_id: i32,
    code: u16,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let mut err = Vec::with_capacity(7);
    err.push(ERRORCODE_SYSTEM_ABILITY);
    err.extend_from_slice(&ability_id.to_le_bytes()); // InstanceID
    err.extend_from_slice(&code.to_le_bytes());
    send(
        owner,
        id,
        ability_id,
        crate::mercury::method_idx::ON_ERROR_CODE,
        err,
        tx,
    )
    .await;
    send_line(owner, id, ability_id, text, tx).await;
}

/// A `CHAN_FEEDBACK` line to `owner`.
pub(super) async fn send_line(
    owner: u32,
    id: PlayerIdentity,
    ability_id: i32,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let chat = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    send(
        owner,
        id,
        ability_id,
        crate::mercury::method_idx::ON_PLAYER_COMMUNICATION,
        chat,
        tx,
    )
    .await;
}

async fn send(
    owner: u32,
    id: PlayerIdentity,
    ability_id: i32,
    method_index: u16,
    args: Vec<u8>,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let row = wire_ledger::prepare(method_index, &args);
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: owner,
            method_index,
            args,
        })
        .await
        .is_err()
    {
        crate::cell::abilities::metrics::wire_send_failed(
            crate::cell::abilities::metrics::WireMessage::from_method(method_index),
            crate::cell::abilities::metrics::UNKNOWN_WORLD,
        );
        tracing::warn!(
            target: "pets.buff",
            event = "feedback_send_failed",
            decision_outcome = "feedback_send_failed",
            reason = "cell_to_base_closed",
            entity_id = owner,
            owner_id = owner,
            account_id = id.account_id,
            player_id = id.player_id,
            ability_id,
            method_index,
            method_name = cimmeria_wire::names::player_client_method(method_index),
            "owner-pet ability feedback could not be queued (base channel closed)"
        );
    } else {
        row.sent_to_owner_as(id, owner, WireCtx::new("owner_pet").ability(ability_id));
    }
}
