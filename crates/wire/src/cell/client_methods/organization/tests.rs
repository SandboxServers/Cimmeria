//! Byte-exact wire tests (TESTING.md type 2) for client methods 34-51.
//!
//! Every expected payload is written out as literal bytes from the `.def`
//! field order (`OrganizationMember.def:61-177`), never rebuilt with the
//! serializer under test.

use cimmeria_entity::organization::{
    OrgLeaveReason, OrgPermission, OrgRank, OrgType, SquadLootType,
};

use super::*;

/// `WSTRING "Al"`: count 2, then `A` `l` as UTF-16LE.
const WS_AL: [u8; 8] = [2, 0, 0, 0, 0x41, 0, 0x6C, 0];
/// `WSTRING "SG"`.
const WS_SG: [u8; 8] = [2, 0, 0, 0, 0x53, 0, 0x47, 0];

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

#[test]
fn method_indices_are_34_to_51() {
    let got = [
        ON_ORGANIZATION_INVITE,
        ON_ORGANIZATION_JOINED,
        ON_ORGANIZATION_LEFT,
        ON_MEMBER_JOINED_ORGANIZATION,
        ON_ORGANIZATION_ROSTER_INFO,
        ON_MEMBER_LEFT_ORGANIZATION,
        ON_MEMBER_RANK_CHANGED_ORGANIZATION,
        ON_STRIKE_TEAM_UPDATE,
        ON_PVP_ORGANIZATION_LEAVE_REQUEST,
        ON_ORGANIZATION_NAME_UPDATE,
        ON_ORGANIZATION_EXPERIENCE_UPDATE,
        ON_ORGANIZATION_MOTD_UPDATE,
        ON_ORGANIZATION_NOTE_UPDATE,
        ON_ORGANIZATION_OFFICER_NOTE_UPDATE,
        ON_ORGANIZATION_CASH_UPDATE,
        ON_ORGANIZATION_RANK_UPDATE,
        ON_ORGANIZATION_RANK_NAME_UPDATE,
        ON_SQUAD_LOOT_TYPE,
    ];
    assert_eq!(
        got,
        [34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51]
    );
}

#[test]
fn cm34_on_organization_invite() {
    let got = build_on_organization_invite("Al", OrgType::Team, 0x2000_0001, "SG", true);
    let want = cat(&[&WS_AL, &[1], &[0x01, 0, 0, 0x20], &WS_SG, &[1]]);
    assert_eq!(got, want);
    // Squad, not a strike team: the type byte and the trailing flag are 0.
    let got = build_on_organization_invite("Al", OrgType::Squad, 7, "", false);
    assert_eq!(
        got,
        cat(&[&WS_AL, &[0], &[7, 0, 0, 0], &[0, 0, 0, 0], &[0]])
    );
}

#[test]
fn cm35_on_organization_joined() {
    let got = build_on_organization_joined(0x0102_0304, OrgType::Command, OrgRank::OFFICER, true);
    assert_eq!(got, [4, 3, 2, 1, 2, 6, 1]);
    let got = build_on_organization_joined(9, OrgType::Team, OrgRank::MEMBER, false);
    assert_eq!(got, [9, 0, 0, 0, 1, 2, 0]);
}

/// Reason first, then the id (the order PR #584 once got wrong, A-30).
#[test]
fn cm36_on_organization_left() {
    assert_eq!(
        build_on_organization_left(OrgLeaveReason::Kicked, 7),
        [1, 7, 0, 0, 0]
    );
    assert_eq!(
        build_on_organization_left(OrgLeaveReason::Logout, 0x4000_0000),
        [3, 0, 0, 0, 0x40]
    );
}

#[test]
fn cm37_on_member_joined_organization() {
    let got = build_on_member_joined_organization("Al", 0x11, 0x4000_0002, OrgRank::MEMBER, false);
    let want = cat(&[&WS_AL, &[0x11, 0, 0, 0], &[2, 0, 0, 0x40], &[2], &[0]]);
    assert_eq!(got, want);
}

