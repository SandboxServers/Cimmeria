//! `move_entity` chain-replay guards for the five seeded rows in
//! `db/resources/Content/Seed/sgc_w1_chains.sql` (issue #613).
//!
//! Unlike the other modules here, these tests do not stop at
//! `resolve_event` — they push the resolved actions through
//! [`execute_actions`] and assert on the `CellToBaseMsg` traffic. That is
//! deliberate: `move_entity` had a *loader* arm all along, so a
//! resolve-only test passed happily while the executor's `other =>`
//! catch-all silently dropped every row. Loading the chain from the DB
//! guards the seed; executing it guards the arm. Both halves are
//! load-bearing.
//!
//! The bug shape: chains 3007 and 3028 carry `use_player: true` with
//! `target_key` NULL — the two SGC elevators. The player pressed the
//! button, the step advanced, the dialog played, and the avatar never
//! moved. Chains 3005 / 3009 / 3011 are the NPC half of the same verb.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::executor::execute_actions;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

/// Entity id of the player firing each chain.
const PLAYER_EID: u32 = 7001;
/// Entity id of the tagged NPC in the NPC-branch test.
const NPC_EID: u32 = 7002;

/// `SGC_W1` space matching the world name every seeded `move_entity`
/// row names in its `world` param. The bounds are wide enough to hold
/// the authored destinations (which are negative on X and Z) so the
/// spatial-grid update in `transport::teleport` / `move_waypoint`
/// doesn't reject them.
fn make_sgc_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="SGC_W1" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="SGC_W1" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

/// Stage the chain-firing player in `SGC_W1` at the origin.
fn stage_player(mgr: &mut SpaceManager) {
    mgr.create_entity(PLAYER_EID, "SGC_W1", [0.0, 0.0, 0.0], [0.0; 3])
        .expect("SGC_W1 startup space must accept the player entity");
    let p = mgr
        .get_entity_mut(PLAYER_EID)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(42);
    mgr.connect_entity(PLAYER_EID);
}

/// Drain the cell→base channel and return every `TeleportPlayer` send.
/// Returning the full list (rather than the first hit) lets the caller
/// assert the *cardinality* too — a dispatch bug that ran both the
/// player and the NPC branch would show up as two sends.
fn drain_teleports(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u32, [f32; 3], [f32; 3])> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::TeleportPlayer {
            entity_id,
            position,
            prev_pos,
            ..
        } = msg
        {
            out.push((entity_id, position, prev_pos));
        }
    }
    out
}

fn assert_close(actual: [f32; 3], expected: [f32; 3], what: &str) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() < 0.001,
            "{what}: axis {axis} must be {} but was {} (full vector {actual:?} vs {expected:?})",
            expected[axis],
            actual[axis],
        );
    }
}

/// Chain 3007 — dialog choice 5357 ("move to armory"). The authored
/// `move_entity` row is `use_player: true` / `target_key` NULL, so it
/// must produce exactly one `CellToBaseMsg::TeleportPlayer` for the
/// firing player at the authored destination. Before the executor arm
/// existed this chain advanced the objective and played its sequence
/// while the avatar stayed put.
#[tokio::test]
async fn chain_3007_move_entity_teleports_the_player_to_the_armory() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 3007)
        .await
        .expect("DB query for chain 3007 must succeed")
        .expect("chain 3007 must exist in seeded content_chains and assemble successfully");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(5357));
    let event = TriggerEvent {
        trigger_type: TriggerType::DialogChoice,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    assert!(
        resolved.actions.iter().any(|(id, _)| *id == 3007),
        "chain 3007 must resolve on dialog_choice 5357 — if this fails the \
         seed or the trigger wiring drifted, not the executor arm",
    );

    let mut mgr = make_sgc_space_mgr();
    stage_player(&mut mgr);
    let (tx, mut rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, 42, &tx, &mut mgr, &exec_engine).await;

    let teleports = drain_teleports(&mut rx);
    assert_eq!(
        teleports.len(),
        1,
        "chain 3007's move_entity(use_player) must produce exactly one \
         TeleportPlayer; got {teleports:?}",
    );
    let (eid, position, prev_pos) = teleports[0];
    assert_eq!(
        eid, PLAYER_EID,
        "TeleportPlayer must target the player who fired the chain",
    );
    // The authored destination in sgc_w1_chains.sql chain 3007.
    assert_close(
        position,
        [-123.625, 1.311, -246.858],
        "chain 3007 destination",
    );
    // prev_pos is the anti-camera-snap reference — it must be where the
    // player *was*, not the destination. A prev_pos equal to the
    // destination means the grid update ran before the capture.
    assert_close(prev_pos, [0.0, 0.0, 0.0], "chain 3007 prev_pos");
    // The server-side position must move too, or the next AoI tick
    // re-broadcasts the stale coordinates and the client snaps back.
    let moved = mgr
        .get_entity(PLAYER_EID)
        .expect("player must still exist after a same-world teleport");
    assert_close(
        [moved.position.x, moved.position.y, moved.position.z],
        [-123.625, 1.311, -246.858],
        "chain 3007 server-side player position",
    );
}

