//! Login restore: the per-organization push, byte for byte, and the
//! `member_online` fanout.

use cimmeria_entity::organization::{default_rank_permissions, OrgPermission};
use cimmeria_wire::cell::client_methods::organization::*;
use tracing::Level;

use super::*;
use crate::base::organization::api::OrgHeader;
use crate::base::organization::handlers::fanout::OnlineMember;
use crate::base::organization::handlers::{org_state_messages, restore_on_login};
use crate::base::organization::persistence::{OrgMembership, RankRow, RosterMember};
use crate::test_support::{require_db_or_skip, LogCapture};

fn roster_row(player_id: i32, name: &str, rank: OrgRank) -> RosterMember {
    RosterMember {
        player_id,
        name: name.into(),
        level: 12,
        archetype: 3,
        rank,
        note: String::new(),
        officer_note: String::new(),
    }
}

/// ORG-E1 Q1: [35], the header ([43] [45] [48] [44]), the ranks ([49]
/// [50]), the roster [38], and only then a [37] per online member. A [37]
/// before the roster would be overwritten with id 0 ("Offline"). The two
/// ends are pinned against literal bytes.
#[test]
fn org_state_messages_follow_the_org_e1_order() {
    let membership = OrgMembership {
        header: OrgHeader {
            org_id: 7,
            org_type: OrgType::Team,
            name: "Ab".into(),
            motd: String::new(),
            cash: 5,
            experience: 0,
        },
        rank: OrgRank::LEADER,
        display_permissions: OrgPermission::ALL,
    };
    let ranks = vec![
        RankRow {
            rank: OrgRank::MEMBER,
            name: None,
            permissions: OrgPermission::from_wire(1),
        },
        RankRow {
            rank: OrgRank::LEADER,
            name: Some("Boss".into()),
            permissions: OrgPermission::ALL,
        },
    ];
    let roster = vec![
        roster_row(100, "A", OrgRank::LEADER),
        roster_row(101, "B", OrgRank::MEMBER),
    ];
    let online = [OnlineMember {
        player_id: 101,
        entity_id: 0x0102_0304,
        account_id: Some(9),
    }];
    let msgs = org_state_messages(&membership, &ranks, &roster, &online, false);
    let order: Vec<u16> = msgs.iter().map(|m| m.0).collect();
    assert_eq!(order, vec![35, 43, 45, 48, 44, 49, 50, 38, 37]);
    // onOrganizationJoined: org 7, Team, Leader, aNewMember 0.
    assert_eq!(msgs[0].1, vec![7, 0, 0, 0, 1, 8, 0]);
    // onOrganizationCashUpdate: org 7, UINT64 5.
    assert_eq!(msgs[3].1, vec![7, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0]);
    // Only the renamed rank: ids [8], names ["Boss"].
    assert_eq!(
        msgs[6].1,
        build_on_organization_rank_name_update(7, &[(OrgRank::LEADER, "Boss")])
    );
    // onMemberJoinedOrganization: "B", entity 0x01020304, org 7, rank 2,
    // aNewMember 0.
    assert_eq!(
        msgs[8].1,
        vec![1, 0, 0, 0, 0x42, 0, 4, 3, 2, 1, 7, 0, 0, 0, 2, 0]
    );
}

