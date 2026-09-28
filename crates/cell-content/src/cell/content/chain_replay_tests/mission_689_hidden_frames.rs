//! Mission 689 (Prison Boot lock, `is_hidden = true`) through the executor:
//! the seeded chains accept and complete it without a single mission frame
//! reaching the client (#715), while the state and the `MissionUpdate`
//! persist still happen.
//!
//! [`super::mission_689`] pins what the chains resolve to. This file pushes
//! the resolved actions through [`execute_actions`] against the seeded
//! mission defs, because the frame gate lives in `cell::missions` and a
//! resolve-only test cannot see it. Remove the `suppress_hidden_mission_frames`
//! call from `accept_mission` or `complete_mission_direct` and the matching
//! assertion fails.

use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use cimmeria_entity::missions::{MISSION_ACTIVE, MISSION_COMPLETED};
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::executor::execute_actions;
use crate::cell::client_methods::missionary::{
    ON_MISSION_UPDATE, ON_OBJECTIVE_UPDATE, ON_STEP_UPDATE,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

const PLAYER_EID: u32 = 7689;
const PLAYER_ID: i32 = 4689;
const MISSION: i32 = 689;

fn make_cellblock_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-1200" MaxX="1200" MinY="-1200" MaxY="1200" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(PLAYER_EID, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .expect("Castle_CellBlock startup space must accept the player");
    let p = mgr.get_entity_mut(PLAYER_EID).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    mgr.connect_entity(PLAYER_EID);
    mgr
}

/// Split the drained channel into (mission frames for the player, number of
/// `MissionUpdate` persists for mission 689).
fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> (Vec<u16>, usize) {
    let mut frames = Vec::new();
    let mut persists = 0;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                ..
            } if entity_id == PLAYER_EID
                && matches!(
                    method_index,
                    ON_MISSION_UPDATE | ON_STEP_UPDATE | ON_OBJECTIVE_UPDATE
                ) =>
            {
                frames.push(method_index)
            }
            CellToBaseMsg::MissionUpdate { mission_id, .. } if mission_id == MISSION => {
                persists += 1
            }
            _ => {}
        }
    }
    (frames, persists)
}

fn mission_status(mgr: &SpaceManager) -> Option<(i8, bool)> {
    mgr.get_entity(PLAYER_EID)?
        .missions
        .get_mission(MISSION)
        .map(|m| (m.status, m.is_hidden))
}

/// Chain 1022 (`player_loaded` → `accept_mission 689`) then chain 1025
/// (Livewire victory → `complete_mission 689`), both from the seed, with
/// mission 689's def and first step loaded from `resources.missions`.
#[tokio::test]
async fn live_db_mission_689_accept_and_complete_send_no_client_frames() {
    let pool = require_db_or_skip!();
    let mut mgr = make_cellblock_mgr();
    mgr.mission_defs = crate::cell::spawner::load_mission_defs(&pool)
        .await
        .expect("mission defs must load");
    mgr.step_objectives = crate::cell::spawner::load_step_objectives(&pool)
        .await
        .expect("step objectives must load");
    assert!(
        mgr.mission_defs.get(&MISSION).is_some_and(|d| d.is_hidden),
        "precondition: the seed marks mission 689 is_hidden"
    );

    // ── Accept: chain 1022 on player_loaded ──
    let accept_chain = load_single_chain_for_test(&pool, 1022)
        .await
        .expect("DB query for chain 1022 must succeed")
        .expect("chain 1022 must exist in seeded content_chains");
    let mut engine = ChainEngine::new();
    engine.register_chain(accept_chain);
    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_689_status".to_string(),
        serde_json::json!("not_active"),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    assert!(
        resolved.actions.iter().any(|(id, _)| *id == 1022),
        "chain 1022 must resolve for a fresh character"
    );

    let (tx, mut rx) = mpsc::channel(256);
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &engine).await;
    let (frames, persists) = drain(&mut rx);
    assert_eq!(
        mission_status(&mgr),
        Some((MISSION_ACTIVE, true)),
        "chain 1022 must accept 689 as an active hidden mission"
    );
    assert!(persists >= 1, "the accept must still persist");
    assert_eq!(
        frames,
        Vec::<u16>::new(),
        "accepting hidden mission 689 must send no mission frames"
    );

    // ── Complete: chain 1025, invoked by id on Livewire victory ──
    let complete_chain = load_single_chain_for_test(&pool, 1025)
        .await
        .expect("DB query for chain 1025 must succeed")
        .expect("chain 1025 must exist in seeded content_chains");
    let resolved = ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: complete_chain
            .actions
            .iter()
            .cloned()
            .map(|a| (1025, a))
            .collect(),
    };
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &engine).await;
    let (frames, persists) = drain(&mut rx);
    assert_eq!(
        mission_status(&mgr).map(|(s, _)| s),
        Some(MISSION_COMPLETED),
        "chain 1025 must complete 689"
    );
    assert!(persists >= 1, "the completion must still persist");
    assert_eq!(
        frames,
        Vec::<u16>::new(),
        "completing hidden mission 689 must send no mission frames"
    );
}
