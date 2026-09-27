//! Organization dispatch arm for `CellToBaseMsg::Org`.
//!
//! Every `OrgCellToBase` variant lands here. Nothing sends them yet (ORG-01
//! only lays the contract), so each arm is a logged no-op; ORG-05 (`Create`),
//! the Bank campaign (`TransferCash`) and ORG-06/07/08 (`ForwardCellCall`)
//! replace them. Later packets add their handlers to this file rather than
//! to `mod.rs`.

use std::ops::RangeInclusive;

use cimmeria_wire::cell::cell_methods::organization::decode_org_cell_method;

use crate::cell::messages::OrgCellToBase;

use super::DispatchCtx;

/// The OrganizationMember cell methods the cell may forward to the base:
/// invite response (8) to rank name (17). CM 18 (squad loot mode) never
/// leaves the cell, and CM 19 has its own `TransferCash` variant, so any
/// other index in a forward is a cell-side bug or a forged message.
const FORWARDABLE: RangeInclusive<u16> = 8..=17;

/// Route one organization message from the cell.
pub(super) async fn route(msg: OrgCellToBase, _ctx: &DispatchCtx<'_>) {
    let (player_id, entity_id) = msg.actor();
    let kind = msg.kind();
    match msg {
        OrgCellToBase::Create { org_type, .. } => {
            tracing::debug!(
                target: "org",
                event = "org.create_unimplemented",
                player_id,
                entity_id,
                kind,
                org_type = org_type.name(),
                "organization message from the cell has no handler yet"
            );
        }
        OrgCellToBase::TransferCash { org_id, dir, .. } => {
            tracing::debug!(
                target: "org",
                event = "org.transfer_cash_unimplemented",
                player_id,
                entity_id,
                kind,
                org_id,
                dir = ?dir,
                "organization message from the cell has no handler yet"
            );
        }
        OrgCellToBase::ForwardCellCall {
            method_index, args, ..
        } => {
            // Range first, before a single byte is decoded.
            if !FORWARDABLE.contains(&method_index) {
                tracing::warn!(
                    target: "org",
                    event = "org.forward_rejected",
                    player_id,
                    entity_id,
                    method_index,
                    reason = "method_out_of_range",
                    "forwarded organization cell call outside 8..=17"
                );
                return;
            }
            match decode_org_cell_method(method_index, &args) {
                Ok(call) => tracing::debug!(
                    target: "org",
                    event = "org.forward_unimplemented",
                    player_id,
                    entity_id,
                    kind,
                    method_index,
                    method = call.method_name(),
                    "organization message from the cell has no handler yet"
                ),
                Err(e) => tracing::warn!(
                    target: "org",
                    event = "org.forward_rejected",
                    player_id,
                    entity_id,
                    method_index,
                    reason = e.reason(),
                    error = %e,
                    "forwarded organization cell call did not decode"
                ),
            }
        }
    }
}
