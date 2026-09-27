//! Tests for the GM crafting grants: the pure kit and learn rules, then
//! each command end to end against a live database.
//!
//! Crafting sentinels (`0x7000_Cxxx`): `0x7000_CB80..=0x7000_CB9F` (slots 0-7)
//! and `0x7000_CBB0..=0x7000_CBB3` (slot 12; slots 8-11 would overlap the
//! vendor supplies test's `0x7000_CBA0..CBA1`), slots (account, player; the account id doubles as the target
//! entity id so outbox rows are deleted by exact entity). Clear of the
//! persistence tests (`..0x7000_CB55`) and the GM expertise grants
//! (`0x7000_CC00..`).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use cimmeria_wire::crafting::{GmCraftGrant, GmCraftGrantKind};
use sqlx::PgPool;

use super::handle_gm_craft_grant;
use crate::base::crafting::request::CraftCtx;
use crate::base::crafting::test_packets::{decode_all, feedback_text, MethodCall};
use crate::base::ConnectedClientState;
use crate::test_support::{test_default_connected_client_state, TestTransport};

mod craftkit;
mod learn_blueprint;
mod rules;

const TEST_BASE: i32 = 0x7000_CB80;

/// One test's ids: `slot` 0-7 of the block.
#[derive(Debug, Clone, Copy)]
struct Ids {
    account: i32,
    player: i32,
    target: u32,
    gm: u32,
    target_addr: SocketAddr,
    gm_addr: SocketAddr,
}

fn ids(slot: i32) -> Ids {
    let account = TEST_BASE + 4 * slot;
    let port = 55900 + 2 * slot as u16;
    Ids {
        account,
        player: account + 1,
        target: account as u32,
        gm: (account + 2) as u32,
        target_addr: format!("127.0.0.1:{port}").parse().unwrap(),
        gm_addr: format!("127.0.0.1:{}", port + 1).parse().unwrap(),
    }
}

async fn cleanup(pool: &PgPool, ids: Ids) {
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
        .bind(ids.account)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
        .bind(ids.player)
        .execute(pool)
        .await;
    crate::base::crafting::test_players::cleanup(pool, ids.account, ids.player).await;
}

async fn insert_player(pool: &PgPool, ids: Ids) {
    crate::base::crafting::test_players::insert_player(pool, ids.account, ids.player).await;
}

type Maps = (
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
);

/// The target's session and the GM's, at `gm_access_level`.
fn sessions(ids: Ids, gm_access_level: u32) -> Maps {
    let mut target = test_default_connected_client_state();
    target.account_id = ids.account as u32;
    target.active_player_id = Some(ids.player);
    let mut gm = test_default_connected_client_state();
    gm.access_level = gm_access_level;
    (
        Arc::new(Mutex::new(HashMap::from([
            (ids.target_addr, target),
            (ids.gm_addr, gm),
        ]))),
        Arc::new(Mutex::new(HashMap::from([
            (ids.target, ids.target_addr),
            (ids.gm, ids.gm_addr),
        ]))),
    )
}

/// Run one grant from the GM for the target; returns what was sent.
async fn run(
    pool: Option<&PgPool>,
    ids: Ids,
    maps: &Maps,
    grant: GmCraftGrantKind,
) -> Arc<TestTransport> {
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let db_pool = pool.map(|p| Arc::new(p.clone()));
    let ctx = CraftCtx {
        db_pool: &db_pool,
        cell_tx: &None,
        transport: &transport,
        connected: &maps.0,
        entity_to_addr: &maps.1,
    };
    handle_gm_craft_grant(
        GmCraftGrant {
            entity_id: ids.target,
            player_id: ids.player,
            gm_entity_id: ids.gm,
            grant,
        },
        &ctx,
    )
    .await;
    typed
}

/// The lines the GM read.
fn gm_lines(typed: &TestTransport, ids: Ids) -> Vec<String> {
    decode_all(&typed.filter_to(ids.gm_addr))
        .iter()
        .map(feedback_text)
        .collect()
}

/// What the target's client was sent.
fn target_calls(typed: &TestTransport, ids: Ids) -> Vec<MethodCall> {
    decode_all(&typed.filter_to(ids.target_addr))
}

/// `(type_id, container_id, stack_size)` of every item the player holds.
async fn inventory(pool: &PgPool, ids: Ids) -> Vec<(i32, i32, i32)> {
    sqlx::query_as(
        "SELECT type_id, container_id, stack_size FROM sgw_inventory \
         WHERE character_id = $1 ORDER BY container_id, slot_id",
    )
    .bind(ids.player)
    .fetch_all(pool)
    .await
    .expect("inventory")
}

/// The identity every GM grant event carries.
fn assert_identity(e: &crate::test_support::Captured, ids: Ids) {
    for (field, value) in [
        ("account_id", ids.account.to_string()),
        ("player_id", ids.player.to_string()),
        ("entity_id", ids.target.to_string()),
        ("gm_entity_id", ids.gm.to_string()),
    ] {
        assert!(e.has_field(field, &value), "{field}: {e:#?}");
    }
}
