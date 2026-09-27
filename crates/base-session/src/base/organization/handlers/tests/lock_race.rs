//! ORG-LOCK (D-ORG04), TESTING.md type 5: a kick racing a transaction that
//! holds the organization lock never lets the kicked member act.
//!
//! A holder transaction takes `lock_org` and keeps it. The Leader's kick of
//! B queues on the row lock first; B's own kick of C queues behind it. When
//! the holder commits, Postgres grants the row to the waiters in order: the
//! kick removes B and commits, then B's action reads B's access **under the
//! lock** and finds no membership. Were B authorized from a read taken
//! before the lock, B's kick of C would go through.

use super::org07_support::wait_for_lock_waiters;
use super::*;
use crate::base::organization::api::lock_org;
use crate::base::organization::handlers::{handle_kick, OrgReject};
use crate::test_support::require_db_or_skip;

#[tokio::test]
async fn kick_racing_a_held_lock_never_lets_the_kicked_member_act() {
    let pool = require_db_or_skip!();
    let fx = Arc::new(Fixture::org07(&pool, 30, 3, &["Org07 Race"]).await);
    let cmd = fx.org(OrgType::Command, "Org07 Race", 0, &[1, 2]).await;
    // B (1) is an Officer, holding `Eject` and above C (2).
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    for i in 0..3 {
        fx.online(i);
    }

    let mut holder = pool.begin().await.unwrap();
    lock_org(&mut holder, cmd).await.unwrap().expect("org");

    let leader_kicks_b = {
        let fx = fx.clone();
        tokio::spawn(async move { handle_kick(&fx.ctx(), &fx.player(0), cmd, &fx.name(1)).await })
    };
    wait_for_lock_waiters(&pool, 1).await;
    let b_kicks_c = {
        let fx = fx.clone();
        tokio::spawn(async move { handle_kick(&fx.ctx(), &fx.player(1), cmd, &fx.name(2)).await })
    };
    wait_for_lock_waiters(&pool, 2).await;
    holder.commit().await.unwrap();

    assert_eq!(leader_kicks_b.await.unwrap(), Ok(fx.player_id(1)));
    assert_eq!(
        b_kicks_c.await.unwrap(),
        Err(OrgReject::NotMember),
        "the kicked Officer acted after the kick"
    );
    assert_eq!(
        fx.member_ids(cmd).await,
        vec![fx.player_id(0), fx.player_id(2)]
    );
    assert_eq!(fx.rank_of(cmd, 2).await, Some(1), "C untouched");
    fx.teardown().await;
}

/// ORG-05's carried gap, fixed in ORG-07: an invite accepted into a Team
/// while the same character founds a Team must not burn an organization
/// id. The accept's `add_member` holds the character's creation lock until
/// it commits, so the founding waits, then its pre-check sees the new
/// membership and refuses before the id sequence moves. Without the lock
/// the founding passes its pre-check, draws an id, and only then fails on
/// the member key.
#[tokio::test]
async fn an_accept_racing_a_creation_burns_no_org_id() {
    use crate::base::organization::persistence::{add_member, create_org, OrgStoreError};

    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 31, 2, &["Org07 Accepting", "Org07 Founding"]).await;
    let team = fx.org(OrgType::Team, "Org07 Accepting", 0, &[]).await;
    let seq = || async {
        sqlx::query_as::<_, (i64, bool)>(
            "SELECT last_value, is_called FROM sgw_organizations_org_id_seq",
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    };

    // The accept, mid-transaction: character 1 added, not yet committed.
    let mut accept = pool.begin().await.unwrap();
    let access = OrgAccess::system(
        &mut accept,
        team,
        SystemActor::Server {
            source: "org07_test",
        },
    )
    .await
    .unwrap()
    .unwrap();
    add_member(&mut accept, &access, team, fx.player_id(1), OrgRank::MEMBER)
        .await
        .expect("add_member");
    let before = seq().await;

    let founding = {
        let pool = pool.clone();
        let founder = fx.player_id(1);
        tokio::spawn(async move {
            let mut tx = pool.begin().await.unwrap();
            let r = create_org(&mut tx, OrgType::Team, "Org07 Founding", founder).await;
            tx.rollback().await.unwrap();
            r.map(|c| c.org_id)
        })
    };
    wait_for_lock_waiters(&pool, 1).await;
    accept.commit().await.unwrap();

    assert!(
        matches!(founding.await.unwrap(), Err(OrgStoreError::AlreadyInType)),
        "the founder is already in a Team"
    );
    assert_eq!(
        seq().await,
        before,
        "a refused founding must not draw an organization id"
    );
    assert_eq!(fx.rank_of(team, 1).await, Some(2));
    fx.teardown().await;
}
