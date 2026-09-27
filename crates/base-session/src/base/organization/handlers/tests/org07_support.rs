//! Helpers the ORG-07 handler tests share: ranks, invites, and the one
//! outcome row.

use std::time::{Duration, Instant};

use tracing::Level;

use super::*;
use crate::base::organization::persistence::set_rank;
use crate::test_support::{Captured, LogCaptureGuard};

impl Fixture {
    /// Set character `i`'s rank in `org_id` as the server (no authority
    /// checks; the persistence rules still hold).
    pub(super) async fn set_rank_of(&self, org_id: i32, i: usize, rank: OrgRank) {
        let mut tx = self.pool.begin().await.unwrap();
        let actor = OrgAccess::system(
            &mut tx,
            org_id,
            SystemActor::Server {
                source: "org07_test",
            },
        )
        .await
        .unwrap()
        .unwrap();
        set_rank(&mut tx, &actor, org_id, self.player_id(i), rank)
            .await
            .expect("set_rank");
        tx.commit().await.unwrap();
    }

    /// Character `i`'s rank in `org_id`, if a member.
    pub(super) async fn rank_of(&self, org_id: i32, i: usize) -> Option<i16> {
        sqlx::query_scalar(
            "SELECT rank FROM sgw_organization_members WHERE org_id = $1 AND player_id = $2",
        )
        .bind(org_id)
        .bind(self.player_id(i))
        .fetch_optional(&self.pool)
        .await
        .unwrap()
    }

    /// The request ids character `i`'s session holds right now.
    pub(super) fn held_invites(&self, i: usize) -> usize {
        self.connected
            .lock()
            .unwrap()
            .get(&self.addr(i))
            .map_or(0, |c| {
                c.org_invites.pending_for(self.player_id(i), Instant::now())
            })
    }

    /// Make character `i`'s session ignore character `j` (SS-C1's cache, by
    /// character id and name).
    pub(super) fn ignore(&self, i: usize, j: usize) {
        let mut clients = self.connected.lock().unwrap();
        let s = clients.get_mut(&self.addr(i)).unwrap();
        s.ignore = crate::base::contact_list::ignore::IgnoreCache::with_player_ids(
            [self.name(j)].into_iter().collect(),
            [self.player_id(j)].into_iter().collect(),
        );
    }

    /// Clear everything captured so far.
    pub(super) fn clear_sent(&self) {
        self.typed.clear();
    }
}

/// The one `event` row in `capture` (asserted exactly one), at INFO on
/// `org`, with `outcome` and, for a refusal, `reason`.
pub(super) fn one_row(
    capture: &LogCaptureGuard,
    event: &str,
    outcome: &str,
    reason: Option<&str>,
) -> Captured {
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "org" && c.has_field("event", event))
        .collect();
    assert_eq!(rows.len(), 1, "exactly one {event} row: {rows:#?}");
    let row = rows.into_iter().next().unwrap();
    assert_eq!(row.level, Level::INFO, "{row:?}");
    assert!(row.has_field("outcome", outcome), "{row:?}");
    if let Some(r) = reason {
        assert!(row.has_field("reason", r), "{row:?}");
    }
    row
}

/// Poll until at least `n` other sessions in this database wait on a lock,
/// so a race test cannot pass without its race.
pub(super) async fn wait_for_lock_waiters(pool: &PgPool, n: i64) {
    for _ in 0..200 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity \
             WHERE wait_event_type = 'Lock' AND datname = current_database() \
               AND pid <> pg_backend_pid()",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        if waiting >= n {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("fewer than {n} statements ever waited on a lock");
}
