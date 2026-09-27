//! Team and Command console commands (organizations campaign ORG-06):
//! `.org_disband <orgId>`.
//!
//! The cell has no database, so the command is forwarded to the base
//! (`OrgCellToBase::GmDisband`), which re-reads the GM's access level from
//! its own session (D-ORG13), honours the vault check (D-ORG20), fans
//! `onOrganizationLeft(Disbanded)` out to the online members and answers the
//! GM on the feedback channel. The base writes the `org.disband` outcome
//! row; the only refusals decided here are the ones the base cannot see (a
//! caller with no character, a malformed id, a dead base channel), each an
//! `org.disband` row too. Those three are not counted on
//! `org_actions_total`: the console crate has no metrics dependency, and a
//! GM typo is not an organization action.

use tokio::sync::mpsc;

use super::parse::parse_i32;
use super::send_gm_feedback;
use crate::cell::messages::{CellToBaseMsg, OrgCellToBase};
use crate::cell::space_manager::SpaceManager;

/// `.org_disband <orgId>`: hand the disband to the base.
#[tracing::instrument(name = "org.disband", level = "info", skip_all, fields(entity_id = caller_id))]
pub(super) async fn disband(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(caller_id);
    let refused = |reason: &'static str, org_id: Option<i32>| {
        tracing::info!(
            target: "org",
            event = "org.disband",
            outcome = "rejected",
            reason,
            account_id = id.account_id,
            player_id = id.player_id,
            entity_id = caller_id,
            org_id,
            "organization action rejected"
        );
    };
    let Some(org_id) = parse_i32(caller_id, args, 0, "orgId", tx).await else {
        refused("org_id_invalid", None);
        return;
    };
    let Some(player_id) = id.player_id else {
        refused("caller_not_player", Some(org_id));
        send_gm_feedback(caller_id, "org_disband: you have no character id", tx).await;
        return;
    };
    let msg = CellToBaseMsg::Org(OrgCellToBase::GmDisband {
        player_id,
        entity_id: caller_id,
        org_id,
    });
    if tx.send(msg).await.is_err() {
        tracing::warn!(
            target: "org",
            event = "org.forward_failed",
            account_id = id.account_id,
            player_id,
            entity_id = caller_id,
            org_id,
            reason = "cell_to_base_closed",
            "org_disband could not reach the base"
        );
        refused("cell_to_base_closed", Some(org_id));
    }
}
