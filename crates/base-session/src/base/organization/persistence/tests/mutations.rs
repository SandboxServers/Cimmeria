//! The Rust writes and reads: typed misses for every `rows_affected == 0`
//! path, the Leader pin, text validation, and the load shapes.

use cimmeria_entity::organization::{
    default_rank_permissions, OrgPermission, OrgRank, OrgType, TextReject,
};

use super::super::super::api::{lock_org, member_access_locked, OrgAccess};
use super::super::{
    add_member, disband, load_memberships, load_ranks, load_roster, name_available, remove_member,
    set_rank, set_rank_permissions, set_text, OrgStoreError, OrgTextTarget,
};
use super::*;
use crate::test_support::require_db_or_skip;

/// A missing organization, a non-member and a rank the type does not use
/// are each a typed refusal, never `Ok`.
#[tokio::test]
async fn misses_are_typed_not_ok() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 12, 2, &["Org02 Miss"]).await;
    let (p0, outsider) = (fx.player(0), fx.player(1));
    let org = create(&pool, OrgType::Team, "Org02 Miss", p0).await.org_id;
    let gone = org + 1_000_000;

    let mut tx = pool.begin().await.unwrap();
    assert_eq!(lock_org(&mut tx, gone).await.unwrap(), None);
    assert_eq!(member_access_locked(&mut tx, gone, p0).await.unwrap(), None);
    assert_eq!(
        member_access_locked(&mut tx, org, outsider).await.unwrap(),
        None
    );

    macro_rules! refused {
        ($call:expr, $pat:pat) => {{
            let res = $call.await;
            assert!(matches!(res, Err($pat)), "{}: {res:?}", stringify!($call));
        }};
    }
    refused!(
        remove_member(&mut tx, org, outsider),
        OrgStoreError::NotAMember
    );
    refused!(
        set_rank(&mut tx, org, outsider, OrgRank::SENIOR_MEMBER),
        OrgStoreError::NotAMember
    );
    refused!(
        set_text(
            &mut tx,
            org,
            OrgTextTarget::Note {
                player_id: outsider
            },
            "hi"
        ),
        OrgStoreError::NotAMember
    );
    refused!(
        set_text(
            &mut tx,
            org,
            OrgTextTarget::OfficerNote {
                player_id: outsider
            },
            "hi"
        ),
        OrgStoreError::NotAMember
    );
    refused!(
        set_text(&mut tx, gone, OrgTextTarget::Motd, "hi"),
        OrgStoreError::NoSuchOrg
    );
    refused!(
        add_member(&mut tx, gone, outsider, OrgRank::MEMBER),
        OrgStoreError::NoSuchOrg
    );
    refused!(disband(&mut tx, gone), OrgStoreError::NoSuchOrg);
    refused!(remove_member(&mut tx, gone, p0), OrgStoreError::NoSuchOrg);
    // Team uses ranks 2, 3 and 8 only.
    refused!(
        set_rank(&mut tx, org, p0, OrgRank::VETERAN),
        OrgStoreError::RankNotInType(OrgRank::VETERAN)
    );
    refused!(
        set_rank_permissions(&mut tx, org, OrgRank::INITIATE, OrgPermission::NONE),
        OrgStoreError::RankNotInType(OrgRank::INITIATE)
    );
    refused!(
        set_text(
            &mut tx,
            org,
            OrgTextTarget::RankName {
                rank: OrgRank::OFFICER
            },
            "Captain"
        ),
        OrgStoreError::RankNotInType(OrgRank::OFFICER)
    );
    refused!(
        add_member(&mut tx, org, outsider, OrgRank::INITIATE),
        OrgStoreError::RankNotInType(OrgRank::INITIATE)
    );
    refused!(
        add_member(&mut tx, org, -1, OrgRank::MEMBER),
        OrgStoreError::NoSuchPlayer
    );
    // Every refusal above left the transaction usable.
    assert!(lock_org(&mut tx, org).await.unwrap().is_some());
    tx.rollback().await.unwrap();

    teardown(&pool, &fx).await;
}

