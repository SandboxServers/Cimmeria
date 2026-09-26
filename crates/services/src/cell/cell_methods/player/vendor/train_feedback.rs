//! Feedback for a rejected `trainAbility` (D-AT08): `onErrorCode`, then an
//! `onTrainerOpen` re-send when a trainer is pinned.
//!
//! Project rule: every button press gets visible feedback on the first
//! press. The error code alone cannot be relied on: AT-E1 found no Lua
//! consumer of `onErrorCode` anywhere in the client, and whether a native
//! listener renders it is unresolved
//! (`docs/reverse-engineering/findings/ability-trainer-ui.md` §2). The
//! trainer re-send is the feedback the player is known to see: the window
//! redraws every node's trainable state from current server truth, so a
//! stale enabled button goes grey.
//!
//! A replayed purchase of a known ability ([`TrainReject::AlreadyKnown`])
//! stays silent: the client already renders a known node as `AbilityKnown`,
//! so only a double-click race or a replay reaches it.

use tokio::sync::mpsc;

use crate::ability_tree::TrainReject;
use crate::cell::client_methods::player::ON_ERROR_CODE;
use crate::cell::interactions::resend_pinned_trainer;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `EErrorCodeSystem::ERRORCODE_SYSTEM_Ability`, the only system the enum
/// defines. The client reads `InstanceID` as an ability id under it.
const ERRORCODE_SYSTEM_ABILITY: u8 = 0;

// `EConditionHandlerFeedback` values (`entities/defs/enumerations.xml`),
// chosen per the AT-E1 mapping (`worknotes/at-e1.md`).

/// `CONDITION_FEEDBACK_NotSpecifiedArchetype`. Exact fit.
pub(super) const NOT_SPECIFIED_ARCHETYPE: u16 = 6;
/// `CONDITION_FEEDBACK_LevelGreaterThanOrEqual`. Close fit: names the
/// "level >= N" comparator the unlock check applies.
pub(super) const LEVEL_GREATER_THAN_OR_EQUAL: u16 = 9;
/// `CONDITION_FEEDBACK_StatValueLessThan`. A documented REUSE, not a
/// recovered value: the 2009 enum has no token for training points or tree
/// spend, so `NotEnoughPoints` and `SpendGate` (AT-03) borrow the generic
/// "stat below threshold" comparator (AT-E1).
pub(super) const STAT_VALUE_LESS_THAN: u16 = 35;
/// `CONDITION_FEEDBACK_OutsideDistanceCheck`. Close fit: the generic
/// proximity failure, used for every trainer-authority rejection (no pin,
/// despawned, not a trainer, not offered here, out of range). The player's
/// remedy is the same for all five: go to a trainer that teaches it.
pub(super) const OUTSIDE_DISTANCE_CHECK: u16 = 43;
/// `CONDITION_FEEDBACK_EntityDoesNotHaveAbility`. Exact fit.
pub(super) const ENTITY_DOES_NOT_HAVE_ABILITY: u16 = 167;

/// The `ErrorCodeID` for a rejection, or `None` when no code applies.
///
/// `None` for `AlreadyKnown`, which stays silent, and for the three
/// resolution failures a legitimate client cannot produce (an ability id
/// with no definition, an entity that is not a loaded character). AT-E1
/// names no code for those, they are already logged at WARN, and answering
/// them would hand a forging client a free `onTrainerOpen` per packet.
///
/// Exhaustive on purpose: a new `TrainReject` variant does not compile
/// until someone decides what the player sees.
pub(super) fn error_code(reject: &TrainReject) -> Option<u16> {
    match reject {
        TrainReject::AlreadyKnown
        | TrainReject::UnknownAbility
        | TrainReject::NoPlayerId
        | TrainReject::NoArchetype => None,
        TrainReject::NotInArchetypeTree => Some(NOT_SPECIFIED_ARCHETYPE),
        TrainReject::LevelTooLow { .. } => Some(LEVEL_GREATER_THAN_OR_EQUAL),
        TrainReject::MissingPrerequisite { .. } => Some(ENTITY_DOES_NOT_HAVE_ABILITY),
        TrainReject::SpendGate { .. } | TrainReject::NotEnoughPoints { .. } => {
            Some(STAT_VALUE_LESS_THAN)
        }
        TrainReject::NoTrainerPinned
        | TrainReject::TrainerDespawned
        | TrainReject::PinNotATrainer
        | TrainReject::NotOfferedByTrainer
        | TrainReject::TrainerOutOfRange => Some(OUTSIDE_DISTANCE_CHECK),
    }
}

/// The seven `onErrorCode` argument bytes:
/// `UINT8 SystemID, INT32 InstanceID, UINT16 ErrorCodeID`, little-endian.
/// `InstanceID` is the ability the player tried to train.
pub(super) fn error_code_args(ability_id: i32, code: u16) -> Vec<u8> {
    let mut args = Vec::with_capacity(7);
    args.push(ERRORCODE_SYSTEM_ABILITY);
    args.extend_from_slice(&ability_id.to_le_bytes());
    args.extend_from_slice(&code.to_le_bytes());
    args
}

/// Send the feedback for `reject`: the error code, then the pinned trainer's
/// `onTrainerOpen`. Silent when [`error_code`] is `None`.
pub(super) async fn send_reject_feedback(
    entity_id: u32,
    ability_id: i32,
    reject: &TrainReject,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    // No code means either the silent duplicate or a rejection a legitimate
    // client cannot produce; neither earns the trainer re-send, which costs a
    // full `onTrainerOpen` build per packet.
    let Some(code) = error_code(reject) else {
        return;
    };
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_ERROR_CODE,
            args: error_code_args(ability_id, code),
        })
        .await
    {
        tracing::warn!(
            target: "abilities",
            event = "train_feedback_send_failed",
            entity_id,
            ability_id,
            reason = reject.reason(),
            error_code = code,
            error = %e,
            "trainAbility: rejection onErrorCode could not be queued (base channel closed)"
        );
    }
    resend_pinned_trainer(entity_id, tx, space_mgr).await;
}
