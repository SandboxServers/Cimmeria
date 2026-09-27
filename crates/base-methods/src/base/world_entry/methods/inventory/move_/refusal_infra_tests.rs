//! Negative-log guards (TESTING.md type 12) for the infrastructure reasons
//! of a refused move: the database failures between the refusal and the
//! snap-back. Each must log `move_rejected` or `move_resync_skipped` under
//! `bank` with its stable `reason=` and the player's correlators.
//!
//! `move_lock_begin_failed` and `refusal_context_query_failed` run against a
//! pool that can never connect, so they need no database. `move_lock_failed`
//! needs a real lock held by another connection and a `lock_timeout`, so it
//! is live-DB. Every lock failure ends in `move_resync_skipped
//! reason=lock_timeout` and no packet: an unlocked resend could overtake a
//! concurrent write. Two reasons have no guard, because each needs the
//! connection to fail after both locks were taken on it, which nothing can
//! inject: `resync_read_failed` (the item read under the locks) and
//! `move_lock_release_failed` (the rollback after the packet has gone).
//!
//! Sentinels: accounts/players `0x7000_B1C2..=0x7000_B1C5` (and player
//! `0x7000_B1C7`, item `0x7000_B1D0` for the no-database test), entities
//! `0x7000_B1EA..=0x7000_B1EC`, ports 40821-40823.

use std::time::Duration;

use tracing::Level;

use super::allowlist_tests::{
    assert_fields, cleanup_all, connected_client, insert_synth_item_type, SYNTH_TYPE_ID,
};
use super::container_policy::{refuse_move, Movable, MoveEnd};
use super::tests::{insert_account_and_player, insert_item};
use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// A pool whose every acquire fails fast: nothing listens on port 1.
fn unreachable_pool() -> Arc<PgPool> {
    Arc::new(
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_millis(50))
            .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/none")
            .expect("connect_lazy must succeed for any well-formed URL"),
    )
}

/// With the database unreachable, a refusal still logs every step it could
/// not take: the lock transaction (`move_lock_begin_failed`), the context
/// read (`refusal_context_query_failed`) and the skipped snap-back
/// (`move_resync_skipped reason=lock_timeout`), each with the player and
/// entity, and it sends nothing.
#[tokio::test]
async fn refusal_with_the_database_down_logs_each_failed_step() {
    let entity_id: u32 = 0x7000_B1EA;
    let player_id = 0x7000_B1C7;
    let item_id = 0x7000_B1D0;
    let (transport, dyn_transport, addr, e2a, conn) = connected_client(entity_id, 40821);
    let pool = unreachable_pool();
    let capture = LogCapture::install();

    refuse_move(
        MoveEnd::Target,
        Movable::VaultSession,
        entity_id,
        player_id,
        item_id,
        -1,
        17,
        0,
        &pool,
        &dyn_transport,
        &conn,
        &e2a,
    )
    .await;

    let correlators = [
        ("player_id", player_id.to_string()),
        ("entity_id", entity_id.to_string()),
        ("item_id", item_id.to_string()),
    ];
    for (event, reason) in [
        ("move_rejected", "move_lock_begin_failed"),
        ("move_rejected", "refusal_context_query_failed"),
        ("move_resync_skipped", "lock_timeout"),
    ] {
        let found = capture
            .find_event(Level::WARN, event, reason)
            .unwrap_or_else(|| panic!("{event} reason={reason} must be logged"));
        assert_eq!(found.target, "bank", "{reason}");
        let mut present = vec![("event", event.to_string())];
        present.extend(correlators.iter().cloned());
        // The account is read by the query that failed, so it is unknown.
        assert_fields(&found, &present, &["account_id"]);
    }
    assert_eq!(
        transport.send_count_to(addr),
        0,
        "nothing is resent without the locks"
    );
}

