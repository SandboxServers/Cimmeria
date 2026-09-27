//! Argument serializers for the OrganizationMember client methods (34-51).
//!
//! Field order and widths are `entities/defs/interfaces/OrganizationMember.def:61-177`,
//! and `RosterInfo` is `entities/defs/alias.xml:27-36`. Each function returns
//! the method's `args` only, for
//! [`crate::mercury::build_player_entity_method_packet`] or a
//! `CellToBaseMsg::EntityMethodCall`. Every one is pinned byte for byte in
//! `organization/tests.rs`.
//!
//! Encodings: `WSTRING` is a `u32` UTF-16 unit count and the units, LE.
//! `ARRAY<of>T</of>` is a `u32` element count and the elements. Every
//! integer is little-endian.
//!
//! The enum arguments are typed (`OrgType`, `OrgRank`, `OrgLeaveReason`,
//! `SquadLootType`), so a caller cannot put a raw wrong value on the wire.

use cimmeria_entity::organization::{
    OrgLeaveReason, OrgPermission, OrgRank, OrgType, SquadLootType,
};

use crate::mercury::write_wstring;

/// One `RosterInfo` FIXED_DICT (`alias.xml:27-36`), in field order.
///
/// There is no online flag: the client derives "Online" from the member id
/// it holds and its own entity table (audit A-11, D-ORG11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterInfo {
    pub name: String,
    pub level: u8,
    pub archetype: u8,
    pub rank: OrgRank,
    pub note: String,
    pub officer_note: String,
}

fn push_i32(buf: &mut Vec<u8>, v: i32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn push_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn push_count(buf: &mut Vec<u8>, n: usize) {
    let n = u32::try_from(n).expect("ARRAY element count exceeds u32");
    buf.extend_from_slice(&n.to_le_bytes());
}

/// `onOrganizationInvite` [34]: `WSTRING aInviterName, UINT8
/// aOrganizationType, INT32 aRequestID, WSTRING aName, UINT8 aIsStrikeTeam`.
///
/// Sent to the invitee. `request_id` is what the client echoes back in
/// `organizationInviteResponse` (CM 8); D-ORG06 sets its range.
pub fn build_on_organization_invite(
    inviter_name: &str,
    org_type: OrgType,
    request_id: i32,
    org_name: &str,
    is_strike_team: bool,
) -> Vec<u8> {
    let mut buf = Vec::new();
    write_wstring(&mut buf, inviter_name);
    buf.push(org_type.as_u8());
    push_i32(&mut buf, request_id);
    write_wstring(&mut buf, org_name);
    buf.push(u8::from(is_strike_team));
    buf
}

/// `onOrganizationJoined` [35]: `INT32 aOrganizationId, UINT8
/// aOrganizationType, UINT8 aRank, UINT8 aNewMember`.
///
/// `new_member` is 1 for a fresh join and 0 for the login replay (ORG-06).
pub fn build_on_organization_joined(
    org_id: i32,
    org_type: OrgType,
    rank: OrgRank,
    new_member: bool,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(7);
    push_i32(&mut buf, org_id);
    buf.push(org_type.as_u8());
    buf.push(rank.as_u8());
    buf.push(u8::from(new_member));
    buf
}

/// `onOrganizationLeft` [36]: `UINT8 aReason, INT32 aOrganizationId`.
///
/// Reason first: PR #584 once sent the org id first (audit A-30).
pub fn build_on_organization_left(reason: OrgLeaveReason, org_id: i32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5);
    buf.push(reason.as_u8());
    push_i32(&mut buf, org_id);
    buf
}

/// `onMemberJoinedOrganization` [37]: `WSTRING aMemberName, INT32 aMember,
/// INT32 aOrganizationId, UINT8 aRank, UINT8 aNewMember`.
///
/// `member_id` is the member's live entity id, or 0 when offline (D-ORG11).
pub fn build_on_member_joined_organization(
    member_name: &str,
    member_id: i32,
    org_id: i32,
    rank: OrgRank,
    new_member: bool,
) -> Vec<u8> {
    let mut buf = Vec::new();
    write_wstring(&mut buf, member_name);
    push_i32(&mut buf, member_id);
    push_i32(&mut buf, org_id);
    buf.push(rank.as_u8());
    buf.push(u8::from(new_member));
    buf
}

/// `onOrganizationRosterInfo` [38]: `INT32 aOrganizationId,
/// ARRAY<of>RosterInfo</of> aRosterInfo`.
pub fn build_on_organization_roster_info(org_id: i32, roster: &[RosterInfo]) -> Vec<u8> {
    let mut buf = Vec::new();
    push_i32(&mut buf, org_id);
    push_count(&mut buf, roster.len());
    for m in roster {
        write_wstring(&mut buf, &m.name);
        buf.push(m.level);
        buf.push(m.archetype);
        buf.push(m.rank.as_u8());
        write_wstring(&mut buf, &m.note);
        write_wstring(&mut buf, &m.officer_note);
    }
    buf
}

/// `onMemberLeftOrganization` [39]: `INT32 aMember, UINT8 aReason, INT32
/// aOrganizationId, WSTRING aMemberName`.
pub fn build_on_member_left_organization(
    member_id: i32,
    reason: OrgLeaveReason,
    org_id: i32,
    member_name: &str,
) -> Vec<u8> {
    let mut buf = Vec::new();
    push_i32(&mut buf, member_id);
    buf.push(reason.as_u8());
    push_i32(&mut buf, org_id);
    write_wstring(&mut buf, member_name);
    buf
}

