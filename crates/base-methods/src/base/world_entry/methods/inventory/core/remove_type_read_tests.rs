//! NT-22 review guard: `remove_instance` reads the row's `type_id` only to
//! name the item on its log lines. That read must not change what a removal
//! does: a partial removal still decrements and a full one still deletes.
//!
//! `sgw_inventory` declares `type_id` without `NOT NULL`, but it inherits the
//! column from `sgw_inventory_base`, where it is `NOT NULL`, so no row can
//! hold a NULL type. The read is still decoded as `Option<i32>` so the log
//! line can never fail a removal; the second test pins the constraint that
//! makes the NULL case unreachable today.
//!
//! Live-DB; skips when `DATABASE_URL` is unset. Sentinels: account and
//! player `0x7000_D240..=0x7000_D241`, entity `0x7000_D2E4`. The typed rows
//! are the seeded Health Slappack TC1 (2893).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::vault::VaultAccess;
use sqlx::PgPool;

use super::access::AccessOp;
use super::remove_instance::{remove_instance, RemoveInstance};
use crate::test_support::{require_db_or_skip, TestTransport};

const ACCOUNT: i32 = 0x7000_D240;
const PLAYER: i32 = 0x7000_D241;
const ENTITY: u32 = 0x7000_D2E4;
const SLAPPACK: i32 = 2893;

async fn cleanup(pool: &PgPool) {
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
        .bind(i64::from(ENTITY))
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
        .bind(PLAYER)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACCOUNT)
        .execute(pool)
        .await;
}

async fn setup(pool: &PgPool) {
    cleanup(pool).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(ACCOUNT)
        .bind(format!("nt22-{ACCOUNT}"))
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, 0)",
    )
    .bind(ACCOUNT)
    .bind(PLAYER)
    .bind(format!("nt22-{PLAYER}"))
    .execute(pool)
    .await
    .expect("insert player");
}

async fn insert(pool: &PgPool, type_id: i32, stack: i32, slot: i32) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
         VALUES ($1, $2, $3, $4, 1, false, 100, 0) RETURNING item_id",
    )
    .bind(PLAYER)
    .bind(type_id)
    .bind(stack)
    .bind(slot)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

async fn stack_of(pool: &PgPool, item_id: i32) -> Option<i32> {
    sqlx::query_scalar("SELECT stack_size FROM sgw_inventory WHERE item_id = $1")
        .bind(item_id)
        .fetch_optional(pool)
        .await
        .expect("stack query")
}

async fn remove(pool: &PgPool, item_id: i32, quantity: i32) -> bool {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    remove_instance(
        RemoveInstance {
            entity_id: ENTITY,
            player_id: PLAYER,
            item_id,
            quantity,
            notify_gm: false,
            vault: VaultAccess::NO_SESSION,
            expected_type_id: None,
            op: AccessOp::Remove,
        },
        &Some(Arc::new(pool.clone())),
        &None,
        &transport,
        &Arc::new(Mutex::new(HashMap::new())),
        &Arc::new(Mutex::new(HashMap::new())),
    )
    .await
}

#[tokio::test]
async fn live_db_remove_keeps_its_behaviour_with_the_type_read() {
    let pool = require_db_or_skip!();
    setup(&pool).await;

    // Partial: 5 - 2 leaves 3, and the removal reports a commit.
    let stack = insert(&pool, SLAPPACK, 5, 0).await;
    assert!(remove(&pool, stack, 2).await, "partial removal commits");
    assert_eq!(stack_of(&pool, stack).await, Some(3));

    // Full: the rest deletes the row.
    assert!(remove(&pool, stack, 3).await, "full removal commits");
    assert_eq!(stack_of(&pool, stack).await, None);

    cleanup(&pool).await;
}

/// The schema rules a NULL `type_id` out (inherited `NOT NULL`). If that
/// constraint is ever dropped, this fails, and the removal test above
/// should gain a NULL-type row.
#[tokio::test]
async fn live_db_inventory_rows_cannot_hold_a_null_type() {
    let pool = require_db_or_skip!();
    setup(&pool).await;
    let inserted = sqlx::query(
        "INSERT INTO sgw_inventory             (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges)          VALUES ($1, NULL, 1, 1, 1, false, 100, 0)",
    )
    .bind(PLAYER)
    .execute(&pool)
    .await;
    let code = inserted.err().and_then(|e| {
        e.as_database_error()
            .and_then(|d| d.code().map(|c| c.into_owned()))
    });
    assert_eq!(
        code.as_deref(),
        Some("23502"),
        "not_null_violation expected"
    );
    cleanup(&pool).await;
}
