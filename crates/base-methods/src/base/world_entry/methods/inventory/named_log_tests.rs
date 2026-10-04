//! NT-22 (Rule 6): the inventory's most-read lines name what they carry.
//! A grant (`grant_container_chosen`, what loot pickup and `gmGiveItem`
//! write), a use (`firing ItemUsed`) and a move (`Inventory move
//! persisted`) each name the player and the item next to their IDs.
//!
//! Live-DB tests skip when `DATABASE_URL` is unset. Sentinels: accounts and
//! players `0x7000_D210..=0x7000_D231`, entities `0x7000_D2E1..=0x7000_D2E3`,
//! ports 40970-40972. The item is the seeded Health Slappack TC1 (2893,
//! `container_sets = {1,17}`, not bandolier-eligible), so a use fires
//! `ItemUsed` rather than the auto-equip move. The NameBook is a test book
//! holding the seed's spelling.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::known_names;
use cimmeria_mercury::transport::Transport;
use cimmeria_names::{NameBook, Table};
use sqlx::PgPool;
use tracing::Level;

use super::{handle_grant_item, handle_move_inventory_item, handle_use_inventory_item};
use crate::base::ConnectedClientState;
use crate::test_support::{
    require_db_or_skip, test_default_connected_client_state, Captured, LogCapture, TestTransport,
};

const SLAPPACK: i32 = 2893;
// The seed's name: `grant_container_chosen` reads it from `resources.items`
// with the placement, the other lines from the NameBook.
const ITEM_NAME: &str = "Health Slappack TC1";

struct Who {
    account_id: i32,
    player_id: i32,
    entity_id: u32,
    name: &'static str,
}

fn name_the_world(who: &Who) {
    let mut book = NameBook::empty();
    book.insert(Table::Items, i64::from(SLAPPACK), ITEM_NAME);
    book.insert(Table::Containers, 1, "MAIN");
    cimmeria_names::global().store(book);
    known_names::remember_player(who.player_id, who.name);
    known_names::remember_account(who.account_id, "nt22_login");
}

async fn cleanup(pool: &PgPool, who: &Who) {
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
        .bind(i64::from(who.entity_id))
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
        .bind(who.player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(who.account_id)
        .execute(pool)
        .await;
}

async fn setup(pool: &PgPool, who: &Who) {
    cleanup(pool, who).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(who.account_id)
        .bind(format!("nt22-{}", who.account_id))
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
    .bind(who.account_id)
    .bind(who.player_id)
    .bind(format!("nt22-{}", who.player_id))
    .execute(pool)
    .await
    .expect("insert player");
}

async fn insert(pool: &PgPool, player_id: i32, slot_id: i32) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
         VALUES ($1, $2, 1, $3, 1, false, 100, 0) RETURNING item_id",
    )
    .bind(player_id)
    .bind(SLAPPACK)
    .bind(slot_id)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

type Session = (
    Arc<dyn Transport>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
);

fn session(entity_id: u32, port: u16) -> Session {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(entity_id);
    (
        transport,
        Arc::new(Mutex::new(HashMap::from([(entity_id, addr)]))),
        Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
    )
}

fn assert_named(line: &Captured, who: &Who) {
    for (k, v) in [
        ("player_id", who.player_id.to_string()),
        ("player_name", who.name.to_string()),
        ("entity_id", who.entity_id.to_string()),
        ("entity_name", who.name.to_string()),
        ("item_name", ITEM_NAME.to_string()),
    ] {
        assert!(line.has_field(k, &v), "{k}={v}: {line:#?}");
    }
}

/// A grant (loot pickup, content `grant_item`, `gmGiveItem`) names the
/// account, the character and the container it chose.
#[tokio::test]
async fn live_db_grant_container_chosen_names_the_player_and_the_bag() {
    let pool = require_db_or_skip!();
    let who = Who {
        account_id: 0x7000_D210,
        player_id: 0x7000_D211,
        entity_id: 0x7000_D2E1,
        name: "Cameron Mitchell",
    };
    setup(&pool, &who).await;
    name_the_world(&who);
    let s = session(who.entity_id, 40970);
    let capture = LogCapture::install();

    handle_grant_item(
        who.entity_id,
        who.player_id,
        SLAPPACK,
        1,
        1,
        false,
        &Some(Arc::new(pool.clone())),
        &None,
        &s.0,
        &s.2,
        &s.1,
    )
    .await;

    let line = capture
        .find_message(Level::INFO, "grant_container_chosen")
        .expect("grant_container_chosen");
    assert_named(&line, &who);
    assert!(line.has_field("account_name", "nt22_login"), "{line:#?}");
    assert!(line.has_field("container_name", "MAIN"), "{line:#?}");

    cleanup(&pool, &who).await;
}

/// A use names the character and the item, not just the instance id the
/// client sent.
#[tokio::test]
async fn live_db_item_use_names_the_player_and_the_item() {
    let pool = require_db_or_skip!();
    let who = Who {
        account_id: 0x7000_D220,
        player_id: 0x7000_D221,
        entity_id: 0x7000_D2E2,
        name: "Jonas Quinn",
    };
    setup(&pool, &who).await;
    name_the_world(&who);
    let item = insert(&pool, who.player_id, 0).await;
    let s = session(who.entity_id, 40971);
    let capture = LogCapture::install();

    handle_use_inventory_item(
        who.entity_id,
        who.player_id,
        item,
        0,
        cimmeria_wire::cell::vault::VaultAccess::NO_SESSION,
        &Some(Arc::new(pool.clone())),
        &None,
        &s.0,
        &s.2,
        &s.1,
    )
    .await;

    let line = capture
        .find_message(Level::INFO, "firing ItemUsed")
        .expect("firing ItemUsed");
    assert_named(&line, &who);
    assert!(line.has_field("item_id", &item.to_string()), "{line:#?}");
    assert!(
        line.has_field("item_type_id", &SLAPPACK.to_string()),
        "{line:#?}"
    );

    cleanup(&pool, &who).await;
}

/// A move between bag slots names the character and the item it moved.
#[tokio::test]
async fn live_db_inventory_move_names_the_player_and_the_item() {
    let pool = require_db_or_skip!();
    let who = Who {
        account_id: 0x7000_D230,
        player_id: 0x7000_D231,
        entity_id: 0x7000_D2E3,
        name: "Hank Landry",
    };
    setup(&pool, &who).await;
    name_the_world(&who);
    let item = insert(&pool, who.player_id, 0).await;
    let s = session(who.entity_id, 40972);
    let capture = LogCapture::install();

    handle_move_inventory_item(
        who.entity_id,
        who.player_id,
        item,
        1,
        5,
        -1,
        &Some(Arc::new(pool.clone())),
        &None,
        &s.0,
        &s.2,
        &s.1,
    )
    .await;

    let line = capture
        .find_message(Level::DEBUG, "Inventory move persisted")
        .expect("Inventory move persisted");
    assert_named(&line, &who);

    cleanup(&pool, &who).await;
}
