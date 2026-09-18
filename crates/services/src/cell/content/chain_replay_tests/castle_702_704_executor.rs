//! Executor-path guards for the Castle missions 702/703/704 chains
//! (`db/resources/Content/Seed/castle_702_704_chains.sql`, packets CA06 and
//! CA07).
//!
//! Organised by risk rather than by mission, like [`super::sgc_w1_move_entity`]
//! and [`super::grant_xp`]: everything here pushes resolved actions through
//! [`execute_actions`] (or through `fire_chain_by_id`) and asserts on the
//! resulting game state and `CellToBaseMsg` traffic. A resolve-only test
//! cannot tell a wired executor arm from the `other =>` catch-all, and it
//! cannot see the two dispatch hops these missions hang on:
//!
//! * `set_follow_target`'s `use_player` resolution — the only way to point
//!   an NPC at a player, since players carry no spawnlist tag. Mission 704
//!   step 2405 is an escort, so the arm IS the feature.
//! * `start_minigame`'s `on_victory_chains` — the victory chain is invoked
//!   by id, not by a trigger, so the launcher-to-victory hop is the single
//!   point of failure for the Data Crystal grant.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use cimmeria_entity::cell_entity::AiState;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::executor::execute_actions;
use crate::cell::content::fire_chain_by_id;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

/// Entity id of the player firing each chain.
const PLAYER_EID: u32 = 7431;
/// Entity id of the `Castle_Zuritska_Cell` escort actor.
const ZURITSKA_CELL_EID: u32 = 7432;
/// Player id carried alongside `PLAYER_EID`.
const PLAYER_ID: i32 = 4242;

/// Castle space wide enough to hold the fixture positions.
fn make_castle_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-1200" MaxX="1200" MinY="-1200" MaxY="1200" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

