//! The member-delete trigger (D-ORG12, D-ORG20), driven the way production
//! drives it: a character delete goes through
//! `organization::character_delete::delete_character`, which is what
//! `handle_delete_character` calls. The memberless case uses a bare
//! `DELETE FROM sgw_player`, which the trigger handles the same way.

use std::time::Duration;

use cimmeria_entity::organization::{OrgPermission, OrgRank, OrgType};

use super::super::super::api::{lock_org, member_access_locked};
use super::super::super::character_delete::delete_character as delete_character_locked;
use super::super::{add_member, remove_member, set_rank, AfterRemoval, OrgStoreError};
use super::*;
use crate::test_support::require_db_or_skip;

async fn delete_character(pool: &PgPool, fx: &Fixture, player_id: i32) {
    let deleted = delete_character_locked(pool, player_id, fx.account_id)
        .await
        .expect("character delete");
    assert!(deleted.deleted, "character {player_id} was deleted");
}

/// Organizations among `org_ids` that have members but no Leader. The
/// invariant is that this is always empty.
async fn leaderless(pool: &PgPool, org_ids: &[i32]) -> Vec<i32> {
    sqlx::query_scalar(
        "SELECT o.org_id FROM sgw_organizations o \
         WHERE o.org_id = ANY($1) \
           AND EXISTS (SELECT 1 FROM sgw_organization_members m WHERE m.org_id = o.org_id) \
           AND NOT EXISTS (SELECT 1 FROM sgw_organization_members m \
                           WHERE m.org_id = o.org_id AND m.rank = 8)",
    )
    .bind(org_ids)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn set_joined(pool: &PgPool, org_id: i32, player_id: i32, days_ago: i32) {
    sqlx::query(
        "UPDATE sgw_organization_members SET joined_at = now() - make_interval(days => $3) \
         WHERE org_id = $1 AND player_id = $2",
    )
    .bind(org_id)
    .bind(player_id)
    .bind(days_ago)
    .execute(pool)
    .await
    .unwrap();
}

async fn add(pool: &PgPool, org_id: i32, player_id: i32, rank: OrgRank) {
    let mut tx = pool.begin().await.unwrap();
    add_member(&mut tx, org_id, player_id, rank)
        .await
        .expect("add_member");
    tx.commit().await.unwrap();
}

/// D-ORG12: deleting the leader's character promotes the highest-ranked
/// remaining member, the longest-standing among equals; deleting the last
/// member disbands; a character leading two organizations is handled in
/// both; and at no point does an organization with members lack a Leader.
#[tokio::test]
async fn leader_delete_leaves_no_leaderless_org() {
    let pool = require_db_or_skip!();
    let names = ["Org02 Heir Cmd", "Org02 Heir Solo", "Org02 Heir Team"];
    let fx = setup(&pool, 8, 4, &names).await;
    let (p0, p1, p2, p3) = (fx.player(0), fx.player(1), fx.player(2), fx.player(3));

    // Command: p0 leads; p1 and p2 are Officers, p2 the longer-standing
    // (and the higher player id, so joined_at, not the id, decides); p3 is
    // an Initiate who joined first of all.
    let cmd = create(&pool, OrgType::Command, names[0], p0).await.org_id;
    add(&pool, cmd, p1, OrgRank::OFFICER).await;
    add(&pool, cmd, p2, OrgRank::OFFICER).await;
    add(&pool, cmd, p3, OrgRank::INITIATE).await;
    set_joined(&pool, cmd, p1, 1).await;
    set_joined(&pool, cmd, p2, 2).await;
    set_joined(&pool, cmd, p3, 9).await;
    // p3 also leads a one-member Team.
    let solo = create(&pool, OrgType::Team, names[1], p3).await.org_id;
    let orgs = [cmd, solo];

    delete_character(&pool, &fx, p0).await;
    assert_eq!(
        member_ranks(&pool, cmd).await,
        vec![(p1, 6), (p2, 8), (p3, 1)],
        "the longer-standing Officer is promoted, not the lower id"
    );
    assert!(leaderless(&pool, &orgs).await.is_empty());

    // p2 now leads the Command and a Team with p1 in it.
    let team = create(&pool, OrgType::Team, names[2], p2).await.org_id;
    add(&pool, team, p1, OrgRank::MEMBER).await;
    let orgs = [cmd, solo, team];

    delete_character(&pool, &fx, p2).await;
    assert_eq!(
        member_ranks(&pool, cmd).await,
        vec![(p1, 8), (p3, 1)],
        "rank beats standing: the Officer, not the older Initiate"
    );
    assert_eq!(member_ranks(&pool, team).await, vec![(p1, 8)]);
    assert!(leaderless(&pool, &orgs).await.is_empty());

    // p3 was the only member of the solo Team: it disbands, ranks and all.
    // The Command, where p3 was an Initiate, is untouched.
    delete_character(&pool, &fx, p3).await;
    assert!(!org_exists(&pool, solo).await);
    assert_eq!(rank_row_count(&pool, solo).await, 0);
    assert_eq!(member_ranks(&pool, cmd).await, vec![(p1, 8)]);

    // p1 was the last member of both.
    delete_character(&pool, &fx, p1).await;
    assert!(!org_exists(&pool, cmd).await);
    assert!(!org_exists(&pool, team).await);
    assert!(leaderless(&pool, &orgs).await.is_empty());

    teardown(&pool, &fx).await;
}

/// D-ORG20: when the last member goes and `org_vault_is_empty_sql` says the
/// vault is not empty, the organization stays, memberless, with its rank
/// rows, and takes a new member only as Leader (the GM recovery path).
///
/// The stub always says "empty", so the test replaces it with one that says
/// "not empty" inside its own transaction and rolls everything back; the
/// replacement is never visible to another session.
#[tokio::test]
async fn last_member_delete_with_vault_leaves_memberless_org() {
    let pool = require_db_or_skip!();
    let names = ["Org02 Vault Cmd", "Org02 Vault Team"];
    let fx = setup(&pool, 9, 3, &names).await;
    let (p0, p1, p2) = (fx.player(0), fx.player(1), fx.player(2));

    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "CREATE OR REPLACE FUNCTION org_vault_is_empty_sql(p_org_id integer) \
         RETURNS boolean LANGUAGE sql STABLE AS $$ SELECT false $$",
    )
    .execute(&mut *tx)
    .await
    .unwrap();

    let cmd = create_org(&mut tx, OrgType::Command, names[0], p0)
        .await
        .unwrap()
        .org_id;
    let team = create_org(&mut tx, OrgType::Team, names[1], p1)
        .await
        .unwrap()
        .org_id;

    // A character delete of the only member.
    sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(p0)
        .execute(&mut *tx)
        .await
        .unwrap();
    // A leave of the only member reports the same outcome.
    let removal = remove_member(&mut tx, team, p1).await.unwrap();
    assert_eq!(removal.old_rank, OrgRank::LEADER);
    assert_eq!(removal.after, AfterRemoval::Memberless);

    for (org_id, ranks) in [(cmd, 8), (team, 3)] {
        let (exists, members, rank_rows): (bool, i64, i64) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM sgw_organizations WHERE org_id = $1), \
                    (SELECT count(*) FROM sgw_organization_members WHERE org_id = $1), \
                    (SELECT count(*) FROM sgw_organization_ranks WHERE org_id = $1)",
        )
        .bind(org_id)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert!(exists, "org {org_id} must survive with its vault");
        assert_eq!(members, 0);
        assert_eq!(rank_rows, ranks, "the rank rows stay for the recovery");
    }

    // GM recovery: nobody joins below Leader first; a Leader may.
    let res = add_member(&mut tx, cmd, p2, OrgRank::INITIATE).await;
    assert!(matches!(res, Err(OrgStoreError::NeedsLeader)), "{res:?}");
    add_member(&mut tx, cmd, p2, OrgRank::LEADER)
        .await
        .expect("a Leader may join a memberless organization");
    let access = member_access_locked(&mut tx, cmd, p2)
        .await
        .unwrap()
        .expect("p2 is a member");
    assert_eq!(access.rank, OrgRank::LEADER);
    assert_eq!(access.permissions, OrgPermission::ALL);

    tx.rollback().await.unwrap();

    // The replacement rolled back with everything else.
    let stub: bool = sqlx::query_scalar("SELECT org_vault_is_empty_sql(0)")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(stub, "the stub must be back to returning true");

    teardown(&pool, &fx).await;
}

