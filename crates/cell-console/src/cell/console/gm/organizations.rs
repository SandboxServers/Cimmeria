//! `gmReloadOrganizations` (`SGWGmPlayer` cell method 164, the stock
//! `/ReloadOrganizations`): re-send the calling GM's own Team and Command
//! state (ORG-10).
//!
//! The method has no arguments. The cell has no database, so it forwards
//! `OrgCellToBase::GmReload` with the GM's own character; the base re-reads
//! the access level (D-ORG13), re-runs the world-entry push for every Team
//! and Command the GM belongs to, answers on the feedback channel and writes
//! the `org.gm_action` row (`action = gm_reload_organizations`). The only
//! refusals decided here are the ones the base cannot see (no character id,
//! a dead base channel); each writes that row too.

use tokio::sync::mpsc;

use super::feedback::send_gm_feedback;
use crate::cell::messages::{CellToBaseMsg, OrgCellToBase};
use crate::cell::space_manager::SpaceManager;

/// The `action` of the command's `org.gm_action` row.
const ACTION: &str = "gm_reload_organizations";

fn refused(entity_id: u32, space_mgr: &SpaceManager, reason: &'static str) {
    let id = space_mgr.player_identity(entity_id);
    tracing::info!(
        target: "org",
        event = "org.gm_action",
        action = ACTION,
        outcome = "rejected",
        reason,
        account_id = id.account_id,
        player_id = id.player_id,
        entity_id,
        "GM organization command rejected"
    );
}

/// `gmReloadOrganizations()`: hand the re-push to the base.
#[tracing::instrument(name = "org.gm_reload", level = "info", skip_all, fields(entity_id = entity_id))]
pub(super) async fn handle_reload_organizations(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    let Some(player_id) = space_mgr.player_identity(entity_id).player_id else {
        refused(entity_id, space_mgr, "caller_not_player");
        send_gm_feedback(
            entity_id,
            "ReloadOrganizations: you have no character id",
            tx,
        )
        .await;
        return true;
    };
    let msg = CellToBaseMsg::Org(OrgCellToBase::GmReload {
        player_id,
        entity_id,
    });
    if tx.send(msg).await.is_err() {
        tracing::warn!(
            target: "org",
            event = "org.forward_failed",
            player_id,
            entity_id,
            kind = "gm_reload",
            reason = "cell_to_base_closed",
            "gmReloadOrganizations could not reach the base"
        );
        refused(entity_id, space_mgr, "cell_to_base_closed");
    }
    true
}
