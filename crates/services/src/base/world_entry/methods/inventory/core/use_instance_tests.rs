//! Live-DB regression guards for `handle_use_inventory_item`'s auto-equip
//! routing. The router gates on bandolier-eligibility
//! (`3 = ANY(container_sets)`); these tests exercise the two sides of
//! that gate against real seed rows so a future regression — including
//! the original `clip_size IS NOT NULL` misinference — fails loudly.
//!
//! Skip cleanly when `DATABASE_URL` is unset.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::handle_use_inventory_item;
use crate::base::ConnectedClientState;
use crate::test_support::{require_db_or_skip, TestTransport};

/// Sentinel base — picks a window well above the move-handler tests
/// (`TEST_BASE = 0x7000_0200`) and well below `i32::MAX` so it can't
/// collide with player_id auto-increment.
const TEST_BASE: i32 = 0x7000_0400;

/// Slappack TC1 — `clip_size = 0`, `container_sets = '{1, 17}'`. Pinned
/// to the seed row at `db/resources/Items/Seed/items.sql:4879`. If that
/// row's container_sets ever gains `3`, this constant must move to a
/// different consumable.
const SLAPPACK_TYPE_ID: i32 = 2893;

/// SI 3 9mm Pistol — `clip_size = 15`, `container_sets = '{3, 1, 17}'`.
/// Pinned to the seed row at `db/resources/Items/Seed/items.sql`. The
/// canonical bandolier-eligible test item.
const PISTOL_TYPE_ID: i32 = 3241;

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

async fn insert_account_and_player(pool: &PgPool, account_id: i32, player_id: i32) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("use-inv-{account_id}"))
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
    .bind(account_id)
    .bind(player_id)
    .bind(format!("test-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
}

async fn insert_item(
    pool: &PgPool,
    player_id: i32,
    type_id: i32,
    container_id: i32,
    slot_id: i32,
) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges) \
         VALUES ($1, $2, 1, $3, $4, false, 100, 0) \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(slot_id)
    .bind(container_id)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

async fn location_of(pool: &PgPool, player_id: i32, item_id: i32) -> Option<(i32, i32)> {
    sqlx::query_as::<_, (i32, i32)>(
        "SELECT container_id, slot_id FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(pool)
    .await
    .expect("location_of query")
}

async fn count_item_used_outbox_rows(pool: &PgPool, entity_id: u32) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cell_event_outbox \
         WHERE entity_id = $1 AND event_type = 'item_used'",
    )
    .bind(entity_id as i32)
    .fetch_one(pool)
    .await
    .expect("count outbox rows")
}

fn make_state(
    entity_id: u32,
) -> (
    Arc<dyn Transport>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let fake_addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(entity_id, fake_addr);
        m
    }));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    (transport, entity_to_addr, connected)
}

/// Slappack in main bag → useItem MUST fire `OnItemUse` (enqueue an
/// `ItemUsed` outbox row) instead of being routed to the move handler.
///
/// Bug shape this guards against: the original
/// `(clip_size IS NOT NULL) AS is_weapon` SQL treated `clip_size = 0`
/// (the slappack's seed value) as truthy and routed the slappack to the
/// move handler with target=bandolier. The move handler then correctly
/// rejected (container_sets doesn't include 3), so the visible failure
/// was "slappack click does nothing" — no heal, no consumption, no chain
/// firing. This test pins the fix at the routing layer; the actual heal
/// is exercised by chain 4001 in the consumables chain replay tests.
#[tokio::test]
async fn slappack_use_fires_on_item_use_not_auto_equip() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE;
    let player_id = TEST_BASE + 1;
    let entity_id = (TEST_BASE + 1) as u32;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    // Slappack in main bag, slot 0.
    let item_id = insert_item(&pool, player_id, SLAPPACK_TYPE_ID, 1, 0).await;

    let (transport, e2a, conn) = make_state(entity_id);
    let db_pool = Some(Arc::new(pool.clone()));
    // No cell channel — the test cares about the outbox enqueue, not the
    // dispatch. The router shouldn't try to move the item regardless.
    let cell_tx: Option<mpsc::Sender<crate::cell::messages::BaseToCellMsg>> = None;

    handle_use_inventory_item(
        entity_id, player_id, item_id, 0, &db_pool, &cell_tx, &transport, &conn, &e2a,
    )
    .await;

    // The slappack must stay in container 1, slot 0 — the move handler
    // would have either moved it to container 3 or rejected it with a
    // warning, but EITHER outcome is wrong for a consumable.
    assert_eq!(
        location_of(&pool, player_id, item_id).await,
        Some((1, 0)),
        "slappack must remain in (container=1, slot=0); the auto-equip \
         router incorrectly tried to relocate it",
    );

    // The `OnItemUse` path enqueues exactly one `item_used` outbox row.
    // Routing to the move handler would have skipped this enqueue.
    assert_eq!(
        count_item_used_outbox_rows(&pool, entity_id).await,
        1,
        "slappack useItem must enqueue exactly one `item_used` outbox \
         row — the auto-equip router incorrectly bypassed `OnItemUse` \
         and the consumables chain (4001) never fired",
    );

    cleanup(&pool, account_id, player_id).await;
}

/// Pistol in main bag → useItem MUST route to the move handler
/// (auto-equip to bandolier slot 0). Confirms the bandolier-eligibility
/// check still works correctly for actual weapons after the fix —
/// without this positive test, a future "always fire OnItemUse"
/// over-correction would silently regress right-click-to-equip.
#[tokio::test]
async fn pistol_use_routes_to_auto_equip_not_on_item_use() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 100;
    let player_id = TEST_BASE + 101;
    let entity_id = (TEST_BASE + 101) as u32;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    // Pistol in main bag, slot 0.
    let item_id = insert_item(&pool, player_id, PISTOL_TYPE_ID, 1, 0).await;

    let (transport, e2a, conn) = make_state(entity_id);
    let db_pool = Some(Arc::new(pool.clone()));
    let cell_tx: Option<mpsc::Sender<crate::cell::messages::BaseToCellMsg>> = None;

    handle_use_inventory_item(
        entity_id, player_id, item_id, 0, &db_pool, &cell_tx, &transport, &conn, &e2a,
    )
    .await;

    // The pistol must have relocated to the first empty bandolier slot
    // (container 3, slot 0).
    assert_eq!(
        location_of(&pool, player_id, item_id).await,
        Some((3, 0)),
        "pistol useItem must auto-equip to bandolier (container=3, slot=0); \
         the bandolier-eligibility check failed to route to the move handler",
    );

    // The move-handler route does NOT enqueue an `item_used` outbox row.
    // Pinning this distinguishes the two paths cleanly.
    assert_eq!(
        count_item_used_outbox_rows(&pool, entity_id).await,
        0,
        "pistol useItem must NOT enqueue `item_used` — that path fires \
         only for non-bandolier-eligible items",
    );

    cleanup(&pool, account_id, player_id).await;
}
