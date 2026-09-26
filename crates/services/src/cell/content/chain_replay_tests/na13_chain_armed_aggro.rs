//! NA13 / D-NA01a: the chain-armed Cellblock spawns stay passive until their
//! chain fires, now that faction 10 is hostile on sight.
//!
//! Spawn 20 (`ArmYourself_NIDGuard`, chain 1008 on entering Region8) and
//! spawn 10 (`ArmYourself_PrisonerRetrievalUnit`, chain 1032 on the Ambernol
//! vial interaction) are loaded from the real seed with
//! [`load_spawns_from_db`], so the `spawnlist.aggression_override` column
//! and the loader are both on the path. A player stands 5 u from each, well
//! inside the aggro radius, and the AI ticks: the mob must not engage. Then
//! the real chain is resolved and executed, and the mob must engage the
//! triggering player.
//!
//! Revert proof: without the seeded NEUTRAL override the faction reaction
//! (Praxis vs Straegis = HOSTILE) engages both mobs on the first tick, before
//! their chain, and the "stays idle" assertions fail.
//!
//! Meshless space on purpose: the claim is about the aggression source, not
//! geometry, and a space with no navmesh passes the line-of-sight gate.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use cimmeria_entity::cell_entity::{AiState, MobAggression};
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::executor::execute_actions;
use crate::cell::space_manager::{spawn_npcs_from_records, SpaceManager};
use crate::cell::spawner::load_spawns_from_db;
use crate::test_support::require_db_or_skip;

const PLAYER: u32 = 7301;
const GUARD_TAG: &str = "ArmYourself_NIDGuard";
const PRU_TAG: &str = "ArmYourself_PrisonerRetrievalUnit";

/// Castle_CellBlock (non-instanced, meshless) with the two seeded spawns and
/// one connected player standing 5 u from `near_tag`.
async fn fixture(pool: &sqlx::PgPool, near_tag: &str) -> (SpaceManager, u32) {
    let records: Vec<_> = load_spawns_from_db(pool)
        .await
        .expect("load spawnlist")
        .into_iter()
        .filter(|r| r.spawn_id == 20 || r.spawn_id == 10)
        .collect();
    assert_eq!(records.len(), 2, "spawns 10 and 20 are seeded");
    for r in &records {
        assert_eq!(
            r.aggression_override,
            Some(MobAggression::Neutral),
            "spawn {} carries the seeded NEUTRAL override",
            r.spawn_id
        );
        assert_eq!(r.faction, Some(10), "spawn {} is faction 10", r.spawn_id);
    }

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-450" MaxX="450" MinY="-450" MaxY="450" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    assert_eq!(spawn_npcs_from_records(&records, &mut mgr), 2);

    let mob = mgr
        .all_npc_entity_ids()
        .into_iter()
        .find(|&id| mgr.get_entity(id).and_then(|e| e.tag.as_deref()) == Some(near_tag))
        .expect("the tagged mob spawned");
    let p = mgr.get_entity(mob).unwrap().position;
    mgr.create_entity(PLAYER, "Castle_CellBlock", [p.x + 5.0, p.y, p.z], [0.0; 3])
        .unwrap();
    if let Some(pl) = mgr.get_entity_mut(PLAYER) {
        pl.is_player = true;
        pl.player_id = Some(42);
        pl.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
    }
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
    assert!(mgr.get_witnesses_of(mob).contains(&PLAYER));
    (mgr, mob)
}

async fn ai_ticks(mgr: &mut SpaceManager, n: usize) {
    let (tx, _rx) = mpsc::channel(256);
    for _ in 0..n {
        crate::cell::service::npc_ai::npc_ai_tick_for_test(&tx, mgr, &ChainEngine::new()).await;
    }
}

fn assert_passive(mgr: &SpaceManager, mob: u32, what: &str) {
    let npc = mgr.get_entity(mob).unwrap();
    assert_eq!(npc.ai_state(), AiState::Idle, "{what}");
    assert!(npc.threat_list.is_empty(), "{what}: no threat yet");
}

fn assert_engaged_by_chain(mgr: &SpaceManager, mob: u32) {
    let npc = mgr.get_entity(mob).unwrap();
    assert_eq!(npc.ai_state(), AiState::Fighting);
    assert!(
        npc.threat_list.get(&PLAYER).copied().unwrap_or(0.0) >= 1000.0,
        "the chain's generate_threat 1000 focuses the triggering player: {:?}",
        npc.threat_list
    );
    assert_eq!(npc.aggro.override_level, Some(MobAggression::Hostile));
}

/// Chain 1008 still owns the first guard: 5 u from a player it waits, and
/// entering Region8 is what arms and engages it.
#[tokio::test]
async fn na13_chain_1008_still_owns_the_first_guard() {
    let pool = require_db_or_skip!();
    let (mut mgr, guard) = fixture(&pool, GUARD_TAG).await;

    ai_ticks(&mut mgr, 3).await;
    assert_passive(&mgr, guard, "the guard must wait for Region8");

    let chain = load_single_chain_for_test(&pool, 1008)
        .await
        .expect("query chain 1008")
        .expect("chain 1008 seeded");
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "region_key".to_string(),
        serde_json::json!("Castle_CellBlock.Region8"),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    let (tx, _rx) = mpsc::channel(256);
    execute_actions(resolved, PLAYER, 42, &tx, &mut mgr, &ChainEngine::new()).await;

    assert_engaged_by_chain(&mgr, guard);
}

/// The PRU does not fire before the vial: 5 u from a player it waits, and
/// the Ambernol vial interaction (chain 1032) is what arms and engages it.
#[tokio::test]
async fn na13_pru_waits_for_the_vial_interaction() {
    let pool = require_db_or_skip!();
    let (mut mgr, pru) = fixture(&pool, PRU_TAG).await;

    ai_ticks(&mut mgr, 3).await;
    assert_passive(&mgr, pru, "the PRU must not engage before the vial");

    let chain = load_single_chain_for_test(&pool, 1032)
        .await
        .expect("query chain 1032")
        .expect("chain 1032 seeded");
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("ArmYourself_AmbernolVial"),
    );
    ctx.set_param(
        "mission_639_step_2145_status".to_string(),
        serde_json::json!("active"),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    assert!(
        resolved.actions.iter().any(|(id, _)| *id == 1032),
        "chain 1032 resolves on the vial interaction"
    );
    let (tx, _rx) = mpsc::channel(1024);
    execute_actions(resolved, PLAYER, 42, &tx, &mut mgr, &ChainEngine::new()).await;

    assert_engaged_by_chain(&mgr, pru);
}
