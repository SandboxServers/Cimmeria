//! The schema's constraints, each hit directly with SQL so the guard is the
//! database's, not the Rust layer's.

use cimmeria_entity::organization::{default_rank_permissions, OrgRank, OrgType};

use super::super::{add_member, create_org, disband, OrgStoreError};
use super::*;
use crate::test_support::require_db_or_skip;

/// Creation writes one rank row per rank the type uses, with the default
/// masks, and the creator as its only member at `Leader`.
#[tokio::test]
async fn live_db_create_org_writes_every_rank_row() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 0, 1, &["Org02 Ranks Cmd", "Org02 Ranks Team"]).await;
    let leader = fx.player(0);

    for (org_type, name) in [
        (OrgType::Command, "Org02 Ranks Cmd"),
        (OrgType::Team, "Org02  ranks   Team "),
    ] {
        let org = create(&pool, org_type, name, leader).await;
        assert!((1..0x4000_0000).contains(&org.org_id));
        let rows: Vec<(i16, Option<String>, i32)> = sqlx::query_as(
            "SELECT rank, name, permissions FROM sgw_organization_ranks \
             WHERE org_id = $1 ORDER BY rank",
        )
        .bind(org.org_id)
        .fetch_all(&pool)
        .await
        .unwrap();
        let expected: Vec<(i16, Option<String>, i32)> = default_rank_permissions(org_type)
            .into_iter()
            .map(|(r, p)| (i16::from(r.as_u8()), None, p.to_wire()))
            .collect();
        assert_eq!(rows, expected, "{org_type:?} rank rows");
        assert_eq!(rows.len(), OrgRank::for_type(org_type).len());
        assert_eq!(
            member_ranks(&pool, org.org_id).await,
            vec![(leader, 8)],
            "the creator is the only member, at Leader"
        );
    }
    // The stored name is the normalised one.
    let team_name: String = sqlx::query_scalar(
        "SELECT name FROM sgw_organizations WHERE name_key = 'org02 ranks team'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(team_name, "Org02 ranks Team");

    teardown(&pool, &fx).await;
}

/// D-ORG18: one Team and one Command per player, enforced by
/// `UNIQUE (player_id, org_type)`. The same player in a Team and a Command
/// at once is fine.
#[tokio::test]
async fn live_db_second_team_for_same_player_is_refused() {
    let pool = require_db_or_skip!();
    let names = [
        "Org02 Uniq A",
        "Org02 Uniq B",
        "Org02 Uniq C",
        "Org02 Uniq Z",
    ];
    let fx = setup(&pool, 1, 2, &names).await;
    let (p, q) = (fx.player(0), fx.player(1));

    let a = create(&pool, OrgType::Team, "Org02 Uniq A", p).await;
    let b = create(&pool, OrgType::Team, "Org02 Uniq B", q).await;
    create(&pool, OrgType::Command, "Org02 Uniq C", p).await;

    // Raw insert: the database refuses it by the named key.
    let err = sqlx::query(
        "INSERT INTO sgw_organization_members (org_id, player_id, account_id, org_type, rank) \
         VALUES ($1, $2, (SELECT account_id FROM sgw_player WHERE player_id = $2), 1, 2)",
    )
    .bind(b.org_id)
    .bind(p)
    .execute(&pool)
    .await
    .expect_err("a second Team for one player must be refused");
    assert_eq!(
        violated(&err).as_deref(),
        Some("sgw_organization_members_player_id_org_type_key")
    );

    // Through the Rust layer it is a typed refusal, and the transaction
    // stays usable.
    let mut tx = pool.begin().await.unwrap();
    let res = as_sys!(add_member, tx, b.org_id, p, OrgRank::MEMBER);
    assert!(matches!(res, Err(OrgStoreError::AlreadyInType)), "{res:?}");
    let res = as_sys!(add_member, tx, a.org_id, p, OrgRank::MEMBER);
    assert!(matches!(res, Err(OrgStoreError::AlreadyMember)), "{res:?}");
    // A second create by a player already in a Team fails the same way, and
    // its organization row is rolled back with it: committing the
    // transaction anyway must not leave a leaderless "Org02 Uniq Z".
    let res = create_org(&mut tx, OrgType::Team, "Org02 Uniq Z", p).await;
    assert!(matches!(res, Err(OrgStoreError::AlreadyInType)), "{res:?}");
    tx.commit().await.unwrap();
    let stranded: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sgw_organizations WHERE name_key = 'org02 uniq z'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stranded, 0, "a refused create_org must write nothing");

    teardown(&pool, &fx).await;
}

