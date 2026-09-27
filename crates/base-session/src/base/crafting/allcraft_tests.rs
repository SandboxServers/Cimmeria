//! Tests for `.allcraft`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::*;
use crate::base::ConnectedClientState;
use crate::test_support::{
    require_db_or_skip, test_default_connected_client_state, LogCapture, TestTransport,
};

#[test]
fn apply_all_craft_sets_everything_and_keeps_asp() {
    let mut state = CraftingState::new();
    state.applied_science_points = 3;
    state.discipline_ids = vec![22];
    state.set_expertise(22, 40);
    state.blueprint_ids = vec![7];

    apply_all_craft(&mut state, &[22, 21], &[7, 5, 9], &[1, 2, 3, 4, 5]);

    assert_eq!(state.discipline_ids, vec![21, 22]);
    assert_eq!(state.get_expertise(21), Some(100));
    assert_eq!(state.get_expertise(22), Some(100));
    assert_eq!(state.blueprint_ids, vec![5, 7, 9]);
    assert_eq!(
        state.racial_paradigm_levels,
        (1..=5).map(|id| (id, 7)).collect()
    );
    assert_eq!(state.applied_science_points, 3, "ASP untouched");
}

/// One 136 per discipline, one 138 per paradigm, then one 139.
#[test]
fn state_bundle_carries_every_update() {
    let mut state = CraftingState::new();
    apply_all_craft(&mut state, &[21, 22, 23], &[5, 7], &[1, 2]);
    assert_eq!(
        crafting_state_bundle(4290, &state).num_messages(),
        3 + 2 + 1
    );
}

/// Crafting sentinels (`0x7000_Cxxx`): `tools_tests.rs` holds `0x7000_CE0x`.
const ACCOUNT: i32 = 0x7000_CD00;
const PLAYER: i32 = 0x7000_CD01;
const TARGET_ENTITY: u32 = 4291;
const GM_ENTITY: u32 = 4292;

async fn cleanup(pool: &PgPool) {
    let _ = sqlx::query("DELETE FROM sgw_player_discipline_expertise WHERE player_id = $1")
        .bind(PLAYER)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(PLAYER)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACCOUNT)
        .execute(pool)
        .await;
}

async fn insert_player(pool: &PgPool) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(ACCOUNT)
        .bind(format!("craft-allcraft-{ACCOUNT}"))
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, applied_science_points\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, 4)",
    )
    .bind(ACCOUNT)
    .bind(PLAYER)
    .bind(format!("craft-allcraft-{PLAYER}"))
    .execute(pool)
    .await
    .expect("insert player");
}

type Sessions = (
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
);

/// A target session and a GM session at `gm_access_level`.
fn sessions(gm_access_level: u32) -> Sessions {
    let target: SocketAddr = "127.0.0.1:55741".parse().unwrap();
    let gm: SocketAddr = "127.0.0.1:55742".parse().unwrap();
    let mut gm_state = test_default_connected_client_state();
    gm_state.access_level = gm_access_level;
    let mut target_state = test_default_connected_client_state();
    target_state.account_id = ACCOUNT as u32;
    target_state.active_player_id = Some(PLAYER);
    (
        Arc::new(Mutex::new(HashMap::from([
            (target, target_state),
            (gm, gm_state),
        ]))),
        Arc::new(Mutex::new(HashMap::from([
            (TARGET_ENTITY, target),
            (GM_ENTITY, gm),
        ]))),
    )
}

async fn run(pool: &PgPool, sessions: &Sessions) {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let db_pool = Some(Arc::new(pool.clone()));
    let ctx = CraftCtx {
        db_pool: &db_pool,
        cell_tx: &None,
        transport: &transport,
        connected: &sessions.0,
        entity_to_addr: &sessions.1,
    };
    handle_gm_all_craft(
        GmAllCraft {
            entity_id: TARGET_ENTITY,
            player_id: PLAYER,
            gm_entity_id: GM_ENTITY,
        },
        &ctx,
    )
    .await;
}

fn craft_anywhere_on(sessions: &Sessions) -> bool {
    super::super::options::craft_anywhere(TARGET_ENTITY, &sessions.0, &sessions.1)
}

