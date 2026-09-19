//! Every abandon path fires `mission_abandoned` exactly once (Harset H54).
//!
//! Three paths reach `cell::missions::abandon_mission`, and each one was
//! silent before this packet:
//!
//! 1. the client-callable `abandonMission` cell method (Missionary index 52),
//! 2. the `abandon_mission` chain action,
//! 3. `gmMissionClear` / `gmMissionAbandon` (two indices, one handler).
//!
//! The tests below drive each *entry point* rather than the dispatcher, so
//! removing a hook fails the test. The counter is bumped by the chain's own
//! action, which makes "exactly once" observable — a dispatcher fired twice
//! would read 2, and one never fired would read 0.

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine, ResolvedActions};
use cimmeria_content_engine::conditions::{ComparisonOp, Condition, MissionStatusValue};
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::missions::MissionInstance;

use crate::cell::cell_methods::gm::{GM_MISSION_ABANDON, GM_MISSION_CLEAR};
use crate::cell::cell_methods::missionary::ABANDON_MISSION;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::executor;

const PLAYER_EID: u32 = 1;
const PLAYER_ID: i32 = 100;
/// Moh'katan's offer chain family; 1324 is the mission the packet names.
const MISSION: i32 = 1324;
const STEP: i32 = 3954;
const CHAIN: i64 = 0x7005_4001;
const COUNTER: &str = "offer_repainted";

fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(PLAYER_EID, "Harset_CmdCenter", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(PLAYER_EID) {
        e.is_player = true;
        e.player_id = Some(PLAYER_ID);
        e.access_level = 2; // GameMaster, for the gm* path
        e.missions
            .add_mission(MissionInstance::new(MISSION, STEP, vec![]));
    }
    mgr.connect_entity(PLAYER_EID);
    mgr
}

/// The repaint chain, shaped like the seed rows this packet adds: keyed on
/// the abandon, gated `mission_status <id> eq not_active`.
///
/// The gate is the H07 ordering contract in test form. The dispatcher
/// populates the context **after** `abandon_mission` removes the instance, so
/// `mission_1324_status` reads `not_active` and the gate passes. Populating
/// before the mutation would leave it `active` and every repaint chain in the
/// seed would fail closed.
fn make_engine() -> ChainEngine {
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: CHAIN,
        name: "test: repaint Moh'katan's offer on abandon".to_string(),
        enabled: true,
        trigger: Trigger::OnMissionAbandoned {
            mission_id: MISSION,
        },
        conditions: vec![Condition::MissionStatus {
            mission_id: MISSION,
            operator: ComparisonOp::Eq,
            expected_status: MissionStatusValue::NotActive,
        }],
        actions: vec![Action::IncrementCounter {
            counter_name: COUNTER.to_string(),
            amount: 1,
        }],
        action_delays: Vec::new(),
        priority: 0,
    });
    engine
}

fn fired(mgr: &SpaceManager) -> i32 {
    mgr.get_entity(PLAYER_EID)
        .and_then(|e| e.counters.get(COUNTER).copied())
        .unwrap_or(0)
}

fn still_holds_mission(mgr: &SpaceManager) -> bool {
    mgr.get_entity(PLAYER_EID)
        .and_then(|e| e.missions.get_mission(MISSION))
        .is_some()
}

/// `WSTRING DesignID` as the GM handlers parse it.
fn design_id(id: i32) -> Vec<u8> {
    let mut args = Vec::new();
    crate::mercury::write_wstring(&mut args, &id.to_string());
    args
}

// ── Path 1: the client-callable cell method ───────────────────────────────

