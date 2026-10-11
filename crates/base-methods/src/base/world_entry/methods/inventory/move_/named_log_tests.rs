//! NT-22 (Rule 6): `move_accepted`, the vault move's success line, names
//! every ID it carries: the account, the character, the item and both
//! containers. No database: the line is written from the committed move.
//!
//! Sentinels: account and player `0x7000_D200..=0x7000_D201`, entity
//! `0x7000_D2E0`. Item and container IDs are the stock ones, named here by a
//! test NameBook so the line does not depend on the seed's spelling.

use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_entity::known_names;
use cimmeria_names::{NameBook, Table};
use cimmeria_wire::cell::vault::VaultAccess;
use tracing::Level;

use super::bank_rules::{log_move_accepted, AcceptedVaultMove, VaultOwner};
use crate::test_support::LogCapture;

const ACCOUNT: i32 = 0x7000_D200;
const PLAYER: i32 = 0x7000_D201;
const ENTITY: u32 = 0x7000_D2E0;
const SLAPPACK: i32 = 2893;
const INSTANCE: i32 = 0x7000_D2A0;

fn name_the_world() {
    let mut book = NameBook::empty();
    book.insert(Table::Items, i64::from(SLAPPACK), "Slappack TC1");
    // The moved instance's id names a decoy: a line that looked the
    // instance id up as an item type would log it instead.
    book.insert(
        Table::Items,
        i64::from(INSTANCE),
        "Decoy: instance id as a type",
    );
    book.insert(Table::Containers, 1, "MAIN");
    book.insert(Table::Containers, 17, "BANK");
    cimmeria_names::global().store(book);
    known_names::remember_player(PLAYER, "Vala Mal Doran");
    known_names::remember_account(ACCOUNT, "nt22_login");
}

/// A deposit from the main bag into the vault names the player, the item
/// and both ends. Fails if any of the name fields is dropped from the line.
#[test]
fn move_accepted_names_the_player_the_item_and_both_containers() {
    name_the_world();
    let capture = LogCapture::install();
    let vault = VaultAccess::Open {
        scope: VaultScope::Personal,
        org_id: None,
        banker_id: Some(0x7000_D2D0),
        distance: Some(2.0),
    };

    log_move_accepted(
        &AcceptedVaultMove {
            owner: VaultOwner {
                account_id: ACCOUNT,
                bank_slots: 40,
            },
            entity_id: ENTITY,
            player_id: PLAYER,
            item_id: INSTANCE,
            type_id: SLAPPACK,
            quantity: 1,
            source_container_id: 1,
            source_slot_id: 4,
            target_container_id: 17,
            target_slot_id: 0,
            kind: "deposit",
            source_stack_before: 1,
            source_stack_after: 0,
            target_stack_before: 0,
            target_stack_after: 1,
        },
        &vault,
    );

    let line = capture
        .find_message(Level::DEBUG, "move_accepted")
        .expect("move_accepted");
    for (k, v) in [
        ("account_id", ACCOUNT.to_string()),
        ("account_name", "nt22_login".to_string()),
        ("player_id", PLAYER.to_string()),
        ("player_name", "Vala Mal Doran".to_string()),
        ("entity_id", ENTITY.to_string()),
        ("entity_name", "Vala Mal Doran".to_string()),
        ("item_type_id", SLAPPACK.to_string()),
        ("item_name", "Slappack TC1".to_string()),
        ("source_container_name", "MAIN".to_string()),
        ("target_container_name", "BANK".to_string()),
    ] {
        assert!(line.has_field(k, &v), "{k}={v}: {line:#?}");
    }
}

/// An ID the server has no name for leaves the name off the line rather
/// than writing a placeholder (Rule 6: absent when unresolved).
#[test]
fn move_accepted_leaves_an_unknown_name_off_the_line() {
    name_the_world();
    let capture = LogCapture::install();

    log_move_accepted(
        &AcceptedVaultMove {
            owner: VaultOwner {
                account_id: ACCOUNT,
                bank_slots: 40,
            },
            entity_id: ENTITY,
            player_id: PLAYER + 0x10,
            item_id: 0x7000_D2A1,
            type_id: 0x7000_D2F0,
            quantity: 1,
            source_container_id: 1,
            source_slot_id: 4,
            target_container_id: 17,
            target_slot_id: 0,
            kind: "deposit",
            source_stack_before: 1,
            source_stack_after: 0,
            target_stack_before: 0,
            target_stack_after: 1,
        },
        &VaultAccess::NO_SESSION,
    );

    let line = capture
        .find_message(Level::DEBUG, "move_accepted")
        .expect("move_accepted");
    for absent in ["player_name", "entity_name", "item_name"] {
        assert!(!line.fields.contains_key(absent), "{absent}: {line:#?}");
    }
    assert!(line.has_field("target_container_name", "BANK"));
}

