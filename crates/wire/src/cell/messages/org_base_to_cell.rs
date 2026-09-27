//! `OrgBaseToCell`: organization traffic from the base to the cell, carried
//! by `BaseToCellMsg::Org`.
//!
//! Squads live on the cell (D-ORG03), so the base forwards the squad forms
//! of the organization base methods here. Every variant carries the acting
//! player's `player_id` and `entity_id` from the base's own session map
//! (`ConnectedClientState`), never from the client payload, and never a
//! privilege bit.

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
            } => (player_id, entity_id),
        }
    }

    /// Short stable name for the `kind` log field.
    pub fn kind(&self) -> &'static str {
        match self {
            OrgBaseToCell::SquadInvite { .. } => "squad_invite",
            OrgBaseToCell::SquadKick { .. } => "squad_kick",
        }
    }
}
