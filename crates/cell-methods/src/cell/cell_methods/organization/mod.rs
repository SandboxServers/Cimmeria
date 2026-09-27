//! OrganizationMember interface exposed CellMethods (indices 8–19): the
//! router.
//!
//! Every method is decoded in full by
//! [`decode_org_cell_method`](cimmeria_wire::cell::cell_methods::organization::decode_org_cell_method)
//! and then routed on the id it carries (D-ORG05, D-ORG06). Each routing
//! decision logs DEBUG `org.forward` with `route` = `squad`, `base` or
//! `rejected`:
//!
//! - CM 8 `organizationInviteResponse`: a request id with
//!   `BASE_INVITE_REQUEST_FLAG` clear is a squad invite, answered by
//!   [`squad`]; a base-issued id is forwarded to the base (ORG-07).
//! - CM 9 `organizationLeave` and CM 10 `BroadcastMinimapPing`: a
//!   squad-range id (or one that routes nowhere, which the squad check then
//!   refuses) goes to [`squad`]; a Team or Command id is forwarded.
//! - CM 11 `strikeTeamResponse` and CM 12 `pvpOrganizationLeaveResponse`
//!   are always refused as unsolicited: no strike-team or PvP-leave request
//!   is ever issued (CAT-M-16, CAT-M-17).
//! - CM 13-17 (texts, rank permissions and names): a Team or Command id is
//!   forwarded; squads have none of these, so any other id gets ORG-01's
//!   answer.
//! - CM 18 `squadSetLootMode`: always [`squad`].
//! - CM 19 `organizationTransferCash`: a Team or Command id is forwarded as
//!   `OrgCellToBase::TransferCash` (the Bank campaign's route); anything
//!   else gets ORG-01's answer.
//!
//! Routing is not authorization: [`squad`] still checks that the caller is
//! in the squad the id names, and the base checks membership under
//! ORG-LOCK. Forwarded calls carry this entity's own character, never the
//! payload's.

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

use cimmeria_entity::organization::{route_invite_request, route_org_id, InviteRoute, OrgRoute};
use cimmeria_wire::cell::cell_methods::organization::{decode_org_cell_method, OrgCellCall};
pub use cimmeria_wire::cell::cell_methods::organization::{
    BROADCAST_MINIMAP_PING, INVITE_RESPONSE, LEAVE, MOTD, NOTE, OFFICER_NOTE, PVP_LEAVE_RESPONSE,
    SET_RANK_NAME, SET_RANK_PERMISSIONS, SQUAD_SET_LOOT_MODE, STRIKE_TEAM_RESPONSE, TRANSFER_CASH,
};

pub mod creation;
mod forward;
pub mod squad;

#[cfg(test)]
mod tests;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    if !(INVITE_RESPONSE..=TRANSFER_CASH).contains(&method_index) {
        return false;
    }
    let call = match decode_org_cell_method(method_index, args) {
        Ok(call) => call,
        Err(e) => {
            // A real client always sends the `.def` shape; a malformed
            // payload is a forged or corrupted call and gets no answer.
            tracing::warn!(
                target: "org",
                event = "org.cell_method_malformed",
                entity_id,
                method_index,
                reason = e.reason(),
                error = %e,
                "organization cell method payload did not decode"
            );
            return true;
        }
    };
    let base = |org_id: i32| route_org_id(org_id) == Some(OrgRoute::Base);
    match call {
        OrgCellCall::InviteResponse {
            request_id,
            response,
        } if route_invite_request(request_id) != Some(InviteRoute::Base) => {
            forward::log_squad_route(entity_id, method_index, None);
            squad::respond(entity_id, request_id, response != 0, tx, space_mgr).await;
        }
        OrgCellCall::InviteResponse { .. } => {
            forward::to_base(entity_id, method_index, None, args, tx, space_mgr).await;
        }
        OrgCellCall::Leave { org_id } if !base(org_id) => {
            forward::log_squad_route(entity_id, method_index, Some(org_id));
            squad::leave(entity_id, org_id, tx, space_mgr).await;
        }
        OrgCellCall::BroadcastMinimapPing { org_id, location } if !base(org_id) => {
            forward::log_squad_route(entity_id, method_index, Some(org_id));
            squad::broadcast_minimap_ping(entity_id, org_id, location, tx, space_mgr).await;
        }
        OrgCellCall::SquadSetLootMode { loot_mode } => {
            forward::log_squad_route(entity_id, method_index, None);
            squad::set_loot_mode(entity_id, loot_mode, tx, space_mgr).await;
        }
        OrgCellCall::StrikeTeamResponse { org_id, response }
        | OrgCellCall::PvpLeaveResponse { org_id, response } => {
            forward::unsolicited(entity_id, method_index, org_id, response, tx, space_mgr).await;
        }
        OrgCellCall::TransferCash { org_id, dir } if base(org_id) => {
            forward::transfer_cash_to_base(entity_id, org_id, dir, tx, space_mgr).await;
        }
        other if other.org_id().is_some_and(base) => {
            forward::to_base(entity_id, method_index, other.org_id(), args, tx, space_mgr).await;
        }
        other => forward::answer(entity_id, method_index, &other, tx).await,
    }
    true
}