/// `.allcraft` persists every discipline at 100, every blueprint and every
/// paradigm at 7, leaves ASP alone, and turns on "craft anywhere"; a relog
/// (a fresh load) sees it all.
#[tokio::test]
async fn allcraft_persists_the_full_crafting_state() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    cleanup(&pool).await;
    insert_player(&pool).await;
    let sessions = sessions(2);

    run(&pool, &sessions).await;
    let reloaded = load_crafting_state(&pool, PLAYER).await;
    cleanup(&pool).await;
    let reloaded = reloaded.expect("reload");

    let catalog = shared_crafting_catalog(&pool).await.expect("catalog");
    assert_eq!(reloaded.discipline_ids.len(), catalog.disciplines.len());
    assert_eq!(reloaded.discipline_ids.len(), 78, "seeded disciplines");
    assert!(reloaded
        .discipline_ids
        .iter()
        .all(|&d| reloaded.get_expertise(d) == Some(ALL_CRAFT_EXPERTISE)));
    assert_eq!(reloaded.blueprint_ids.len(), 498, "seeded blueprints");
    assert_eq!(
        reloaded.racial_paradigm_levels,
        (1..=5).map(|id| (id, ALL_CRAFT_PARADIGM_LEVEL)).collect()
    );
    assert_eq!(reloaded.applied_science_points, 4, "ASP untouched");
    assert!(craft_anywhere_on(&sessions));

    let event = capture
        .find_message(tracing::Level::INFO, "allcraft granted")
        .expect("gm_allcraft is logged");
    assert_eq!(event.target, "crafting");
    for (field, value) in [
        ("event", "gm_allcraft".to_string()),
        ("outcome", "granted".to_string()),
        ("account_id", ACCOUNT.to_string()),
        ("player_id", PLAYER.to_string()),
        ("entity_id", TARGET_ENTITY.to_string()),
        ("disciplines_before", "0".to_string()),
        ("disciplines_after", "78".to_string()),
        ("blueprints_before", "0".to_string()),
        ("blueprints_after", "498".to_string()),
        (
            "paradigm_levels_after",
            "[(1, 7), (2, 7), (3, 7), (4, 7), (5, 7)]".to_string(),
        ),
    ] {
        assert!(event.has_field(field, &value), "{field}: {event:#?}");
    }
    assert!(capture
        .all()
        .iter()
        .any(|e| e.has_field("event", "options_changed") && e.has_field("cause", "gm_anywhere")));
}

/// A caller below GameMaster changes nothing in the database and turns
/// nothing on.
#[tokio::test]
async fn allcraft_from_a_non_gm_writes_nothing() {
    let pool = require_db_or_skip!();
    cleanup(&pool).await;
    insert_player(&pool).await;
    let sessions = sessions(0);

    run(&pool, &sessions).await;
    let reloaded = load_crafting_state(&pool, PLAYER).await;
    cleanup(&pool).await;
    let reloaded = reloaded.expect("reload");

    assert!(reloaded.discipline_ids.is_empty());
    assert!(reloaded.blueprint_ids.is_empty());
    assert!(!craft_anywhere_on(&sessions));
}

/// A save that matches no player row is a WARN `persist_failed` naming the
/// phase, with the paired `rows_affected` / `expected`, and turns nothing on.
#[tokio::test]
async fn allcraft_for_a_missing_player_logs_the_shortfall() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    cleanup(&pool).await;
    let sessions = sessions(2);

    run(&pool, &sessions).await;

    let event = capture
        .find_message(tracing::Level::WARN, "allcraft save failed")
        .expect("persist_failed is logged");
    for (field, value) in [
        ("event", "persist_failed"),
        ("phase", "save_crafting_state"),
        ("rows_affected", "0"),
        ("expected", "1"),
    ] {
        assert!(event.has_field(field, value), "{field}: {event:#?}");
    }
    assert!(
        event.has_field("player_id", &PLAYER.to_string()),
        "{event:#?}"
    );
    assert!(!craft_anywhere_on(&sessions));
}