// ---- TG-ITM-04: inline move refusals carry `event` and `reason` ----------
//
// Live-DB: each refusal is reached through the real move path. Sentinels:
// account and player `0x7000_D300..=0x7000_D349` (stride 0x10), entities
// `0x7000_D3E0..=0x7000_D3E4`, ports 40860-40864, item type `0x7000_B1F0`
// (shared with the allowlist guards). Skip when `DATABASE_URL` is unset.

use super::allowlist_tests::{
    assert_fields, cleanup_all, connected_client, insert_synth_item_type, ClientState,
    SYNTH_TYPE_ID,
};
use super::tests::{insert_account_and_player, insert_item, pick_main_bag_type_ids};
use super::*;
use crate::test_support::{require_db_or_skip, LogCaptureGuard};

const REFUSAL_BASE: i32 = 0x7000_D300;
const REFUSAL_LOGIN: &str = "tg_itm_04_login";

/// A connected client whose session carries the account, so the refusal
/// can name it.
fn signed_in_client(entity_id: u32, port: u16, account_id: i32) -> ClientState {
    let client = connected_client(entity_id, port);
    {
        let mut sessions = client.4.lock().unwrap();
        let session = sessions.get_mut(&client.2).expect("session");
        session.account_id = account_id as u32;
        session.account_name = Some(REFUSAL_LOGIN.to_string());
    }
    client
}

/// Run one move through the real path and return the single
/// `move_rejected` WARN with the wanted `reason`, after the shared checks:
/// WARN level, `inventory` target, `event`, and the identity pair.
#[allow(clippy::too_many_arguments)]
async fn refused_move(
    client: &ClientState,
    capture: &LogCaptureGuard,
    pool: &sqlx::PgPool,
    entity_id: u32,
    player_id: i32,
    account_id: i32,
    item_id: i32,
    target: (i32, i32),
    quantity: i32,
    reason: &str,
) -> crate::test_support::Captured {
    let db_pool = Some(Arc::new(pool.clone()));
    handle_move_inventory_item(
        entity_id, player_id, item_id, target.0, target.1, quantity, &db_pool, &None, &client.1,
        &client.4, &client.3,
    )
    .await;
    let event = capture
        .find_event(Level::WARN, "", reason)
        .unwrap_or_else(|| panic!("no WARN with reason={reason}"));
    assert_eq!(event.target, "inventory");
    assert_fields(
        &event,
        &[
            ("event", "move_rejected".into()),
            ("reason", reason.into()),
            ("account_id", account_id.to_string()),
            ("account_name", REFUSAL_LOGIN.into()),
            ("player_id", player_id.to_string()),
            ("item_id", item_id.to_string()),
        ],
        &[],
    );
    event
}

/// A slot past the container's ceiling is refused as `invalid_slot`.
#[tokio::test]
async fn live_db_move_refusal_invalid_slot_logs_event_and_reason() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (REFUSAL_BASE, REFUSAL_BASE + 1);
    let entity_id: u32 = 0x7000_D3E0;
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let types = pick_main_bag_type_ids(&pool, 1).await;
    let item = insert_item(&pool, player_id, types[0], 1, 0, 1).await;
    let client = signed_in_client(entity_id, 40860, account_id);
    let capture = LogCapture::install();

    let event = refused_move(
        &client,
        &capture,
        &pool,
        entity_id,
        player_id,
        account_id,
        item,
        (1, 9_999),
        -1,
        "invalid_slot",
    )
    .await;
    assert_fields(
        &event,
        &[
            ("target_container_id", "1".into()),
            ("target_slot_id", "9999".into()),
        ],
        &[],
    );

    cleanup_all(&pool, account_id, player_id).await;
}

