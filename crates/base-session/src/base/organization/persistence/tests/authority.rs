//! The authorize-inside-the-lock rule and the database guards behind it
//! (PR #881 review round): `OrgAccess` is tied to its transaction, member
//! identity and the Leader rank are immutable outside the delete trigger,
//! ranks are per type, and every character delete keeps the lock order.

use std::time::Duration;

use cimmeria_entity::organization::{OrgRank, OrgType};
use tracing::Level;

use super::super::super::api::{member_access_locked, org_vault_is_empty, OrgAccess, SystemActor};
use super::super::super::character_delete::delete_character;
use super::super::{add_member, remove_member, set_rank, AfterRemoval, OrgStoreError};
use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// Poll until some other session in this database waits on a lock, so a
/// concurrency test cannot pass without its race.
async fn wait_until_blocked(pool: &PgPool) {
    for _ in 0..100 {
        if lock_waiters(pool).await > 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the concurrent statement never waited on a lock");
}

/// How many other sessions in this database wait on a lock.
async fn lock_waiters(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM pg_stat_activity \
         WHERE wait_event_type = 'Lock' AND datname = current_database() \
           AND pid <> pg_backend_pid()",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn add(pool: &PgPool, org_id: i32, player_id: i32, rank: OrgRank) {
    let mut tx = pool.begin().await.unwrap();
    as_sys!(add_member, tx, org_id, player_id, rank).unwrap();
    tx.commit().await.unwrap();
}

/// An access read in one transaction is refused in another
/// (`StaleAccess`); a GM system access logs `org.gm_action` with the GM.
#[tokio::test]
async fn access_is_tied_to_its_transaction() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 25, 2, &["Org02 Stale"]).await;
    let (p0, p1) = (fx.player(0), fx.player(1));
    let org = create(&pool, OrgType::Command, "Org02 Stale", p0)
        .await
        .org_id;
    add(&pool, org, p1, OrgRank::INITIATE).await;

    let mut first = pool.begin().await.unwrap();
    let leader = member_access_locked(&mut first, org, p0)
        .await
        .unwrap()
        .expect("p0 leads");
    first.commit().await.unwrap();

    let mut second = pool.begin().await.unwrap();
    let res = set_rank(&mut second, &leader, org, p1, OrgRank::OFFICER).await;
    assert!(matches!(res, Err(OrgStoreError::StaleAccess)), "{res:?}");
    let res = remove_member(&mut second, &leader, org, p1).await;
    assert!(matches!(res, Err(OrgStoreError::StaleAccess)), "{res:?}");

    // Read again under this transaction's lock, it works.
    let leader = member_access_locked(&mut second, org, p0)
        .await
        .unwrap()
        .unwrap();
    set_rank(&mut second, &leader, org, p1, OrgRank::OFFICER)
        .await
        .expect("fresh access");

    let capture = LogCapture::install();
    let gm = OrgAccess::system(
        &mut second,
        org,
        SystemActor::Gm {
            account_id: Some(fx.account_id),
            player_id: Some(p1),
            command: ".org_disband",
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert!(gm.is_system() && gm.rank() == OrgRank::LEADER);
    let logged = capture.all().into_iter().any(|c| {
        c.level == Level::INFO
            && c.has_field("event", "org.gm_action")
            && c.has_field("command", ".org_disband")
            && c.has_field("account_id", &fx.account_id.to_string())
            && c.has_field("org_id", &org.to_string())
    });
    assert!(logged, "{:#?}", capture.all());
    drop(capture);
    second.rollback().await.unwrap();

    teardown(&pool, &fx).await;
}

/// The member BEFORE UPDATE trigger: identity columns never change, and
/// the Leader rank moves only by the delete trigger's promotion.
#[tokio::test]
async fn member_identity_and_leader_rank_are_immutable_in_sql() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 26, 2, &["Org02 Immutable"]).await;
    let (p0, p1) = (fx.player(0), fx.player(1));
    let org = create(&pool, OrgType::Command, "Org02 Immutable", p0)
        .await
        .org_id;
    add(&pool, org, p1, OrgRank::INITIATE).await;

    for (sql, constraint) in [
        (
            "UPDATE sgw_organization_members SET rank = 8 WHERE org_id = $1 AND player_id = $2",
            "sgw_organization_members_leader_pinned",
        ),
        (
            "UPDATE sgw_organization_members SET rank = 7 WHERE org_id = $1 AND player_id <> $2",
            "sgw_organization_members_leader_pinned",
        ),
        (
            "UPDATE sgw_organization_members SET account_id = account_id + 1 \
             WHERE org_id = $1 AND player_id = $2",
            "sgw_organization_members_identity_immutable",
        ),
        (
            "UPDATE sgw_organization_members SET player_id = player_id + 100 \
             WHERE org_id = $1 AND player_id = $2",
            "sgw_organization_members_identity_immutable",
        ),
    ] {
        let err = sqlx::query(sql)
            .bind(org)
            .bind(p1)
            .execute(&pool)
            .await
            .expect_err(sql);
        assert_eq!(violated(&err).as_deref(), Some(constraint), "{sql}");
    }
    // An ordinary rank change still works.
    sqlx::query(
        "UPDATE sgw_organization_members SET rank = 6 WHERE org_id = $1 AND player_id = $2",
    )
    .bind(org)
    .bind(p1)
    .execute(&pool)
    .await
    .expect("a non-Leader rank change");
    // The promotion inside the delete trigger still works.
    let mut tx = pool.begin().await.unwrap();
    let r = as_sys!(remove_member, tx, org, p0).unwrap();
    assert_eq!(r.after, AfterRemoval::LeaderPromoted { player_id: p1 });
    tx.commit().await.unwrap();

    teardown(&pool, &fx).await;
}

