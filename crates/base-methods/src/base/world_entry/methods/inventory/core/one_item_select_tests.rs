//! Guards for the one-item inventory read the move refusal uses.
//!
//! `moveItem` is client-callable, so a refused move must cost one indexed row
//! however large the inventory is: the owner check and the item filter live
//! in SQL (`INVENTORY_ONE_ITEM_SELECT`), not in a Rust filter over the whole
//! inventory.
//!
//! Sentinels: accounts `0x7000_B1A0` / `0x7000_B1B0`, players `0x7000_B1A1`
//! / `0x7000_B1B1`, entity `0x7000_B1E9`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::{send_inventory_item_update_via, INVENTORY_ITEM_SELECT, INVENTORY_ONE_ITEM_SELECT};
use crate::base::ConnectedClientState;
use crate::test_support::{require_db_or_skip, test_default_connected_client_state, TestTransport};

/// Both selects are built from one shared head, so the row layout the
/// refusal decodes is the layout every other inventory read decodes. Only
/// the filter differs.
#[test]
fn one_item_select_shares_the_row_layout_and_filters_in_sql() {
    let head_end = INVENTORY_ITEM_SELECT
        .find("WHERE ")
        .expect("the full select has a WHERE");
    let head = &INVENTORY_ITEM_SELECT[..head_end];
    assert!(
        INVENTORY_ONE_ITEM_SELECT.starts_with(head),
        "the one-item select must share the full select's columns and joins"
    );
    assert_eq!(
        &INVENTORY_ONE_ITEM_SELECT[head_end..],
        "WHERE inv.character_id = $1 AND inv.item_id = $2\n",
        "owner check and item filter both in SQL, no ORDER BY"
    );
}

/// The refusal's read must be the filtered select with `fetch_optional`,
/// not the full select filtered in Rust. Pinned on the source because the
/// two produce the same packet; only the database cost differs.
#[test]
fn send_inventory_item_update_via_reads_one_row() {
    let source = include_str!("mod.rs");
    let start = source
        .find("pub(crate) async fn send_inventory_item_update_via")
        .expect("function present");
    let body = &source[start..];
    let body = &body[..body
        .find("\n}\n")
        .or_else(|| body.find("\r\n}\r\n"))
        .expect("function end")];
    assert!(
        body.contains("INVENTORY_ONE_ITEM_SELECT"),
        "the one-item send must use the item-filtered select"
    );
    assert!(
        !body.contains("INVENTORY_ITEM_SELECT)"),
        "the one-item send must not read the whole inventory"
    );
    assert!(
        body.contains(".fetch_optional("),
        "one row at most: fetch_optional"
    );
}

async fn cleanup(pool: &PgPool) {
    for player_id in [0x7000_B1A1_i32, 0x7000_B1B1] {
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
    }
    for account_id in [0x7000_B1A0_i32, 0x7000_B1B0] {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }
}

async fn insert_player(pool: &PgPool, account_id: i32, player_id: i32) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("one-item-{account_id}"))
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

async fn insert_item(pool: &PgPool, player_id: i32, slot_id: i32) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
         SELECT $1, item_id, 1, $2, 1, false, 100, 0 FROM resources.items \
         WHERE container_sets IS NULL OR 1 = ANY(container_sets) ORDER BY item_id LIMIT 1 \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(slot_id)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

/// The owner check is in the query: another player's `item_id` reads no
/// row, so nothing is sent. The player's own item sends exactly one packet.
#[tokio::test]
async fn one_item_send_is_owner_checked_in_sql() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_B1E9;
    cleanup(&pool).await;
    insert_player(&pool, 0x7000_B1A0, 0x7000_B1A1).await;
    insert_player(&pool, 0x7000_B1B0, 0x7000_B1B1).await;
    let own = insert_item(&pool, 0x7000_B1A1, 0).await;
    let _second_own = insert_item(&pool, 0x7000_B1A1, 1).await;
    let someone_elses = insert_item(&pool, 0x7000_B1B1, 0).await;

    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:40819".parse().unwrap();
    let e2a: Arc<Mutex<HashMap<u32, SocketAddr>>> =
        Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let conn: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> = Arc::new(Mutex::new(
        HashMap::from([(addr, test_default_connected_client_state())]),
    ));

    assert!(
        !send_inventory_item_update_via(
            entity_id,
            0x7000_B1A1,
            someone_elses,
            &pool,
            &dyn_transport,
            &conn,
            &e2a,
        )
        .await,
        "another player's item must read no row"
    );
    assert_eq!(transport.send_count_to(addr), 0, "and send nothing");

    assert!(
        send_inventory_item_update_via(
            entity_id,
            0x7000_B1A1,
            own,
            &pool,
            &dyn_transport,
            &conn,
            &e2a,
        )
        .await,
        "the player's own item reads its row"
    );
    assert_eq!(transport.send_count_to(addr), 1, "one packet for one item");

    cleanup(&pool).await;
}
