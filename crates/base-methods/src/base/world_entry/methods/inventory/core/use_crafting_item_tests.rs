//! Live-DB guards for `useItem` on a crafting item, end to end through
//! `handle_use_inventory_item`: the item is routed to the crafting use
//! (never `OnItemUse`), consumed with the crafting change, and the client's
//! inventory follows.
//!
//! Sentinels in the crafting `0x7000_Cxxx` block: `0x7000_CEC0..0x7000_CECF`
//! (account, player pairs; the account id doubles as the entity id).

use cimmeria_wire::cell::vault::VaultAccess;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::inventory::INV_CRAFTING;
use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::crafting::racial_paradigm_level_args;
use sqlx::PgPool;

use super::handle_use_inventory_item;
use crate::base::crafting::feedback::feedback_text_args;
use crate::mercury::{build_player_entity_method_packet, method_idx};
use crate::test_support::{require_db_or_skip, test_default_connected_client_state, TestTransport};

const TEST_BASE: i32 = 0x7000_CEC0;
/// Slappack TC1: an ordinary item, no crafting effect rows.
const SLAPPACK: i32 = 2893;
/// "Racial Paradigm Guide: Goa'uld" (paradigm 3).
const GOAULD_GUIDE: i32 = 7808;

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    for sql in [
        "DELETE FROM cell_event_outbox WHERE entity_id = $1",
        "DELETE FROM sgw_inventory WHERE character_id = $1",
        "DELETE FROM sgw_player_discipline_expertise WHERE player_id = $1",
        "DELETE FROM sgw_player WHERE player_id = $1",
    ] {
        let id = if sql.contains("outbox") {
            account_id
        } else {
            player_id
        };
        let _ = sqlx::query(sql).bind(id).execute(pool).await;
    }
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

async fn insert_player(pool: &PgPool, account_id: i32, player_id: i32) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("use-craft-{account_id}"))
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
    .bind(account_id)
    .bind(player_id)
    .bind(format!("use-craft-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
}

type Session = (
    Arc<TestTransport>,
    Arc<dyn Transport>,
    Arc<Mutex<HashMap<SocketAddr, crate::base::ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    SocketAddr,
);

fn session(entity_id: u32, port: u16) -> Session {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        test_default_connected_client_state(),
    )])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    (typed, transport, connected, entity_to_addr, addr)
}

/// The bytes a player method send produces on a fresh session.
fn packet(entity_id: u32, seq: u32, method: u16, args: &[u8]) -> Vec<u8> {
    build_player_entity_method_packet(
        &[0u8; 32],
        seq,
        &[],
        entity_id,
        method,
        args,
        EncryptionVersion::V1,
    )
}

/// A guide used from the crafting bag goes to the crafting use: the
/// paradigm rises, the item is gone, the client gets 138 and then
/// `onRemoveItem`, and the cell is told the item was removed. No
/// `item_used` row is queued, so no content chain can also consume it.
#[tokio::test]
async fn live_db_a_guide_is_used_by_crafting_not_on_item_use() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE, TEST_BASE + 1);
    let entity_id = account_id as u32;
    cleanup(&pool, account_id, player_id).await;
    insert_player(&pool, account_id, player_id).await;
    let item_id: i32 = sqlx::query_scalar(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, container_id) \
         VALUES ($1, $2, 1, 0, $3) RETURNING item_id",
    )
    .bind(player_id)
    .bind(GOAULD_GUIDE)
    .bind(INV_CRAFTING)
    .fetch_one(&pool)
    .await
    .expect("insert guide");
    let (typed, transport, connected, entity_to_addr, addr) = session(entity_id, 55810);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_use_inventory_item(
        entity_id,
        player_id,
        item_id,
        0,
        VaultAccess::NO_SESSION,
        &db_pool,
        &None,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    let levels: Vec<i32> =
        sqlx::query_scalar("SELECT racial_paradigm_levels FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .fetch_one(&pool)
            .await
            .expect("levels");
    let left: Option<i32> =
        sqlx::query_scalar("SELECT item_id FROM sgw_inventory WHERE item_id = $1")
            .bind(item_id)
            .fetch_optional(&pool)
            .await
            .expect("instance");
    let outbox: Vec<String> = sqlx::query_scalar(
        "SELECT event_type FROM cell_event_outbox WHERE entity_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(&pool)
    .await
    .expect("outbox");
    let sent = typed.filter_to(addr);
    cleanup(&pool, account_id, player_id).await;

    assert_eq!(levels, vec![5, 1, 2, 1, 1], "Goa'uld raised from 1 to 2");
    assert_eq!(left, None, "the guide was consumed");
    assert_eq!(outbox, vec!["inventory_item_removed".to_string()]);
    let mut remove_args = 1u32.to_le_bytes().to_vec();
    remove_args.extend_from_slice(&item_id.to_le_bytes());
    assert_eq!(
        sent[..2],
        [
            packet(
                entity_id,
                0,
                cimmeria_wire::cell::client_methods::player::ON_UPDATE_RACIAL_PARADIGM_LEVEL,
                &racial_paradigm_level_args(3, 2)
            ),
            packet(entity_id, 1, method_idx::ON_REMOVE_ITEM, &remove_args),
        ]
    );
    assert_eq!(
        sent[2],
        packet(
            entity_id,
            2,
            method_idx::ON_UPDATE_ITEM,
            &0u32.to_le_bytes()
        ),
        "the inventory update, now empty"
    );
}

/// Insert one instance of `type_id` in bag 15 for `player_id`.
async fn insert_owned(pool: &PgPool, player_id: i32, type_id: i32) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, container_id) \
         VALUES ($1, $2, 1, 0, $3) RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(INV_CRAFTING)
    .fetch_one(pool)
    .await
    .expect("insert item")
}

