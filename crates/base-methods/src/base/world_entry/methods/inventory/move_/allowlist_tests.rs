//! Live-DB regression guards for the player-movable allowlist (D-BV07).
//!
//! Every test uses one synthetic item type whose `container_sets` is
//! `{1,15,17}`, so `item_allows_container` passes for main, crafting and the
//! vault alike. That isolates the allowlist: a refusal here can only come
//! from `player_movable`. No seeded item allows both 1 and 15 today, so the
//! 1 <-> 15 guard needs the synthetic type anyway.
//!
//! Sentinels: accounts and players `0x7000_B100..=0x7000_B131` and
//! `0x7000_B160..=0x7000_B191` (B140/B141 and B150/B151 belong to the grant
//! and player-load guards), the item
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

/// Assert a captured event carries each `(field, value)` exactly, and none
/// of `absent`. Values compare as the capture layer records them (integers
/// in decimal, strings verbatim).
fn assert_fields(
    event: &crate::test_support::Captured,
    present: &[(&str, String)],
    absent: &[&str],
) {
    for (key, value) in present {
        assert_eq!(
            event.fields.get(*key),
            Some(value),
            "field `{key}` on {:?}",
            event.message
        );
    }
    for key in absent {
        assert!(
            !event.fields.contains_key(*key),
            "field `{key}` must be absent on {:?}",
            event.message
        );
    }
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

/// Free-buyback guard. A sold item sits in the buyback bag (16) with its sale price
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
         buyback is left only through buybackItems, which charges"
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
    assert_fields(
        &event,
        &[
            ("account_id", account_id.to_string()),
            ("player_id", player_id.to_string()),
            ("entity_id", entity_id.to_string()),
            ("item_id", sold.to_string()),
            ("type_id", SYNTH_TYPE_ID.to_string()),
            ("quantity", "-1".into()),
            ("stack_size", "1".into()),
            ("source_container_id", "16".into()),
            ("source_slot_id", "0".into()),
            ("target_container_id", "1".into()),
            ("target_slot_id", "5".into()),
        ],
        &[],
    );
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
    let event = capture
        .find_event(
            Level::WARN,
            "move_rejected",
            "target_container_needs_vault_session",
        )
        .expect("refusal must log move_rejected with reason=target_container_needs_vault_session");
    assert_eq!(event.target, "bank");
    // Refused at the target end, before the move path reads the source row:
    // the refusal reads the source position itself.
    assert_fields(
        &event,
        &[
            ("account_id", account_id.to_string()),
            ("player_id", player_id.to_string()),
            ("entity_id", entity_id.to_string()),
            ("item_id", item.to_string()),
            ("type_id", SYNTH_TYPE_ID.to_string()),
            ("quantity", "-1".into()),
            ("source_container_id", "1".into()),
            ("source_slot_id", "0".into()),
            ("target_container_id", "17".into()),
            ("target_slot_id", "0".into()),
        ],
        &[],
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

/// The one `onUpdateItem` packet a refusal should send for a synthetic-type
/// item at `(container, db_slot)`: an array of exactly that item. Packet
/// sequence 0, the first packet on a fresh test client.
fn expected_item_resync(entity_id: u32, item_id: i32, container_id: i32, db_slot: i32) -> Vec<u8> {
    use crate::mercury::{build_player_entity_method_packet, method_idx};
    use cimmeria_entity::inventory::InvItem;

    let mut args = Vec::new();
    args.extend_from_slice(&1u32.to_le_bytes());
    InvItem {
        id: item_id,
        dbid: SYNTH_TYPE_ID,
        stack_size: 1,
        slot_id: db_slot + 1, // wire slots are 1-based
        container_id,
        is_bound: false,
        durability: 100,
        ammo_types: vec![],
        cur_ammo_type: 0,
        charges: 0,
    }
    .serialize(&mut args);
    build_player_entity_method_packet(
        &[0u8; 32],
        0,
        &[],
        entity_id,
        method_idx::ON_UPDATE_ITEM,
        &args,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    )
}

/// Concurrency guard: the refusal resends the refused item under the
/// per-player move lock, so a move of the SAME item that is mid-commit is
/// finished first and the client gets its committed slot.
///
/// A second connection plays that move: it holds the `(player, 0)` move lock
/// and has moved the item from slot 0 to slot 4 without committing. The
/// refused move must send nothing while that lock is held, and after the
/// commit its one packet must show slot 4.
///
/// Without the lock the refusal read the row straight away, saw slot 0 and
/// sent it at once, so the stale position could land after the committed
/// move's own update.
#[tokio::test]
async fn refusal_resync_waits_for_the_move_lock_and_sends_the_committed_state() {
    use std::time::Duration;

    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 0x30;
    let player_id = TEST_BASE + 0x31;
    let entity_id: u32 = 0x7000_B1E3;
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    insert_synth_item_type(&pool).await;
    let item = insert_item(&pool, player_id, SYNTH_TYPE_ID, 1, 0, 1).await;

    let (transport, dyn_transport, addr, e2a, conn) = connected_client(entity_id, 40814);
    let db_pool = Some(Arc::new(pool.clone()));

    // The concurrent move of the same item: lock taken, row moved, not yet
    // committed.
    let mut other = pool.begin().await.expect("begin concurrent move");
    sqlx::query("SELECT pg_advisory_xact_lock($1, 0)")
        .bind(player_id)
        .execute(&mut *other)
        .await
        .expect("take the move lock");
    sqlx::query("UPDATE sgw_inventory SET slot_id = 4 WHERE item_id = $1")
        .bind(item)
        .execute(&mut *other)
        .await
        .expect("move the row inside the concurrent transaction");

    // A move into the vault is refused at the target end, before the move
    // path takes any lock of its own, so only the refusal's resync waits.
    let refused = handle_move_inventory_item(
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
    );
    tokio::pin!(refused);
    assert!(
        tokio::time::timeout(Duration::from_millis(500), &mut refused)
            .await
            .is_err(),
        "the refusal must wait for the move lock"
    );
    assert_eq!(
        transport.send_count_to(addr),
        0,
        "no resync may be sent while another move holds the lock"
    );

    other.commit().await.expect("commit the concurrent move");
    tokio::time::timeout(Duration::from_secs(10), &mut refused)
        .await
        .expect("the refusal must finish once the lock is released");

    let sent = transport.drain();
    assert_eq!(sent.len(), 1, "exactly one resync packet");
    assert_eq!(
        sent[0].1,
        expected_item_resync(entity_id, item, 1, 4),
        "the resync must carry the committed slot (4), not the pre-commit one"
    );

    cleanup_all(&pool, account_id, player_id).await;
}

/// A refusal resends only the item the refused move named: one
/// `onUpdateItem` whose array holds that item and nothing else, even when
/// the player owns other items.
///
/// A full-inventory snapshot here would also carry every other row as of
/// the read, and a grant committing between that read and the send (grants
/// take per-container locks, not the move lock) would then be hidden on the
/// client by the older snapshot.
#[tokio::test]
async fn refusal_resends_only_the_refused_item() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 0x60;
    let player_id = TEST_BASE + 0x61;
    let entity_id: u32 = 0x7000_B1E5;
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    insert_synth_item_type(&pool).await;
    let _other_item = insert_item(&pool, player_id, SYNTH_TYPE_ID, 1, 0, 1).await;
    let sold = insert_item(&pool, player_id, SYNTH_TYPE_ID, 16, 2, 1).await;

    let (transport, dyn_transport, addr, e2a, conn) = connected_client(entity_id, 40815);
    let db_pool = Some(Arc::new(pool.clone()));

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

    let sent = transport.drain();
    assert_eq!(sent.len(), 1, "a refusal sends exactly one packet");
    assert_eq!(sent[0].0, addr, "to the refused player's own client");
    assert_eq!(
        sent[0].1,
        expected_item_resync(entity_id, sold, 16, 2),
        "the packet is onUpdateItem for the refused item alone, at its unchanged position"
    );

    cleanup_all(&pool, account_id, player_id).await;
}

/// A refused move naming an `item_id` the player does not own (a forged
/// packet) sends nothing: there is no row to snap back, and the refusal is
/// already logged.
#[tokio::test]
async fn refusal_of_an_unknown_item_sends_nothing() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 0x70;
    let player_id = TEST_BASE + 0x71;
    let entity_id: u32 = 0x7000_B1E6;
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    insert_synth_item_type(&pool).await;
    let owned = insert_item(&pool, player_id, SYNTH_TYPE_ID, 1, 0, 1).await;

    let (transport, dyn_transport, addr, e2a, conn) = connected_client(entity_id, 40816);
    let db_pool = Some(Arc::new(pool.clone()));
    let capture = LogCapture::install();

    handle_move_inventory_item(
        entity_id,
        player_id,
        owned + 1_000_000,
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

    let forged = owned + 1_000_000;
    let rejected = capture
        .find_event(
            Level::WARN,
            "move_rejected",
            "target_container_needs_vault_session",
        )
        .expect("the refusal is still logged");
    assert_eq!(rejected.target, "bank");
    assert_fields(
        &rejected,
        &[
            ("account_id", account_id.to_string()),
            ("player_id", player_id.to_string()),
            ("item_id", forged.to_string()),
            ("target_container_id", "17".into()),
        ],
        &[
            "type_id",
            "stack_size",
            "source_container_id",
            "source_slot_id",
        ],
    );
    let skipped = capture
        .find_event(Level::WARN, "move_resync_skipped", "refused_item_not_owned")
        .expect("a refusal with nothing to resend must log move_resync_skipped");
    assert_eq!(skipped.target, "bank");
    assert_fields(
        &skipped,
        &[
            ("account_id", account_id.to_string()),
            ("player_id", player_id.to_string()),
            ("entity_id", entity_id.to_string()),
            ("item_id", forged.to_string()),
        ],
        &[],
    );
    assert_eq!(
        transport.send_count_to(addr),
        0,
        "no packet for an item the player does not own"
    );

    cleanup_all(&pool, account_id, player_id).await;
}

/// `move_rejected reason=target_container_not_player_movable`: a move INTO
/// buyback (16). Only seed data kept this refused before BV-01; the
/// allowlist refuses it whatever the item's `container_sets` say.
#[tokio::test]
async fn move_into_buyback_logs_target_not_player_movable() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 0x80;
    let player_id = TEST_BASE + 0x81;
    let entity_id: u32 = 0x7000_B1E7;
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    insert_synth_item_type(&pool).await;
    let item = insert_item(&pool, player_id, SYNTH_TYPE_ID, 1, 3, 1).await;

    let (transport, dyn_transport, addr, e2a, conn) = connected_client(entity_id, 40817);
    let db_pool = Some(Arc::new(pool.clone()));
    let capture = LogCapture::install();

    handle_move_inventory_item(
        entity_id,
        player_id,
        item,
        16,
        2,
        1,
        &db_pool,
        &None,
        &dyn_transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(row_of(&pool, player_id, item).await, Some((1, 3, 1, 0)));
    let event = capture
        .find_event(
            Level::WARN,
            "move_rejected",
            "target_container_not_player_movable",
        )
        .expect("a move into buyback must log reason=target_container_not_player_movable");
    assert_eq!(event.target, "bank");
    assert_fields(
        &event,
        &[
            ("account_id", account_id.to_string()),
            ("player_id", player_id.to_string()),
            ("entity_id", entity_id.to_string()),
            ("item_id", item.to_string()),
            ("type_id", SYNTH_TYPE_ID.to_string()),
            ("quantity", "1".into()),
            ("stack_size", "1".into()),
            ("source_container_id", "1".into()),
            ("source_slot_id", "3".into()),
            ("target_container_id", "16".into()),
            ("target_slot_id", "2".into()),
        ],
        &[],
    );
    assert_eq!(transport.send_count_to(addr), 1, "one-item snap-back");

    cleanup_all(&pool, account_id, player_id).await;
}

/// `move_rejected reason=source_container_needs_vault_session`: a row
/// already in the vault (17), moved out without a vault session.
#[tokio::test]
async fn move_out_of_vault_logs_source_needs_vault_session() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 0x90;
    let player_id = TEST_BASE + 0x91;
    let entity_id: u32 = 0x7000_B1E8;
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    insert_synth_item_type(&pool).await;
    let item = insert_item(&pool, player_id, SYNTH_TYPE_ID, 17, 7, 1).await;

    let (transport, dyn_transport, addr, e2a, conn) = connected_client(entity_id, 40818);
    let db_pool = Some(Arc::new(pool.clone()));
    let capture = LogCapture::install();

    handle_move_inventory_item(
        entity_id,
        player_id,
        item,
        1,
        0,
        -1,
        &db_pool,
        &None,
        &dyn_transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(row_of(&pool, player_id, item).await, Some((17, 7, 1, 0)));
    let event = capture
        .find_event(
            Level::WARN,
            "move_rejected",
            "source_container_needs_vault_session",
        )
        .expect("a move out of the vault must log reason=source_container_needs_vault_session");
    assert_eq!(event.target, "bank");
    assert_fields(
        &event,
        &[
            ("account_id", account_id.to_string()),
            ("player_id", player_id.to_string()),
            ("entity_id", entity_id.to_string()),
            ("item_id", item.to_string()),
            ("type_id", SYNTH_TYPE_ID.to_string()),
            ("quantity", "-1".into()),
            ("stack_size", "1".into()),
            ("source_container_id", "17".into()),
            ("source_slot_id", "7".into()),
            ("target_container_id", "1".into()),
            ("target_slot_id", "0".into()),
        ],
        &[],
    );
    assert_eq!(transport.send_count_to(addr), 1, "one-item snap-back");

    cleanup_all(&pool, account_id, player_id).await;
}
