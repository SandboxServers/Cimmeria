//! Organization cell methods that are not squad calls: Team and Command
//! ids and base-issued invite request ids, which go to the base
//! ([`to_base`], [`transfer_cash_to_base`]); the strike-team and PvP-leave
//! responses nothing ever asked for ([`unsolicited`]); and squad-range ids
//! on methods squads do not have, answered with ORG-01's "not available
//! yet" pair ([`answer`]).
//!
//! Every routing decision logs DEBUG `org.forward` with `route` (`squad`,
//! `base` or `rejected`); a closed base channel is WARN
//! `org.forward_failed`.

use crate::cell::messages::{CellToBaseMsg, OrgCellToBase};
use crate::cell::org_creation::count_org_action;
use crate::cell::space_manager::SpaceManager;

use super::feedback_line;
use tokio::sync::mpsc;

use cimmeria_entity::organization::CashDir;

use cimmeria_wire::cell::cell_methods::organization::OrgCellCall;
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
        route = "rejected",
        reason = "not_available",
        entity_id,
        method_index,
        method_name = cimmeria_wire::names::player_cell_method(method_index),
        method = call.method_name(),
        org_id = call.org_id(),
        text_units = text_units(call),
        "UNIMPLEMENTED: {}",
        call.method_name()
    );
    let instance_id = call.org_id().unwrap_or(0);
    send_unavailable_feedback(entity_id, method_index, instance_id, tx).await;
}

/// Log the router's `route = squad` decision (the squad handler logs the
/// outcome).
pub(super) fn log_squad_route(entity_id: u32, method_index: u16, org_id: Option<i32>) {
    tracing::debug!(
        target: "org",
        event = "org.forward",
        route = "squad",
        entity_id,
        method_index,
        method_name = cimmeria_wire::names::player_cell_method(method_index),
        org_id,
        "organization cell method routed to the squad handler"
    );
}

/// Forward a Team or Command cell method (CM 8 with a base request id, CM
/// 9, 10 and 13-17 with a base org id) to the base, which owns persistent
/// organizations (D-ORG04, D-ORG05), as the raw method index and argument
/// bytes. The actor is this entity's own character, never the payload; the
/// base re-checks it against the session and authorizes under ORG-LOCK.
///
/// Logs DEBUG `org.forward` with `route = base`. An entity with no
/// character is answered with the refusal pair and `route = rejected`,
/// `reason = not_a_player`; a closed base channel is WARN
/// `org.forward_failed`.
pub(super) async fn to_base(
    entity_id: u32,
    method_index: u16,
    org_id: Option<i32>,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    let Some(player_id) = id.player_id else {
        not_a_player(entity_id, method_index, org_id, id.account_id, tx).await;
        return;
    };
    let msg = CellToBaseMsg::Org(OrgCellToBase::ForwardCellCall {
        player_id,
        entity_id,
        method_index,
        args: args.to_vec(),
    });
    send_to_base(
        msg,
        entity_id,
        method_index,
        org_id,
        id.account_id,
        player_id,
        tx,
    )
    .await;
}

/// Forward `organizationTransferCash` (CM 19) for a Team or Command id as
/// `OrgCellToBase::TransferCash`, the Bank campaign's route (ORG-API). The
/// base answers it; until the Bank's BV-08 lands, with a refusal.
pub(super) async fn transfer_cash_to_base(
    entity_id: u32,
    org_id: i32,
    dir: CashDir,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    let Some(player_id) = id.player_id else {
        not_a_player(
            entity_id,
            super::TRANSFER_CASH,
            Some(org_id),
            id.account_id,
            tx,
        )
        .await;
        return;
    };
    let msg = CellToBaseMsg::Org(OrgCellToBase::TransferCash {
        player_id,
        entity_id,
        org_id,
        dir,
    });
    send_to_base(
        msg,
        entity_id,
        super::TRANSFER_CASH,
        Some(org_id),
        id.account_id,
        player_id,
        tx,
    )
    .await;
}

/// The line a zero `organizationTransferCash` amount gets.
pub(crate) const ZERO_CASH_TEXT: &str = "Enter an amount of naquadah to transfer.";

