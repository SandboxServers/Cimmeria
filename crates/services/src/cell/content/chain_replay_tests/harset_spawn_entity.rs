//! `spawn_entity` / `despawn_entity` chain-replay guard (Harset H03).
//!
//! Like [`super::grant_xp`], this module seeds its own chains: there are
//! **zero** `spawn_entity` rows in `db/resources/Content/Seed/` today, and
//! the verb had neither a loader arm nor an executor arm until H03. A
//! sentinel chain is the only way to guard both halves before the Harset
//! mission packets start authoring against them.
//!
//! The scenario is the one the ledger's acceptance criterion names, and the
//! one H21 (Rin'la) / H23 / H25 / H45 all reduce to:
//!
//! > a `mission_accepted` chain spawns a tagged NPC, and an
//! > `entity_dead_tag` chain on **that same tag** then completes an
//! > objective.
//!
//! The linkage is the point. The death event is built from the tag read
//! **off the spawned entity**, not from the constant — so a regression that
//! spawned the NPC without its tag, or wrote a different tag than the seed
//! row asked for, breaks the second half rather than passing quietly.
//!
//! Three separate things fail if a piece is reverted:
//!
//! - Removing the `"spawn_entity"` arm from
//!   `crates/content-engine/src/loader/action_spawn.rs` (or the
//!   fallthrough delegation in `loader/action.rs`) makes the chain load
//!   with zero actions → the resolve assertion fails.
//! - Removing the `Action::SpawnEntity` arm from
//!   `crates/services/src/cell/content/executor/mod.rs` drops execution
//!   into the `other =>` catch-all → no entity appears → the tag lookup
//!   fails. **A resolve-only test cannot tell those two apart**, which is
//!   why this pushes everything through `execute_actions`.
//! - Removing `SpaceManager::spawn_templates` / its startup load makes the
//!   template lookup miss → same.
//!
//! Sentinel id range: `0x7003_0000..0x7003_ffff` (Harset campaign).
//! Neighbours in `crates/services` run `0x7000_1000..0x7000_1B00`,
//! `0x7000_2000`, `0x7000_3000`, `0x7000_4000`, `0x7000_4242` and
//! `0x7000_5000`. Cleanup deletes the exact ids inserted, never a range.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::executor::execute_actions;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

/// Chain that spawns the mission NPC on mission accept.
const SPAWN_CHAIN_ID: i32 = 0x7003_0001;
/// Chain that completes the objective when the spawned NPC dies.
const KILL_CHAIN_ID: i32 = 0x7003_0002;
/// Chain that despawns the NPC by tag.
const DESPAWN_CHAIN_ID: i32 = 0x7003_0003;
const ALL_CHAIN_IDS: [i32; 3] = [SPAWN_CHAIN_ID, KILL_CHAIN_ID, DESPAWN_CHAIN_ID];

/// Sentinel mission / objective. Both are ids the seed does not use; the
/// mission never has to exist as a `missions` row because the trigger
/// matches on the event's `mission_id` param, not on mission state.
const MISSION_ID: i32 = 0x7003_0010;
const OBJECTIVE_ID: i32 = 0x7003_0011;

/// Namespaced so it can never collide with a real `spawnlist.tag`.
const SPAWN_TAG: &str = "CIMMERIA_TEST_H03_MISSION_NPC";

const PLAYER_EID: u32 = 7301;
const PLAYER_ID: i32 = 7302;

/// `Castle_CellBlock` is instanced in the fixture XML, matching the real
/// `spaces.xml` — the spawn arm refuses non-instanced worlds, so the
/// fixture has to use one or the whole scenario is untestable.
const WORLD: &str = "Castle_CellBlock";