/// The Leader rank is pinned: never assigned by a rank change, the leader
/// is never moved off it, its mask is never edited, and a Leader cannot be
/// added to an organization that has members.
#[tokio::test]
async fn leader_rank_is_pinned() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 13, 2, &["Org02 Pin"]).await;
    let (leader, other) = (fx.player(0), fx.player(1));
    let org = create(&pool, OrgType::Command, "Org02 Pin", leader)
        .await
        .org_id;

    let mut tx = pool.begin().await.unwrap();
    let res = add_member(&mut tx, org, other, OrgRank::LEADER).await;
    assert!(matches!(res, Err(OrgStoreError::LeaderPinned)), "{res:?}");
    add_member(&mut tx, org, other, OrgRank::INITIATE)
        .await
        .unwrap();
    let res = set_rank(&mut tx, org, other, OrgRank::LEADER).await;
    assert!(matches!(res, Err(OrgStoreError::LeaderPinned)), "{res:?}");
    let res = set_rank(&mut tx, org, leader, OrgRank::OFFICER).await;
    assert!(matches!(res, Err(OrgStoreError::LeaderPinned)), "{res:?}");
    let res = set_rank_permissions(&mut tx, org, OrgRank::LEADER, OrgPermission::NONE).await;
    assert!(matches!(res, Err(OrgStoreError::LeaderPinned)), "{res:?}");
    tx.commit().await.unwrap();

    assert_eq!(
        member_ranks(&pool, org).await,
        vec![(leader, 8), (other, 1)],
        "nothing moved"
    );

    teardown(&pool, &fx).await;
}

/// Rank changes and permission edits land, return the old value, and
/// show up in `member_access_locked` in the same transaction.
#[tokio::test]
async fn rank_and_permission_writes_round_trip() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 14, 2, &["Org02 Perms"]).await;
    let (leader, m) = (fx.player(0), fx.player(1));
    let org = create(&pool, OrgType::Command, "Org02 Perms", leader)
        .await
        .org_id;

    let mut tx = pool.begin().await.unwrap();
    add_member(&mut tx, org, m, OrgRank::INITIATE)
        .await
        .unwrap();
    let old = set_rank(&mut tx, org, m, OrgRank::OFFICER).await.unwrap();
    assert_eq!(old, OrgRank::INITIATE);

    let officer_default = default_rank_permissions(OrgType::Command)
        .into_iter()
        .find(|(r, _)| *r == OrgRank::OFFICER)
        .unwrap()
        .1;
    let new_mask = OrgPermission::INVITE | OrgPermission::MOTD;
    let old_mask = set_rank_permissions(&mut tx, org, OrgRank::OFFICER, new_mask)
        .await
        .unwrap();
    assert_eq!(old_mask, officer_default);
    assert_eq!(
        member_access_locked(&mut tx, org, m).await.unwrap(),
        Some(OrgAccess {
            org_id: org,
            org_type: OrgType::Command,
            rank: OrgRank::OFFICER,
            permissions: new_mask,
        })
    );
    tx.commit().await.unwrap();

    teardown(&pool, &fx).await;
}

