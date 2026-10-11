//! TG-ITM-10 review guard: the removal's persisted row carries the stack
//! counts, the origin and the container, so a partial decrement can be read
//! from the log alone.
//!
//! Live-DB; skips when `DATABASE_URL` is unset. Sentinels: account and
//! player `0x7000_D260..=0x7000_D261`, entity `0x7000_D2E6`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::vault::VaultAccess;
use sqlx::PgPool;

use super::access::AccessOp;
use super::remove_instance::{remove_instance, RemoveInstance};
use crate::test_support::{require_db_or_skip, LogCapture, TestTransport};

const ACCOUNT: i32 = 0x7000_D260;
const PLAYER: i32 = 0x7000_D261;
const ENTITY: u32 = 0x7000_D2E6;
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
        .bind(format!("tg-itm-10-{ACCOUNT}"))
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
    .bind(format!("tg-itm-10-{PLAYER}"))
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
async fn partial_remove_logs_stack_counts() {
    let pool = require_db_or_skip!();
    setup(&pool).await;
    let stack = insert(&pool, SLAPPACK, 3, 0).await;
    let capture = LogCapture::install();

    assert!(remove(&pool, stack, 1).await, "partial removal commits");

    // The persisted row is DEBUG; a reverted or renamed field fails here.
    let line = capture
        .find_message(tracing::Level::DEBUG, "Inventory remove persisted")
        .expect("a committed removal must log its persisted row");
    assert!(line.has_field("stack_before", "3"), "{line:#?}");
    assert!(line.has_field("stack_after", "2"), "{line:#?}");
    assert!(line.has_field("removed_all", "false"), "{line:#?}");
    assert!(line.has_field("origin", "remove"), "{line:#?}");
    assert!(line.has_field("container_id", "1"), "{line:#?}");

    cleanup(&pool).await;
}
