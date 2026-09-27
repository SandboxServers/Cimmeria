//! BV-03 guards for the player-accessible container rule on use and
//! removal (`useItem`, `removeItem`, content `RemoveItem` by type), the
//! `use_rejected` event, and the `bank_slots` bound on vault slot
//! reservation.
//!
//! Live-DB tests skip when `DATABASE_URL` is unset; the two infrastructure
//! guards at the end need no database. Sentinels: accounts and players
//! `0x7000_B600..=0x7000_B641`, entities `0x7000_B6E0..=0x7000_B6E5`,
//! ports 40850-40859. The item is the seeded Slappack TC1 (2893,
//! `container_sets = {1,17}`, not bandolier-eligible), so a use fires
//! `ItemUsed` rather than the auto-equip move.

use cimmeria_entity::cell_entity::VaultScope;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::vault::VaultAccess;
use sqlx::PgPool;
use tracing::Level;

use super::access::{refuse_inaccessible, AccessOp};
use super::{
    handle_remove_inventory_item, handle_remove_inventory_item_by_type, handle_use_inventory_item,
};
use crate::base::world_entry::methods::vendor::serializers::reserve_free_inventory_slots;
use crate::base::ConnectedClientState;
use crate::test_support::{
    require_db_or_skip, test_default_connected_client_state, LogCapture, TestTransport,
};

const BASE: i32 = 0x7000_B600;
const SLAPPACK: i32 = 2893;
const OPEN: VaultAccess = VaultAccess::Open {
    scope: VaultScope::Personal,
    banker_id: Some(0x7000_B6D0),
    distance: Some(1.0),
};

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32, entity_id: u32) {
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
        .bind(i64::from(entity_id))
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

async fn setup(pool: &PgPool, account_id: i32, player_id: i32, entity_id: u32) {
    cleanup(pool, account_id, player_id, entity_id).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("bv03-access-{account_id}"))
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
    .bind(format!("bv03-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
}

async fn insert(pool: &PgPool, player_id: i32, container_id: i32, slot_id: i32) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
         VALUES ($1, $2, 1, $3, $4, false, 100, 0) RETURNING item_id",
    )
    .bind(player_id)
    .bind(SLAPPACK)
    .bind(slot_id)
    .bind(container_id)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

async fn containers_of(pool: &PgPool, player_id: i32) -> Vec<i32> {
    sqlx::query_scalar(
        "SELECT container_id FROM sgw_inventory WHERE character_id = $1 ORDER BY container_id",
    )
    .bind(player_id)
    .fetch_all(pool)
    .await
    .expect("containers query")
}

async fn item_used_rows(pool: &PgPool, entity_id: u32) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM cell_event_outbox \
         WHERE entity_id = $1 AND event_type = 'item_used'",
    )
    .bind(i64::from(entity_id))
    .fetch_one(pool)
    .await
    .expect("outbox count")
}

type Session = (
    Arc<TestTransport>,
    Arc<dyn Transport>,
    SocketAddr,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
);

fn session(entity_id: u32, port: u16) -> Session {
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(entity_id);
    (
        transport,
        dyn_transport,
        addr,
        Arc::new(Mutex::new(HashMap::from([(entity_id, addr)]))),
        Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
    )
}

fn saw_text(transport: &TestTransport, addr: SocketAddr, needle: &str) -> bool {
    let want: Vec<u8> = needle.encode_utf16().flat_map(u16::to_le_bytes).collect();
    // The test session's key is all zeros (`test_default_connected_client_state`).
    let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
    transport
        .filter_to(addr)
        .iter()
        .filter_map(|p| enc.decrypt(p).ok())
        .any(|p| p.windows(want.len()).any(|w| w == want.as_slice()))
}

async fn use_item(
    pool: &PgPool,
    s: &Session,
    entity_id: u32,
    player_id: i32,
    item: i32,
    vault: VaultAccess,
) {
    handle_use_inventory_item(
        entity_id,
        player_id,
        item,
        0,
        vault,
        &Some(Arc::new(pool.clone())),
        &None,
        &s.1,
        &s.4,
        &s.3,
    )
    .await;
}