/// An instance id with no row is refused as `source_not_found`.
#[tokio::test]
async fn live_db_move_refusal_source_not_found_logs_event_and_reason() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (REFUSAL_BASE + 0x10, REFUSAL_BASE + 0x11);
    let entity_id: u32 = 0x7000_D3E1;
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let client = signed_in_client(entity_id, 40861, account_id);
    let capture = LogCapture::install();

    refused_move(
        &client,
        &capture,
        &pool,
        entity_id,
        player_id,
        account_id,
        0x7000_D3F0,
        (1, 7),
        1,
        "source_not_found",
    )
    .await;

    cleanup_all(&pool, account_id, player_id).await;
}

/// A quantity above the stack is refused as `over_stack`, naming the source
/// container pair, and leaves the stack alone.
#[tokio::test]
async fn live_db_move_refusal_over_stack_logs_event_and_reason() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (REFUSAL_BASE + 0x20, REFUSAL_BASE + 0x21);
    let entity_id: u32 = 0x7000_D3E2;
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let types = pick_main_bag_type_ids(&pool, 1).await;
    let item = insert_item(&pool, player_id, types[0], 1, 0, 2).await;
    let client = signed_in_client(entity_id, 40862, account_id);
    let capture = LogCapture::install();

    let event = refused_move(
        &client,
        &capture,
        &pool,
        entity_id,
        player_id,
        account_id,
        item,
        (1, 5),
        5,
        "over_stack",
    )
    .await;
    assert_fields(
        &event,
        &[
            ("quantity", "5".into()),
            ("stack_size", "2".into()),
            ("source_container_id", "1".into()),
        ],
        &[],
    );

    cleanup_all(&pool, account_id, player_id).await;
}

/// A type whose `container_sets` excludes the target is refused as
/// `not_allowed_in_container`, naming both containers.
#[tokio::test]
async fn live_db_move_refusal_not_allowed_in_container_logs_event_and_reason() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (REFUSAL_BASE + 0x30, REFUSAL_BASE + 0x31);
    let entity_id: u32 = 0x7000_D3E3;
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    insert_synth_item_type(&pool).await;
    sqlx::query("UPDATE resources.items SET container_sets = '{1}' WHERE item_id = $1")
        .bind(SYNTH_TYPE_ID)
        .execute(&pool)
        .await
        .expect("restrict the synthetic type to the main bag");
    let item = insert_item(&pool, player_id, SYNTH_TYPE_ID, 1, 0, 1).await;
    let client = signed_in_client(entity_id, 40863, account_id);
    let capture = LogCapture::install();

    let event = refused_move(
        &client,
        &capture,
        &pool,
        entity_id,
        player_id,
        account_id,
        item,
        (15, 0),
        -1,
        "not_allowed_in_container",
    )
    .await;
    assert_fields(
        &event,
        &[
            ("item_type_id", SYNTH_TYPE_ID.to_string()),
            ("source_container_id", "1".into()),
            ("target_container_id", "15".into()),
        ],
        &[],
    );

    cleanup_all(&pool, account_id, player_id).await;
}

/// A partial stack dropped on a different item is refused as
/// `split_onto_occupied`, naming both containers.
#[tokio::test]
async fn live_db_move_refusal_split_onto_occupied_logs_event_and_reason() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (REFUSAL_BASE + 0x40, REFUSAL_BASE + 0x41);
    let entity_id: u32 = 0x7000_D3E4;
    cleanup_all(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let types = pick_main_bag_type_ids(&pool, 2).await;
    let item = insert_item(&pool, player_id, types[0], 1, 0, 5).await;
    insert_item(&pool, player_id, types[1], 1, 3, 1).await;
    let client = signed_in_client(entity_id, 40864, account_id);
    let capture = LogCapture::install();

    let event = refused_move(
        &client,
        &capture,
        &pool,
        entity_id,
        player_id,
        account_id,
        item,
        (1, 3),
        2,
        "split_onto_occupied",
    )
    .await;
    assert_fields(
        &event,
        &[
            ("source_container_id", "1".into()),
            ("target_container_id", "1".into()),
            ("target_slot_id", "3".into()),
        ],
        &[],
    );

    cleanup_all(&pool, account_id, player_id).await;
}