/// D-ORG10: names are unique per type on the case-folded key. The same
/// name is free in the other type.
#[tokio::test]
async fn live_db_duplicate_name_key_is_refused() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 2, 3, &["Org02 Dup"]).await;

    let first = create(&pool, OrgType::Command, "Org02 Dup", fx.player(0)).await;

    let err = sqlx::query(
        "INSERT INTO sgw_organizations (org_type, name, name_key) VALUES (2, 'ORG02 DUP', $1)",
    )
    .bind("org02 dup")
    .execute(&pool)
    .await
    .expect_err("a duplicate name key in one type must be refused");
    assert_eq!(
        violated(&err).as_deref(),
        Some("sgw_organizations_org_type_name_key_key")
    );

    let mut tx = pool.begin().await.unwrap();
    let res = create_org(&mut tx, OrgType::Command, "  ORG02   dup ", fx.player(1)).await;
    assert!(matches!(res, Err(OrgStoreError::NameTaken)), "{res:?}");
    // NameTaken is refused before anything is written, so the same
    // transaction can go on to create a Team with that name.
    let team = create_org(&mut tx, OrgType::Team, "Org02 Dup", fx.player(1))
        .await
        .expect("the name is free for a Team");
    assert_ne!(team.org_id, first.org_id);
    tx.commit().await.unwrap();

    teardown(&pool, &fx).await;
}

/// The member row's `org_type` copy cannot differ from its organization's:
/// the composite foreign key refuses both an insert and an update.
#[tokio::test]
async fn live_db_member_org_type_cannot_drift() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 3, 2, &["Org02 Drift"]).await;
    let cmd = create(&pool, OrgType::Command, "Org02 Drift", fx.player(0)).await;

    let err = sqlx::query(
        "INSERT INTO sgw_organization_members (org_id, player_id, account_id, org_type, rank) \
         VALUES ($1, $2, (SELECT account_id FROM sgw_player WHERE player_id = $2), 1, 2)",
    )
    .bind(cmd.org_id)
    .bind(fx.player(1))
    .execute(&pool)
    .await
    .expect_err("a Team-typed member row in a Command must be refused");
    assert_eq!(
        violated(&err).as_deref(),
        Some("sgw_organization_members_org_fkey")
    );

    let err = sqlx::query("UPDATE sgw_organization_members SET org_type = 1 WHERE org_id = $1")
        .bind(cmd.org_id)
        .execute(&pool)
        .await
        .expect_err("retyping a member row must be refused");
    // The member BEFORE UPDATE trigger refuses it before the foreign key
    // is checked.
    assert_eq!(
        violated(&err).as_deref(),
        Some("sgw_organization_members_identity_immutable")
    );

    teardown(&pool, &fx).await;
}

/// D-ORG05: Team and Command ids stay below the squad range. The CHECK
/// refuses a hand-inserted id, and the sequence stops short of the range.
#[tokio::test]
async fn live_db_org_id_outside_base_range_is_refused() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 4, 1, &["Org02 Range Hi", "Org02 Range Lo"]).await;

    for (org_id, name) in [(0x4000_0000_i32, "org02 range hi"), (0, "org02 range lo")] {
        let err = sqlx::query(
            "INSERT INTO sgw_organizations (org_id, org_type, name, name_key) \
             VALUES ($1, 2, $2, $2)",
        )
        .bind(org_id)
        .bind(name)
        .execute(&pool)
        .await
        .expect_err("an org id outside 1..=0x3FFF_FFFF must be refused");
        assert_eq!(
            violated(&err).as_deref(),
            Some("sgw_organizations_org_id_range_check"),
            "org_id {org_id:#x}"
        );
    }
    let seq_max: i64 = sqlx::query_scalar(
        "SELECT seqmax FROM pg_sequence \
         WHERE seqrelid = 'sgw_organizations_org_id_seq'::regclass",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(seq_max, 0x3FFF_FFFF);

    teardown(&pool, &fx).await;
}

