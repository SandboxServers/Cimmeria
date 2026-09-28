//! Live-DB guards for `handle_consume_item_for_use`, the base half of the
//! native consumable round trip: one unit per request, the answer only for
//! a unit that was really taken, and the design-id guard.
//!
//! Sentinels: the consumable block `0x7000_D100`..`0x7000_D13F` (account
//! `0x7000_D100`, player `0x7000_D101`, entity `0x7000_D101`). The
//! `cimmeria-services` round trip uses `0x7000_D140`..`0x7000_D17F`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::vault::VaultAccess;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::handle_consume_item_for_use;
use crate::base::ConnectedClientState;
use crate::cell::messages::{BaseToCellMsg, ConsumeItemForUse, ItemUseConsumed};
use crate::test_support::{require_db_or_skip, TestTransport};

const ACCOUNT: i32 = 0x7000_D100;
const PLAYER: i32 = 0x7000_D101;
const ENTITY: u32 = 0x7000_D101;
/// Health Slappack TC1, a stackable (10) native consumable.
const SLAPPACK: i32 = 2893;
/// Mark III Stimpack: Coordination, a native consumable of another type.
const STIM: i32 = 6677;

async fn cleanup(pool: &PgPool) {
    for sql in [
        "DELETE FROM cell_event_outbox WHERE entity_id = $1",
        "DELETE FROM sgw_inventory WHERE character_id = $1",
    ] {
        sqlx::query(sql).bind(PLAYER).execute(pool).await.unwrap();
    }
    sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACCOUNT)
        .execute(pool)
        .await
        .unwrap();
}

async fn seed(pool: &PgPool) {
    cleanup(pool).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(ACCOUNT)
        .bind(format!("consume-{ACCOUNT}"))
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
    .bind(format!("consume-{PLAYER}"))
    .execute(pool)
    .await
    .expect("insert player");
}

/// A stack of `stack_size` of `type_id` in the main bag at `slot_id`.
async fn stack(pool: &PgPool, type_id: i32, stack_size: i32, slot_id: i32) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
         VALUES ($1, $2, $3, $4, 1, false, 100, 0) RETURNING item_id",
    )
    .bind(PLAYER)
    .bind(type_id)
    .bind(stack_size)
    .bind(slot_id)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

async fn stack_size(pool: &PgPool, instance: i32) -> Option<i32> {
    sqlx::query_scalar(
        "SELECT stack_size FROM sgw_inventory WHERE character_id = $1 AND item_id = $2",
    )
    .bind(PLAYER)
    .bind(instance)
    .fetch_optional(pool)
    .await
    .unwrap()
}

struct Base {
    transport: Arc<dyn Transport>,
    e2a: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    conn: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pool: Option<Arc<PgPool>>,
}

impl Base {
    fn new(pool: &PgPool) -> Base {
        let addr: SocketAddr = "127.0.0.1:65534".parse().unwrap();
        Base {
            transport: Arc::new(TestTransport::new()),
            e2a: Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)]))),
            conn: Arc::new(Mutex::new(HashMap::new())),
            pool: Some(Arc::new(pool.clone())),
        }
    }

    /// Run one consume and return every `ItemUseConsumed` the cell got.
    async fn consume(&self, instance_id: i32, type_id: i32) -> Vec<ItemUseConsumed> {
        let (tx, mut rx) = mpsc::channel(16);
        handle_consume_item_for_use(
            ConsumeItemForUse {
                entity_id: ENTITY,
                player_id: PLAYER,
                instance_id,
                type_id,
                vault: VaultAccess::NO_SESSION,
            },
            &self.pool,
            &Some(tx),
            &self.transport,
            &self.conn,
            &self.e2a,
        )
        .await;
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let BaseToCellMsg::ItemUseConsumed(c) = msg {
                out.push(c);
            }
        }
        out
    }
}

#[tokio::test]
async fn live_db_consume_for_use_takes_one_unit_and_answers_once() {
    let pool = require_db_or_skip!();
    seed(&pool).await;
    let instance = stack(&pool, SLAPPACK, 3, 0).await;
    let base = Base::new(&pool);

    let answers = base.consume(instance, SLAPPACK).await;
    assert_eq!(
        answers,
        vec![ItemUseConsumed {
            entity_id: ENTITY,
            player_id: PLAYER,
            instance_id: instance,
            type_id: SLAPPACK,
        }]
    );
    assert_eq!(
        stack_size(&pool, instance).await,
        Some(2),
        "exactly one unit"
    );
    cleanup(&pool).await;
}

/// The double-click on the last unit: the second consume finds no row and
/// must not answer, or the cell would apply the effect twice for one unit.
#[tokio::test]
async fn live_db_consuming_the_last_unit_twice_answers_once() {
    let pool = require_db_or_skip!();
    seed(&pool).await;
    let instance = stack(&pool, SLAPPACK, 1, 0).await;
    let base = Base::new(&pool);

    let first = base.consume(instance, SLAPPACK).await;
    let second = base.consume(instance, SLAPPACK).await;
    assert_eq!(first.len(), 1);
    assert!(
        second.is_empty(),
        "no unit left, so no effect may be applied: {second:?}"
    );
    assert_eq!(stack_size(&pool, instance).await, None, "the row is gone");
    cleanup(&pool).await;
}

/// The consume is held to the design id the cell resolved the effect for:
/// an instance of another type is neither consumed nor answered.
#[tokio::test]
async fn live_db_consume_for_use_refuses_an_instance_of_another_type() {
    let pool = require_db_or_skip!();
    seed(&pool).await;
    let instance = stack(&pool, STIM, 1, 0).await;
    let base = Base::new(&pool);

    let answers = base.consume(instance, SLAPPACK).await;
    assert!(answers.is_empty(), "{answers:?}");
    assert_eq!(
        stack_size(&pool, instance).await,
        Some(1),
        "nothing consumed"
    );
    cleanup(&pool).await;
}

/// An instance id the player has no row for is neither consumed nor
/// answered.
#[tokio::test]
async fn live_db_consume_for_use_of_a_missing_instance_answers_nothing() {
    let pool = require_db_or_skip!();
    seed(&pool).await;
    // A row owned by nobody we seeded: an id past every real one.
    let foreign: i32 =
        sqlx::query_scalar("SELECT COALESCE(MAX(item_id), 10000) + 1 FROM sgw_inventory")
            .fetch_one(&pool)
            .await
            .unwrap();
    let base = Base::new(&pool);
    assert!(base.consume(foreign, SLAPPACK).await.is_empty());
    cleanup(&pool).await;
}
