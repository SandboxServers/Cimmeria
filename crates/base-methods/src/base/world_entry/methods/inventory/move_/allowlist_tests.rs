//! Live-DB regression guards for the player-movable allowlist (D-BV07,
//! issue #798).
//!
//! Every test uses one synthetic item type whose `container_sets` is
//! `{1,15,17}`, so `item_allows_container` passes for main, crafting and the
//! vault alike. That isolates the allowlist: a refusal here can only come
//! from `player_movable`. No seeded item allows both 1 and 15 today, so the
//! 1 <-> 15 guard needs the synthetic type anyway.
//!
//! Sentinels: accounts and players `0x7000_B100..=0x7000_B131`, the item
//! type `0x7000_B1F0`, entities `0x7000_B1E0..`. Skip when `DATABASE_URL`
//! is unset.

use tracing::Level;

use super::tests::{cleanup, insert_account_and_player, insert_item};
use super::*;
use crate::test_support::{
    require_db_or_skip, test_default_connected_client_state, LogCapture, TestTransport,
};

const TEST_BASE: i32 = 0x7000_B100;
const SYNTH_TYPE_ID: i32 = 0x7000_B1F0;

async fn insert_synth_item_type(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO resources.items (\
            item_id, description, name, quality_id, tech_comp, tier, \
            max_stack_size, container_sets \
         ) VALUES ($1, '', 'bv01-allowlist', 'ITEM_QUALITY_Normal', 0, 1, 1, '{1,15,17}') \
         ON CONFLICT (item_id) DO UPDATE SET container_sets = EXCLUDED.container_sets",
    )
    .bind(SYNTH_TYPE_ID)
    .execute(pool)
    .await
    .expect("insert synthetic item type");
}

async fn cleanup_all(pool: &PgPool, account_id: i32, player_id: i32) {
    cleanup(pool, account_id, player_id).await;
    let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
        .bind(SYNTH_TYPE_ID)
        .execute(pool)
        .await;
}

/// Container, slot, stack and flags of one inventory row.
async fn row_of(pool: &PgPool, player_id: i32, item_id: i32) -> Option<(i32, i32, i32, i32)> {
    sqlx::query_as::<_, (i32, i32, i32, i32)>(
        "SELECT container_id, slot_id, stack_size, flags FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(pool)
    .await
    .expect("row_of query")
}

async fn naquadah_of(pool: &PgPool, player_id: i32) -> i32 {
    sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .expect("naquadah_of query")
}

type ClientState = (
    Arc<TestTransport>,
    Arc<dyn Transport>,
    SocketAddr,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
);

/// A connected client, so the refusal's resync reaches a real address.
fn connected_client(entity_id: u32, port: u16) -> ClientState {
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        test_default_connected_client_state(),
    )])));
    (transport, dyn_transport, addr, entity_to_addr, connected)
}

/// #798 guard. A sold item sits in the buyback bag (16) with its sale price
/// in `flags`. Moving it straight back to the main bag must be refused:
/// the row stays in 16, the balance is untouched, the refusal is logged
/// under `bank`, and the client is resynced so the drag snaps back.
///
/// Before the fix the move path checked only the target, so the row landed
/// in (1, 5) and the player kept both the item and the sale price.
#[tokio::test]
async fn move_out_of_buyback_is_refused_and_no_row_changes() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE;
    let player_id = TEST_BASE + 1;
    let entity_id: u32 = 0x7000_B1E0;
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    insert_synth_item_type(&pool).await;
    sqlx::query("UPDATE sgw_player SET naquadah = 1100 WHERE player_id = $1")
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("set balance");
    let sold = insert_item(&pool, player_id, SYNTH_TYPE_ID, 16, 0, 1).await;
    sqlx::query("UPDATE sgw_inventory SET flags = 1000 WHERE item_id = $1")
        .bind(sold)
        .execute(&pool)
        .await
        .expect("set buyback price");

    let (transport, dyn_transport, addr, e2a, conn) = connected_client(entity_id, 40811);
    let db_pool = Some(Arc::new(pool.clone()));
    let capture = LogCapture::install();

    handle_move_inventory_item(
        entity_id,
        player_id,
        sold,
        1,
        5,
        -1,
        &db_pool,
        &None,
        &dyn_transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        row_of(&pool, player_id, sold).await,
        Some((16, 0, 1, 1000)),
        "a move out of buyback must leave the row in (16, 0) with its price: \
         buyback is left only through buybackItems, which charges (#798)"
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        1100,
        "the refused move must not touch the balance"
    );
    let event = capture
        .find_event(
            Level::WARN,
            "move_rejected",
            "source_container_not_player_movable",
        )
        .expect("refusal must log move_rejected with reason=source_container_not_player_movable");
    assert_eq!(event.target, "bank");
    assert!(
        transport.send_count_to(addr) > 0,
        "the refusal must resync the client so the dragged item snaps back"
    );

    cleanup_all(&pool, account_id, player_id).await;
}