/// Chain 3028 — the second elevator, gated on mission 1562 step 4624
/// being active. Same `use_player` shape as 3007 with a different
/// destination (Carter's lab), and it arrives via `interact_tag` rather
/// than `dialog_choice`, so it also pins that the executor arm is
/// reached from the interaction path.
#[tokio::test]
async fn chain_3028_move_entity_teleports_the_player_to_carters_lab() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 3028)
        .await
        .expect("DB query for chain 3028 must succeed")
        .expect("chain 3028 must exist in seeded content_chains and assemble successfully");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("SGC_W1_ElevatorButton2"),
    );
    // Satisfies the seeded `step_status(1562, 4624) = active` condition.
    ctx.set_param(
        "mission_1562_step_4624_status".to_string(),
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
        resolved.actions.iter().any(|(id, _)| *id == 3028),
        "chain 3028 must resolve on interact_tag SGC_W1_ElevatorButton2 \
         while step 4624 is active",
    );

    let mut mgr = make_sgc_space_mgr();
    stage_player(&mut mgr);
    let (tx, mut rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, 42, &tx, &mut mgr, &exec_engine).await;

    let teleports = drain_teleports(&mut rx);
    assert_eq!(
        teleports.len(),
        1,
        "chain 3028's move_entity(use_player) must produce exactly one \
         TeleportPlayer; got {teleports:?}",
    );
    let (eid, position, _) = teleports[0];
    assert_eq!(
        eid, PLAYER_EID,
        "TeleportPlayer must target the firing player"
    );
    // The authored destination in sgc_w1_chains.sql chain 3028.
    assert_close(position, [-53.86, 1.311, 62.136], "chain 3028 destination");
}

/// Chain 3009 — the NPC half of the same verb. `target_key` names
/// `SGCW1_AirmanWalking` and there is no `use_player`, so the arm must
/// take the waypoint branch: the *tagged NPC* moves and the player is
/// left alone. Asserting "zero TeleportPlayer" is the dispatch guard —
/// a `use_player`-defaulting-true bug would teleport the player to the
/// airman's waypoint, which is a far worse failure than the original
/// no-op.
#[tokio::test]
async fn chain_3009_move_entity_repositions_the_tagged_airman_not_the_player() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 3009)
        .await
        .expect("DB query for chain 3009 must succeed")
        .expect("chain 3009 must exist in seeded content_chains and assemble successfully");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(1559));
    let event = TriggerEvent {
        trigger_type: TriggerType::MissionCompleted,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    assert!(
        resolved.actions.iter().any(|(id, _)| *id == 3009),
        "chain 3009 must resolve on mission_completed 1559",
    );

    let mut mgr = make_sgc_space_mgr();
    stage_player(&mut mgr);
    // The tagged NPC starts nowhere near the authored waypoint so a
    // "position unchanged" regression can't accidentally pass.
    mgr.create_entity(NPC_EID, "SGC_W1", [1.0, 0.0, 2.0], [0.0; 3])
        .expect("SGC_W1 startup space must accept the NPC entity");
    mgr.get_entity_mut(NPC_EID)
        .expect("NPC entity must exist immediately after create_entity")
        .tag = Some("SGCW1_AirmanWalking".to_string());

    let (tx, mut rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, 42, &tx, &mut mgr, &exec_engine).await;

    let npc = mgr
        .get_entity(NPC_EID)
        .expect("NPC must survive a waypoint reposition");
    // The authored destination in sgc_w1_chains.sql chain 3009.
    assert_close(
        [npc.position.x, npc.position.y, npc.position.z],
        [237.326, 1.312, 17.921],
        "chain 3009 tagged-NPC position",
    );

    let teleports = drain_teleports(&mut rx);
    assert!(
        teleports.is_empty(),
        "an NPC-tagged move_entity row must not teleport the player; got {teleports:?}",
    );
    let player = mgr
        .get_entity(PLAYER_EID)
        .expect("player must be untouched by an NPC reposition");
    assert_close(
        [player.position.x, player.position.y, player.position.z],
        [0.0, 0.0, 0.0],
        "chain 3009 must leave the player where they were",
    );
}
