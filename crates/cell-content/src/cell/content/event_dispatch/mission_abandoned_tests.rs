//! The `abandon_mission` chain action fires `mission_abandoned` exactly once
//! (Harset H54).
//!
//! One of the three abandon paths into `cell::missions::abandon_mission`.
//! The other two, the client-callable `abandonMission` cell method and
//! `gmMissionClear` / `gmMissionAbandon`, drive cell-method dispatchers that
//! sit above this crate; their guards are `cimmeria-services`'
//! `cell::content_tests::mission_abandoned`, whose fixtures these are copies
//! of (split in wave C3 of docs/architecture/services-crate-split.md).
//!
//! The counter is bumped by the chain's own action, which makes "exactly
//! once" observable — a dispatcher fired twice would read 2, and one never
//! fired would read 0.

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine, ResolvedActions};
use cimmeria_content_engine::conditions::{ComparisonOp, Condition, MissionStatusValue};
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::missions::MissionInstance;

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
