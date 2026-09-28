//! Live-DB guards for what a refused move sends back: one `onUpdateItem`
//! for the refused item, read and sent under the move lock and the item's
//! row lock, so no concurrent write to that item can be overtaken by it.
//!
//! Shares the allowlist guards' helpers and sentinel ranges (see
//! `allowlist_tests.rs`), plus account/player `0x7000_B1C0`/`0x7000_B1C1`,
//! entity `0x7000_B1ED` and port 40820 for the stack-merge guard. Skip when
//! `DATABASE_URL` is unset.

use tracing::Level;

use super::allowlist_tests::{
    assert_fields, cleanup_all, connected_client, insert_synth_item_type, SYNTH_TYPE_ID, TEST_BASE,
};
use super::tests::{insert_account_and_player, insert_item};
use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// The one `onUpdateItem` packet a refusal should send for a synthetic-type
/// item at `(container, db_slot)` holding `stack_size`: an array of exactly that item. Packet
/// sequence 0, the first packet on a fresh test client.
fn expected_item_resync(
    entity_id: u32,
    item_id: i32,
    container_id: i32,
    db_slot: i32,
    stack_size: i32,
) -> Vec<u8> {
    use crate::mercury::{build_player_entity_method_packet, method_idx};
    use cimmeria_entity::inventory::InvItem;

    let mut args = Vec::new();
    args.extend_from_slice(&1u32.to_le_bytes());
    InvItem {
        id: item_id,
        dbid: SYNTH_TYPE_ID,
        stack_size,
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
async fn live_db_refusal_resync_waits_for_the_move_lock_and_sends_the_committed_state() {
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
        expected_item_resync(entity_id, item, 1, 4, 1),
        "the resync must carry the committed slot (4), not the pre-commit one"
    );

    cleanup_all(&pool, account_id, player_id).await;
}

/// Concurrency guard for writers that do not take the move lock. A grant
/// that merges into an existing stack takes `(player, container)` and
/// updates the row; it never touches `(player, 0)`. Here a second
/// connection plays that grant: it holds `(player, 1)`, has added 4 to the
/// refused item's stack, and has not committed. The refusal must send
/// nothing while that write is open, and after the commit its one packet
/// must carry the merged stack (5).
///
/// With only the move lock the refusal read the row straight away, saw a
/// stack of 1 and sent it at once, so the grant's own `onUpdateItem` could
/// be overtaken by a stale one and the client would show the old stack.
#[tokio::test]
async fn live_db_refusal_resync_waits_for_a_concurrent_stack_merge() {
    use std::time::Duration;

    let pool = require_db_or_skip!();
    let account_id = 0x7000_B1C0;
    let player_id = 0x7000_B1C1;
    let entity_id: u32 = 0x7000_B1ED;
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    insert_synth_item_type(&pool).await;
    let item = insert_item(&pool, player_id, SYNTH_TYPE_ID, 1, 0, 1).await;

    let (transport, dyn_transport, addr, e2a, conn) = connected_client(entity_id, 40820);
    let db_pool = Some(Arc::new(pool.clone()));

    // The concurrent grant: its per-container lock, then the stack merge,
    // exactly the statement `grant_item.rs` runs; not yet committed.
    let mut grant = pool.begin().await.expect("begin concurrent grant");
    sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
        .bind(player_id)
        .bind(1)
        .execute(&mut *grant)
        .await
        .expect("take the grant's container lock");
    sqlx::query("UPDATE sgw_inventory SET stack_size = stack_size + $1 WHERE item_id = $2")
        .bind(4)
        .bind(item)
        .execute(&mut *grant)
        .await
        .expect("merge into the stack inside the concurrent transaction");

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
        "the refusal must wait for the in-flight write to the refused item's row"
    );
    assert_eq!(
        transport.send_count_to(addr),
        0,
        "no resync may be sent while a write to the refused item is open"
    );

    grant.commit().await.expect("commit the concurrent grant");
    tokio::time::timeout(Duration::from_secs(10), &mut refused)
        .await
        .expect("the refusal must finish once the row is released");

    let sent = transport.drain();
    assert_eq!(sent.len(), 1, "exactly one resync packet");
    assert_eq!(
        sent[0].1,
        expected_item_resync(entity_id, item, 1, 0, 5),
        "the resync must carry the merged stack (5), not the pre-grant one"
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
async fn live_db_refusal_resends_only_the_refused_item() {
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
        expected_item_resync(entity_id, sold, 16, 2, 1),
        "the packet is onUpdateItem for the refused item alone, at its unchanged position"
    );

    cleanup_all(&pool, account_id, player_id).await;
}

/// A refused move naming an `item_id` the player does not own (a forged
/// packet) sends nothing: there is no row to snap back, and the refusal is
/// already logged.
#[tokio::test]
async fn live_db_refusal_of_an_unknown_item_sends_nothing() {
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
        .find_event(Level::WARN, "move_rejected", "no_vault_session")
        .expect("the refusal is still logged");
    assert_eq!(rejected.target, "bank");
    assert_fields(
        &rejected,
        &[
            ("event", "move_rejected".into()),
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
            ("event", "move_resync_skipped".into()),
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
