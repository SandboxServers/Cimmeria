//! Organization cell methods that are not squad calls: Team and Command
//! ids, base-issued invite request ids, and the methods no packet has
//! implemented yet.
//!
//! Each is logged and answered with ORG-01's "not available yet" pair, so
//! the press is never silent. ORG-07 replaces [`answer`] with a forward to
//! the base (`OrgCellToBase::ForwardCellCall`); ORG-04 and ORG-08 take their
//! methods out of here.

use crate::cell::messages::CellToBaseMsg;
use tokio::sync::mpsc;

use cimmeria_wire::cell::cell_methods::organization::OrgCellCall;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use cimmeria_wire::cell::client_methods::organization::ORG_NOT_AVAILABLE_TEXT;
use cimmeria_wire::cell::client_methods::player::{
    build_on_error_code, CONDITION_FEEDBACK_INVALID_ENTITY, ERRORCODE_SYSTEM_ABILITY, ON_ERROR_CODE,
};

/// UTF-16 units of client text the call carries, for the log line. The text
/// itself is player-authored and is not logged.
fn text_units(call: &OrgCellCall) -> usize {
    let units = |s: &str| s.encode_utf16().count();
    match call {
        OrgCellCall::Motd { motd, .. } => units(motd),
        OrgCellCall::Note { note, .. } => units(note),
        OrgCellCall::OfficerNote { name, note, .. } => units(name) + units(note),
        OrgCellCall::SetRankName { name, .. } => units(name),
        _ => 0,
    }
}

/// Log a decoded call no handler serves yet and answer it.
pub(super) async fn answer(
    entity_id: u32,
    method_index: u16,
    call: &OrgCellCall,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    tracing::debug!(
        target: "org",
        event = "org.cell_method_unimplemented",
        entity_id,
        method_index,
        method = call.method_name(),
        org_id = call.org_id(),
        text_units = text_units(call),
        "UNIMPLEMENTED: {}",
        call.method_name()
    );
    let instance_id = call.org_id().unwrap_or(0);
    send_unavailable_feedback(entity_id, method_index, instance_id, tx).await;
}

/// Answer an organization request the server cannot serve yet, so the press
/// is not silent: `onErrorCode(ERRORCODE_SYSTEM_Ability, instance_id,
/// CONDITION_FEEDBACK_InvalidEntity)` and then a feedback chat line, the
/// same pair the base arm for 0xCF-0xD2 sends. The chat line is the part
/// the player sees: the client has no text for an organization error code
/// (ORG-E1 Q4).
pub(crate) async fn send_unavailable_feedback(
    entity_id: u32,
    method_index: u16,
    instance_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    send_error_and_line(
        entity_id,
        method_index,
        instance_id,
        ORG_NOT_AVAILABLE_TEXT,
        tx,
    )
    .await;
}

/// `onErrorCode(0, instance_id, 0)` then `text` on the feedback channel:
/// the organization rejection pair.
pub(super) async fn send_error_and_line(
    entity_id: u32,
    method_index: u16,
    instance_id: i32,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let msgs = [
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_ERROR_CODE,
            args: build_on_error_code(
                ERRORCODE_SYSTEM_ABILITY,
                instance_id,
                CONDITION_FEEDBACK_INVALID_ENTITY,
            ),
        },
        feedback_line(entity_id, text),
    ];
    for msg in msgs {
        if tx.send(msg).await.is_err() {
            tracing::warn!(
                target: "org",
                event = "org.feedback_send_failed",
                entity_id,
                method_index,
                reason = "cell_to_base_closed",
                "organization feedback could not be queued"
            );
            return;
        }
    }
}

/// `text` from `SYSTEM` on the feedback channel, as an entity-method call
/// on `entity_id`'s own player.
pub(super) fn feedback_line(entity_id: u32, text: &str) -> CellToBaseMsg {
    CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: ON_PLAYER_COMMUNICATION,
        args: serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text),
    }
}
