//! Live-DB guard for the shutdown flush (TG-DB-02, D-TG15): a player still in
//! the world when the server stops keeps the position they stood at.
//!
//! Runs the real services: `Orchestrator::start_all`, a player entity created
//! and connected on the cell through the orchestrator's base-to-cell sender,
//! then `stop_all`. The cell's shutdown arm sends `ShutdownFlush`, the base's
//! cell-message task writes `sgw_player`, and `stop_all` returns only after
//! the answer. Before the flush, a stop wrote nothing: the row kept the
//! position of the last logout, gate arrival or creation.
//!
//! Sentinels `0x7000_1F10`/`0x7000_1F11`, past `character_create`'s
//! `0x7000_1E07`.

use std::time::Duration;

use cimmeria_common::ServerConfig;
use sqlx::PgPool;

use crate::cell::messages::BaseToCellMsg;
use crate::orchestrator::Orchestrator;
use crate::test_support::require_db_or_skip;

const ACCOUNT_ID: i32 = 0x7000_1F10;
const PLAYER_ID: i32 = 0x7000_1F11;
/// Cell entity ids for the sentinel player and the barrier entity.
const PLAYER_ENTITY: u32 = 0x7000_1F11;
const BARRIER_ENTITY: u32 = 0x7000_1F12;

/// Where the row says the player is before the stop.
const STORED: [f32; 3] = [-334.25, 73.5, -228.0];
/// Where the player stands in the world when the server stops.
const LIVE: [f32; 3] = [411.25, 70.5, 987.75];

async fn cleanup(pool: &PgPool) {
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(PLAYER_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACCOUNT_ID)
        .execute(pool)
        .await;
}

async fn insert_player(pool: &PgPool) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(ACCOUNT_ID)
        .bind(format!("shutdown-flush-{ACCOUNT_ID}"))
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
         ) VALUES ($1, $2, 1, 1, 1, 1, $3, '', 'Castle', \
                   'BS_HumanMale.BS_HumanMale', $4, $5, $6, 0, 0, 0)",
    )
    .bind(ACCOUNT_ID)
    .bind(PLAYER_ID)
    .bind(format!("sflush-{PLAYER_ID}"))
    .bind(STORED[0])
    .bind(STORED[1])
    .bind(STORED[2])
    .execute(pool)
    .await
    .expect("insert player");
}

async fn read_position(pool: &PgPool) -> (String, f32, f32, f32) {
    sqlx::query_as(
        "SELECT world_location, pos_x, pos_y, pos_z FROM sgw_player WHERE player_id = $1",
    )
    .bind(PLAYER_ID)
    .fetch_one(pool)
    .await
    .expect("read position")
}

/// `CellService::start` reads `entities/spaces.xml` relative to the working
/// directory, which under the test runner is `crates/services/`. Without the
/// space definitions "Castle" is unknown and the player lands in no space.
fn chdir_to_repo_root() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/services/../.. is the repo root");
    std::env::set_current_dir(root).expect("chdir to the repo root");
}

fn config(db_url: String) -> ServerConfig {
    let port = || {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    };
    let udp_port = || {
        std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    };
    ServerConfig {
        logon_port: port(),
        auth_tls_port: port(),
        base_port: udp_port(),
        cell_port: port(),
        admin_port: port(),
        minigame_port: port(),
        db_connection_string: db_url,
        developer_mode: false,
        ..ServerConfig::loopback()
    }
}

/// Create an entity on the cell and wait for the cell's answer. Every
/// message sent before it has been handled once the answer arrives.
async fn create_entity(
    cell_tx: &tokio::sync::mpsc::Sender<BaseToCellMsg>,
    entity_id: u32,
    position: [f32; 3],
    player_id: Option<i32>,
) {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    cell_tx
        .send(BaseToCellMsg::CreateEntity {
            entity_id,
            world_name: "Castle".to_string(),
            position,
            rotation: [0.0; 3],
            destination_space_id: None,
            account_id: player_id.map(|_| ACCOUNT_ID as u32),
            player_id,
            account_name: player_id.map(|_| format!("shutdown-flush-{ACCOUNT_ID}")),
            player_name: player_id.map(|p| format!("sflush-{p}")),
            reply_tx,
        })
        .await
        .expect("send CreateEntity");
    tokio::time::timeout(Duration::from_secs(10), reply_rx)
        .await
        .expect("CreateEntity answered in time")
        .expect("CreateEntity created the entity");
}

/// **Regression guard.** A player in the world at a stop keeps the live
/// position: `sgw_player.pos_*` moves from the stored point to where the
/// entity stood. With the flush reverted (the shutdown arm only reports),
/// nothing writes the row and it keeps the stored point.
#[tokio::test]
async fn live_db_shutdown_flushes_player_position() {
    let pool = require_db_or_skip!();
    let db_url = crate::test_support::database_url().expect("the live-DB gate passed");
    cleanup(&pool).await;
    insert_player(&pool).await;

    chdir_to_repo_root();
    let orch = Orchestrator::new(config(db_url));
    orch.start_all().await.expect("start_all");

    let cell_tx = orch
        .state()
        .read()
        .await
        .cell_tx
        .clone()
        .expect("the orchestrator holds a base-to-cell sender");
    create_entity(&cell_tx, PLAYER_ENTITY, LIVE, Some(PLAYER_ID)).await;
    cell_tx
        .send(BaseToCellMsg::ConnectEntity {
            entity_id: PLAYER_ENTITY,
        })
        .await
        .expect("send ConnectEntity");
    // A barrier: once this entity exists, ConnectEntity has been handled and
    // the player is in the world before the stop is signalled.
    create_entity(&cell_tx, BARRIER_ENTITY, [0.0; 3], None).await;

    assert_eq!(
        read_position(&pool).await,
        ("Castle".to_string(), STORED[0], STORED[1], STORED[2]),
        "fixture: nothing has written the row before the stop"
    );

    tokio::time::timeout(Duration::from_secs(30), orch.stop_all())
        .await
        .expect("stop_all returns within the flush deadline");

    let after = read_position(&pool).await;
    cleanup(&pool).await;
    assert_eq!(
        after,
        ("Castle".to_string(), LIVE[0], LIVE[1], LIVE[2]),
        "a stop must save the in-world player's live position"
    );
}