/// An item sitting in buyback (16) cannot be used: before BV-03 `useItem`
/// found it by id in any container and fired `ItemUsed`. Now no outbox row
/// is written, `use_rejected reason=container_not_accessible` names the
/// container, and the player is told.
#[tokio::test]
async fn using_an_item_in_buyback_is_refused() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE, BASE + 1, 0x7000_B6E0);
    setup(&pool, account_id, player_id, entity_id).await;
    let item = insert(&pool, player_id, 16, 0).await;
    let s = session(entity_id, 40850);
    let capture = LogCapture::install();

    use_item(&pool, &s, entity_id, player_id, item, OPEN).await;

    assert_eq!(item_used_rows(&pool, entity_id).await, 0, "no ItemUsed");
    let event = capture
        .find_event(Level::WARN, "use_rejected", "container_not_accessible")
        .expect("use_rejected");
    assert_eq!(event.target, "bank");
    for (k, v) in [
        ("event", "use_rejected".to_string()),
        ("account_id", account_id.to_string()),
        ("player_id", player_id.to_string()),
        ("entity_id", entity_id.to_string()),
        ("item_id", item.to_string()),
        ("container", "16".to_string()),
        ("op", "use".to_string()),
    ] {
        assert!(event.has_field(k, &v), "{k}={v}: {event:#?}");
    }
    assert!(saw_text(&s.0, s.2, "That item is not in your inventory."));

    cleanup(&pool, account_id, player_id, entity_id).await;
}

/// A banked item is usable only with a vault session: refused with none
/// (no `ItemUsed`, `vault_reason=no_vault_session`), accepted with one.
#[tokio::test]
async fn using_a_banked_item_needs_a_vault_session() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x10, BASE + 0x11, 0x7000_B6E1);
    setup(&pool, account_id, player_id, entity_id).await;
    let item = insert(&pool, player_id, 17, 0).await;
    let s = session(entity_id, 40851);
    let capture = LogCapture::install();

    use_item(
        &pool,
        &s,
        entity_id,
        player_id,
        item,
        VaultAccess::NO_SESSION,
    )
    .await;
    assert_eq!(
        item_used_rows(&pool, entity_id).await,
        0,
        "no session: no use"
    );
    let event = capture
        .find_event(Level::WARN, "use_rejected", "container_not_accessible")
        .expect("use_rejected");
    assert!(event.has_field("container", "17"), "{event:#?}");
    assert!(
        event.has_field("vault_reason", "no_vault_session"),
        "{event:#?}"
    );
    assert!(saw_text(&s.0, s.2, "That item is in your vault."));

    use_item(&pool, &s, entity_id, player_id, item, OPEN).await;
    assert_eq!(
        item_used_rows(&pool, entity_id).await,
        1,
        "at the Banker: used"
    );

    cleanup(&pool, account_id, player_id, entity_id).await;
}

/// `removeItem` of a banked item without a session keeps the row and logs
/// `use_rejected op=remove`; with a session it deletes it.
#[tokio::test]
async fn removing_a_banked_item_needs_a_vault_session() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x20, BASE + 0x21, 0x7000_B6E2);
    setup(&pool, account_id, player_id, entity_id).await;
    let item = insert(&pool, player_id, 17, 0).await;
    let s = session(entity_id, 40852);
    let db_pool = Some(Arc::new(pool.clone()));
    let capture = LogCapture::install();

    for vault in [VaultAccess::NO_SESSION, OPEN] {
        handle_remove_inventory_item(
            entity_id, player_id, item, 1, false, vault, &db_pool, &None, &s.1, &s.4, &s.3,
        )
        .await;
        if !vault.is_open() {
            assert_eq!(containers_of(&pool, player_id).await, vec![17], "kept");
            let event = capture
                .find_event(Level::WARN, "use_rejected", "container_not_accessible")
                .expect("use_rejected");
            assert!(event.has_field("op", "remove"), "{event:#?}");
        }
    }
    assert!(
        containers_of(&pool, player_id).await.is_empty(),
        "removed at the Banker"
    );

    cleanup(&pool, account_id, player_id, entity_id).await;
}

