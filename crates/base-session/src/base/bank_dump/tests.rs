//! `.bankdump` on the base (BV-04): the lines, the read, and the
//! `gm_action` event for every outcome.
//!
//! Live-DB sentinels: account `0x7000_B4A0`, player `0x7000_B4A1`,
//! inventory items `0x7000_B4A4` to `0x7000_B4A7`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::*;
use crate::test_support::{
    require_db_or_skip, test_default_connected_client_state, LogCapture, TestTransport,
};

const ACCOUNT_ID: i32 = 0x7000_B4A0;
const PLAYER_ID: i32 = 0x7000_B4A1;
const ITEM_SLOT_0: i32 = 0x7000_B4A4;
const ITEM_SLOT_3: i32 = 0x7000_B4A5;
const ITEM_BEYOND: i32 = 0x7000_B4A6;
/// In the main bag, so the dump must not list it.
const ITEM_MAIN_BAG: i32 = 0x7000_B4A7;
/// `SI 3 9mm Pistol`, a seeded item type.
const PISTOL: i32 = 55;
/// Not the column default (40), so the dump is proven to read the column.
const BANK_SLOTS: i16 = 50;
const NAME: &str = "BankDumpSentinel";

const CALLER: DumpCaller = DumpCaller {
    entity_id: 77,
    account_id: Some(5),
    player_id: Some(6),
};

fn row(slot_id: i32, item_id: i32, stack_size: i32) -> VaultRow {
    VaultRow {
        slot_id,
        item_id,
        type_id: PISTOL,
        stack_size,
        name: Some("Pistol".to_string()),
    }
}

fn gm_actions(
    capture: &crate::test_support::LogCaptureGuard,
) -> Vec<crate::test_support::Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", "gm_action"))
        .collect()
}

/// The correlators every `gm_action` carries.
fn assert_correlated(event: &crate::test_support::Captured) {
    assert!(event.has_field("action", "bankdump"), "{event:#?}");
    assert!(event.has_field("account_id", "5"), "{event:#?}");
    assert!(event.has_field("player_id", "6"), "{event:#?}");
    assert!(event.has_field("entity_id", "77"), "{event:#?}");
}

#[test]
fn empty_vault_is_one_line_with_the_size() {
    let dump = VaultDump {
        player_id: 9,
        player_name: "Ann".into(),
        bank_slots: 40,
        rows: vec![],
    };
    assert_eq!(
        dump_lines(&dump),
        vec!["bankdump: Ann (player 9): the vault is empty (40 slots)".to_string()]
    );
}

/// One summary line, then one line per row in slot order; a row past
/// `bank_slots` is flagged, and an unknown type still prints.
#[test]
fn items_list_one_line_each_and_flag_rows_beyond_the_size() {
    let mut unknown = row(1, 10_002, 1);
    unknown.name = None;
    let dump = VaultDump {
        player_id: 9,
        player_name: "Ann".into(),
        bank_slots: 40,
        rows: vec![row(0, 10_001, 3), unknown, row(40, 10_003, 1)],
    };
    assert_eq!(
        dump_lines(&dump),
        vec![
            "bankdump: Ann (player 9): 3 item(s) in the vault (40 slots)".to_string(),
            "    slot 0: Pistol (type 55) x3 [item 10001]".to_string(),
            "    slot 1: unknown item (type 55) x1 [item 10002]".to_string(),
            "    slot 40: Pistol (type 55) x1 [item 10003] (beyond bank_slots)".to_string(),
        ]
    );
}

#[test]
fn item_lines_are_capped() {
    let dump = VaultDump {
        player_id: 9,
        player_name: "Ann".into(),
        bank_slots: 100,
        rows: (0..MAX_ITEM_LINES as i32 + 5)
            .map(|i| row(i, 10_000 + i, 1))
            .collect(),
    };
    let lines = dump_lines(&dump);
    assert_eq!(lines.len(), 1 + MAX_ITEM_LINES + 1);
    assert!(lines[0].contains("105 item(s)"), "{}", lines[0]);
    assert_eq!(lines.last().unwrap(), "    ... and 5 more");
}