/// The treasury can never go negative, and a rank mask stays inside the 26
/// defined bits with the Leader row pinned to all of them.
#[tokio::test]
async fn live_db_cash_and_rank_masks_are_range_checked() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 5, 1, &["Org02 Checks"]).await;
    let org = create(&pool, OrgType::Command, "Org02 Checks", fx.player(0)).await;

    let err = sqlx::query("UPDATE sgw_organizations SET cash = cash - 1 WHERE org_id = $1")
        .bind(org.org_id)
        .execute(&pool)
        .await
        .expect_err("cash below zero must be refused");
    assert_eq!(
        violated(&err).as_deref(),
        Some("sgw_organizations_cash_nonneg_check")
    );

    for (rank, mask, constraint) in [
        (2_i16, -1_i32, "sgw_organization_ranks_permissions_check"),
        (2, 0x400_0000, "sgw_organization_ranks_permissions_check"),
        (8, 0x3FF_FFFE, "sgw_organization_ranks_leader_all_check"),
    ] {
        let err = sqlx::query(
            "UPDATE sgw_organization_ranks SET permissions = $3 WHERE org_id = $1 AND rank = $2",
        )
        .bind(org.org_id)
        .bind(rank)
        .bind(mask)
        .execute(&pool)
        .await
        .expect_err("an out-of-range mask must be refused");
        assert_eq!(violated(&err).as_deref(), Some(constraint), "rank {rank}");
    }

    teardown(&pool, &fx).await;
}

/// A member's rank must be one of the organization's rank rows: rank 0 is
/// refused in any type, and Team rank 5 (a Command-only rank) in a Team.
#[tokio::test]
async fn live_db_member_rank_must_have_a_rank_row() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 6, 2, &["Org02 RankFk"]).await;
    let team = create(&pool, OrgType::Team, "Org02 RankFk", fx.player(0)).await;

    for rank in [0_i16, 5] {
        let err = sqlx::query(
            "INSERT INTO sgw_organization_members (org_id, player_id, account_id, org_type, rank) \
             VALUES ($1, $2, (SELECT account_id FROM sgw_player WHERE player_id = $2), 1, $3)",
        )
        .bind(team.org_id)
        .bind(fx.player(1))
        .bind(rank)
        .execute(&pool)
        .await
        .expect_err("a rank with no rank row must be refused");
        assert_eq!(
            violated(&err).as_deref(),
            Some("sgw_organization_members_rank_fkey"),
            "rank {rank}"
        );
    }

    teardown(&pool, &fx).await;
}

/// Disband deletes the organization, and its ranks and members cascade,
/// with no error from the member-delete trigger on the way (it sees the
/// organization gone and stands down). Returns the members for fanout.
#[tokio::test]
async fn live_db_disband_cascades_ranks_and_members() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 7, 3, &["Org02 Disband"]).await;
    let org = create(&pool, OrgType::Command, "Org02 Disband", fx.player(0)).await;
    let mut tx = pool.begin().await.unwrap();
    as_sys!(add_member, tx, org.org_id, fx.player(1), OrgRank::INITIATE).unwrap();
    as_sys!(add_member, tx, org.org_id, fx.player(2), OrgRank::INITIATE).unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let members = as_sys!(disband, tx, org.org_id).expect("disband");
    tx.commit().await.unwrap();

    assert_eq!(members, fx.players);
    assert!(!org_exists(&pool, org.org_id).await);
    assert_eq!(rank_row_count(&pool, org.org_id).await, 0);
    assert!(member_ranks(&pool, org.org_id).await.is_empty());
    // The characters survive their organization.
    let players: i64 =
        sqlx::query_scalar("SELECT count(*) FROM sgw_player WHERE player_id = ANY($1)")
            .bind(&fx.players)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(players, 3);

    teardown(&pool, &fx).await;
}

/// One Leader per organization (a unique partial index), and a rank row
/// that members hold cannot be deleted out from under them.
#[tokio::test]
async fn live_db_one_leader_and_held_ranks_are_enforced() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 17, 2, &["Org02 OneLeader"]).await;
    let org = create(&pool, OrgType::Team, "Org02 OneLeader", fx.player(0)).await;

    let err = sqlx::query(
        "INSERT INTO sgw_organization_members (org_id, player_id, account_id, org_type, rank) \n         VALUES ($1, $2, (SELECT account_id FROM sgw_player WHERE player_id = $2), 1, 8)",
    )
    .bind(org.org_id)
    .bind(fx.player(1))
    .execute(&pool)
    .await
    .expect_err("a second Leader must be refused");
    assert_eq!(
        violated(&err).as_deref(),
        Some("sgw_organization_members_one_leader_idx")
    );

    let err = sqlx::query("DELETE FROM sgw_organization_ranks WHERE org_id = $1 AND rank = 8")
        .bind(org.org_id)
        .execute(&pool)
        .await
        .expect_err("a rank row a member holds must not be deletable");
    assert_eq!(
        violated(&err).as_deref(),
        Some("sgw_organization_members_rank_fkey")
    );
    assert_eq!(
        member_ranks(&pool, org.org_id).await,
        vec![(fx.player(0), 8)]
    );

    teardown(&pool, &fx).await;
}
