//! `OrgBaseToCell`: organization traffic from the base to the cell, carried
//! by `BaseToCellMsg::Org`.
//!
//! Squads live on the cell (D-ORG03), so the base forwards the squad forms
//! of the organization base methods here. Every variant carries the acting
//! player's `player_id` and `entity_id` from the base's own session map
//! (`ConnectedClientState`), never from the client payload, and never a
//! privilege bit.

use cimmeria_entity::organization::OrgLeaveReason;

/// Organization messages sent from BaseApp to CellApp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrgBaseToCell {
    /// `organizationInviteByType` (0xD0) with type 0 (ORG-03). The base
    /// resolves nothing: the cell looks `target_name` up with
    /// `find_online_player_by_name` and runs every squad check.
    SquadInvite {
        player_id: i32,
        entity_id: u32,
        target_name: String,
    },

    /// `organizationKick` (0xD1) with an org id in the squad range
    /// (D-ORG05), if ORG-E1 confirms the client's `squadKick` travels this
    /// way (ORG-03). The cell checks that the actor leads that squad.
    SquadKick {
        player_id: i32,
        entity_id: u32,
        org_id: i32,
        target_name: String,
    },

    /// An online player stopped being a member of a Team or Command (ORG-06):
    /// the base sends it beside every `onOrganizationLeft` [36] to an online
    /// player (a leave, a disband, and from ORG-07 a kick), after the
    /// commit. The cell logs `org.membership_ended`; the Bank campaign's
    /// BV-07 extends that arm to close an open organization vault session.
    /// `player_id` is the member's character, so the cell can tell a stale
    /// entity id from a live one.
    OrgMembershipEnded {
        player_id: i32,
        entity_id: u32,
        org_id: i32,
        reason: OrgLeaveReason,
    },
}

impl OrgBaseToCell {
    /// The acting player's `(player_id, entity_id)`.
    pub fn actor(&self) -> (i32, u32) {
        match *self {
            OrgBaseToCell::SquadInvite {
                player_id,
                entity_id,
                ..
            }
            | OrgBaseToCell::SquadKick {
                player_id,
                entity_id,
                ..
            }
            | OrgBaseToCell::OrgMembershipEnded {
                player_id,
                entity_id,
                ..
            } => (player_id, entity_id),
        }
    }

    /// Short stable name for the `kind` log field.
    pub fn kind(&self) -> &'static str {
        match self {
            OrgBaseToCell::SquadInvite { .. } => "squad_invite",
            OrgBaseToCell::SquadKick { .. } => "squad_kick",
            OrgBaseToCell::OrgMembershipEnded { .. } => "org_membership_ended",
        }
    }
}
