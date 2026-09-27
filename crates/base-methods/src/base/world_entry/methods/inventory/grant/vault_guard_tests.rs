//! Live-DB guard: a grant never writes into the vaults (17-20).
//!
//! The vault has a capacity for `onBagInfo`, so without
//! `grant_container_refused` a grant into the vault would land there. An
//! item that also lists a carried bag (the `{17,15}` crafting components)
//! falls through to that bag instead (`fall_through_tests`); the refusal is
//! for an item that lists only storage containers, which the seed does not
//! have, so the guard uses a synthetic `{17}` type.
//!
//! Sentinels: account `0x7000_B140`, player `0x7000_B141`, entity
//! `0x7000_B1E4`, the synthetic item type `0x7000_C4F0`.

use tracing::Level;

use super::*;
use crate::test_support::{require_db_or_skip, LogCapture, TestTransport};

const ACCOUNT_ID: i32 = 0x7000_B140;
const PLAYER_ID: i32 = 0x7000_B141;
const ENTITY_ID: u32 = 0x7000_B1E4;
/// A storage-only item type (`container_sets = {17}`).
const STORAGE_ONLY_TYPE_ID: i32 = 0x7000_C4F0;

async fn cleanup(pool: &PgPool) {
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
        .bind(ENTITY_ID as i32)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
        .bind(PLAYER_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACCOUNT_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
        .bind(STORAGE_ONLY_TYPE_ID)
        .execute(pool)
        .await;
}

async fn insert_account_and_player(pool: &PgPool) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(ACCOUNT_ID)
        .bind(format!("bv01-grant-{ACCOUNT_ID}"))
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
    .bind(ACCOUNT_ID)
    .bind(PLAYER_ID)
    .bind(format!("test-{PLAYER_ID}"))
    .execute(pool)
    .await
    .expect("insert player");
}

#[tokio::test]
async fn grant_into_vault_is_refused() {
    let pool = require_db_or_skip!();
    cleanup(&pool).await;
    insert_account_and_player(&pool).await;
    // An item that may only sit in the vault: no carried bag to fall
    // through to.
    let type_id = STORAGE_ONLY_TYPE_ID;
    sqlx::query(
        "INSERT INTO resources.items (\
            item_id, description, name, quality_id, tech_comp, tier, \
            max_stack_size, container_sets \
         ) VALUES ($1, '', 'storage-only', 'ITEM_QUALITY_Normal', 0, 1, 1, '{17}')",
    )
    .bind(type_id)
    .execute(&pool)
    .await
    .expect("insert the storage-only item type");

    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let e2a: Arc<Mutex<HashMap<u32, SocketAddr>>> = Arc::new(Mutex::new(HashMap::new()));
    let conn: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let db_pool = Some(Arc::new(pool.clone()));
    let capture = LogCapture::install();

    handle_grant_item(
        ENTITY_ID, PLAYER_ID, type_id, 17, 1, false, &db_pool, &None, &transport, &conn, &e2a,
    )
    .await;

    let rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_inventory WHERE character_id = $1")
            .bind(PLAYER_ID)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 0, "a grant into the vault (17) must write no row");
    let event = capture
        .find_event(
            Level::WARN,
            "grant_rejected",
            "grant_into_storage_container",
        )
        .expect("refusal must log grant_rejected with reason=grant_into_storage_container");
    assert_eq!(event.target, "bank");
    // A grant names a type, not an instance: no instance `item_id`, no
    // source position.
    for (key, value) in [
        ("event", "grant_rejected".to_string()),
        ("account_id", ACCOUNT_ID.to_string()),
        ("player_id", PLAYER_ID.to_string()),
        ("entity_id", ENTITY_ID.to_string()),
        ("type_id", type_id.to_string()),
        ("quantity", "1".to_string()),
        ("target_container_id", "17".to_string()),
    ] {
        assert_eq!(event.fields.get(key), Some(&value), "field `{key}`");
    }
    for key in ["item_id", "source_container_id", "source_slot_id"] {
        assert!(
            !event.fields.contains_key(key),
            "field `{key}` must be absent"
        );
    }

    cleanup(&pool).await;
}

/// `grant_rejected reason=account_lookup_failed`: the database is
/// unreachable, so the refusal cannot read the account id. The grant is
/// still refused, and the failed lookup is logged with every correlator it
/// has (player, entity, type, target container) and no `account_id`. No
/// database needed: the pool can never connect.
#[tokio::test]
async fn grant_refusal_logs_account_lookup_failed_when_the_database_is_down() {
    let pool = Arc::new(
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(std::time::Duration::from_millis(50))
            .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/none")
            .expect("connect_lazy must succeed for any well-formed URL"),
    );
    let capture = LogCapture::install();

    let refused =
        super::validation::refuse_storage_grant(&pool, ENTITY_ID, PLAYER_ID, 4242, 17, 1).await;

    assert!(
        refused,
        "the grant is refused whether or not the lookup works"
    );
    let event = capture
        .find_event(Level::WARN, "grant_rejected", "account_lookup_failed")
        .expect("a failed account lookup must log grant_rejected reason=account_lookup_failed");
    assert_eq!(event.target, "bank");
    for (key, value) in [
        ("event", "grant_rejected".to_string()),
        ("player_id", PLAYER_ID.to_string()),
        ("entity_id", ENTITY_ID.to_string()),
        ("type_id", "4242".to_string()),
        ("target_container_id", "17".to_string()),
    ] {
        assert_eq!(event.fields.get(key), Some(&value), "field `{key}`");
    }
    assert!(
        !event.fields.contains_key("account_id"),
        "the account id is what failed to load"
    );
}
