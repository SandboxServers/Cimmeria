//! Chain 1009 (`castle_cellblock_chains.sql`) — re-arm the Region8 guard on
//! a relog past the trap.
//!
//! Chain 1008 arms `ArmYourself_NIDGuard` (spawnlist row 20, seeded NEUTRAL)
//! on a once-per-player Region8 ENTRY. A relog builds a fresh instance with
//! the guard back at NEUTRAL and returns the player at the persisted logout
//! position, already past Region8, so the edge never fires again and the
//! guard stays yellow and passive (colo 2026-09-26, tester note 02:53:23: "cellblock
//! guard is indicated as yellow after relog? guards should always be
//! hostile"). Chain 1009 re-arms it on `player_loaded` once mission 622 is
//! completed.
//!
//! Driven through the real `fire_player_loaded` with the real seeded spawn
//! and chain, so the trigger key, the mission gate, the tag and the level
//! are all on the path. Revert proof: without chain 1009 in the seed the
//! loader returns `None` and the first `expect` fails; with the chain but a
//! wrong gate the guard stays NEUTRAL and never engages.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::{AiState, MobAggression};
use cimmeria_entity::missions::MissionInstance;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::fire_player_loaded;
use crate::cell::space_manager::{spawn_npcs_from_records, SpaceManager};
use crate::cell::spawner::load_spawns_from_db;
use crate::test_support::require_db_or_skip;

const PLAYER: u32 = 7302;
const PLAYER_ID: i32 = 43;
const GUARD_TAG: &str = "ArmYourself_NIDGuard";

/// A fresh Castle_CellBlock instance with the seeded guard (spawn 20) and a
/// reconnected player 5 u from it whose mission 622 is `complete_622`.
async fn relogged_past_region8(pool: &sqlx::PgPool, complete_622: bool) -> (SpaceManager, u32) {
    let records: Vec<_> = load_spawns_from_db(pool)
        .await
        .expect("load spawnlist")
        .into_iter()
        .filter(|r| r.spawn_id == 20)
        .collect();
    assert_eq!(records.len(), 1, "spawn 20 is seeded");
    assert_eq!(records[0].aggression_override, Some(MobAggression::Neutral));

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-450" MaxX="450" MinY="-450" MaxY="450" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    assert_eq!(spawn_npcs_from_records(&records, &mut mgr), 1);
    let guard = mgr
        .all_npc_entity_ids()
        .into_iter()
        .find(|&id| mgr.get_entity(id).and_then(|e| e.tag.as_deref()) == Some(GUARD_TAG))
        .expect("the Region8 guard spawned");

    let p = mgr.get_entity(guard).unwrap().position;
    mgr.create_entity(PLAYER, "Castle_CellBlock", [p.x + 5.0, p.y, p.z], [0.0; 3])
        .unwrap();
    if let Some(pl) = mgr.get_entity_mut(PLAYER) {
        pl.is_player = true;
        pl.player_id = Some(PLAYER_ID);
        pl.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
        let mut m = MissionInstance::new(622, 700, vec![]);
        if complete_622 {
            m.complete();
        }
        pl.missions.add_mission(m);
    }
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
    (mgr, guard)
}

async fn chain_1009(pool: &sqlx::PgPool) -> ChainEngine {
    let chain = load_single_chain_for_test(pool, 1009)
        .await
        .expect("DB query for chain 1009 must succeed")
        .expect("chain 1009 must exist in seeded content_chains");
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

async fn ai_ticks(mgr: &mut SpaceManager, n: usize) {
    let (tx, _rx) = mpsc::channel(256);
    for _ in 0..n {
        crate::cell::service::npc_ai::npc_ai_tick_for_test(
            &tx,
            mgr,
            &crate::cell::content::EngineEvents(&ChainEngine::new()),
        )
        .await;
    }
}

/// Relog with 622 completed: the zone load re-arms the guard HOSTILE and
/// it engages the player standing next to it.
#[tokio::test]
async fn live_db_chain_1009_rearms_the_guard_on_relog_past_region8() {
    let pool = require_db_or_skip!();
    let (mut mgr, guard) = relogged_past_region8(&pool, true).await;
    let engine = chain_1009(&pool).await;

    let (tx, _rx) = mpsc::channel(1024);
    fire_player_loaded(
        PLAYER,
        PLAYER_ID,
        "Castle_CellBlock",
        &engine,
        &tx,
        &mut mgr,
    )
    .await;

    assert_eq!(
        mgr.get_entity(guard).unwrap().aggro.override_level,
        Some(MobAggression::Hostile),
        "the relogged player's zone load must re-arm the guard -- otherwise \
         it stays at the seeded NEUTRAL (yellow) forever"
    );
    ai_ticks(&mut mgr, 3).await;
    assert_eq!(
        mgr.get_entity(guard).unwrap().ai_state(),
        AiState::Fighting,
        "an armed guard 5 u away engages on its hostile Idle scan"
    );
}

/// Before 622 completes the player has not been through the trap yet: the
/// zone load must leave the guard NEUTRAL so Region8 (chain 1008) still owns
/// the ambush.
#[tokio::test]
async fn live_db_chain_1009_leaves_the_trap_alone_before_622_completes() {
    let pool = require_db_or_skip!();
    let (mut mgr, guard) = relogged_past_region8(&pool, false).await;
    let engine = chain_1009(&pool).await;

    let (tx, _rx) = mpsc::channel(1024);
    fire_player_loaded(
        PLAYER,
        PLAYER_ID,
        "Castle_CellBlock",
        &engine,
        &tx,
        &mut mgr,
    )
    .await;

    assert_eq!(
        mgr.get_entity(guard).unwrap().aggro.override_level,
        Some(MobAggression::Neutral),
    );
    ai_ticks(&mut mgr, 3).await;
    assert_eq!(mgr.get_entity(guard).unwrap().ai_state(), AiState::Idle);
}