/// Put a connected player into the Castle fixture space.
fn stage_player(mgr: &mut SpaceManager) {
    mgr.create_entity(PLAYER_EID, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
        .expect("Castle startup space must accept the player entity");
    let p = mgr
        .get_entity_mut(PLAYER_EID)
        .expect("player must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    mgr.connect_entity(PLAYER_EID);
}

/// Put the tagged `Castle_Zuritska_Cell` actor into the fixture space at
/// `position`, optionally already mid-escort.
fn stage_zuritska(mgr: &mut SpaceManager, position: [f32; 3], following: bool) {
    mgr.spawn_npc(ZURITSKA_CELL_EID, "Castle", position, [0.0; 3])
        .expect("Castle startup space must accept the Zuritska actor");
    let z = mgr
        .get_entity_mut(ZURITSKA_CELL_EID)
        .expect("Zuritska actor must exist immediately after spawn_npc");
    z.tag = Some("Castle_Zuritska_Cell".to_string());
    if following {
        z.follow_target_id = Some(PLAYER_EID);
        z.ai_state = AiState::Follow;
        z.nav_path
            .push_back(cimmeria_common::Vector3::new(9.0, 0.0, 9.0));
    }
}

/// Load a seeded chain, register it alone, and resolve one synthetic event.
async fn resolve_seeded(
    pool: &sqlx::PgPool,
    chain_id: i32,
    trigger_type: TriggerType,
    params: &[(&str, serde_json::Value)],
) -> cimmeria_content_engine::chain::ResolvedActions {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    for (k, v) in params {
        ctx.set_param((*k).to_string(), v.clone());
    }
    let event = TriggerEvent {
        trigger_type,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    assert!(
        resolved
            .actions
            .iter()
            .any(|(id, _)| *id == chain_id as i64),
        "chain {chain_id} must resolve before the executor half can mean anything",
    );
    resolved
}

/// Mission 702 chain 1263 (the rescue) pushed through `execute_actions`:
/// the `Castle_Zuritska_Cell` actor must end up following the triggering
/// PLAYER's entity id.
///
/// `use_player` resolution lives in the executor arm and is guarded — it
/// refuses when the triggering entity is not a player. A regression that
/// dropped the `is_player` check on the dialog path, or that swapped
/// `use_player` for a `target_tag` lookup, would leave `follow_target_id`
/// at `None` while every resolve assertion still passed.
#[tokio::test]
async fn chain_1263_makes_zuritska_follow_the_rescuing_player() {
    let pool = require_db_or_skip!();
    let resolved = resolve_seeded(
        &pool,
        1263,
        TriggerType::DialogChoice,
        &[
            ("dialog_id", serde_json::json!(2577)),
            ("mission_702_step_2419_status", serde_json::json!("active")),
            ("mission_704_status", serde_json::json!("not_active")),
        ],
    )
    .await;

    let mut mgr = make_castle_space_mgr();
    stage_player(&mut mgr);
    stage_zuritska(&mut mgr, [20.0, 0.0, 20.0], false);

    let (tx, _rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;

    let zuritska = mgr
        .get_entity(ZURITSKA_CELL_EID)
        .expect("Zuritska must survive the chain");
    assert_eq!(
        zuritska.follow_target_id,
        Some(PLAYER_EID),
        "chain 1263 must point Castle_Zuritska_Cell at the rescuing player's \
         entity id — this is 704 step 2405's escort",
    );
    assert_eq!(
        zuritska.ai_state,
        AiState::Follow,
        "a resolved follow target must also transition the actor into Follow",
    );
}

/// Mission 704 chain 1291 (Comms Room arrival) against an actor that is
/// mid-escort: `follow_target_id` must come back `None` and the actor must
/// drop out of `Follow`.
///
/// The empty params object only *means* "clear" because the loader maps a
/// missing `target_tag` to `None` and the arm maps an unresolved target to
/// a cleared follow. A regression in either half — a loader default that
/// invented a tag, or an arm that left a stale `follow_target_id` — would
/// leave Zuritska trailing the player for the rest of the mission while
/// every seed assertion still passed.
#[tokio::test]
async fn chain_1291_clears_the_escort_follow() {
    let pool = require_db_or_skip!();
    let resolved = resolve_seeded(
        &pool,
        1291,
        TriggerType::RegionEnter,
        &[
            ("region_key", serde_json::json!("Castle.CommsRoom")),
            ("world_name", serde_json::json!("Castle")),
            ("mission_704_step_2405_status", serde_json::json!("active")),
        ],
    )
    .await;

    let mut mgr = make_castle_space_mgr();
    stage_player(&mut mgr);
    // Staged mid-escort so an "already cleared" regression can't pass.
    stage_zuritska(&mut mgr, [3.0, 0.0, 3.0], true);

    let (tx, _rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;

    let zuritska = mgr
        .get_entity(ZURITSKA_CELL_EID)
        .expect("Zuritska must survive the chain");
    assert_eq!(
        zuritska.follow_target_id, None,
        "arriving in the Communications Room must END the escort",
    );
    assert_eq!(
        zuritska.ai_state,
        AiState::Idle,
        "a cleared follow must drop the actor back to Idle",
    );
    assert!(
        zuritska.nav_path.is_empty(),
        "the in-flight follow path must be cleared or she keeps walking to \
         the player's last position",
    );
}

/// The launcher-to-victory hop. Chain 1292 emits
/// `StartMinigame { on_victory_chains: [1293] }`, and the minigame callback
/// later invokes 1293 by id through `fire_chain_by_id`. Nothing else
/// connects the two — 1293 has no trigger row — so this hop is the single
/// point of failure for the Data Crystal grant.
///
/// The test drives both halves: it executes 1292 and reads the id straight
/// off the emitted `CellToBaseMsg`, then feeds that id back through
/// `fire_chain_by_id` and asserts the grant actually reaches base. A wrong
/// id in the seed, or a `get_chain_actions` lookup that missed a
/// triggerless chain, would break the mission silently.
#[tokio::test]
async fn the_livewire_victory_hop_reaches_the_data_crystal_grant() {
    let pool = require_db_or_skip!();
    let resolved = resolve_seeded(
        &pool,
        1292,
        TriggerType::InteractTag,
        &[
            ("entity_tag", serde_json::json!("Castle_CommsTerminal")),
            ("mission_704_step_2406_status", serde_json::json!("active")),
        ],
    )
    .await;

    let mut mgr = make_castle_space_mgr();
    stage_player(&mut mgr);

    let (tx, mut rx) = mpsc::channel(64);
    let launcher_engine = ChainEngine::new();
    execute_actions(
        resolved,
        PLAYER_EID,
        PLAYER_ID,
        &tx,
        &mut mgr,
        &launcher_engine,
    )
    .await;

    let mut victory_chains: Option<Vec<i64>> = None;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::StartMinigame {
            game_name,
            on_victory_chains,
            ..
        } = msg
        {
            assert_eq!(game_name, "Livewire", "D-CA09: Livewire, provisional");
            victory_chains = Some(on_victory_chains);
        }
    }
    let victory_chains = victory_chains
        .expect("chain 1292 must emit a StartMinigame message through the executor arm");
    assert_eq!(
        victory_chains,
        vec![1293_i64],
        "the launcher must hand the minigame exactly one victory chain id",
    );

    // Now the other half of the hop: the id the launcher published must
    // resolve to a chain whose actions actually execute.
    let victory_chain = load_single_chain_for_test(&pool, 1293)
        .await
        .expect("DB query for chain 1293 must succeed")
        .expect("chain 1293 must exist in seeded content_chains");
    let mut victory_engine = ChainEngine::new();
    victory_engine.register_chain(victory_chain);

    fire_chain_by_id(
        victory_chains[0],
        PLAYER_EID,
        PLAYER_ID,
        &victory_engine,
        &tx,
        &mut mgr,
    )
    .await;

    let mut grants = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::GrantItem {
            item_id,
            count,
            container_id,
            ..
        } = msg
        {
            grants.push((item_id, count, container_id));
        }
    }
    assert_eq!(
        grants.len(),
        1,
        "the victory hop must grant exactly one item; got {grants:?}",
    );
    let (item_id, count, container_id) = grants[0];
    assert_eq!(item_id, 5029, "must be the Data Crystal");
    assert_eq!(count, 1);
    // `container: 0` in the seed falls through to the item's own
    // `container_sets[1]`, which the executor resolves from
    // `space_mgr.item_containers`. That cache is empty in this fixture, so
    // the arm's documented INV_Main fallback (1) is what lands here — the
    // point of the assertion is that the sentinel never reaches base as a
    // literal 0, which is not a valid container.
    assert_ne!(
        container_id, 0,
        "the container: 0 sentinel must be resolved before the message \
         leaves the cell; got {container_id}",
    );
}

/// The `use_player` guard: an NPC-triggered chain must not bind a follow
/// target to a non-player entity. Chain 1263 only ever fires from a player
/// dialog choice today, but the arm is shared, and the guard is the thing
/// standing between a future NPC-triggered caller and Zuritska following a
/// guard around the Interrogation Block forever.
#[tokio::test]
async fn chain_1263_refuses_to_follow_a_non_player_trigger_entity() {
    const NOT_A_PLAYER_EID: u32 = 7433;

    let pool = require_db_or_skip!();
    let resolved = resolve_seeded(
        &pool,
        1263,
        TriggerType::DialogChoice,
        &[
            ("dialog_id", serde_json::json!(2577)),
            ("mission_702_step_2419_status", serde_json::json!("active")),
            ("mission_704_status", serde_json::json!("not_active")),
        ],
    )
    .await;

    let mut mgr = make_castle_space_mgr();
    stage_zuritska(&mut mgr, [20.0, 0.0, 20.0], false);
    mgr.spawn_npc(NOT_A_PLAYER_EID, "Castle", [1.0, 0.0, 1.0], [0.0; 3])
        .expect("Castle startup space must accept the second NPC");

    let (tx, _rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();
    execute_actions(
        resolved,
        NOT_A_PLAYER_EID,
        PLAYER_ID,
        &tx,
        &mut mgr,
        &exec_engine,
    )
    .await;

    let zuritska = mgr
        .get_entity(ZURITSKA_CELL_EID)
        .expect("Zuritska must survive the chain");
    assert_eq!(
        zuritska.follow_target_id, None,
        "`use_player` must resolve to nothing when the triggering entity is \
         not a player, rather than binding the NPC",
    );
    assert_eq!(zuritska.ai_state, AiState::Idle);
}