#[test]
fn refusal_reasons_are_stable() {
    assert_eq!(DumpRefusal::TargetNotFound.reason(), "target_not_found");
    assert_eq!(DumpRefusal::DbUnavailable.reason(), "db_unavailable");
    assert_eq!(
        DumpRefusal::QueryFailed(String::new()).reason(),
        "query_failed"
    );
}

/// No pool: WARN `gm_action result=refused reason=db_unavailable`, and a
/// line that says so.
#[tokio::test]
async fn no_pool_logs_db_unavailable() {
    let capture = LogCapture::install();
    let lines = run_gm_dump(CALLER, &BankSubject::Player(PLAYER_ID), None).await;
    assert_eq!(lines, vec!["bankdump: no live DB connection".to_string()]);
    let events = gm_actions(&capture);
    assert_eq!(events.len(), 1, "{events:#?}");
    assert_eq!(events[0].level, tracing::Level::WARN);
    assert_correlated(&events[0]);
    assert!(events[0].has_field("result", "refused"));
    assert!(events[0].has_field("reason", "db_unavailable"));
    assert!(events[0].has_field("target_player_id", &PLAYER_ID.to_string()));
}

/// A pool that cannot connect: WARN `reason=query_failed`, the GM is told
/// to look at the log.
#[tokio::test]
async fn unreachable_db_logs_query_failed() {
    let unreachable = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(200))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .expect("lazy pool");
    let capture = LogCapture::install();
    let lines = run_gm_dump(
        CALLER,
        &BankSubject::Name(NAME.to_string()),
        Some(&unreachable),
    )
    .await;
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("could not read the vault"), "{lines:?}");
    let events = gm_actions(&capture);
    assert_eq!(events.len(), 1, "{events:#?}");
    assert_eq!(events[0].level, tracing::Level::WARN);
    assert_correlated(&events[0]);
    assert!(events[0].has_field("reason", "query_failed"));
    assert!(events[0].has_field("target_name", NAME));
    assert!(events[0].fields.contains_key("error"));
}

async fn cleanup(pool: &PgPool) {
    let players = [PLAYER_ID];
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = ANY($1)")
        .bind(players.as_slice())
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = ANY($1)")
        .bind(players.as_slice())
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACCOUNT_ID)
        .execute(pool)
        .await;
}

async fn insert_player(pool: &PgPool, player_id: i32, name: &str) {
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah, bank_slots\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, 0, $4)",
    )
    .bind(ACCOUNT_ID)
    .bind(player_id)
    .bind(name)
    .bind(BANK_SLOTS)
    .execute(pool)
    .await
    .expect("insert player");
}