/// `organizationTransferCash` (CM 19) with `aCash = 0`: nothing to move.
/// The client never sends it (`Team.lua` / `Command.lua` check `cashAmt >
/// 0`), so it is a forged or corrupted call, but the press is answered all
/// the same: WARN `org_cash_rejected reason=zero_amount` on `bank` (the
/// base's treasury events share the target) and the refusal pair with a
/// line. Never forwarded.
pub(super) async fn zero_cash(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    // The decode read the organization id before it refused the amount.
    let org_id = args
        .get(..4)
        .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .unwrap_or(0);
    let id = space_mgr.player_identity(entity_id);
    tracing::warn!(
        target: "bank",
        event = "org_cash_rejected",
        account_id = id.account_id,
        player_id = id.player_id,
        entity_id,
        org_id,
        amount = 0,
        reason = "zero_amount",
        "org_cash_rejected: a transfer of zero naquadah -- nothing moved, the player sees a line"
    );
    send_error_and_line(entity_id, super::TRANSFER_CASH, org_id, ZERO_CASH_TEXT, tx).await;
}

/// The refusal for a forward from an entity with no character.
async fn not_a_player(
    entity_id: u32,
    method_index: u16,
    org_id: Option<i32>,
    account_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    tracing::debug!(
        target: "org",
        event = "org.forward",
        route = "rejected",
        reason = "not_a_player",
        account_id,
        entity_id,
        method_index,
        method_name = cimmeria_wire::names::player_cell_method(method_index),
        org_id,
        "organization call from an entity with no character"
    );
    send_unavailable_feedback(entity_id, method_index, org_id.unwrap_or(0), tx).await;
}

async fn send_to_base(
    msg: CellToBaseMsg,
    entity_id: u32,
    method_index: u16,
    org_id: Option<i32>,
    account_id: Option<u32>,
    player_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    match tx.send(msg).await {
        Ok(()) => tracing::debug!(
            target: "org",
            event = "org.forward",
            route = "base",
            account_id,
            player_id,
            entity_id,
            method_index,
            method_name = cimmeria_wire::names::player_cell_method(method_index),
            org_id,
            "organization call forwarded to the base"
        ),
        Err(_) => tracing::warn!(
            target: "org",
            event = "org.forward_failed",
            account_id,
            player_id,
            entity_id,
            method_index,
            method_name = cimmeria_wire::names::player_cell_method(method_index),
            org_id,
            reason = "cell_to_base_closed",
            "organization call could not be forwarded to the base"
        ),
    }
}

/// `strikeTeamResponse` (CM 11) or `pvpOrganizationLeaveResponse` (CM 12):
/// the answers to `onStrikeTeamUpdate` [41] and
/// `onPvPOrganizationLeaveRequest` [42], which this server never sends. So
/// every one is unsolicited (CAT-M-16, CAT-M-17): refused with one INFO
/// `org.strike_team_response` / `org.pvp_leave_response` row (`reason =
/// unsolicited`), counted, answered with the refusal pair, and never
/// forwarded.
pub(super) async fn unsolicited(
    entity_id: u32,
    method_index: u16,
    org_id: i32,
    response: u8,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let (event, action) = if method_index == super::STRIKE_TEAM_RESPONSE {
        ("org.strike_team_response", "strike_team_response")
    } else {
        ("org.pvp_leave_response", "pvp_leave_response")
    };
    let id = space_mgr.player_identity(entity_id);
    tracing::info!(
        target: "org",
        event,
        outcome = "rejected",
        reason = "unsolicited",
        route = "rejected",
        account_id = id.account_id,
        player_id = id.player_id,
        entity_id,
        org_id,
        response,
        "organization response refused: nothing asked for it"
    );
    count_org_action(action, "rejected", "unsolicited");
    send_error_and_line(entity_id, method_index, org_id, UNSOLICITED_TEXT, tx).await;
}

/// The line an unsolicited strike-team or PvP-leave response gets.
pub(crate) const UNSOLICITED_TEXT: &str = "There is no request to answer.";

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
                method_name = cimmeria_wire::names::player_client_method(method_index),
                reason = "cell_to_base_closed",
                "organization feedback could not be queued"
            );
            return;
        }
    }
}