/// Insert the three sentinel chains. `spawn_entity` carries the full
/// descriptor a Harset mission row would.
async fn seed_sentinel_chains(pool: &PgPool, template_id: i32) {
    for (chain_id, desc) in [
        (SPAWN_CHAIN_ID, "H03 spawn_entity sentinel"),
        (KILL_CHAIN_ID, "H03 entity_dead_tag sentinel"),
        (DESPAWN_CHAIN_ID, "H03 despawn_entity sentinel"),
    ] {
        sqlx::query(
            "INSERT INTO resources.content_chains \
             (chain_id, description, scope_type, scope_id, enabled, priority) \
             VALUES ($1, $2, 'space', NULL, true, 0)",
        )
        .bind(chain_id)
        .bind(desc)
        .execute(pool)
        .await
        .expect("sentinel content_chains insert must succeed");
    }

    for (chain_id, event_type, event_key) in [
        (SPAWN_CHAIN_ID, "mission_accepted", MISSION_ID.to_string()),
        (KILL_CHAIN_ID, "entity_dead_tag", SPAWN_TAG.to_string()),
        (DESPAWN_CHAIN_ID, "interact_tag", SPAWN_TAG.to_string()),
    ] {
        sqlx::query(
            "INSERT INTO resources.content_triggers \
             (chain_id, event_type, event_key, scope, once, sort_order) \
             VALUES ($1, $2, $3, 'player', false, 0)",
        )
        .bind(chain_id)
        .bind(event_type)
        .bind(event_key)
        .execute(pool)
        .await
        .expect("sentinel content_triggers insert must succeed");
    }

    // spawn_entity: target_id = template, target_key = tag, params = the
    // rest of the descriptor. Deliberately non-round coordinates so a
    // hard-coded default or a dropped field cannot reproduce them.
    sqlx::query(
        "INSERT INTO resources.content_actions \
         (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) \
         VALUES ($1, 'spawn_entity', $2, $3, $4::jsonb, 0, 0)",
    )
    .bind(SPAWN_CHAIN_ID)
    .bind(template_id)
    .bind(SPAWN_TAG)
    .bind(r#"{"x": -123.625, "y": 1.311, "z": -246.858, "heading": 2.25, "aggression": 1}"#)
    .execute(pool)
    .await
    .expect("sentinel spawn_entity action insert must succeed");

    sqlx::query(
        "INSERT INTO resources.content_actions \
         (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) \
         VALUES ($1, 'complete_objective', $2, $3, '{}'::jsonb, 0, 0)",
    )
    .bind(KILL_CHAIN_ID)
    .bind(MISSION_ID)
    .bind(OBJECTIVE_ID.to_string())
    .execute(pool)
    .await
    .expect("sentinel complete_objective action insert must succeed");

    sqlx::query(
        "INSERT INTO resources.content_actions \
         (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) \
         VALUES ($1, 'despawn_entity', NULL, $2, '{}'::jsonb, 0, 0)",
    )
    .bind(DESPAWN_CHAIN_ID)
    .bind(SPAWN_TAG)
    .execute(pool)
    .await
    .expect("sentinel despawn_entity action insert must succeed");
}

/// Delete by exact chain id, children first (FK order).
async fn cleanup_sentinel_chains(pool: &PgPool) {
    for chain_id in ALL_CHAIN_IDS {
        for stmt in [
            "DELETE FROM resources.content_actions WHERE chain_id = $1",
            "DELETE FROM resources.content_triggers WHERE chain_id = $1",
            "DELETE FROM resources.content_chains WHERE chain_id = $1",
        ] {
            sqlx::query(stmt)
                .bind(chain_id)
                .execute(pool)
                .await
                .expect("sentinel cleanup must succeed");
        }
    }
}

/// Pick a real, fully-populated `entity_templates` row rather than a
/// hard-coded id — per TESTING.md "don't trust seed data", template ids
/// churn. Filtered to the columns `build_prototype` reads as NOT NULL so
/// the cache actually holds it.
async fn pick_template_id(pool: &PgPool) -> i32 {
    use sqlx::Row;
    sqlx::query(
        "SELECT template_id FROM resources.entity_templates \
         WHERE template_name IS NOT NULL AND class IS NOT NULL AND body_set IS NOT NULL \
         ORDER BY template_id LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("seed must contain at least one fully-populated entity_template")
    .get("template_id")
}

/// `SpaceManager` with the instanced fixture world and the **real**
/// `entity_templates` cache loaded from the DB — so this also guards
/// `spawner::load_spawn_templates`, not just the executor arm.
async fn make_space_mgr(pool: &PgPool) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    </Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.spawn_templates = crate::cell::spawner::load_spawn_templates(pool)
        .await
        .expect("load_spawn_templates must succeed against the seeded DB");
    mgr
}

/// Put the firing player into a fresh instance of the fixture world.
fn stage_player(mgr: &mut SpaceManager) {
    mgr.create_entity(PLAYER_EID, WORLD, [0.0, 0.0, 0.0], [0.0; 3])
        .expect("instanced world must accept the player entity");
    let p = mgr
        .get_entity_mut(PLAYER_EID)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    mgr.connect_entity(PLAYER_EID);
}

fn register(engine: &mut ChainEngine, chain: cimmeria_content_engine::chain::Chain) {
    engine.register_chain(chain);
}

/// End-to-end: `mission_accepted` spawns the tagged NPC into the player's
/// own instance, and `entity_dead_tag` on the tag that entity actually
/// carries resolves the objective completion.
#[tokio::test]
async fn mission_accept_spawns_a_tagged_npc_that_entity_dead_tag_can_complete_on() {
    let pool = require_db_or_skip!();
    cleanup_sentinel_chains(&pool).await;
    let template_id = pick_template_id(&pool).await;
    seed_sentinel_chains(&pool, template_id).await;

    let spawn_chain = load_single_chain_for_test(&pool, SPAWN_CHAIN_ID).await;
    let kill_chain = load_single_chain_for_test(&pool, KILL_CHAIN_ID).await;

    // Drop the sentinel rows before asserting so a failure can't leave a
    // live chain registered in the shared test database.
    cleanup_sentinel_chains(&pool).await;

    let spawn_chain = spawn_chain
        .expect("DB query for the spawn chain must succeed")
        .expect("spawn chain must exist *and* load — None means the trigger row was rejected");
    let kill_chain = kill_chain
        .expect("DB query for the kill chain must succeed")
        .expect("kill chain must exist *and* load");

    let mut engine = ChainEngine::new();
    register(&mut engine, spawn_chain);
    register(&mut engine, kill_chain);

    let mut mgr = make_space_mgr(&pool).await;
    stage_player(&mut mgr);
    let player_space = mgr
        .get_entity_space_id(PLAYER_EID)
        .expect("player must be in a space");

    // ── Phase 1: mission accepted → spawn ──
    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(MISSION_ID));
    let accept_event = TriggerEvent {
        trigger_type: TriggerType::MissionAccepted,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&accept_event, &ctx);
    assert_eq!(
        resolved.actions.len(),
        1,
        "the spawn chain's single spawn_entity row must survive \
         convert_action — zero actions means the loader has no \
         \"spawn_entity\" arm and the row was dropped with an \
         \"Unknown action_type\" warn"
    );
    assert!(
        matches!(resolved.actions[0].1, Action::SpawnEntity { .. }),
        "the resolved action must be SpawnEntity, got {:?}",
        resolved.actions[0].1
    );

    let (tx, mut rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;

    let npc_id = mgr.find_entity_by_tag(PLAYER_EID, SPAWN_TAG).expect(
        "spawn_entity must place the tagged NPC in the player's space; None \
         here means the executor has no Action::SpawnEntity arm (the action \
         fell through the `other =>` catch-all), the entity_templates cache \
         is empty, or the tag was dropped on the way in",
    );
    assert_eq!(
        mgr.get_entity_space_id(npc_id),
        Some(player_space),
        "the NPC must land in the acting player's own instance"
    );
    let npc = mgr
        .get_entity(npc_id)
        .expect("spawned NPC must be readable");
    assert_eq!(npc.template_id, Some(template_id));
    assert_eq!(
        [npc.position.x, npc.position.y, npc.position.z],
        [-123.625, 1.311, -246.858],
        "position must come from the seeded action params"
    );
    assert_eq!(npc.aggression, 1, "the seeded aggression param must apply");
    assert_eq!(
        npc.respawn_secs, None,
        "a content-scoped spawn is one-shot regardless of the template row"
    );
    // The spawn itself emits no cell→base traffic; AoI fan-out introduces
    // the NPC on the next 100ms tick. Pin that so a future "helpful" extra
    // send has to be deliberate.
    assert!(
        rx.try_recv().is_err(),
        "spawn_entity must not emit cell→base traffic of its own — client \
         visibility is the AoI tick's job"
    );

    // ── Phase 2: that NPC dies → objective completes ──
    // The tag is read back off the *spawned entity*, not from the constant:
    // that is what proves the two chains are actually linked.
    let live_tag = npc
        .tag
        .clone()
        .expect("the spawned entity must carry a tag or no death chain can find it");
    let mut death_ctx = ExecutionContext::new();
    death_ctx.set_param("entity_tag".to_string(), serde_json::json!(live_tag));
    let death_event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: death_ctx.params.clone(),
    };
    let death_resolved = engine.resolve_event(&death_event, &death_ctx);
    assert_eq!(
        death_resolved.actions.len(),
        1,
        "the kill chain must resolve exactly one action for the tag the \
         spawn actually wrote"
    );
    match &death_resolved.actions[0].1 {
        Action::CompleteObjective {
            mission_id,
            objective_id,
        } => {
            assert_eq!(*mission_id, MISSION_ID);
            assert_eq!(*objective_id, OBJECTIVE_ID);
        }
        other => panic!("expected CompleteObjective, got {other:?}"),
    }
}