#[test]
fn cm38_on_organization_roster_info() {
    let roster = [RosterInfo {
        name: "Al".into(),
        level: 20,
        archetype: 3,
        rank: OrgRank::LEADER,
        note: "n".into(),
        officer_note: String::new(),
    }];
    let got = build_on_organization_roster_info(9, &roster);
    let want = cat(&[
        &[9, 0, 0, 0],          // aOrganizationId
        &[1, 0, 0, 0],          // ARRAY count
        &WS_AL,                 // name
        &[20, 3, 8],            // level, archetype, rank
        &[1, 0, 0, 0, 0x6E, 0], // note "n"
        &[0, 0, 0, 0],          // officerNote ""
    ]);
    assert_eq!(got, want);
    assert_eq!(
        build_on_organization_roster_info(9, &[]),
        [9, 0, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn cm39_on_member_left_organization() {
    let got = build_on_member_left_organization(0x11, OrgLeaveReason::Disbanded, 9, "Al");
    assert_eq!(got, cat(&[&[0x11, 0, 0, 0], &[2], &[9, 0, 0, 0], &WS_AL]));
}

#[test]
fn cm40_on_member_rank_changed_organization() {
    let got = build_on_member_rank_changed_organization(0x11, OrgRank::SENIOR_OFFICER, 9, "Al");
    assert_eq!(got, cat(&[&[0x11, 0, 0, 0], &[7], &[9, 0, 0, 0], &WS_AL]));
}

#[test]
fn cm41_on_strike_team_update() {
    assert_eq!(build_on_strike_team_update(9, 1), [9, 0, 0, 0, 1]);
}

#[test]
fn cm42_on_pvp_organization_leave_request() {
    assert_eq!(
        build_on_pvp_organization_leave_request(-2, 0),
        [0xFE, 0xFF, 0xFF, 0xFF, 0]
    );
}

#[test]
fn cm43_on_organization_name_update() {
    assert_eq!(
        build_on_organization_name_update(9, "SG"),
        cat(&[&[9, 0, 0, 0], &WS_SG])
    );
}

#[test]
fn cm44_on_organization_experience_update() {
    assert_eq!(
        build_on_organization_experience_update(9, 0x0102_0304_0506_0708),
        [9, 0, 0, 0, 8, 7, 6, 5, 4, 3, 2, 1]
    );
}

#[test]
fn cm45_on_organization_motd_update() {
    assert_eq!(
        build_on_organization_motd_update(9, ""),
        [9, 0, 0, 0, 0, 0, 0, 0]
    );
    // A non-ASCII unit, and a supplementary-plane character as its
    // surrogate pair: the count is UTF-16 units, not chars or bytes.
    assert_eq!(
        build_on_organization_motd_update(9, "\u{E9}\u{1F680}"),
        [9, 0, 0, 0, 3, 0, 0, 0, 0xE9, 0, 0x3D, 0xD8, 0x80, 0xDE]
    );
}

#[test]
fn cm46_on_organization_note_update() {
    assert_eq!(
        build_on_organization_note_update(9, "Al", "n"),
        cat(&[&[9, 0, 0, 0], &WS_AL, &[1, 0, 0, 0, 0x6E, 0]])
    );
}

#[test]
fn cm47_on_organization_officer_note_update() {
    assert_eq!(
        build_on_organization_officer_note_update(9, "SG", ""),
        cat(&[&[9, 0, 0, 0], &WS_SG, &[0, 0, 0, 0]])
    );
}

#[test]
fn cm48_on_organization_cash_update() {
    assert_eq!(
        build_on_organization_cash_update(9, 1_000_000),
        [9, 0, 0, 0, 0x40, 0x42, 0x0F, 0, 0, 0, 0, 0]
    );
}

#[test]
fn cm49_on_organization_rank_update() {
    let ranks = [
        (
            OrgRank::MEMBER,
            OrgPermission::ROSTER_NOTES | OrgPermission::DEPOSIT_BANK,
        ),
        (OrgRank::LEADER, OrgPermission::ALL),
    ];
    let got = build_on_organization_rank_update(9, &ranks);
    let want = cat(&[
        &[9, 0, 0, 0],
        &[2, 0, 0, 0],             // aRankIds count
        &[2, 0, 0, 0, 8, 0, 0, 0], // Member, Leader
        &[2, 0, 0, 0],             // aRankFlags count
        &[0x20, 0, 0x01, 0],       // 32 | 65536
        &[0xFF, 0xFF, 0xFF, 0x03], // 0x3FF_FFFF
    ]);
    assert_eq!(got, want);
}

#[test]
fn cm50_on_organization_rank_name_update() {
    let got = build_on_organization_rank_name_update(
        9,
        &[(OrgRank::MEMBER, "M"), (OrgRank::LEADER, "L")],
    );
    let want = cat(&[
        &[9, 0, 0, 0],
        &[2, 0, 0, 0],
        &[2, 0, 0, 0, 8, 0, 0, 0],
        &[2, 0, 0, 0],
        &[1, 0, 0, 0, 0x4D, 0], // "M"
        &[1, 0, 0, 0, 0x4C, 0], // "L"
    ]);
    assert_eq!(got, want);
}

#[test]
fn cm51_on_squad_loot_type() {
    assert_eq!(
        build_on_squad_loot_type(0x4000_0001, SquadLootType::FreeForAll),
        [1, 0, 0, 0x40, 1, 0, 0, 0]
    );
    assert_eq!(
        build_on_squad_loot_type(0x4000_0001, SquadLootType::RoundRobin),
        [1, 0, 0, 0x40, 0, 0, 0, 0]
    );
}