/// `remove_member` reports what the trigger did: nothing for a plain
/// member, a promotion for the leader, a disband for the last member.
#[tokio::test]
async fn remove_member_reports_what_the_trigger_did() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 10, 3, &["Org02 Leave"]).await;
    let (p0, p1, p2) = (fx.player(0), fx.player(1), fx.player(2));
    let org = create(&pool, OrgType::Team, "Org02 Leave", p0).await.org_id;
    add(&pool, org, p1, OrgRank::MEMBER).await;
    add(&pool, org, p2, OrgRank::MEMBER).await;
    let mut tx = pool.begin().await.unwrap();
    set_rank(&mut tx, org, p2, OrgRank::SENIOR_MEMBER)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let r = remove_member(&mut tx, org, p1).await.unwrap();
    assert_eq!(
        (r.old_rank, r.after),
        (OrgRank::MEMBER, AfterRemoval::Unchanged)
    );
    let r = remove_member(&mut tx, org, p0).await.unwrap();
    assert_eq!(
        (r.old_rank, r.after),
        (
            OrgRank::LEADER,
            AfterRemoval::LeaderPromoted { player_id: p2 }
        )
    );
    let r = remove_member(&mut tx, org, p2).await.unwrap();
    assert_eq!(
        (r.old_rank, r.after),
        (OrgRank::LEADER, AfterRemoval::Disbanded)
    );
    let res = remove_member(&mut tx, org, p2).await;
    assert!(matches!(res, Err(OrgStoreError::NoSuchOrg)), "{res:?}");
    tx.commit().await.unwrap();
    assert!(!org_exists(&pool, org).await);

    teardown(&pool, &fx).await;
}

