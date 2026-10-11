//! Live-DB and LogCapture tests for the contact-list header ops.

use super::*;
use crate::test_support::{
    require_db_or_skip, test_default_connected_client_state, Captured, LogCapture, TestTransport,
};
use std::net::SocketAddr;

/// Sentinel block for the header-op live-DB rows (distinct from the
/// persistence tests' `0x7000_4000` block and the consumable
/// tests' `0x7000_D100` block).
const ACCOUNT_ID: u32 = 0x7000_D500;
const PLAYER_ID: i32 = 0x7000_D501;
const ENTITY_ID: u32 = 0x7000_D502;
const ACCOUNT_NAME: &str = "cl-header-ops-test";
const PLAYER_NAME: &str = "cl-header-ops";

fn session() -> (
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let addr: SocketAddr = "127.0.0.1:54720".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.account_id = ACCOUNT_ID;
    state.account_name = Some(ACCOUNT_NAME.to_string());
    state.active_player_id = Some(PLAYER_ID);
    state.player_name = Some(PLAYER_NAME.to_string());
    state.player_entity_id = Some(ENTITY_ID);
    (
        Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
        Arc::new(Mutex::new(HashMap::from([(ENTITY_ID, addr)]))),
    )
}

/// Every captured row whose `event` is `event`, with its identity
/// fields asserted: `account_id` and its name pair.
fn rows(capture: &crate::test_support::LogCaptureGuard, event: &str) -> Vec<Captured> {
    let found: Vec<Captured> = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", event))
        .collect();
    assert!(!found.is_empty(), "no `{event}` row captured");
    for c in &found {
        assert!(c.has_field("account_id", &ACCOUNT_ID.to_string()), "{c:#?}");
        assert!(c.has_field("account_name", ACCOUNT_NAME), "{c:#?}");
    }
    found
}

async fn cleanup(pool: &PgPool) {
    // Lists cascade from the player row.
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(PLAYER_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACCOUNT_ID as i32)
        .execute(pool)
        .await;
}

async fn insert_player(pool: &PgPool) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(ACCOUNT_ID as i32)
        .bind(ACCOUNT_NAME)
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0)",
    )
    .bind(ACCOUNT_ID as i32)
    .bind(PLAYER_ID)
    .bind(PLAYER_NAME)
    .execute(pool)
    .await
    .expect("insert player");
}

/// No pool: every header op logs a WARN row with its `event` and the
/// account identity, and sends nothing.
#[tokio::test]
async fn no_pool_rows_carry_event_and_account() {
    let (connected, entity_to_addr) = session();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let capture = LogCapture::install();
    handle_create(
        ENTITY_ID,
        PLAYER_ID,
        "Allies".into(),
        0,
        &None,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    handle_delete(
        ENTITY_ID,
        PLAYER_ID,
        7,
        &None,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    handle_rename(
        ENTITY_ID,
        PLAYER_ID,
        7,
        "Friends".into(),
        &None,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    handle_flags_update(
        ENTITY_ID,
        PLAYER_ID,
        7,
        4,
        &None,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    for event in [
        EVENT_CREATED,
        EVENT_DELETED,
        EVENT_RENAMED,
        EVENT_FLAGS_UPDATED,
    ] {
        let found = rows(&capture, event);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].level, tracing::Level::WARN);
        assert!(found[0].has_field("player_id", &PLAYER_ID.to_string()));
    }
}

/// Live DB: create, duplicate create, rename, flags, delete. The create
/// row is INFO; the duplicate is WARN with `reason=duplicate_name`.
#[tokio::test]
async fn live_db_header_ops_log_event_account_and_duplicate_reason() {
    let pool = require_db_or_skip!();
    cleanup(&pool).await;
    insert_player(&pool).await;
    let db = Some(Arc::new(pool.clone()));
    let (connected, entity_to_addr) = session();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

    let capture = LogCapture::install();
    handle_create(
        ENTITY_ID,
        PLAYER_ID,
        "Allies".into(),
        0,
        &db,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    handle_create(
        ENTITY_ID,
        PLAYER_ID,
        "Allies".into(),
        0,
        &db,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    let list_id: i32 =
        sqlx::query_scalar("SELECT list_id FROM sgw_contact_list WHERE player_id = $1")
            .bind(PLAYER_ID)
            .fetch_one(&pool)
            .await
            .expect("list row");
    handle_rename(
        ENTITY_ID,
        PLAYER_ID,
        list_id,
        "Friends".into(),
        &db,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    handle_flags_update(
        ENTITY_ID,
        PLAYER_ID,
        list_id,
        4,
        &db,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    handle_delete(
        ENTITY_ID,
        PLAYER_ID,
        list_id,
        &db,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    cleanup(&pool).await;

    let created = rows(&capture, EVENT_CREATED);
    assert_eq!(created.len(), 2, "{created:#?}");
    assert!(created.iter().any(|c| c.level == tracing::Level::INFO));
    let dup = created
        .iter()
        .find(|c| c.has_field("reason", "duplicate_name"))
        .unwrap_or_else(|| panic!("no duplicate_name row: {created:#?}"));
    assert_eq!(dup.level, tracing::Level::WARN);
    assert!(dup.has_field("player_id", &PLAYER_ID.to_string()));

    for (event, level) in [
        (EVENT_RENAMED, tracing::Level::INFO),
        (EVENT_FLAGS_UPDATED, tracing::Level::INFO),
        (EVENT_DELETED, tracing::Level::INFO),
    ] {
        let found = rows(&capture, event);
        assert!(
            found.iter().any(|c| c.level == level),
            "{event}: {found:#?}"
        );
    }
}
