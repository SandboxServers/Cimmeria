//! The shapes of a Team or Command vault move (bank-vault BV-07b): split,
//! merge, a swap within the vault and a swap across it, each conserving
//! the count and never duplicating an `item_id`; and the concurrency guard
//! of two members moving onto one stack (TESTING.md types 3 and 5).

use std::time::{Duration, Instant};

use cimmeria_entity::cell_entity::VaultScope;

use super::moves::{at_banker, mv};
use super::*;
use crate::test_support::require_db_or_skip;

/// A split deposit leaves the rest in the bag and a new row (a fresh id) in
/// the vault; a merge withdrawal onto a carried stack of the same type
/// empties the vault row and deletes it. The total stays 7 throughout.
#[tokio::test]
async fn split_and_merge_conserve_the_count() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 9, 1).await;
    let team = fx.org(0, 0, &[]).await;
    let vault = at_banker(VaultScope::Team, team);
    let client = Client::in_world(fx.entity(0), 40879);
    let item = fx.item(0);
    fx.carry(0, item, 1, 0, BANKABLE, 7, false).await;

    mv(&fx, &client, 0, item, 19, 2, 3, vault).await;
    assert_eq!(fx.bag(0).await, vec![(item, 1, 0, 4)]);
    let rows = fx.vault(team).await;
    assert_eq!(rows.len(), 1);
    let (split, slot, stack) = rows[0];
    assert_ne!(split, item, "a split takes a new id");
    assert_eq!((slot, stack), (2, 3));

    // Merge the vault's 3 back onto the carried 4.
    mv(&fx, &client, 0, split, 1, 0, -1, vault).await;
    assert!(fx.vault(team).await.is_empty(), "the merged row is gone");
    assert_eq!(fx.bag(0).await, vec![(item, 1, 0, 7)]);
    assert!(fx.duplicated_ids().await.is_empty());
    let kinds: Vec<(String, String)> = fx
        .log(team)
        .await
        .into_iter()
        .map(|(d, k, ..)| (d, k))
        .collect();
    assert_eq!(
        kinds,
        vec![
            ("deposit".to_owned(), "split".to_owned()),
            ("withdraw".to_owned(), "merge".to_owned())
        ]
    );
    fx.teardown().await;
}

/// A swap of two vault slots (one statement, the deferrable slot key) and a
/// swap across the vault (the leader drags a carried item onto a vault
/// stack it cannot merge with): each item ends where the other was.
#[tokio::test]
async fn swaps_within_and_across_the_vault() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 10, 1).await;
    let team = fx.org(0, 0, &[]).await;
    let vault = at_banker(VaultScope::Team, team);
    let client = Client::in_world(fx.entity(0), 40880);
    let (a, b, c) = (fx.item(0), fx.item(1), fx.item(2));
    fx.put(team, a, 0, BANKABLE, 2).await;
    fx.put(team, b, 1, MISSION, 1).await;

    mv(&fx, &client, 0, a, 19, 1, -1, vault).await;
    assert_eq!(
        fx.vault(team).await,
        vec![(b, 0, 1), (a, 1, 2)],
        "swapped within"
    );

    // A carried stack of the same type but other charges cannot merge, so
    // it swaps with the vault's `a`.
    fx.carry(0, c, 1, 4, BANKABLE, 1, false).await;
    sqlx::query("UPDATE sgw_inventory SET charges = 5 WHERE item_id = $1")
        .bind(c)
        .execute(&pool)
        .await
        .unwrap();
    mv(&fx, &client, 0, c, 19, 1, -1, vault).await;
    assert_eq!(fx.vault(team).await, vec![(b, 0, 1), (c, 1, 1)]);
    assert_eq!(fx.bag(0).await, vec![(a, 1, 4, 2)]);
    assert!(fx.duplicated_ids().await.is_empty());
    let last = fx.log(team).await.pop().unwrap();
    assert_eq!((last.0.as_str(), last.1.as_str()), ("deposit", "swap"));
    fx.teardown().await;
}