/// ORG-LOCK in the trigger (TESTING.md type 5). A transaction holds the
/// organization lock and removes the only other member while the leader's
/// character is being deleted. The trigger must wait for the lock and then
/// see that nobody remains, and disband. Without the lock it would read the
/// uncommitted member as still present, try to promote them, find the row
/// gone after the wait, and leave an empty organization behind that nothing
/// ever disbands.
#[tokio::test]
async fn trigger_waits_for_the_org_lock() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 11, 2, &["Org02 Lock"]).await;
    let (leader, member) = (fx.player(0), fx.player(1));
    let org = create(&pool, OrgType::Command, "Org02 Lock", leader)
        .await
        .org_id;
    add(&pool, org, member, OrgRank::INITIATE).await;

    let mut holder = pool.begin().await.unwrap();
    lock_org(&mut holder, org)
        .await
        .unwrap()
        .expect("org exists");
    let r = remove_member(&mut holder, org, member).await.unwrap();
    assert_eq!(r.after, AfterRemoval::Unchanged);

    let delete_pool = pool.clone();
    let delete = tokio::spawn(async move {
        sqlx::query("/* org02-lock-probe */ DELETE FROM sgw_player WHERE player_id = $1")
            .bind(leader)
            .execute(&delete_pool)
            .await
            .map(|r| r.rows_affected())
    });

    // Commit only once the delete is really blocked, or the test would pass
    // without exercising the race.
    let mut blocked = false;
    for _ in 0..100 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity \
             WHERE wait_event_type = 'Lock' AND query LIKE '/* org02-lock-probe */%'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        if waiting > 0 {
            blocked = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(blocked, "the character delete never waited on the org lock");
    holder.commit().await.unwrap();

    assert_eq!(delete.await.unwrap().expect("character delete"), 1);
    assert!(
        !org_exists(&pool, org).await,
        "the last member's delete must disband, not strand an empty org"
    );

    teardown(&pool, &fx).await;
}