/// The vault (17) now has a capacity of 100, so the slot-range check alone
/// would accept it. Until BV-03 wires the vault session, the allowlist must
/// still refuse every move into it.
#[tokio::test]
async fn move_into_vault_is_still_refused() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 0x10;
    let player_id = TEST_BASE + 0x11;
    let entity_id: u32 = 0x7000_B1E1;
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    insert_synth_item_type(&pool).await;
    let item = insert_item(&pool, player_id, SYNTH_TYPE_ID, 1, 0, 1).await;

    let (transport, dyn_transport, addr, e2a, conn) = connected_client(entity_id, 40812);
    let db_pool = Some(Arc::new(pool.clone()));
    let capture = LogCapture::install();

    handle_move_inventory_item(
        entity_id,
        player_id,
        item,
        17,
        0,
        -1,
        &db_pool,
        &None,
        &dyn_transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        row_of(&pool, player_id, item).await,
        Some((1, 0, 1, 0)),
        "a move into the vault without a vault session must leave the row in (1, 0)"
    );
    assert!(
        capture
            .find_event(
                Level::WARN,
                "move_rejected",
                "target_container_needs_vault_session",
            )
            .is_some(),
        "refusal must log move_rejected with reason=target_container_needs_vault_session"
    );
    assert!(
        transport.send_count_to(addr) > 0,
        "the refusal must resync the client so the dragged item snaps back"
    );

    cleanup_all(&pool, account_id, player_id).await;
}

/// Crafting depends on moves between the main bag (1) and the crafting bag
/// (15) in both directions (D-BV04). The allowlist must keep both legal.
#[tokio::test]
async fn moves_between_main_and_crafting_succeed_both_ways() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 0x20;
    let player_id = TEST_BASE + 0x21;
    let entity_id: u32 = 0x7000_B1E2;
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    insert_synth_item_type(&pool).await;
    let item = insert_item(&pool, player_id, SYNTH_TYPE_ID, 1, 0, 1).await;

    let (_transport, dyn_transport, _addr, e2a, conn) = connected_client(entity_id, 40813);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_move_inventory_item(
        entity_id,
        player_id,
        item,
        15,
        3,
        -1,
        &db_pool,
        &None,
        &dyn_transport,
        &conn,
        &e2a,
    )
    .await;
    assert_eq!(
        row_of(&pool, player_id, item).await,
        Some((15, 3, 1, 0)),
        "main -> crafting must still move"
    );

    handle_move_inventory_item(
        entity_id,
        player_id,
        item,
        1,
        7,
        -1,
        &db_pool,
        &None,
        &dyn_transport,
        &conn,
        &e2a,
    )
    .await;
    assert_eq!(
        row_of(&pool, player_id, item).await,
        Some((1, 7, 1, 0)),
        "crafting -> main must still move"
    );

    cleanup_all(&pool, account_id, player_id).await;
}
