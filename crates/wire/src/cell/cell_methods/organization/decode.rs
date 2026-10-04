//! Typed decoders for the organization cell methods: OrganizationMember
//! 8-19 (`OrganizationMember.def:266-413`) and SGWPlayer 94
//! `onOrganizationCreation` (`SGWPlayer.def:877-880`).
//!
//! Each decoder reads exactly the `.def` arguments and rejects a short
//! payload, a `WSTRING` longer than the bytes left, an unpaired surrogate,
//! and trailing bytes. Values are returned raw: range checks (rank in type,
//! loot mode 0 or 1, org id routing) and authorization belong to the
//! handlers (D-ORG05, D-ORG09, D-ORG16), and text rules to
//! `cimmeria_entity::organization::org_text`.

use cimmeria_entity::organization::CashDir;

use super::reader::{ArgReader, OrgDecodeError};
use super::{
    BROADCAST_MINIMAP_PING, INVITE_RESPONSE, LEAVE, MOTD, NOTE, OFFICER_NOTE, PVP_LEAVE_RESPONSE,
    SET_RANK_NAME, SET_RANK_PERMISSIONS, SQUAD_SET_LOOT_MODE, STRIKE_TEAM_RESPONSE, TRANSFER_CASH,
};

/// One decoded OrganizationMember cell method (8-19).
#[derive(Debug, Clone, PartialEq)]
pub enum OrgCellCall {
    /// CM 8 `organizationInviteResponse(INT32 aRequestID, UINT8 aResponse)`.
    /// Any non-zero response accepts.
    InviteResponse { request_id: i32, response: u8 },
    /// CM 9 `organizationLeave(INT32 aOrganizationId)`.
    Leave { org_id: i32 },
    /// CM 10 `BroadcastMinimapPing(INT32 aOrganzationId, VECTOR3 aLocation)`.
    /// Every coordinate is finite.
    BroadcastMinimapPing { org_id: i32, location: [f32; 3] },
    /// CM 11 `strikeTeamResponse(INT32 aOrganizationId, UINT8 aResponse)`.
    StrikeTeamResponse { org_id: i32, response: u8 },
    /// CM 12 `pvpOrganizationLeaveResponse(INT32 aOrganizationId, UINT8
    /// aResponse)`.
    PvpLeaveResponse { org_id: i32, response: u8 },
    /// CM 13 `organizationMOTD(INT32 aOrganizationId, WSTRING aMOTD)`.
    Motd { org_id: i32, motd: String },
    /// CM 14 `organizationNote(INT32 aOrganizationId, WSTRING aNote)`: the
    /// caller's own roster note.
    Note { org_id: i32, note: String },
    /// CM 15 `organizationOfficerNote(INT32 aOrganizationId, WSTRING aName,
    /// WSTRING aNote)`: `name` is the member the note is about.
    OfficerNote {
        org_id: i32,
        name: String,
        note: String,
    },
    /// CM 16 `organizationSetRankPermissions(INT32 aOrganizationId, INT32
    /// aRank, INT32 aPermissions)`.
    SetRankPermissions {
        org_id: i32,
        rank: i32,
        permissions: i32,
    },
    /// CM 17 `organizationSetRankName(INT32 aOrganizationId, INT32 aRank,
    /// WSTRING aName)`.
    SetRankName {
        org_id: i32,
        rank: i32,
        name: String,
    },
    /// CM 18 `squadSetLootMode(INT32 aLootMode)`. No org id on the wire:
    /// the caller's own squad is implied.
    SquadSetLootMode { loot_mode: i32 },
    /// CM 19 `organizationTransferCash(INT32 aOrganizationId, INT32 aCash)`.
    /// The signed amount is split into a direction and a magnitude (a
    /// positive amount deposits, a negative one withdraws); zero is rejected
    /// at decode.
    TransferCash { org_id: i32, dir: CashDir },
}

impl OrgCellCall {
    /// The cell-method index the call arrived as.
    pub fn method_index(&self) -> u16 {
        match self {
            OrgCellCall::InviteResponse { .. } => INVITE_RESPONSE,
            OrgCellCall::Leave { .. } => LEAVE,
            OrgCellCall::BroadcastMinimapPing { .. } => BROADCAST_MINIMAP_PING,
            OrgCellCall::StrikeTeamResponse { .. } => STRIKE_TEAM_RESPONSE,
            OrgCellCall::PvpLeaveResponse { .. } => PVP_LEAVE_RESPONSE,
            OrgCellCall::Motd { .. } => MOTD,
            OrgCellCall::Note { .. } => NOTE,
            OrgCellCall::OfficerNote { .. } => OFFICER_NOTE,
            OrgCellCall::SetRankPermissions { .. } => SET_RANK_PERMISSIONS,
            OrgCellCall::SetRankName { .. } => SET_RANK_NAME,
            OrgCellCall::SquadSetLootMode { .. } => SQUAD_SET_LOOT_MODE,
            OrgCellCall::TransferCash { .. } => TRANSFER_CASH,
        }
    }