/// `useItem(item_id)` from `entity_id` through the real entry point.
async fn use_through_entry(
    pool: &PgPool,
    s: &Session,
    entity_id: u32,
    player_id: i32,
    item_id: i32,
) {
    handle_use_inventory_item(
        entity_id,
        player_id,
        item_id,
        0,
        VaultAccess::NO_SESSION,
        &Some(Arc::new(pool.clone())),
        &None,
        &s.1,
        &s.2,
        &s.3,
    )
    .await;
}

const GONE: &str = "That item is no longer in your inventory.";

/// Another character's guide: the lookup by owner misses, and the use gets
/// the crafting refusal line instead of silence. The owner keeps the guide.
#[tokio::test]
async fn live_db_another_characters_guide_is_refused_with_a_line() {
    let pool = require_db_or_skip!();
    let (user_account, user) = (TEST_BASE + 2, TEST_BASE + 3);
    let (owner_account, owner) = (TEST_BASE + 4, TEST_BASE + 5);
    let entity_id = user_account as u32;
    for (a, p) in [(user_account, user), (owner_account, owner)] {
        cleanup(&pool, a, p).await;
        insert_player(&pool, a, p).await;
    }
    let item_id = insert_owned(&pool, owner, GOAULD_GUIDE).await;
    let s = session(entity_id, 55811);

    use_through_entry(&pool, &s, entity_id, user, item_id).await;

    let owner_has: Option<i32> =
        sqlx::query_scalar("SELECT character_id FROM sgw_inventory WHERE item_id = $1")
            .bind(item_id)
            .fetch_optional(&pool)
            .await
            .expect("instance");
    let sent = s.0.filter_to(s.4);
    for (a, p) in [(user_account, user), (owner_account, owner)] {
        cleanup(&pool, a, p).await;
    }
    assert_eq!(owner_has, Some(owner), "the owner keeps the guide");
    assert_eq!(
        sent,
        vec![packet(
            entity_id,
            0,
            method_idx::ON_PLAYER_COMMUNICATION,
            &feedback_text_args(GONE)
        )]
    );
}

/// A second press on a guide already used up: the row is gone, and the
/// press still gets the refusal line.
#[tokio::test]
async fn live_db_a_replayed_guide_use_is_refused_with_a_line() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE + 6, TEST_BASE + 7);
    let entity_id = account_id as u32;
    cleanup(&pool, account_id, player_id).await;
    insert_player(&pool, account_id, player_id).await;
    let item_id = insert_owned(&pool, player_id, GOAULD_GUIDE).await;
    let s = session(entity_id, 55812);

    use_through_entry(&pool, &s, entity_id, player_id, item_id).await;
    let after_first = s.0.filter_to(s.4).len();
    use_through_entry(&pool, &s, entity_id, player_id, item_id).await;

    let levels: Vec<i32> =
        sqlx::query_scalar("SELECT racial_paradigm_levels FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .fetch_one(&pool)
            .await
            .expect("levels");
    let sent = s.0.filter_to(s.4);
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(levels, vec![5, 1, 2, 1, 1], "raised once");
    assert_eq!(sent.len(), after_first + 1, "one more packet: the line");
    assert_eq!(
        sent[after_first],
        packet(
            entity_id,
            after_first as u32,
            method_idx::ON_PLAYER_COMMUNICATION,
            &feedback_text_args(GONE)
        )
    );
}

/// Another character's ordinary item keeps the old behavior: nothing is
/// sent and nothing is queued.
#[tokio::test]
async fn live_db_another_characters_ordinary_item_stays_silent() {
    let pool = require_db_or_skip!();
    let (user_account, user) = (TEST_BASE + 8, TEST_BASE + 9);
    let (owner_account, owner) = (TEST_BASE + 10, TEST_BASE + 11);
    let entity_id = user_account as u32;
    for (a, p) in [(user_account, user), (owner_account, owner)] {
        cleanup(&pool, a, p).await;
        insert_player(&pool, a, p).await;
    }
    let item_id = insert_owned(&pool, owner, SLAPPACK).await;
    let s = session(entity_id, 55813);

    use_through_entry(&pool, &s, entity_id, user, item_id).await;

    let outbox: i64 =
        sqlx::query_scalar("SELECT count(*) FROM cell_event_outbox WHERE entity_id = $1")
            .bind(user_account)
            .fetch_one(&pool)
            .await
            .expect("outbox");
    let sent = s.0.filter_to(s.4);
    for (a, p) in [(user_account, user), (owner_account, owner)] {
        cleanup(&pool, a, p).await;
    }
    assert!(sent.is_empty(), "no packet: {sent:?}");
    assert_eq!(outbox, 0);
}
