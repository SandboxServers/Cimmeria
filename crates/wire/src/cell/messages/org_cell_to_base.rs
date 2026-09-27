//! `OrgCellToBase`: organization traffic from the cell to the base, carried
//! by `CellToBaseMsg::Org`.
//!
//! One nested enum per direction keeps later organization packets out of
//! `cell_to_base.rs`: they add a variant here instead (work-packets.md
//! § Messages). Every variant carries the acting player's `player_id` and
//! `entity_id` **from the cell's own session state** (`CellEntity`), never
//! from the client payload, and never carries a privilege bit: the base
//! re-reads access level and membership itself (D-ORG04, D-ORG13).

use cimmeria_entity::organization::OrgType;

/// Organization messages sent from CellApp to BaseApp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrgCellToBase {
    /// Create a Team or Command (ORG-05). The cell sends it after
    /// `onOrganizationCreation` (CM 94) matched a pending creation: the
    /// `org_type` comes from that pending entry, never from the wire, and
    /// `name` has passed `org_text::validate`. The base re-checks
    /// eligibility and runs `create_org` under ORG-LOCK.
    Create {
        player_id: i32,
        entity_id: u32,
        org_type: OrgType,
        name: String,
    },

    /// `organizationTransferCash` (CM 19) for a Team or Command id (the Bank
    /// campaign's route, work-packets.md § ORG-API). `amount` is the signed
    /// wire value: its sign chooses deposit or withdraw. Until the bank
    /// lands, the base rejects it with feedback.
    TransferCash {
        player_id: i32,
        entity_id: u32,
        org_id: i32,
        amount: i32,
    },

    /// Any other OrganizationMember cell method (8-17) whose org id or
    /// request id routes to the base (D-ORG05, D-ORG06): the raw method
    /// index and argument bytes, which the base decodes with
    /// `cell::cell_methods::organization::decode_org_cell_method`. Used by
    /// ORG-06 (leave), ORG-07 (invite response) and ORG-08 (texts, ranks).
    ForwardCellCall {
        player_id: i32,
        entity_id: u32,
        method_index: u16,
        args: Vec<u8>,
    },
}

impl OrgCellToBase {
    /// The acting player's `(player_id, entity_id)`.
    pub fn actor(&self) -> (i32, u32) {
        match *self {
            OrgCellToBase::Create {
                player_id,
                entity_id,
                ..
            }
            | OrgCellToBase::TransferCash {
                player_id,
                entity_id,
                ..
            }
            | OrgCellToBase::ForwardCellCall {
                player_id,
                entity_id,
                ..
            } => (player_id, entity_id),
        }
    }

    /// Short stable name for the `kind` log field.
    pub fn kind(&self) -> &'static str {
        match self {
            OrgCellToBase::Create { .. } => "create",
            OrgCellToBase::TransferCash { .. } => "transfer_cash",
            OrgCellToBase::ForwardCellCall { .. } => "forward_cell_call",
        }
    }
}
