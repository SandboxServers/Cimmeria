//! Live-DB guards: a grant that asks for the vault, or for a bag the item
//! does not allow, lands in the first carried bag the item lists.
//!
//! The seeded crafting components list `{17,15}`. Loot and content ask for
//! the cell cache's container (17 before the cache skipped storage, 15
//! now), and `gmGiveItem` asks for the main bag (1). All three must land in
//! the crafting bag (15). Removing the placement step sends the 17 request
//! to the vault guard (refused, no row) and the 1 request into the main
//! bag, and fails the first test.
//!
//! Sentinels: accounts/players `0x7000_C400..=0x7000_C405`, entities
//! `0x7000_C4E0..=0x7000_C4E2`.

use tracing::Level;

use super::*;
use crate::test_support::{require_db_or_skip, LogCapture, TestTransport};

pub(super) async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32, entity_id: u32) {
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
        .bind(entity_id as i32)
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

pub(super) async fn insert_account_and_player(pool: &PgPool, account_id: i32, player_id: i32) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("grant-fall-through-{account_id}"))
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

/// The first seeded item with this `container_sets` and stack limit.
pub(super) async fn seeded_item(pool: &PgPool, container_sets: &str, stackable: bool) -> i32 {
    sqlx::query_scalar(
        "SELECT item_id FROM resources.items \
         WHERE container_sets = $1::integer[] AND (max_stack_size > 1) = $2 \
         ORDER BY item_id LIMIT 1",
    )
    .bind(container_sets)
    .bind(stackable)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("the seed must hold a {container_sets} item: {e}"))
}

/// `(container_id, rows, summed stack)` per container the player holds.
pub(super) async fn bags(pool: &PgPool, player_id: i32) -> Vec<(i32, i64, i64)> {
    sqlx::query_as(
        "SELECT container_id, COUNT(*), SUM(stack_size)::bigint FROM sgw_inventory \
         WHERE character_id = $1 GROUP BY container_id ORDER BY container_id",
    )
    .bind(player_id)
    .fetch_all(pool)
    .await
    .expect("bags query")
}

pub(super) fn state() -> (
    Arc<dyn Transport>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    (
        Arc::new(TestTransport::new()),
        Arc::new(Mutex::new(HashMap::new())),
        Arc::new(Mutex::new(HashMap::new())),
    )
}

/// Loot (the old cache's 17 and the new cache's 15), content and
/// `gmGiveItem` (1): a `{17,15}` component lands in the crafting bag
/// whatever the caller asked for, and the choice is logged with the full
/// identity.
#[tokio::test]
async fn bank_first_component_lands_in_the_crafting_bag_for_every_caller() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C400, 0x7000_C401, 0x7000_C4E0_u32);
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let type_id = seeded_item(&pool, "{17,15}", false).await;
    let (transport, conn, e2a) = state();
    let db_pool = Some(Arc::new(pool.clone()));
    let capture = LogCapture::install();

    for requested in [17, 15, 1] {
        handle_grant_item(
            entity_id, player_id, type_id, requested, 1, false, &db_pool, &None, &transport, &conn,
            &e2a,
        )
        .await;
    }

    assert_eq!(
        bags(&pool, player_id).await,
        vec![(15, 3, 3)],
        "all three grants must land in the crafting bag, none in the main bag or the vault"
    );
    let event = capture
        .all()
        .into_iter()
        .find(|c| {
            c.level == Level::INFO
                && c.message_contains("grant_container_chosen")
                && c.has_field("requested_container_id", "17")
        })
        .expect("the vault request must log grant_container_chosen");
    assert_eq!(event.target, "inventory");
    for (key, value) in [
        ("event", "grant_container_chosen".to_string()),
        ("account_id", account_id.to_string()),
        ("player_id", player_id.to_string()),
        ("entity_id", entity_id.to_string()),
        ("type_id", type_id.to_string()),
        ("quantity", "1".to_string()),
        ("container_sets", "{17,15}".to_string()),
        ("skipped_storage", "true".to_string()),
        ("container_id", "15".to_string()),
        ("slot_id", "0".to_string()),
        ("qty_before", "0".to_string()),
        ("qty_after", "1".to_string()),
    ] {
        assert_eq!(event.fields.get(key), Some(&value), "field `{key}`");
    }
    assert!(
        capture
            .find_event(
                Level::WARN,
                "grant_rejected",
                "grant_into_storage_container"
            )
            .is_none(),
        "an item that lists a carried bag must not be refused"
    );

    cleanup(&pool, account_id, player_id, entity_id).await;
}

/// A second grant of a stackable `{17,15}` component merges into the
/// crafting-bag stack, and the event shows the stack before and after.
#[tokio::test]
async fn stack_merge_in_the_crafting_bag_logs_quantity_before_and_after() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C402, 0x7000_C403, 0x7000_C4E1_u32);
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let type_id = seeded_item(&pool, "{17,15}", true).await;
    let (transport, conn, e2a) = state();
    let db_pool = Some(Arc::new(pool.clone()));

    handle_grant_item(
        entity_id, player_id, type_id, 17, 1, false, &db_pool, &None, &transport, &conn, &e2a,
    )
    .await;
    let capture = LogCapture::install();
    handle_grant_item(
        entity_id, player_id, type_id, 17, 1, false, &db_pool, &None, &transport, &conn, &e2a,
    )
    .await;

    assert_eq!(bags(&pool, player_id).await, vec![(15, 1, 2)]);
    let event = capture
        .find_message(Level::INFO, "grant_container_chosen")
        .expect("the merge must log grant_container_chosen");
    for (key, value) in [
        ("container_id", "15"),
        ("slot_id", "0"),
        ("qty_before", "1"),
        ("qty_after", "2"),
    ] {
        assert_eq!(
            event.fields.get(key).map(String::as_str),
            Some(value),
            "field `{key}`"
        );
    }

    cleanup(&pool, account_id, player_id, entity_id).await;
}

/// The fall-through only moves requests the item does not allow: a weapon
/// given into the main bag stays there, and loot's bandolier request is
/// honoured.
#[tokio::test]
async fn allowed_requests_keep_their_container() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C404, 0x7000_C405, 0x7000_C4E2_u32);
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let weapon = seeded_item(&pool, "{3,1,17}", false).await;
    let (transport, conn, e2a) = state();
    let db_pool = Some(Arc::new(pool.clone()));
    let capture = LogCapture::install();

    handle_grant_item(
        entity_id, player_id, weapon, 1, 1, false, &db_pool, &None, &transport, &conn, &e2a,
    )
    .await;
    handle_grant_item(
        entity_id, player_id, weapon, 3, 1, false, &db_pool, &None, &transport, &conn, &e2a,
    )
    .await;

    assert_eq!(bags(&pool, player_id).await, vec![(1, 1, 1), (3, 1, 1)]);
    let event = capture
        .find_message(Level::INFO, "grant_container_chosen")
        .expect("grant_container_chosen");
    assert_eq!(
        event.fields.get("skipped_storage").map(String::as_str),
        Some("false")
    );

    cleanup(&pool, account_id, player_id, entity_id).await;
}
