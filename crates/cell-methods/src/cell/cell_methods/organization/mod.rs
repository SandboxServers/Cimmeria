//! OrganizationMember interface exposed CellMethods (indices 8–19): the
//! router.
//!
//! Every method is decoded in full by
//! [`decode_org_cell_method`](cimmeria_wire::cell::cell_methods::organization::decode_org_cell_method)
//! and then routed on the id it carries (D-ORG05, D-ORG06):
//!
//! - CM 8 `organizationInviteResponse`: a request id with
//!   `BASE_INVITE_REQUEST_FLAG` clear is a squad invite, answered by
//!   [`squad`]; a base-issued id goes to [`forward`].
//! - CM 9 `organizationLeave`: a squad-range org id (or an id that routes
//!   nowhere, which the squad check then refuses) goes to [`squad`]; a Team
//!   or Command id is forwarded to the base (ORG-06).
//! - CM 10 `BroadcastMinimapPing`: like CM 9, a squad-range or unroutable
//!   id goes to [`squad`] (ORG-04), a Team or Command id to [`forward`].
//! - CM 18 `squadSetLootMode`: always [`squad`].
//! - Everything else: [`forward`].
//!
//! Routing is not authorization: [`squad`] still checks that the caller is
//! in the squad the id names. [`forward`] answers with ORG-01's "not
//! available yet" until ORG-07 forwards Team and Command calls to the base
//! (`docs/analysis/organizations/work-packets.md`).

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
    match call {
        OrgCellCall::InviteResponse {
            request_id,
            response,
        } if route_invite_request(request_id) != Some(InviteRoute::Base) => {
            squad::respond(entity_id, request_id, response != 0, tx, space_mgr).await;
        }
        OrgCellCall::Leave { org_id } if route_org_id(org_id) != Some(OrgRoute::Base) => {
            squad::leave(entity_id, org_id, tx, space_mgr).await;
        }
        OrgCellCall::Leave { org_id } => {
            forward::leave_to_base(entity_id, org_id, args, tx, space_mgr).await;
        }
        OrgCellCall::BroadcastMinimapPing { org_id, location }
            if route_org_id(org_id) != Some(OrgRoute::Base) =>
        {
            squad::broadcast_minimap_ping(entity_id, org_id, location, tx, space_mgr).await;
        }
        OrgCellCall::SquadSetLootMode { loot_mode } => {
            squad::set_loot_mode(entity_id, loot_mode, tx, space_mgr).await;
        }
        other => forward::answer(entity_id, method_index, &other, tx).await,
    }
    true
}