async fn insert_item(pool: &PgPool, item_id: i32, container_id: i32, slot_id: i32, stack: i32) {
    sqlx::query(
        "INSERT INTO sgw_inventory (item_id, character_id, type_id, container_id, slot_id, stack_size) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(item_id)
    .bind(PLAYER_ID)
    .bind(PISTOL)
    .bind(container_id)
    .bind(slot_id)
    .bind(stack)
    .execute(pool)
    .await
    .expect("insert inventory row");
}

/// The sentinel character with two vault rows, one row past `bank_slots`,
/// and one main-bag row.
async fn seed(pool: &PgPool) {
    cleanup(pool).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(ACCOUNT_ID)
        .bind(format!("bv04-dump-{ACCOUNT_ID}"))
        .execute(pool)
        .await
        .expect("insert account");
    insert_player(pool, PLAYER_ID, NAME).await;
    insert_item(pool, ITEM_SLOT_3, INV_BANK, 3, 1).await;
    insert_item(pool, ITEM_SLOT_0, INV_BANK, 0, 2).await;
    insert_item(pool, ITEM_BEYOND, INV_BANK, i32::from(BANK_SLOTS), 1).await;
    insert_item(pool, ITEM_MAIN_BAG, 1, 0, 1).await;
}

/// By name and by id, the read lists exactly container 17 in slot order,
/// with the item names and the character's own `bank_slots`. A main-bag
/// row never appears.
#[tokio::test]
async fn load_reads_only_container_17_by_name_or_id() {
    let pool = require_db_or_skip!();
    seed(&pool).await;

    let by_name = load_vault_dump(&pool, &BankSubject::Name(NAME.to_string())).await;
    let by_id = load_vault_dump(&pool, &BankSubject::Player(PLAYER_ID)).await;
    cleanup(&pool).await;

    let dump = by_name.expect("the sentinel resolves by name");
    assert_eq!(by_id, Ok(dump.clone()), "by id reads the same vault");
    assert_eq!(dump.player_id, PLAYER_ID);
    assert_eq!(dump.player_name, NAME);
    assert_eq!(dump.bank_slots, BANK_SLOTS);
    let got: Vec<(i32, i32, i32)> = dump
        .rows
        .iter()
        .map(|r| (r.slot_id, r.item_id, r.stack_size))
        .collect();
    assert_eq!(
        got,
        vec![
            (0, ITEM_SLOT_0, 2),
            (3, ITEM_SLOT_3, 1),
            (i32::from(BANK_SLOTS), ITEM_BEYOND, 1)
        ]
    );
    assert!(
        dump.rows
            .iter()
            .all(|r| r.name.as_deref().is_some_and(|n| !n.is_empty())),
        "the item name comes from resources.items: {:?}",
        dump.rows
    );
}

/// Success: INFO `gm_action result=ok` with the target, the count and the
/// size, and the lines reach the GM's own client, one packet per line.
#[tokio::test]
async fn dump_logs_gm_action_ok_and_sends_every_line_to_the_gm() {
    let pool = require_db_or_skip!();
    seed(&pool).await;

    let gm: SocketAddr = "127.0.0.1:54710".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(CALLER.entity_id);
    let connected = Arc::new(Mutex::new(HashMap::from([(gm, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(CALLER.entity_id, gm)])));
    let test_transport = Arc::new(TestTransport::default());
    let transport: Arc<dyn Transport> = test_transport.clone();

    let capture = LogCapture::install();
    handle_gm_dump(
        CALLER,
        BankSubject::Name(NAME.to_string()),
        &Some(Arc::new(pool.clone())),
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    cleanup(&pool).await;

    let events = gm_actions(&capture);
    assert_eq!(events.len(), 1, "{events:#?}");
    let e = &events[0];
    assert_eq!(e.level, tracing::Level::INFO);
    assert_correlated(e);
    assert!(e.has_field("result", "ok"));
    assert!(e.has_field("target_player_id", &PLAYER_ID.to_string()));
    assert!(e.has_field("item_count", "3"));
    assert!(e.has_field("bank_slots", &BANK_SLOTS.to_string()));
    assert!(!e.fields.contains_key("reason"));
    assert!(capture
        .all()
        .iter()
        .any(|c| c.target == "span:bank.gm_dump" && c.level == tracing::Level::INFO));

    // Summary plus three item lines.
    assert_eq!(test_transport.filter_to(gm).len(), 4);
}

/// A name nobody has, and an id nobody has: INFO `gm_action
/// result=refused reason=target_not_found`, and a line saying why.
#[tokio::test]
async fn unknown_targets_are_refused_with_a_reason() {
    let pool = require_db_or_skip!();
    seed(&pool).await;

    let capture = LogCapture::install();
    let by_name = run_gm_dump(
        CALLER,
        &BankSubject::Name("NoSuchBankDumpName".to_string()),
        Some(&pool),
    )
    .await;
    // The sentinel with its row gone: an id with no character.
    cleanup(&pool).await;
    let by_id = run_gm_dump(CALLER, &BankSubject::Player(PLAYER_ID), Some(&pool)).await;

    assert!(
        by_name[0].contains("no character is named NoSuchBankDumpName"),
        "{by_name:?}"
    );
    assert!(
        by_id[0].contains("no character is named player"),
        "{by_id:?}"
    );
    let events = gm_actions(&capture);
    assert_eq!(events.len(), 2, "{events:#?}");
    for e in &events {
        assert_eq!(e.level, tracing::Level::INFO);
        assert_correlated(e);
        assert!(e.has_field("result", "refused"));
        assert!(e.has_field("reason", "target_not_found"), "{e:#?}");
    }
    assert!(events[0].has_field("target_name", "NoSuchBankDumpName"));
    assert!(events[1].has_field("target_player_id", &PLAYER_ID.to_string()));
}