/// The adjacent wrong state: a death event carrying a *different* tag must
/// resolve nothing. Without this, a kill chain that matched on tag
/// presence rather than tag equality would look correct above.
#[tokio::test]
async fn entity_dead_tag_does_not_fire_for_a_different_tag() {
    let pool = require_db_or_skip!();
    cleanup_sentinel_chains(&pool).await;
    let template_id = pick_template_id(&pool).await;
    seed_sentinel_chains(&pool, template_id).await;

    let kill_chain = load_single_chain_for_test(&pool, KILL_CHAIN_ID).await;
    cleanup_sentinel_chains(&pool).await;

    let kill_chain = kill_chain
        .expect("DB query must succeed")
        .expect("kill chain must exist and load");
    let mut engine = ChainEngine::new();
    register(&mut engine, kill_chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("CIMMERIA_TEST_H03_SOME_OTHER_NPC"),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    assert!(
        engine.resolve_event(&event, &ctx).actions.is_empty(),
        "a death on a different tag must resolve nothing"
    );
}

/// The despawn half of the round trip: an `interact_tag` chain carrying a
/// `despawn_entity` row removes the spawned NPC and tells the witnessing
/// player it left.
///
/// This is the H-B6 shape at chain level. Reverting `despawn_by_tag` to the
/// old bare `SpaceManager::destroy_entity` still removes the entity — so
/// the `LeftAoI` assertion, not the `get_entity` one, is the guard.
#[tokio::test]
async fn despawn_entity_chain_removes_the_spawned_npc_and_notifies_witnesses() {
    let pool = require_db_or_skip!();
    cleanup_sentinel_chains(&pool).await;
    let template_id = pick_template_id(&pool).await;
    seed_sentinel_chains(&pool, template_id).await;

    let spawn_chain = load_single_chain_for_test(&pool, SPAWN_CHAIN_ID).await;
    let despawn_chain = load_single_chain_for_test(&pool, DESPAWN_CHAIN_ID).await;
    cleanup_sentinel_chains(&pool).await;

    let mut engine = ChainEngine::new();
    register(
        &mut engine,
        spawn_chain
            .expect("DB query must succeed")
            .expect("spawn chain must exist and load"),
    );
    register(
        &mut engine,
        despawn_chain.expect("DB query must succeed").expect(
            "despawn chain must exist and load — None means the loader \
                     has no \"despawn_entity\" arm",
        ),
    );

    let mut mgr = make_space_mgr(&pool).await;
    stage_player(&mut mgr);

    let (tx, mut rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();

    // Spawn.
    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(MISSION_ID));
    let accept = TriggerEvent {
        trigger_type: TriggerType::MissionAccepted,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&accept, &ctx);
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;
    let npc_id = mgr
        .find_entity_by_tag(PLAYER_EID, SPAWN_TAG)
        .expect("spawn must succeed before the despawn can be tested");

    // Make the player an actual witness — the AoI tick would do this, but
    // the test drives the executor directly, and `despawn_npc` only
    // notifies players whose witness set contains the target.
    mgr.get_entity_mut(PLAYER_EID)
        .unwrap()
        .witnesses
        .insert(cimmeria_common::EntityId(npc_id as i32));
    while rx.try_recv().is_ok() {}

    // Despawn.
    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(SPAWN_TAG));
    let interact = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&interact, &ctx);
    assert_eq!(
        resolved.actions.len(),
        1,
        "the despawn chain's single despawn_entity row must survive the loader"
    );
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;

    let mut left_aoi = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::LeftAoI {
            witness_id,
            entity_id,
        } = msg
        {
            left_aoi.push((witness_id, entity_id));
        }
    }
    assert_eq!(
        left_aoi,
        vec![(PLAYER_EID, npc_id)],
        "the witnessing player must get exactly one LeftAoI — zero means the \
         executor still calls bare destroy_entity and the client keeps \
         rendering a ghost (audit H-B6 / the #582 shape)"
    );
    assert!(
        mgr.find_entity_by_tag(PLAYER_EID, SPAWN_TAG).is_none(),
        "the NPC itself must be gone"
    );
}