/// Type 5: two members, each holding 2 of a type, drop them at the same
/// moment onto the vault's stack of 18 (max 20). The organization lock and
/// the occupant's row lock serialize them: exactly one merges, the vault
/// holds 20, and the other keeps its 2 (the fallback swap needs
/// `WithdrawBank`, which a Member lacks). Without the locks both read 18,
/// both merge, and the vault holds 22, past the stack limit.
///
/// The race is forced: the test holds `SHARE` on the vault table (which lets
/// the reads and `FOR UPDATE` through and blocks every write) until both
/// moves are parked behind it, counting only sessions the gate holds.
#[tokio::test]
async fn two_members_merging_onto_one_stack_cannot_overfill_it() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 11, 3).await;
    let team = fx.org(0, 2, &[0, 1]).await;
    let vault = at_banker(VaultScope::Team, team);
    let stack = fx.item(0);
    fx.put(team, stack, 0, BANKABLE, 18).await;
    let (x, y) = (fx.item(1), fx.item(2));
    fx.carry(0, x, 1, 0, BANKABLE, 2, false).await;
    fx.carry(1, y, 1, 0, BANKABLE, 2, false).await;
    let (ca, cb) = (
        Client::in_world(fx.entity(0), 40881),
        Client::in_world(fx.entity(1), 40882),
    );

    let mut gate = pool.begin().await.unwrap();
    let gate_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    sqlx::query("LOCK TABLE sgw_organization_vault_items IN SHARE MODE")
        .execute(&mut *gate)
        .await
        .unwrap();
    let release = async {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let waiting: i64 = sqlx::query_scalar(
                "WITH held AS (\
                     SELECT pid FROM pg_stat_activity WHERE $1 = ANY(pg_blocking_pids(pid))\
                 ) \
                 SELECT COUNT(*) FROM pg_stat_activity a \
                 WHERE a.pid IN (SELECT pid FROM held) \
                    OR EXISTS (SELECT 1 FROM held h WHERE h.pid = ANY(pg_blocking_pids(a.pid)))",
            )
            .bind(gate_pid)
            .fetch_one(&pool)
            .await
            .unwrap();
            if waiting >= 2 {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "both moves should be parked behind the gate (saw {waiting})"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        gate.commit().await.unwrap();
    };
    tokio::join!(
        mv(&fx, &ca, 0, x, 19, 0, -1, vault),
        mv(&fx, &cb, 1, y, 19, 0, -1, vault),
        release,
    );

    assert_eq!(
        fx.vault(team).await,
        vec![(stack, 0, 20)],
        "one merge, never 22"
    );
    let left: Vec<_> = [fx.bag(0).await, fx.bag(1).await].concat();
    assert_eq!(left.len(), 1, "exactly one member keeps their 2: {left:?}");
    assert_eq!(left[0].3, 2);
    assert!(fx.duplicated_ids().await.is_empty());
    assert_eq!(fx.log(team).await.len(), 1, "one logged move");
    fx.teardown().await;
}

/// The vault table carries every instance column `sgw_inventory` has, so an
/// `INSERT ... SELECT` between them loses nothing. Fails when a column is
/// added to one table and not the other (the move SQL in
/// `move_/org/apply.rs` must then name it too).
#[tokio::test]
async fn the_vault_table_carries_every_inventory_column() {
    let pool = require_db_or_skip!();
    let columns = |table: &'static str| {
        let pool = pool.clone();
        async move {
            let mut cols: Vec<String> = sqlx::query_scalar(
                "SELECT column_name::text FROM information_schema.columns \
                 WHERE table_schema = 'public' AND table_name = $1",
            )
            .bind(table)
            .fetch_all(&pool)
            .await
            .unwrap();
            cols.sort();
            cols
        }
    };
    let owner_only = ["character_id", "container_id", "slot_id", "item_id"];
    let vault_only = [
        "org_id",
        "org_type",
        "container_id",
        "slot_id",
        "item_id",
        "deposited_by_player_id",
        "deposited_at",
    ];
    let carried: Vec<String> = columns("sgw_inventory")
        .await
        .into_iter()
        .filter(|c| !owner_only.contains(&c.as_str()))
        .collect();
    let vaulted: Vec<String> = columns("sgw_organization_vault_items")
        .await
        .into_iter()
        .filter(|c| !vault_only.contains(&c.as_str()))
        .collect();
    assert!(!carried.is_empty());
    assert_eq!(
        carried, vaulted,
        "instance columns differ between the tables"
    );
}