#[tokio::test]
async fn the_client_abandon_method_fires_mission_abandoned_once() {
    let mut mgr = make_mgr();
    let engine = make_engine();
    let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(256);

    let handled = crate::cell::cell_methods::missionary::dispatch(
        PLAYER_EID,
        ABANDON_MISSION,
        &MISSION.to_le_bytes(),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(handled, "index 52 belongs to the Missionary interface");
    assert!(!still_holds_mission(&mgr), "the mission is gone");
    assert_eq!(
        fired(&mgr),
        1,
        "abandonMission must fire mission_abandoned exactly once"
    );
}

/// Abandoning a mission the player does not hold removes nothing, so nothing
/// may fire. A client can send this index for any id; without the guard it
/// would be a free re-trigger of every repaint chain in the seed.
#[tokio::test]
async fn abandoning_a_mission_the_player_does_not_hold_fires_nothing() {
    let mut mgr = make_mgr();
    if let Some(e) = mgr.get_entity_mut(PLAYER_EID) {
        e.missions.remove_mission(MISSION);
    }
    let engine = make_engine();
    let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(256);

    crate::cell::cell_methods::missionary::dispatch(
        PLAYER_EID,
        ABANDON_MISSION,
        &MISSION.to_le_bytes(),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert_eq!(
        fired(&mgr),
        0,
        "no removal happened, so no mission_abandoned event may fire"
    );
}

// ── Path 2: the `abandon_mission` chain action ────────────────────────────

#[tokio::test]
async fn the_abandon_mission_chain_action_fires_mission_abandoned_once() {
    let mut mgr = make_mgr();
    let engine = make_engine();
    let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(256);

    let resolved = ResolvedActions {
        actions: vec![(
            0x7005_4000,
            Action::AbandonMission {
                mission_id: MISSION,
            },
        )],
        action_delays: vec![0],
        params: std::collections::HashMap::new(),
    };
    executor::execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &engine).await;

    assert!(!still_holds_mission(&mgr));
    assert_eq!(
        fired(&mgr),
        1,
        "the abandon_mission action must fire mission_abandoned exactly once"
    );
}

// ── Path 3: the two GM indices, one handler ───────────────────────────────

/// `gmMissionClear` and `gmMissionAbandon` route to the same handler, so both
/// indices are exercised — an index table that drifted would send one of them
/// somewhere else and this would catch it.
#[tokio::test]
async fn both_gm_abandon_indices_fire_mission_abandoned_once() {
    for index in [GM_MISSION_CLEAR, GM_MISSION_ABANDON] {
        let mut mgr = make_mgr();
        let engine = make_engine();
        let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(256);

        let handled = crate::cell::cell_methods::gm::dispatch(
            PLAYER_EID,
            index,
            &design_id(MISSION),
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        assert!(
            handled,
            "index {index} must be handled by the gm dispatcher"
        );
        assert!(!still_holds_mission(&mgr), "index {index}: mission is gone");
        assert_eq!(
            fired(&mgr),
            1,
            "index {index} must fire mission_abandoned exactly once"
        );
    }
}

// ── Ordering: the context is populated after the mutation ─────────────────

/// The mirror image of the gate in [`make_engine`]: a chain gated on the
/// mission still being **active** must NOT fire. If the dispatcher populated
/// the context before `abandon_mission` removed the instance, this would fire
/// and the `not_active` chain above would not — which is precisely the H07
/// contract failure the `world_context_contract_tests` module exists to
/// catch, in its mission-status form.
#[tokio::test]
async fn the_context_reflects_the_post_removal_state() {
    let mut mgr = make_mgr();
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 0x7005_4002,
        name: "test: fires only if the mission is still active".to_string(),
        enabled: true,
        trigger: Trigger::OnMissionAbandoned {
            mission_id: MISSION,
        },
        conditions: vec![Condition::MissionStatus {
            mission_id: MISSION,
            operator: ComparisonOp::Eq,
            expected_status: MissionStatusValue::Active,
        }],
        actions: vec![Action::IncrementCounter {
            counter_name: COUNTER.to_string(),
            amount: 1,
        }],
        action_delays: Vec::new(),
        priority: 0,
    });
    let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(256);

    crate::cell::cell_methods::missionary::dispatch(
        PLAYER_EID,
        ABANDON_MISSION,
        &MISSION.to_le_bytes(),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert_eq!(
        fired(&mgr),
        0,
        "the context must be populated AFTER the removal, so `mission_status \
         eq active` can no longer hold"
    );
}
