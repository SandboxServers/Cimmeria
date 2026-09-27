//! Live-DB guard: a grant never writes into the vaults (17-20).
//!
//! Loot and content grants target an item's first `container_sets` entry,
//! and the seeded crafting components list 17 first (`{17,15}`). Before
//! BV-01 such a grant failed at slot reservation because 17 had no
//! capacity. BV-01 gave 17 a capacity for `onBagInfo`, so without
//! `grant_container_refused` the loot would land in the bank.
//!
//! Sentinels: account `0x7000_B140`, player `0x7000_B141`, entity
//! `0x7000_B1E4`.

use tracing::Level;

use super::*;
use crate::test_support::{require_db_or_skip, LogCapture, TestTransport};

const ACCOUNT_ID: i32 = 0x7000_B140;
const PLAYER_ID: i32 = 0x7000_B141;
const ENTITY_ID: u32 = 0x7000_B1E4;

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
    // A seeded item whose preferred container (what loot and content grant
    // into) is the vault.
    let type_id: i32 = sqlx::query_scalar(
        "SELECT item_id FROM resources.items WHERE container_sets[1] = 17 \
         ORDER BY item_id LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("the seed must hold an item whose first container_sets entry is 17");

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
