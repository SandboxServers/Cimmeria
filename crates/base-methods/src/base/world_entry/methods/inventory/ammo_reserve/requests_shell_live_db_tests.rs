//! Live-DB tests for `handle_ammo_reserve_request`, the shell around the
//! committed reserve writes: the answer the cell gets, and the catalog
//! events on target `ammo` (TESTING.md type 12). Fixtures are shared with
//! `requests_live_db_tests` (sentinel slots `k = 8..`).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::ammo_telemetry::{events, reasons};
use cimmeria_entity::ammo_type::{BULLET_ARMOR_PIERCING, BULLET_HOLLOW_POINT};
use cimmeria_entity::inventory::INV_MAIN;
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing::Level;

use super::requests_live_db_tests::{
    cleanup, fixture, hp_total, pistol, put_stack, put_weapon, HOLLOW_POINT_ITEM, SLOT,
};
use super::*;
use crate::base::ConnectedClientState;
use crate::cell::messages::{AmmoReserveAnswer, AmmoReserveRequest, BaseToCellMsg, ReserveRefusal};
use crate::test_support::{require_db_or_skip, LogCapture, TestTransport};

const ENTITY: u32 = 0x7000_A53E;

/// Run one request through the shell; return the cell's answers.
async fn run(pool: &PgPool, req: AmmoReserveRequest) -> Vec<AmmoReserveAnswer> {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let addr: SocketAddr = "127.0.0.1:65533".parse().unwrap();
    let e2a = Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)])));
    let conn: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let db = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(8);
    let cell_tx = Some(tx);
    handle_ammo_reserve_request(
        req,
        ReserveIo {
            db_pool: &db,
            cell_tx: &cell_tx,
            transport: &transport,
            connected: &conn,
            entity_to_addr: &e2a,
        },
    )
    .await;
    let mut out = Vec::new();
    while let Ok(BaseToCellMsg::AmmoReserve(a)) = rx.try_recv() {
        out.push(a);
    }
    out
}

/// A draw answers `ReloadDrawn { drawn: 12 }` and logs `reload_draw` with
/// the before/after values.
#[tokio::test]
async fn live_db_reload_draw_answers_and_logs_reload_draw() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 8).await;
    let (_, clip) = pistol(&pool).await;
    let w = put_weapon(&pool, player_id, clip - 12, BULLET_HOLLOW_POINT).await;
    put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_MAIN, 3, 100).await;
    let capture = LogCapture::install();

    let answers = run(
        &pool,
        AmmoReserveRequest::ReloadDraw {
            entity_id: ENTITY,
            player_id,
            slot_id: SLOT,
            instance_id: w,
            ammo_type: BULLET_HOLLOW_POINT,
            clip_before: clip - 12,
        },
    )
    .await;
    let row = capture
        .all()
        .into_iter()
        .find(|c| c.target == "ammo" && c.has_field("event", events::RELOAD_DRAW));
    cleanup(&pool, account_id, player_id).await;

    assert_eq!(
        answers,
        vec![AmmoReserveAnswer::ReloadDrawn {
            entity_id: ENTITY,
            player_id,
            slot_id: SLOT,
            instance_id: w,
            ammo_type: BULLET_HOLLOW_POINT,
            drawn: 12,
            stack_after: 88,
            result: Ok(()),
        }]
    );
    let row = row.expect("reload_draw event");
    assert_eq!(row.level, Level::DEBUG);
    for (k, v) in [
        ("drawn", "12".to_string()),
        ("clip_before", (clip - 12).to_string()),
        ("clip_after", clip.to_string()),
        ("stack_before", "100".to_string()),
        ("stack_after", "88".to_string()),
        ("player_id", player_id.to_string()),
    ] {
        assert!(row.has_field(k, &v), "{k}={v} in {:?}", row.fields);
    }
}

/// An empty reserve answers `StackEmpty` and logs `reload_refused` at WARN
/// with `reason=stack_empty`.
#[tokio::test]
async fn live_db_empty_reserve_answers_stack_empty_and_logs_reload_refused() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 9).await;
    let (_, clip) = pistol(&pool).await;
    let w = put_weapon(&pool, player_id, clip - 12, BULLET_HOLLOW_POINT).await;
    let capture = LogCapture::install();

    let answers = run(
        &pool,
        AmmoReserveRequest::ReloadDraw {
            entity_id: ENTITY,
            player_id,
            slot_id: SLOT,
            instance_id: w,
            ammo_type: BULLET_HOLLOW_POINT,
            clip_before: clip - 12,
        },
    )
    .await;
    let row = capture.find_event(Level::WARN, "reload refused", reasons::STACK_EMPTY);
    cleanup(&pool, account_id, player_id).await;

    assert!(matches!(
        answers.as_slice(),
        [AmmoReserveAnswer::ReloadDrawn {
            drawn: 0,
            result: Err(ReserveRefusal::StackEmpty),
            ..
        }]
    ));
    let row = row.expect("reload_refused reason=stack_empty");
    assert!(row.has_field("event", events::RELOAD_REFUSED));
}

/// A switch return answers with the split and logs `ammo_switch_return`.
#[tokio::test]
async fn live_db_switch_return_answers_and_logs_ammo_switch_return() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 10).await;
    let w = put_weapon(&pool, player_id, 20, BULLET_HOLLOW_POINT).await;
    let capture = LogCapture::install();

    let answers = run(
        &pool,
        AmmoReserveRequest::SwitchReturn {
            entity_id: ENTITY,
            player_id,
            slot_id: SLOT,
            instance_id: w,
            from_ammo_type: BULLET_HOLLOW_POINT,
            to_ammo_type: BULLET_ARMOR_PIERCING,
            rounds: 20,
        },
    )
    .await;
    let total = hp_total(&pool, player_id).await;
    let row = capture
        .all()
        .into_iter()
        .find(|c| c.target == "ammo" && c.has_field("event", events::AMMO_SWITCH_RETURN));
    cleanup(&pool, account_id, player_id).await;

    assert_eq!(total, 20, "a new stack opened for the returned rounds");
    assert!(matches!(
        answers.as_slice(),
        [AmmoReserveAnswer::SwitchReturned {
            rounds: 20,
            returned: 20,
            remainder: 0,
            result: Ok(()),
            ..
        }]
    ));
    let row = row.expect("ammo_switch_return event");
    for (k, v) in [
        ("returned", "20"),
        ("remainder", "0"),
        ("stack_before", "0"),
        ("stack_after", "20"),
    ] {
        assert!(row.has_field(k, v), "{k}={v} in {:?}", row.fields);
    }
}
