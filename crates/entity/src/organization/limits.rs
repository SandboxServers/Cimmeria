//! Organization id spaces, size limits and text caps.
//!
//! The id constants are project policy (D-ORG05, D-ORG06), not recovered
//! data: the client uses one `INT32` id field for all three organization
//! types, and one `INT32` request id for every invite, so the server splits
//! each space by value to route without a lookup.

/// Highest Team or Command id. The schema pins
/// `CHECK (org_id BETWEEN 1 AND 1073741823)` on `sgw_organizations`, so a
/// database id can never reach the squad range (D-ORG05).
pub const MAX_ORG_ID: i32 = 0x3FFF_FFFF;

/// First squad organization id (D-ORG05). Squad ids come from a monotonic
/// cell counter in `[SQUAD_ORG_ID_MIN, SQUAD_ORG_ID_MAX]` and are never
/// reused within a server run. Cell methods that carry an org id route on
/// this range: squad ids stay on the cell, positive ids below it go to the
/// base, anything else is rejected. Routing is not authorization.
///
/// This is an **organization id** threshold (bit 30), owned by the cell's
/// squad registry. Do not confuse it with [`BASE_INVITE_REQUEST_FLAG`]
/// (bit 29), which marks a **request id** the base issued.
pub const SQUAD_ORG_ID_MIN: i32 = 0x4000_0000;

/// Last squad organization id: the top of the positive `INT32` range.
pub const SQUAD_ORG_ID_MAX: i32 = i32::MAX;

/// Set on every invite **request id** the base issues for a Team or Command
/// invite (D-ORG06). The cell allocates squad request ids from `1` with this
/// bit clear, so `organizationInviteResponse` (CM 8) routes on it: flag set
/// goes to the base, flag clear stays in the squad registry.
///
/// This is a **request id** flag (bit 29), owned by the base's pending-invite
/// map. It is deliberately a different bit from [`SQUAD_ORG_ID_MIN`] (bit
/// 30), which splits the **organization id** space, so the two can never be
/// mistaken for each other.
pub const BASE_INVITE_REQUEST_FLAG: i32 = 1 << 29;

/// Where a client-supplied organization id is handled (D-ORG05).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrgRoute {
    /// `[SQUAD_ORG_ID_MIN, SQUAD_ORG_ID_MAX]`: the cell's squad registry.
    Squad,
    /// `[1, MAX_ORG_ID]`: a Team or Command, on the base.
    Base,
}

/// Route an organization id from the wire. Zero and negative ids route
/// nowhere (`None`) and the caller rejects them. Routing is not
/// authorization: the handler still checks membership.
pub fn route_org_id(org_id: i32) -> Option<OrgRoute> {
    if org_id <= 0 {
        None
    } else if org_id >= SQUAD_ORG_ID_MIN {
        Some(OrgRoute::Squad)
    } else {
        Some(OrgRoute::Base)
    }
}

/// Where an invite response (CM 8) is matched (D-ORG06).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteRoute {
    /// Flag clear: a squad invite the cell issued.
    Cell,
    /// [`BASE_INVITE_REQUEST_FLAG`] set: a Team or Command invite the base
    /// issued.
    Base,
}

/// Route an invite request id from the wire. Zero and negative ids route
/// nowhere (`None`): neither side ever issues one, and checking the flag
/// only on positive values keeps a forged negative id (whose bit 29 may be
/// set) from reaching the base map.
pub fn route_invite_request(request_id: i32) -> Option<InviteRoute> {
    if request_id <= 0 {
        None
    } else if request_id & BASE_INVITE_REQUEST_FLAG != 0 {
        Some(InviteRoute::Base)
    } else {
        Some(InviteRoute::Cell)
    }
}

/// Most members a squad holds (audit A-18: six unit frames in the client).
pub const MAX_SQUAD_SIZE: usize = 6;

// D-ORG10 caps, in UTF-16 code units (the unit the client's `WSTRING` and
// edit boxes count). Server policy, except the name range, which is the
// client's own `CreateTeamMoniker` / `CreateCommandMoniker` string (audit
// A-13). The database `varchar` limits count code points, which is harmless
// because these are stricter.

/// Shortest organization name, after normalisation.
pub const MIN_NAME_UNITS: usize = 1;
/// Longest organization name, after normalisation.
pub const MAX_NAME_UNITS: usize = 60;
/// Longest message of the day.
pub const MAX_MOTD_UNITS: usize = 255;
/// Longest member note.
pub const MAX_NOTE_UNITS: usize = 128;
/// Longest officer note.
pub const MAX_OFFICER_NOTE_UNITS: usize = 128;
/// Longest rank name.
pub const MAX_RANK_NAME_UNITS: usize = 32;

/// Longest chat line (`sendPlayerCommunication` text). Not an organization
/// field: the social-systems campaign's D-SS12 cap ("the client's input
/// cap, never more than 255"), kept here so the one D-ORG10 implementation
/// holds every text cap. Provisional until SS-E1 reports the client's own
/// input limit; it may only go down.
pub const MAX_CHAT_TEXT_UNITS: usize = 255;
