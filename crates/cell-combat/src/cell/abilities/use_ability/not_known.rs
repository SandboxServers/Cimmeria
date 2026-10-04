//! The answer to a player who presses an ability they do not know.

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::PlayerIdentity;

use super::super::super::messages::CellToBaseMsg;
use crate::cell::abilities::wire_ledger::{self, WireCtx};

/// `CONDITION_FEEDBACK_EntityDoesNotHaveAbility`
/// (`entities/defs/enumerations.xml`). Exact fit: the caster does not have
/// the ability. The trainer uses the same code for a missing prerequisite
/// (AT-04) and for a respec with nothing to reset (AT-08).
const CONDITION_FEEDBACK_ENTITY_DOES_NOT_HAVE_ABILITY: u16 = 167;

/// `onErrorCode(ERRORCODE_SYSTEM_Ability, ability_id, 167)` to a player who
/// pressed an ability they do not know. The action bar is client-side, so
/// after a respec (AT-08) a button can still name a refunded ability; before
/// this the press was refused silently.
pub(super) async fn send_not_known_feedback(
    entity_id: u32,
    who: PlayerIdentity,
    ability_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let mut args = Vec::with_capacity(7);
    args.push(0u8); // SystemID: ERRORCODE_SYSTEM_Ability
    args.extend_from_slice(&ability_id.to_le_bytes()); // InstanceID
    args.extend_from_slice(&CONDITION_FEEDBACK_ENTITY_DOES_NOT_HAVE_ABILITY.to_le_bytes());
    let row = wire_ledger::prepare(crate::mercury::method_idx::ON_ERROR_CODE, &args);
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::mercury::method_idx::ON_ERROR_CODE,
            args,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            target: "abilities",
            event = "not_known_feedback_send_failed",
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id,
            ability_id,
            "useAbility: the not-known onErrorCode could not be queued (base channel closed)"
        );
    } else {
        row.sent_to_owner_as(
            who,
            entity_id,
            WireCtx::new("not_known").reason("ability_not_known"),
        );
    }
}
