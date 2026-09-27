//! `BaseToCellMsg::Org` handler: organization traffic from the base.
//!
//! The base forwards the squad forms of its organization methods here
//! (D-ORG03: squads live on the cell), with the actor's ids from its own
//! session. The squad handlers re-check that the entity is still that
//! character before acting. Later packets add their handlers to this file
//! rather than to `mod.rs`.

use tokio::sync::mpsc;

use super::super::super::cell_methods::organization::squad;
use super::super::super::messages::{CellToBaseMsg, OrgBaseToCell};
use super::super::super::space_manager::SpaceManager;

/// Handle one organization message from the base.
pub(super) async fn handle(
    msg: OrgBaseToCell,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match msg {
        OrgBaseToCell::SquadInvite {
            player_id,
            entity_id,
            target_name,
        } => {
            squad::handle_invite(player_id, entity_id, &target_name, tx, space_mgr).await;
        }
        OrgBaseToCell::SquadKick {
            player_id,
            entity_id,
            org_id,
            target_name,
        } => {
            squad::handle_kick(player_id, entity_id, org_id, &target_name, tx, space_mgr).await;
        }
        OrgBaseToCell::OrgMembershipEnded {
            player_id,
            entity_id,
            org_id,
            reason,
        } => {
            // The Bank campaign's BV-07 extends this arm to close an open
            // Team or Command vault session for this player.
            let live = space_mgr.player_identity(entity_id);
            tracing::debug!(
                target: "org",
                event = "org.membership_ended",
                account_id = live.account_id,
                player_id,
                entity_id,
                org_id,
                reason = reason.as_u8(),
                live_entity = live.player_id == Some(player_id),
                "a player left a Team or Command"
            );
        }
    }
}