/// `onMemberRankChangedOrganization` [40]: `INT32 aMember, UINT8 aRank,
/// INT32 aOrganizationId, WSTRING aMemberName`.
pub fn build_on_member_rank_changed_organization(
    member_id: i32,
    rank: OrgRank,
    org_id: i32,
    member_name: &str,
) -> Vec<u8> {
    let mut buf = Vec::new();
    push_i32(&mut buf, member_id);
    buf.push(rank.as_u8());
    push_i32(&mut buf, org_id);
    write_wstring(&mut buf, member_name);
    buf
}

/// `onStrikeTeamUpdate` [41]: `INT32 aOrganizationId, UINT8 aPvPValue`.
///
/// Built for completeness; no strike-team feature issues it (CAT-M-16).
pub fn build_on_strike_team_update(org_id: i32, pvp_value: u8) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5);
    push_i32(&mut buf, org_id);
    buf.push(pvp_value);
    buf
}

/// `onPvPOrganizationLeaveRequest` [42]: `INT32 aOrganizationId, UINT8
/// aPvPValue`.
///
/// Built for completeness; no PvP-flag feature issues it (CAT-M-17).
pub fn build_on_pvp_organization_leave_request(org_id: i32, pvp_value: u8) -> Vec<u8> {
    build_on_strike_team_update(org_id, pvp_value)
}

/// `onOrganizationNameUpdate` [43]: `INT32 aOrganizationId, WSTRING aName`.
pub fn build_on_organization_name_update(org_id: i32, name: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    push_i32(&mut buf, org_id);
    write_wstring(&mut buf, name);
    buf
}

/// `onOrganizationExperienceUpdate` [44]: `INT32 aOrganizationId, UINT64
/// aExperience`. Always 0 in this campaign (out of scope).
pub fn build_on_organization_experience_update(org_id: i32, experience: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(12);
    push_i32(&mut buf, org_id);
    push_u64(&mut buf, experience);
    buf
}

/// `onOrganizationMOTDUpdate` [45]: `INT32 aOrganizationId, WSTRING aMOTD`.
pub fn build_on_organization_motd_update(org_id: i32, motd: &str) -> Vec<u8> {
    build_on_organization_name_update(org_id, motd)
}

/// `onOrganizationNoteUpdate` [46]: `INT32 aOrganizationId, WSTRING aName,
/// WSTRING aNote`. `name` is the member the note belongs to.
pub fn build_on_organization_note_update(org_id: i32, name: &str, note: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    push_i32(&mut buf, org_id);
    write_wstring(&mut buf, name);
    write_wstring(&mut buf, note);
    buf
}

/// `onOrganizationOfficerNoteUpdate` [47]: same layout as [46]. Sent only to
/// members holding `OfficerNotes` (ORG-08).
pub fn build_on_organization_officer_note_update(org_id: i32, name: &str, note: &str) -> Vec<u8> {
    build_on_organization_note_update(org_id, name, note)
}

/// `onOrganizationCashUpdate` [48]: `INT32 aOrganizationId, UINT64 aCash`.
///
/// Part of the Bank campaign API (work-packets.md § ORG-API). The column is
/// `bigint CHECK (cash >= 0)`, so a caller converts with `u64::try_from`.
pub fn build_on_organization_cash_update(org_id: i32, cash: u64) -> Vec<u8> {
    build_on_organization_experience_update(org_id, cash)
}

/// `onOrganizationRankUpdate` [49]: `INT32 aOrganizationId,
/// ARRAY<of>INT32</of> aRankIds, ARRAY<of>INT32</of> aRankFlags`.
///
/// Takes pairs so the two parallel arrays can never differ in length.
pub fn build_on_organization_rank_update(
    org_id: i32,
    ranks: &[(OrgRank, OrgPermission)],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(12 + ranks.len() * 8);
    push_i32(&mut buf, org_id);
    push_count(&mut buf, ranks.len());
    for (rank, _) in ranks {
        push_i32(&mut buf, i32::from(rank.as_u8()));
    }
    push_count(&mut buf, ranks.len());
    for (_, perms) in ranks {
        push_i32(&mut buf, perms.to_wire());
    }
    buf
}

/// `onOrganizationRankNameUpdate` [50]: `INT32 aOrganizationId,
/// ARRAY<of>INT32</of> aRankIds, ARRAY<of>WSTRING</of> aRankNames`.
pub fn build_on_organization_rank_name_update<S: AsRef<str>>(
    org_id: i32,
    ranks: &[(OrgRank, S)],
) -> Vec<u8> {
    let mut buf = Vec::new();
    push_i32(&mut buf, org_id);
    push_count(&mut buf, ranks.len());
    for (rank, _) in ranks {
        push_i32(&mut buf, i32::from(rank.as_u8()));
    }
    push_count(&mut buf, ranks.len());
    for (_, name) in ranks {
        write_wstring(&mut buf, name.as_ref());
    }
    buf
}

/// `onSquadLootType` [51]: `INT32 aOrganizationId, INT32 aLootType`.
pub fn build_on_squad_loot_type(org_id: i32, loot_type: SquadLootType) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8);
    push_i32(&mut buf, org_id);
    push_i32(&mut buf, loot_type.as_i32());
    buf
}