/// One statement deleting several members of one organization: the first
/// trigger firing already sees them all gone, promotes the survivor, and a
/// statement that deletes everyone disbands.
#[tokio::test]
async fn multi_row_member_delete_promotes_or_disbands_once() {
    let pool = require_db_or_skip!();
    let names = ["Org02 Multi A", "Org02 Multi B"];
    let fx = setup(&pool, 18, 3, &names).await;
    let (p0, p1, p2) = (fx.player(0), fx.player(1), fx.player(2));
    let a = create(&pool, OrgType::Command, names[0], p0).await.org_id;
    add(&pool, a, p1, OrgRank::OFFICER).await;
    add(&pool, a, p2, OrgRank::INITIATE).await;
    let b = create(&pool, OrgType::Team, names[1], p1).await.org_id;
    add(&pool, b, p2, OrgRank::MEMBER).await;

    sqlx::query("DELETE FROM sgw_organization_members WHERE org_id = $1 AND player_id <> $2")
        .bind(a)
        .bind(p2)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(member_ranks(&pool, a).await, vec![(p2, 8)]);

    sqlx::query("DELETE FROM sgw_organization_members WHERE org_id = $1")
        .bind(b)
        .execute(&pool)
        .await
        .unwrap();
    assert!(!org_exists(&pool, b).await);
    assert!(leaderless(&pool, &[a, b]).await.is_empty());

    teardown(&pool, &fx).await;
}

/// ORG-LOCK on the character-delete path (TESTING.md type 5). A kick holds
/// the organization lock while the kicked member's character is being
/// deleted. `delete_character` waits for the organization lock *before* it
/// deletes anything, so the kick goes through and then the delete does.
/// A bare `DELETE FROM sgw_player` would hold the member row and wait for
/// the organization inside the trigger while the kick waits for the member
/// row: a deadlock, and Postgres aborts one of the two.
#[tokio::test]
async fn kick_during_character_delete_does_not_deadlock() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 19, 2, &["Org02 Kick"]).await;
    let (leader, member) = (fx.player(0), fx.player(1));
    let org = create(&pool, OrgType::Command, "Org02 Kick", leader)
        .await
        .org_id;
    add(&pool, org, member, OrgRank::INITIATE).await;

    let mut kick = pool.begin().await.unwrap();
    lock_org(&mut kick, org).await.unwrap().expect("org exists");

    let delete_pool = pool.clone();
    let account_id = fx.account_id;
    let delete =
        tokio::spawn(
            async move { delete_character_locked(&delete_pool, member, account_id).await },
        );

    let mut blocked = false;
    for _ in 0..100 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity              WHERE wait_event_type = 'Lock' AND datname = current_database()                AND pid <> pg_backend_pid()",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        if waiting > 0 {
            blocked = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(blocked, "the character delete never waited on the org lock");

    let kicked = remove_member(&mut kick, org, member)
        .await
        .expect("the kick must not deadlock");
    assert_eq!(kicked.after, AfterRemoval::Unchanged);
    kick.commit().await.expect("the kick commits");

    assert!(
        delete
            .await
            .unwrap()
            .expect("the character delete must not deadlock")
            .deleted,
        "the character was deleted"
    );
    assert_eq!(member_ranks(&pool, org).await, vec![(leader, 8)]);

    teardown(&pool, &fx).await;
}