/// Hold `lock_sql` in another transaction, run a refusal on a pool whose
/// `lock_timeout` is short. The refusal must give up on the lock, log
/// `move_lock_failed`, still log the refusal itself, and then skip the
/// snap-back (`move_resync_skipped reason=lock_timeout`) and send nothing:
/// the write holding the lock may be about to send its own update, and an
/// unlocked resend could overtake it.
async fn refusal_under_a_held_lock(
    account_id: i32,
    player_id: i32,
    entity_id: u32,
    port: u16,
    lock_sql: &'static str,
) {
    let pool = require_db_or_skip!();
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    insert_synth_item_type(&pool).await;
    let item = insert_item(&pool, player_id, SYNTH_TYPE_ID, 1, 0, 1).await;

    let options = (*pool.connect_options())
        .clone()
        .options([("lock_timeout", "200ms")]);
    let impatient = Arc::new(
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .expect("connect the lock-timeout pool"),
    );

    let mut holder = pool.begin().await.expect("begin the lock holder");
    sqlx::query(lock_sql)
        .bind(player_id)
        .bind(item)
        .execute(&mut *holder)
        .await
        .expect("take the lock the refusal will wait on");

    let (transport, dyn_transport, addr, e2a, conn) = connected_client(entity_id, port);
    let capture = LogCapture::install();
    refuse_move(
        MoveEnd::Target,
        Movable::VaultSession,
        entity_id,
        player_id,
        item,
        -1,
        17,
        0,
        &impatient,
        &dyn_transport,
        &conn,
        &e2a,
    )
    .await;

    let failed = capture
        .find_event(Level::WARN, "move_rejected", "move_lock_failed")
        .expect("a lock that times out must log move_lock_failed");
    assert_eq!(failed.target, "bank");
    assert_fields(
        &failed,
        &[
            ("event", "move_rejected".into()),
            ("player_id", player_id.to_string()),
            ("entity_id", entity_id.to_string()),
            ("item_id", item.to_string()),
        ],
        &[],
    );
    assert!(
        capture
            .find_event(
                Level::WARN,
                "move_rejected",
                "target_container_needs_vault_session"
            )
            .is_some(),
        "the refusal itself is still logged"
    );
    let skipped = capture
        .find_event(Level::WARN, "move_resync_skipped", "lock_timeout")
        .expect("a refusal that could not lock must log move_resync_skipped reason=lock_timeout");
    assert_eq!(skipped.target, "bank");
    assert_fields(
        &skipped,
        &[
            ("event", "move_resync_skipped".into()),
            ("account_id", account_id.to_string()),
            ("player_id", player_id.to_string()),
            ("entity_id", entity_id.to_string()),
            ("item_id", item.to_string()),
        ],
        &[],
    );
    assert_eq!(
        transport.send_count_to(addr),
        0,
        "no unlocked snap-back may be sent"
    );

    holder.rollback().await.expect("release the held lock");
    cleanup_all(&pool, account_id, player_id).await;
}

/// `move_lock_failed` then `move_resync_skipped reason=lock_timeout` on the
/// per-player move lock: another move holds `(player, 0)`.
#[tokio::test]
async fn refusal_logs_move_lock_failed_when_the_move_lock_times_out() {
    refusal_under_a_held_lock(
        0x7000_B1C2,
        0x7000_B1C3,
        0x7000_B1EB,
        40822,
        // `$2` is bound but unused, so both lock shapes share one helper.
        "SELECT pg_advisory_xact_lock($1, 0), $2::integer",
    )
    .await;
}

/// `move_lock_failed` then `move_resync_skipped reason=lock_timeout` on the
/// refused item's row lock: a write to the row (a grant's stack merge, a
/// remove, a trade) is still open.
#[tokio::test]
async fn refusal_logs_move_lock_failed_when_the_item_row_lock_times_out() {
    refusal_under_a_held_lock(
        0x7000_B1C4,
        0x7000_B1C5,
        0x7000_B1EC,
        40823,
        "SELECT 1 FROM sgw_inventory WHERE character_id = $1 AND item_id = $2 FOR UPDATE",
    )
    .await;
}