    /// The `.def` method name, for logs, from the generated table
    /// (`crate::names`).
    pub fn method_name(&self) -> &'static str {
        crate::names::player_cell_method(self.method_index()).unwrap_or("unknown")
    }

    /// The organization id the call names, if it carries one. CM 8 carries
    /// a request id and CM 18 nothing; both route by other means (D-ORG05,
    /// D-ORG06).
    pub fn org_id(&self) -> Option<i32> {
        match *self {
            OrgCellCall::InviteResponse { .. } | OrgCellCall::SquadSetLootMode { .. } => None,
            OrgCellCall::Leave { org_id }
            | OrgCellCall::BroadcastMinimapPing { org_id, .. }
            | OrgCellCall::StrikeTeamResponse { org_id, .. }
            | OrgCellCall::PvpLeaveResponse { org_id, .. }
            | OrgCellCall::Motd { org_id, .. }
            | OrgCellCall::Note { org_id, .. }
            | OrgCellCall::OfficerNote { org_id, .. }
            | OrgCellCall::SetRankPermissions { org_id, .. }
            | OrgCellCall::SetRankName { org_id, .. }
            | OrgCellCall::TransferCash { org_id, .. } => Some(org_id),
        }
    }
}

/// Decode OrganizationMember cell method `method_index` (8-19).
pub fn decode_org_cell_method(
    method_index: u16,
    args: &[u8],
) -> Result<OrgCellCall, OrgDecodeError> {
    let mut r = ArgReader::new(args);
    let call = match method_index {
        INVITE_RESPONSE => OrgCellCall::InviteResponse {
            request_id: r.i32("aRequestID")?,
            response: r.u8("aResponse")?,
        },
        LEAVE => OrgCellCall::Leave {
            org_id: r.i32("aOrganizationId")?,
        },
        BROADCAST_MINIMAP_PING => OrgCellCall::BroadcastMinimapPing {
            org_id: r.i32("aOrganzationId")?,
            location: [
                r.finite_f32("aLocation.x")?,
                r.finite_f32("aLocation.y")?,
                r.finite_f32("aLocation.z")?,
            ],
        },
        STRIKE_TEAM_RESPONSE => OrgCellCall::StrikeTeamResponse {
            org_id: r.i32("aOrganizationId")?,
            response: r.u8("aResponse")?,
        },
        PVP_LEAVE_RESPONSE => OrgCellCall::PvpLeaveResponse {
            org_id: r.i32("aOrganizationId")?,
            response: r.u8("aResponse")?,
        },
        MOTD => OrgCellCall::Motd {
            org_id: r.i32("aOrganizationId")?,
            motd: r.wstring("aMOTD")?,
        },
        NOTE => OrgCellCall::Note {
            org_id: r.i32("aOrganizationId")?,
            note: r.wstring("aNote")?,
        },
        OFFICER_NOTE => OrgCellCall::OfficerNote {
            org_id: r.i32("aOrganizationId")?,
            name: r.wstring("aName")?,
            note: r.wstring("aNote")?,
        },
        SET_RANK_PERMISSIONS => OrgCellCall::SetRankPermissions {
            org_id: r.i32("aOrganizationId")?,
            rank: r.i32("aRank")?,
            permissions: r.i32("aPermissions")?,
        },
        SET_RANK_NAME => OrgCellCall::SetRankName {
            org_id: r.i32("aOrganizationId")?,
            rank: r.i32("aRank")?,
            name: r.wstring("aName")?,
        },
        SQUAD_SET_LOOT_MODE => OrgCellCall::SquadSetLootMode {
            loot_mode: r.i32("aLootMode")?,
        },
        TRANSFER_CASH => OrgCellCall::TransferCash {
            org_id: r.i32("aOrganizationId")?,
            dir: CashDir::from_wire(r.i32("aCash")?)
                .ok_or(OrgDecodeError::InvalidValue { field: "aCash" })?,
        },
        other => return Err(OrgDecodeError::UnknownMethod(other)),
    };
    r.finish()?;
    Ok(call)
}

/// Decode SGWPlayer cell method 94 `onOrganizationCreation(WSTRING
/// aOrganizationName)`. The type is not on the wire (audit A-9).
pub fn decode_on_organization_creation(args: &[u8]) -> Result<String, OrgDecodeError> {
    let mut r = ArgReader::new(args);
    let name = r.wstring("aOrganizationName")?;
    r.finish()?;
    Ok(name)
}