/// D-ORG07 in the database: a rank row must be one its type uses, and its
/// org_type must be its organization's.
#[tokio::test]
async fn rank_rows_are_per_type_in_sql() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 27, 1, &["Org02 RankType"]).await;
    let team = create(&pool, OrgType::Team, "Org02 RankType", fx.player(0))
        .await
        .org_id;
    for (org_type, rank, constraint) in [
        (1_i16, 5_i16, "sgw_organization_ranks_rank_in_type_check"),
        (1, 1, "sgw_organization_ranks_rank_in_type_check"),
        (2, 5, "sgw_organization_ranks_org_fkey"),
    ] {
        let err = sqlx::query(
            "INSERT INTO sgw_organization_ranks (org_id, org_type, rank, permissions) \
             VALUES ($1, $2, $3, 0)",
        )
        .bind(team)
        .bind(org_type)
        .bind(rank)
        .execute(&pool)
        .await
        .expect_err("a rank the Team does not use must be refused");
        assert_eq!(
            violated(&err).as_deref(),
            Some(constraint),
            "type {org_type} rank {rank}"
        );
    }

    teardown(&pool, &fx).await;
}

/// The Rust vault predicate and its SQL twin must agree (both stubs today;
/// the Bank campaign replaces both).
#[tokio::test]
async fn vault_stubs_agree() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 28, 1, &["Org02 Vault Twins"]).await;
    let org = create(&pool, OrgType::Command, "Org02 Vault Twins", fx.player(0))
        .await
        .org_id;
    let mut tx = pool.begin().await.unwrap();
    let rust = org_vault_is_empty(&mut tx, org).await.unwrap();
    let sql: bool = sqlx::query_scalar("SELECT org_vault_is_empty_sql($1)")
        .bind(org)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(
        rust, sql,
        "api::org_vault_is_empty and org_vault_is_empty_sql disagree"
    );

    teardown(&pool, &fx).await;
}

/// An account delete cascades to its characters without the Rust
/// character-delete path. The `sgw_player` BEFORE DELETE trigger still
/// locks each character's organizations before the member rows cascade,
/// so a kick holding the organization finishes instead of deadlocking.
#[tokio::test]
async fn account_delete_cascade_keeps_the_lock_order() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 29, 2, &["Org02 Account"]).await;
    let (p0, p1) = (fx.player(0), fx.player(1));
    let org = create(&pool, OrgType::Command, "Org02 Account", p0)
        .await
        .org_id;
    add(&pool, org, p1, OrgRank::INITIATE).await;

    let mut kick = pool.begin().await.unwrap();
    let actor = sys(&mut kick, org).await;

    let delete_pool = pool.clone();
    let account_id = fx.account_id;
    let delete = tokio::spawn(async move {
        sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(&delete_pool)
            .await
            .map(|r| r.rows_affected())
    });
    wait_until_blocked(&pool).await;

    let kicked = remove_member(&mut kick, &actor, org, p0)
        .await
        .expect("the kick must not deadlock");
    assert_eq!(kicked.after, AfterRemoval::LeaderPromoted { player_id: p1 });
    kick.commit().await.expect("the kick commits");
    assert_eq!(
        delete
            .await
            .unwrap()
            .expect("the account delete must not deadlock"),
        1
    );
    assert!(
        !org_exists(&pool, org).await,
        "its last member went with it"
    );

    teardown(&pool, &fx).await;
}