/// Content `RemoveItem` by type searches only reachable containers: with
/// instances in buyback and the vault and none carried, nothing is taken
/// without a session, and with one the vault instance is taken while the
/// buyback one stays. Before BV-03 the first call took the buyback row.
#[tokio::test]
async fn remove_by_type_never_searches_buyback_or_a_closed_vault() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x30, BASE + 0x31, 0x7000_B6E3);
    setup(&pool, account_id, player_id, entity_id).await;
    insert(&pool, player_id, 16, 0).await;
    insert(&pool, player_id, 17, 0).await;
    let s = session(entity_id, 40853);
    let db_pool = Some(Arc::new(pool.clone()));

    for (vault, want) in [(VaultAccess::NO_SESSION, vec![16, 17]), (OPEN, vec![16])] {
        handle_remove_inventory_item_by_type(
            entity_id, player_id, SLAPPACK, 1, vault, &db_pool, &None, &s.1, &s.4, &s.3,
        )
        .await;
        assert_eq!(containers_of(&pool, player_id).await, want, "{vault:?}");
    }

    cleanup(&pool, account_id, player_id, entity_id).await;
}

/// Reserving vault slots stops at the player's `bank_slots`, not at the
/// ceiling of 100: with slots 0-39 full in a 40-slot vault there is no
/// room; after the vault grows to 50, slot 40 is the one reserved. Fails
/// with the bound removed (slot 40 is handed out to a 40-slot player).
#[tokio::test]
async fn reserving_vault_slots_stops_at_bank_slots() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x40, BASE + 0x41, 0x7000_B6E4);
    setup(&pool, account_id, player_id, entity_id).await;
    for slot in 0..40 {
        insert(&pool, player_id, 17, slot).await;
    }

    let mut tx = pool.begin().await.unwrap();
    let full = reserve_free_inventory_slots(&mut tx, player_id, 17, 1)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(full, None, "a full 40-slot vault has no free slot");

    sqlx::query("UPDATE sgw_player SET bank_slots = 50 WHERE player_id = $1")
        .bind(player_id)
        .execute(&pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    let grown = reserve_free_inventory_slots(&mut tx, player_id, 17, 1)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(grown, Some(vec![40]));

    cleanup(&pool, account_id, player_id, entity_id).await;
}

fn unreachable_pool() -> PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(50))
        .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/none")
        .expect("connect_lazy must succeed for any well-formed URL")
}

/// Database down: the refusal still logs `use_rejected`, first with
/// `reason=account_lookup_failed` and then `container_not_accessible`
/// without `account_id`, and still tells the player.
#[tokio::test]
async fn use_refusal_logs_account_lookup_failed_when_the_database_is_down() {
    let s = session(0x7000_B6E5, 40858);
    let capture = LogCapture::install();

    refuse_inaccessible(
        AccessOp::Use,
        0x7000_B6E5,
        0x7000_B651,
        77,
        16,
        &VaultAccess::NO_SESSION,
        &unreachable_pool(),
        &s.1,
        &s.4,
        &s.3,
    )
    .await;

    let failed = capture
        .find_event(Level::WARN, "use_rejected", "account_lookup_failed")
        .expect("account_lookup_failed");
    assert_eq!(failed.target, "bank");
    assert!(failed.has_field("player_id", &0x7000_B651.to_string()));
    assert!(failed.has_field("entity_id", &0x7000_B6E5u32.to_string()));
    let refused = capture
        .find_event(Level::WARN, "use_rejected", "container_not_accessible")
        .expect("the refusal itself");
    assert!(!refused.fields.contains_key("account_id"), "{refused:#?}");
    assert!(saw_text(&s.0, s.2, "That item is not in your inventory."));
}

/// No client address for the entity: `bank_feedback_send_failed
/// reason=no_client_address`, instead of a silent drop.
#[tokio::test]
async fn use_refusal_without_a_client_address_logs_bank_feedback_send_failed() {
    let s = session(0x7000_B6E5, 40859);
    let capture = LogCapture::install();

    refuse_inaccessible(
        AccessOp::Remove,
        0x7000_B6E6,
        0x7000_B651,
        78,
        17,
        &VaultAccess::NO_SESSION,
        &unreachable_pool(),
        &s.1,
        &s.4,
        &Arc::new(Mutex::new(HashMap::new())),
    )
    .await;

    let event = capture
        .find_event(
            Level::WARN,
            "bank_feedback_send_failed",
            "no_client_address",
        )
        .expect("bank_feedback_send_failed");
    assert_eq!(event.target, "bank");
    assert!(event.has_field("entity_id", &0x7000_B6E6u32.to_string()));
    assert_eq!(s.0.send_count_to(s.2), 0);
}