/// Character 0 belongs to a Team (leader, with 1 and 2) and a Command
/// (member, led by 1). With 0 and 1 online, 0's world entry sends one
/// bundle per organization, Team first, each exactly the ORG-E1 sequence
/// with the stored header, ranks and roster; 1 gets one [37] per shared
/// organization carrying 0's entity id; 2 (offline) gets nothing.
#[tokio::test]
async fn login_push_for_a_two_org_player_is_byte_exact() {
    let pool = require_db_or_skip!();
    let fx = Fixture::new(&pool, 0, 3, &["Org06 Push Team", "Org06 Push Cmd"]).await;
    let team = fx.org(OrgType::Team, "Org06 Push Team", 0, &[1, 2]).await;
    let cmd = fx.org(OrgType::Command, "Org06 Push Cmd", 1, &[0]).await;
    sqlx::query("UPDATE sgw_organizations SET motd = 'Hi', cash = 1234 WHERE org_id = $1")
        .bind(team)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE sgw_organization_ranks SET name = 'Vet' WHERE org_id = $1 AND rank = 3")
        .bind(team)
        .execute(&pool)
        .await
        .unwrap();
    fx.online(0);
    fx.online(1);

    let capture = LogCapture::install();
    let pushed = restore_on_login(&fx.ctx(), &fx.player(0)).await;
    assert_eq!(pushed.len(), 2);

    let (n0, n1, n2) = (fx.name(0), fx.name(1), fx.name(2));
    let roster = |name: &str, i: i32, rank: OrgRank| RosterInfo {
        name: name.into(),
        level: 10 + i as u8,
        archetype: i as u8,
        rank,
        note: String::new(),
        officer_note: String::new(),
    };
    let e0 = fx.entity(0) as i32;
    let e1 = fx.entity(1) as i32;
    let masks = |t| -> Vec<(OrgRank, OrgPermission)> { default_rank_permissions(t) };
    let no_names: &[(OrgRank, &str)] = &[];

    let team_bundle: Vec<Call> = vec![
        (
            35,
            build_on_organization_joined(team, OrgType::Team, OrgRank::LEADER, false),
        ),
        (
            43,
            build_on_organization_name_update(team, "Org06 Push Team"),
        ),
        (45, build_on_organization_motd_update(team, "Hi")),
        (48, build_on_organization_cash_update(team, 1234)),
        (44, build_on_organization_experience_update(team, 0)),
        (
            49,
            build_on_organization_rank_update(team, &masks(OrgType::Team)),
        ),
        (
            50,
            build_on_organization_rank_name_update(team, &[(OrgRank::SENIOR_MEMBER, "Vet")]),
        ),
        (
            38,
            build_on_organization_roster_info(
                team,
                &[
                    roster(&n0, 0, OrgRank::LEADER),
                    roster(&n1, 1, OrgRank::MEMBER),
                    roster(&n2, 2, OrgRank::MEMBER),
                ],
            ),
        ),
        (
            37,
            build_on_member_joined_organization(&n0, e0, team, OrgRank::LEADER, false),
        ),
        (
            37,
            build_on_member_joined_organization(&n1, e1, team, OrgRank::MEMBER, false),
        ),
    ];
    let cmd_bundle: Vec<Call> = vec![
        (
            35,
            build_on_organization_joined(cmd, OrgType::Command, OrgRank::INITIATE, false),
        ),
        (43, build_on_organization_name_update(cmd, "Org06 Push Cmd")),
        (45, build_on_organization_motd_update(cmd, "")),
        (48, build_on_organization_cash_update(cmd, 0)),
        (44, build_on_organization_experience_update(cmd, 0)),
        (
            49,
            build_on_organization_rank_update(cmd, &masks(OrgType::Command)),
        ),
        (50, build_on_organization_rank_name_update(cmd, no_names)),
        (
            38,
            build_on_organization_roster_info(
                cmd,
                &[
                    roster(&n1, 1, OrgRank::LEADER),
                    roster(&n0, 0, OrgRank::INITIATE),
                ],
            ),
        ),
        (
            37,
            build_on_member_joined_organization(&n0, e0, cmd, OrgRank::INITIATE, false),
        ),
        (
            37,
            build_on_member_joined_organization(&n1, e1, cmd, OrgRank::LEADER, false),
        ),
    ];
    assert_eq!(fx.bundles_to(0), vec![team_bundle, cmd_bundle]);

    assert_eq!(
        fx.bundles_to(1),
        vec![
            vec![(
                37,
                build_on_member_joined_organization(&n0, e0, team, OrgRank::LEADER, false)
            )],
            vec![(
                37,
                build_on_member_joined_organization(&n0, e0, cmd, OrgRank::INITIATE, false)
            )],
        ]
    );
    assert!(
        fx.typed.filter_to(fx.addr(2)).is_empty(),
        "offline members get nothing"
    );

    let restore = capture
        .find_message(Level::INFO, "restored at world entry")
        .expect("org.login_restore row");
    assert!(
        restore.has_field("event", "org.login_restore"),
        "{restore:?}"
    );
    assert!(restore.has_field("outcome", "ok"), "{restore:?}");
    assert!(restore.has_field("org_count", "2"), "{restore:?}");
    let online_rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "org" && c.has_field("event", "member_online"))
        .collect();
    assert_eq!(online_rows.len(), 2);
    assert!(online_rows.iter().all(|c| c.has_field("recipients", "1")));
    fx.teardown().await;
}