/// Copilot #881 (_foreign_keys.sql): an account delete cascades to all its
/// characters in one statement. Locking organizations character by
/// character would take them out of `org_id` order: character A (in the
/// higher org) locks it, then waits on character B's row; a
/// single-character delete of C, a member of both, locks the lower org and
/// waits on the higher; B's turn then waits on the lower org: 40P01. The
/// `account` BEFORE DELETE trigger locks every character first, then every
/// organization in order, so both deletes finish.
#[tokio::test]
async fn account_delete_locks_all_its_characters_orgs_in_order() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 32, 2, &["Org02 Acct Lo", "Org02 Acct Hi"]).await;
    let other = setup(&pool, 33, 1, &[]).await;
    let (a, b, c) = (fx.player(0), fx.player(1), other.player(0));
    // Created in this order, so lo < hi.
    let lo = create(&pool, OrgType::Team, "Org02 Acct Lo", c)
        .await
        .org_id;
    let hi = create(&pool, OrgType::Command, "Org02 Acct Hi", c)
        .await
        .org_id;
    assert!(lo < hi);
    add(&pool, lo, b, OrgRank::MEMBER).await;
    add(&pool, hi, a, OrgRank::INITIATE).await;

    // Hold character B's row so the account delete stops between its
    // characters.
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT 1 FROM sgw_player WHERE player_id = $1 FOR UPDATE")
        .bind(b)
        .execute(&mut *blocker)
        .await
        .unwrap();

    let account_pool = pool.clone();
    let account_id = fx.account_id;
    let account_delete = tokio::spawn(async move {
        sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(&account_pool)
            .await
            .map(|r| r.rows_affected())
    });
    wait_until_blocked(&pool).await;

    let c_pool = pool.clone();
    let c_account = other.account_id;
    let c_delete = tokio::spawn(async move { delete_character(&c_pool, c, c_account).await });
    // With the fix C's delete finishes here; without it, it waits on the
    // higher org the account delete holds.
    for _ in 0..100 {
        if c_delete.is_finished() || lock_waiters(&pool).await >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    blocker.commit().await.unwrap();

    let c_deleted = c_delete
        .await
        .unwrap()
        .expect("the character delete must not deadlock");
    assert!(c_deleted.deleted);
    assert_eq!(
        account_delete
            .await
            .unwrap()
            .expect("the account delete must not deadlock"),
        1
    );
    assert!(!org_exists(&pool, lo).await && !org_exists(&pool, hi).await);

    teardown(&pool, &other).await;
    teardown(&pool, &fx).await;
}

/// Copilot #881 (character_delete.rs): a join into an organization the
/// character already belongs to, by a transaction holding that
/// organization, while the character is being deleted. `add_member`
/// checks membership before it touches the character's row, so it refuses
/// at once instead of waiting on the row the delete holds.
#[tokio::test]
async fn duplicate_join_during_character_delete_does_not_deadlock() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 30, 2, &["Org02 Join Race"]).await;
    let (p0, p1) = (fx.player(0), fx.player(1));
    let org = create(&pool, OrgType::Command, "Org02 Join Race", p0)
        .await
        .org_id;
    add(&pool, org, p1, OrgRank::INITIATE).await;

    let mut join = pool.begin().await.unwrap();
    let actor = sys(&mut join, org).await;

    let delete_pool = pool.clone();
    let account_id = fx.account_id;
    let delete = tokio::spawn(async move { delete_character(&delete_pool, p1, account_id).await });
    wait_until_blocked(&pool).await;

    let res = add_member(&mut join, &actor, org, p1, OrgRank::INITIATE).await;
    assert!(
        matches!(res, Err(OrgStoreError::AlreadyMember)),
        "must refuse without waiting on the deleted character: {res:?}"
    );
    join.commit().await.unwrap();
    let deletion = delete.await.unwrap().expect("the delete must not deadlock");
    assert!(deletion.deleted);
    assert_eq!(member_ranks(&pool, org).await, vec![(p0, 8)]);

    teardown(&pool, &fx).await;
}

/// Copilot #881 (character_delete.rs): a delete request for a character
/// another account owns locks nothing, not even the character's
/// organizations, and deletes nothing.
#[tokio::test]
async fn foreign_account_delete_takes_no_locks() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 31, 1, &["Org02 Foreign"]).await;
    let p0 = fx.player(0);
    let org = create(&pool, OrgType::Command, "Org02 Foreign", p0)
        .await
        .org_id;

    let mut holder = pool.begin().await.unwrap();
    sys(&mut holder, org).await;

    let deletion = tokio::time::timeout(
        Duration::from_secs(3),
        delete_character(&pool, p0, fx.account_id + 1),
    )
    .await
    .expect("another account's request must not wait on the org lock")
    .unwrap();
    assert!(!deletion.deleted);
    holder.rollback().await.unwrap();
    assert_eq!(member_ranks(&pool, org).await, vec![(p0, 8)]);

    teardown(&pool, &fx).await;
}
