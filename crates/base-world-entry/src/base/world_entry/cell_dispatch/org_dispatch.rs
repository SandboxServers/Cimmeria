//! Organization dispatch arm for `CellToBaseMsg::Org`.
//!
//! Every `OrgCellToBase` variant lands here. Nothing sends them yet (ORG-01
//! only lays the contract), so each arm is a logged no-op; ORG-05 (`Create`),
//! the Bank campaign (`TransferCash`) and ORG-06/07/08 (`ForwardCellCall`)
//! replace them. Later packets add their handlers to this file rather than
//! to `mod.rs`.

use crate::cell::messages::OrgCellToBase;

use super::DispatchCtx;

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
        OrgCellToBase::TransferCash { org_id, amount, .. } => {
            tracing::debug!(
                target: "org",
                event = "org.transfer_cash_unimplemented",
                player_id,
                entity_id,
                kind,
                org_id,
                amount,
                "organization message from the cell has no handler yet"
            );
        }
        OrgCellToBase::ForwardCellCall { method_index, .. } => {
            tracing::debug!(
                target: "org",
                event = "org.forward_unimplemented",
                player_id,
                entity_id,
                kind,
                method_index,
                "organization message from the cell has no handler yet"
            );
        }
    }
}
