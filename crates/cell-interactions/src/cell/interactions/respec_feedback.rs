//! Feedback for a refused respec (`resetMyAbilities`, AT-08): `onErrorCode`,
//! then the pinned trainer's `onTrainerOpen` re-send.
//!
//! Project rule: every button press gets visible feedback on the first
//! press. As for a refused `trainAbility` (D-AT08), the error code cannot be
//! relied on alone: no client Lua consumes `onErrorCode`, and whether a
//! native listener renders it is unresolved (AT-E1 §2). The trainer re-send
//! redraws the window from server truth, so it is the feedback the player is
//! known to see whenever a trainer is pinned.
//!
//! Two callers share it: the cell's own gate on `resetMyAbilities` (not at a
//! trainer, nothing trainer-bought) and the base's refusal
//! (`RespecOutcome::NotEnoughNaquadah`, or `NothingToReset` when the base
//! saw what the cell's mirror did not).

use tokio::sync::mpsc;

use super::trainer_authority::resend_pinned_trainer;
use crate::ability_tree::respec_error_code_args;
use crate::cell::client_methods::player::ON_ERROR_CODE;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Send `onErrorCode(code)` for a refused respec, then re-send the pinned
/// trainer (when the pin is a live trainer). `reason` is the log label.
pub async fn send_respec_rejection(
    entity_id: u32,
    code: u16,
    reason: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_ERROR_CODE,
            args: respec_error_code_args(code),
        })
        .await
    {
        tracing::warn!(
            target: "abilities",
            event = "respec_feedback_send_failed",
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            reason,
            error_code = code,
            error_name = cimmeria_names::book().error_code(code),
            error = %e,
            "resetMyAbilities: rejection onErrorCode could not be queued (base channel closed)"
        );
    }
    resend_pinned_trainer(entity_id, tx, space_mgr).await;
}