/// Texts are validated before they are stored; the stored form is the
/// validated one; a refused text writes nothing.
#[tokio::test]
async fn texts_are_validated_and_stored() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 15, 1, &["Org02 Texts"]).await;
    let p = fx.player(0);
    let org = create(&pool, OrgType::Command, "Org02 Texts", p)
        .await
        .org_id;

    let mut tx = pool.begin().await.unwrap();
    let motd = "Raid at 8.\nBring ammo.";
    assert_eq!(
        set_text(&mut tx, org, OrgTextTarget::Motd, motd)
            .await
            .unwrap(),
        motd
    );
    set_text(
        &mut tx,
        org,
        OrgTextTarget::Note { player_id: p },
        "my note",
    )
    .await
    .unwrap();
    set_text(
        &mut tx,
        org,
        OrgTextTarget::OfficerNote { player_id: p },
        "reliable",
    )
    .await
    .unwrap();
    let stored = set_text(
        &mut tx,
        org,
        OrgTextTarget::RankName {
            rank: OrgRank::OFFICER,
        },
        "  First   Prime ",
    )
    .await
    .unwrap();
    assert_eq!(stored, "First Prime");

    // A right-to-left override is refused and nothing changes.
    let res = set_text(&mut tx, org, OrgTextTarget::Motd, "evil\u{202E}txt").await;
    assert!(
        matches!(
            res,
            Err(OrgStoreError::InvalidText(TextReject::Bidi('\u{202E}')))
        ),
        "{res:?}"
    );
    let res = set_text(&mut tx, org, OrgTextTarget::Motd, &"x".repeat(256)).await;
    assert!(
        matches!(
            res,
            Err(OrgStoreError::InvalidText(TextReject::TooLong { .. }))
        ),
        "{res:?}"
    );
    tx.commit().await.unwrap();

    let header = load_memberships(&pool, p).await.unwrap();
    assert_eq!(header.len(), 1);
    assert_eq!(header[0].header.motd, motd);
    let roster = load_roster(&pool, org).await.unwrap();
    assert_eq!(roster[0].note, "my note");
    assert_eq!(roster[0].officer_note, "reliable");
    let ranks = load_ranks(&pool, org).await.unwrap();
    let officer = ranks.iter().find(|r| r.rank == OrgRank::OFFICER).unwrap();
    assert_eq!(officer.name.as_deref(), Some("First Prime"));

    teardown(&pool, &fx).await;
}

/// The login-push reads: a player in a Team and a Command gets both,
/// Team first, with their own rank and mask; the roster carries the
/// `RosterInfo` fields; the rank table is lowest first. Also the advisory
/// name check.
#[tokio::test]
async fn loads_return_what_the_login_push_needs() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 16, 2, &["Org02 Load Cmd", "Org02 Load Team"]).await;
    let (a, b) = (fx.player(0), fx.player(1));
    let cmd = create(&pool, OrgType::Command, "Org02 Load Cmd", a)
        .await
        .org_id;
    let team = create(&pool, OrgType::Team, "Org02 Load Team", b)
        .await
        .org_id;
    let mut tx = pool.begin().await.unwrap();
    add_member(&mut tx, cmd, b, OrgRank::INITIATE)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let ms = load_memberships(&pool, b).await.unwrap();
    let got: Vec<(i32, OrgType, OrgRank)> = ms
        .iter()
        .map(|m| (m.header.org_id, m.header.org_type, m.rank))
        .collect();
    assert_eq!(
        got,
        vec![
            (team, OrgType::Team, OrgRank::LEADER),
            (cmd, OrgType::Command, OrgRank::INITIATE),
        ]
    );
    assert_eq!(ms[0].permissions, OrgPermission::ALL);
    assert_eq!(ms[0].header.name, "Org02 Load Team");
    assert_eq!((ms[1].header.cash, ms[1].header.experience), (0, 0));

    let roster = load_roster(&pool, cmd).await.unwrap();
    let got: Vec<(i32, OrgRank, i32, i32)> = roster
        .iter()
        .map(|r| (r.player_id, r.rank, r.level, r.archetype))
        .collect();
    assert_eq!(
        got,
        vec![(a, OrgRank::LEADER, 7, 3), (b, OrgRank::INITIATE, 7, 3)]
    );
    assert_eq!(roster[1].name, format!("org02-{b}"));

    let ranks: Vec<OrgRank> = load_ranks(&pool, team)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.rank)
        .collect();
    assert_eq!(ranks, OrgRank::for_type(OrgType::Team));

    assert!(!name_available(&pool, OrgType::Command, "org02 LOAD cmd")
        .await
        .unwrap());
    assert!(name_available(&pool, OrgType::Team, "Org02 Load Cmd")
        .await
        .unwrap());
    let res = name_available(&pool, OrgType::Team, "Org02\u{200B}Load").await;
    assert!(matches!(res, Err(OrgStoreError::InvalidText(_))), "{res:?}");
    let res = name_available(&pool, OrgType::Squad, "Org02 Squad").await;
    assert!(matches!(res, Err(OrgStoreError::NotPersistent)), "{res:?}");

    teardown(&pool, &fx).await;
}
