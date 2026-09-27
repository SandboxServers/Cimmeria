//! `OrgCellToBase`: organization traffic from the cell to the base, carried
//! by `CellToBaseMsg::Org`.
//!
//! One nested enum per direction keeps later organization packets out of
//! `cell_to_base.rs`: they add a variant here instead (work-packets.md
//! § Messages). Every variant carries the acting player's `player_id` and
//! `entity_id` **from the cell's own session state** (`CellEntity`), never
//! from the client payload, and never carries a privilege bit: the base
//! re-reads access level and membership itself (D-ORG04, D-ORG13).

use cimmeria_entity::organization::{CashDir, OrgType};

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

    /// A player right-clicked an organization registrar in range (ORG-05).
    /// `org_type` comes from the registrar's seed data, never from the
    /// client. The base checks D-ORG18 eligibility and answers with
    /// `OrgBaseToCell::RegistrarEligible`, or refuses with feedback itself.
    RegistrarOpen {
        player_id: i32,
        entity_id: u32,
        npc_entity_id: u32,
        org_type: OrgType,
    },

    /// GM `.org_create <team|command> <name>` (ORG-05): create directly,
    /// with no registrar and no pending creation. The cell has checked the
    /// caller's GM access level; the base re-reads it from its own session
    /// map before acting (D-ORG13), and applies every other creation rule.
    GmCreate {
        player_id: i32,
        entity_id: u32,
        org_type: OrgType,
        name: String,
    },

    /// `organizationTransferCash` (CM 19) for a Team or Command id (the Bank
    /// campaign's route, work-packets.md § ORG-API). `dir` is the decoded
    /// direction and magnitude; a zero amount never gets this far. Until the
    /// bank lands, the base rejects it with feedback.
    TransferCash {
        player_id: i32,
        entity_id: u32,
        org_id: i32,
        dir: CashDir,
    },

    /// Any other OrganizationMember cell method (8-17) whose org id or
    /// request id routes to the base (D-ORG05, D-ORG06): the raw method
    /// index and argument bytes, which the base decodes with
    /// `cell::cell_methods::organization::decode_org_cell_method`. Used by
    /// ORG-06 (leave), ORG-07 (invite response; CM 10 and 13-17 reach the
    /// base too and are answered there) and ORG-08 (texts, ranks).
    ForwardCellCall {
        player_id: i32,
        entity_id: u32,
        method_index: u16,
        args: Vec<u8>,
    },

    /// `.org_disband <orgId>` (ORG-06). The cell's `.`-console only runs a
    /// GM's line, but no privilege bit travels: the base re-reads the
    /// session's access level itself (D-ORG13) and still honours the vault
    /// check (D-ORG20).
    GmDisband {
        player_id: i32,
        entity_id: u32,
        org_id: i32,
    },

    /// `.org_join <orgId> [player]` (ORG-07): add `target_name` (the GM
    /// themself when `None`) to the organization, skipping the member
    /// permission checks but not the type and one-per-type rules. The base
    /// re-reads the access level (D-ORG13).
    GmJoin {
        player_id: i32,
        entity_id: u32,
        org_id: i32,
        target_name: Option<String>,
    },

    /// `.org_rank <player> <rank> [orgId]` (ORG-07): set a member's rank,
    /// skipping D-ORG09 (1)-(2) but not the rank rules (a rank the type
    /// uses, never `Leader`). `org_id` may be omitted when the member is in
    /// only one Team or Command. The base re-reads the access level.
    GmRank {
        player_id: i32,
        entity_id: u32,
        target_name: String,
        rank: u8,
        org_id: Option<i32>,
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
            | OrgCellToBase::RegistrarOpen {
                player_id,
                entity_id,
                ..
            }
            | OrgCellToBase::GmCreate {
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
            }
            | OrgCellToBase::GmDisband {
                player_id,
                entity_id,
                ..
            }
            | OrgCellToBase::GmJoin {
                player_id,
                entity_id,
                ..
            }
            | OrgCellToBase::GmRank {
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
            OrgCellToBase::RegistrarOpen { .. } => "registrar_open",
            OrgCellToBase::GmCreate { .. } => "gm_create",
            OrgCellToBase::TransferCash { .. } => "transfer_cash",
            OrgCellToBase::ForwardCellCall { .. } => "forward_cell_call",
            OrgCellToBase::GmDisband { .. } => "gm_disband",
            OrgCellToBase::GmJoin { .. } => "gm_join",
            OrgCellToBase::GmRank { .. } => "gm_rank",
        }
    }
}
