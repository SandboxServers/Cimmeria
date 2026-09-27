//! `BaseToCellMsg::Org` handler: organization traffic from the base.
//!
//! Nothing sends these yet (ORG-01 only lays the contract), so each arm is a
//! logged no-op. ORG-03 replaces them with the squad registry (D-ORG03);
//! later packets add their handlers to this file rather than to `mod.rs`.

use super::super::super::messages::OrgBaseToCell;

/// Handle one organization message from the base.
pub(super) fn handle(msg: OrgBaseToCell) {
    let (player_id, entity_id) = msg.actor();
    let kind = msg.kind();
    match msg {
        OrgBaseToCell::SquadInvite { .. } => {
            tracing::debug!(
                target: "squad",
                event = "squad.invite_unimplemented",
                player_id,
                entity_id,
                kind,
                "squad message from the base has no handler yet"
            );
        }
        OrgBaseToCell::SquadKick { org_id, .. } => {
            tracing::debug!(
                target: "squad",
                event = "squad.kick_unimplemented",
                player_id,
                entity_id,
                kind,
                org_id,
                "squad message from the base has no handler yet"
            );
        }
    }
}
