//! Tests for `.allcraft`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::persistence::load_crafting_state;
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

/// The pushed bundle is the login sync's state bundle: one 136 per
/// discipline, one 138 per paradigm, one 139, then the unchanged ASP total.
#[test]
fn state_bundle_carries_every_update() {
    use super::super::sync::crafting_state_messages;
    let mut state = CraftingState::new();
    state.applied_science_points = 3;
    apply_all_craft(&mut state, &[21, 22, 23], &[5, 7], &[1, 2]);
    let methods: Vec<u16> = crafting_state_messages(&state)
        .into_iter()
        .map(|(method, _)| method)
        .collect();
    assert_eq!(methods, [136, 136, 136, 138, 138, 139, 7]);
    assert_eq!(
        build_crafting_state_bundle(4290, &state).num_messages(),
        methods.len()
    );
}

/// The GM's line is one sentence with single spaces.
#[test]
fn granted_text_reads_as_one_sentence() {
    assert_eq!(
        granted_text(4291, 78, 498, 5),
        "allcraft [4291]: 78 disciplines at 100, 498 blueprints, 5 paradigms at 7; \
         craft anywhere is on until logout."
    );
}

/// Crafting sentinels (`0x7000_Cxxx`): `0x7000_CD20..0x7000_CD21`. The sync
/// tests hold `0x7000_CD00..0x7000_CD1F`, the tool tests `0x7000_CD30..`.
const ACCOUNT: i32 = 0x7000_CD20;
const PLAYER: i32 = 0x7000_CD21;
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

async fn run(pool: &PgPool, sessions: &Sessions) -> Arc<TestTransport> {
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
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
    typed
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

    let typed = run(&pool, &sessions).await;
    let reloaded = load_crafting_state(&pool, PLAYER).await;
    cleanup(&pool).await;
    let reloaded = reloaded.expect("reload");

    // The target's client gets the saved state as the login sync's bundle,
    // then the craft-anywhere options.
    let target: SocketAddr = "127.0.0.1:55741".parse().unwrap();
    let (bundle, _) = build_crafting_state_bundle(TARGET_ENTITY, &reloaded).finalize(
        cimmeria_mercury::packet::FLAG_RELIABLE | cimmeria_mercury::packet::FLAG_ON_CHANNEL,
        0,
        |p| {
            crate::mercury::encrypt_packet(
                p,
                &[0u8; 32],
                cimmeria_mercury::encryption::EncryptionVersion::V1,
            )
        },
    );
    let sent = typed.filter_to(target);
    assert_eq!(sent.len(), bundle.len() + 1, "state bundle, then 140");
    assert_eq!(&sent[..bundle.len()], &bundle[..]);

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

/// Live DB, the lost-update guard: `.allcraft` reads and writes under the
/// player-row lock. A writer holding the row with an uncommitted ASP change
/// makes the grant wait, and the committed change survives it. A read
/// outside the lock sees the old ASP, and the save (which waits on the row
/// anyway) writes that stale value back over the committed change.
#[tokio::test]
async fn allcraft_waits_for_a_concurrent_writer_and_keeps_its_change() {
    let pool = require_db_or_skip!();
    cleanup(&pool).await;
    insert_player(&pool).await;
    let sessions = sessions(2);

    let mut holder = pool.begin().await.expect("begin");
    sqlx::query("UPDATE sgw_player SET applied_science_points = 9 WHERE player_id = $1")
        .bind(PLAYER)
        .execute(&mut *holder)
        .await
        .expect("hold the row with an uncommitted change");
    let (grant_pool, grant_sessions) = (pool.clone(), sessions.clone());
    let grant = tokio::spawn(async move {
        run(&grant_pool, &grant_sessions).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let finished_while_held = grant.is_finished();
    holder.commit().await.expect("commit");
    tokio::time::timeout(std::time::Duration::from_secs(10), grant)
        .await
        .expect("the grant finishes once the row is free")
        .expect("grant task");
    let reloaded = load_crafting_state(&pool, PLAYER).await;
    cleanup(&pool).await;
    let reloaded = reloaded.expect("reload");

    assert!(!finished_while_held, "the grant waits for the row lock");
    assert_eq!(
        reloaded.applied_science_points, 9,
        "the concurrent writer's committed ASP survives the grant"
    );
    assert_eq!(reloaded.discipline_ids.len(), 78, "and the grant landed");
}

/// A player row that is not there is a WARN `persist_failed` naming the
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
        ("phase", "load_crafting_state_locked"),
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
