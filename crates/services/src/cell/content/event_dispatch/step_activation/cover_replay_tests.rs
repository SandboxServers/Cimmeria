//! Guards for the `player_entered_cover` form of the step-activation replay.
//!
//! As in the region guards next door, "the replay happens at all" is tested
//! through the **executor arm**, because the hook inside that arm is what
//! regresses. The cover edge is spent by running the real detection tick
//! first, so the fixture is the production sequence: tick sees the player in
//! cover, chain fails its step gate, step activates afterwards.

use std::collections::HashMap;
use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_common::{EntityId, Vector3};
use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine, ResolvedActions};
use cimmeria_content_engine::conditions::{ComparisonOp, Condition, StepStatusValue};
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::missions::{MissionInstance, MissionObjective, STATUS_ACTIVE};

use crate::cell::cover::{
    run_detection_tick, Cover, CoverHeight, CoverNode, CoverQuality,
    COVER_DURATION_MILESTONES_SECS, COVER_PROXIMITY_RADIUS,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::super::executor;

const PLAYER_EID: u32 = 1;
const PLAYER_ID: i32 = 100;

/// Mission 639's shape: the vial step, then the defend / take-cover step.
const MISSION: i32 = 639;
const STEP_VIAL: i32 = 2145;
const STEP_DEFEND: i32 = 2144;
const OBJ: i32 = 2484;
/// The med-station desk.
const DESK_SET: i32 = 1381;

/// `0x7005_3xxx` — this file's block of the reserved test chain-id range.
const FIRING_CHAIN: i64 = 0x7005_3000;
const CHAIN_GATED: i64 = 0x7005_3001;
const CHAIN_UNGATED: i64 = 0x7005_3002;

fn desk_node() -> CoverNode {
    CoverNode {
        chunk_id: DESK_SET,
        node_id: 0,
        pos: Vector3::new(0.0, 0.0, 0.0),
        orient: 0.0,
        height: CoverHeight::Low,
        quality: CoverQuality::Good,
        tail: [0; 4],
    }
}

/// One connected player at `pos` in a world whose only cover is the desk.
fn make_mgr(pos: [f32; 3]) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(PLAYER_EID, "Castle", pos, [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(PLAYER_EID) {
        e.is_player = true;
        e.player_id = Some(PLAYER_ID);
        e.missions.add_mission(MissionInstance::new(
            MISSION,
            STEP_VIAL,
            vec![MissionObjective {
                objective_id: OBJ,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        ));
    }
    mgr.connect_entity(PLAYER_EID);
    mgr.cover = Cover::from_loaded(Vec::new(), vec![desk_node()]);
    mgr
}

/// Run the real detection pass, which is what spends the enter edge and puts
/// the set in the detection table. Returns how many enter edges it produced.
fn detection_tick(mgr: &mut SpaceManager) -> usize {
    let pos = mgr.get_entity(PLAYER_EID).unwrap().position;
    run_detection_tick(
        &mgr.cover,
        &[(EntityId(PLAYER_EID as i32), pos)],
        &mut mgr.cover_detection,
        Instant::now(),
        COVER_PROXIMITY_RADIUS,
        COVER_DURATION_MILESTONES_SECS,
    )
    .entered
    .len()
}

fn cover_chain(id: i64, conditions: Vec<Condition>, counter_name: &str) -> Chain {
    Chain {
        id,
        name: format!("test: cover replay {id}"),
        enabled: true,
        trigger: Trigger::OnPlayerEnteredCover {
            cover_set_id: Some(DESK_SET),
        },
        conditions,
        actions: vec![Action::IncrementCounter {
            counter_name: counter_name.to_string(),
            amount: 1,
        }],
        action_delays: Vec::new(),
        priority: 0,
    }
}

fn defend_step_gate() -> Condition {
    Condition::StepStatus {
        mission_id: MISSION,
        step_id: STEP_DEFEND,
        operator: ComparisonOp::Eq,
        expected_status: StepStatusValue::Active,
    }
}

fn counter(mgr: &SpaceManager, name: &str) -> i32 {
    mgr.get_entity(PLAYER_EID)
        .and_then(|e| e.counters.get(name).copied())
        .unwrap_or(0)
}

/// Advance to the defend step through the real executor arm, as chain 1032's
/// vial pickup does.
async fn pick_up_the_vial(mgr: &mut SpaceManager, engine: &ChainEngine) {
    let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(8192);
    let resolved = ResolvedActions {
        actions: vec![(
            FIRING_CHAIN,
            Action::AdvanceStep {
                mission_id: MISSION,
                step_id: STEP_DEFEND,
            },
        )],
        action_delays: vec![0],
        params: HashMap::new(),
    };
    executor::execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, mgr, engine).await;
}

/// The 2026-09-20 repro. The vial is inside the desk's cover radius, so the
/// detection tick spends the enter edge while the mission is still on the vial
/// step; the take-cover chain fails its step gate; the step activates a second
/// later and the player never leaves the radius. Without the replay the
/// counter stays 0 and the player stands on the marker forever.
#[tokio::test]
async fn step_activation_replays_a_cover_set_the_player_is_already_in() {
    let mut mgr = make_mgr([1.0, 0.0, 1.0]);
    let mut engine = ChainEngine::new();
    engine.register_chain(cover_chain(
        CHAIN_GATED,
        vec![defend_step_gate()],
        "took_cover",
    ));

    assert_eq!(
        detection_tick(&mut mgr),
        1,
        "the tick sees the player in cover"
    );
    assert_eq!(
        counter(&mgr, "took_cover"),
        0,
        "pre-condition: the edge is spent before the step is active"
    );

    pick_up_the_vial(&mut mgr, &engine).await;

    assert_eq!(
        counter(&mgr, "took_cover"),
        1,
        "a step-gated player_entered_cover chain must fire when its step activates \
         with the player already in the set; 0 means the cover replay is gone"
    );
    assert_eq!(
        detection_tick(&mut mgr),
        0,
        "the replay must not disturb the detection table — a second enter edge here \
         would double-fire the chain on the next tick"
    );
    assert!(mgr.step_region_replay.is_idle());
}

/// Control: a player outside the radius has spent no edge, so there is nothing
/// to replay. Without this, a replay that fired every cover set in the world
/// would also pass the test above.
#[tokio::test]
async fn step_activation_does_not_replay_a_cover_set_the_player_is_outside() {
    let mut mgr = make_mgr([40.0, 0.0, 0.0]);
    let mut engine = ChainEngine::new();
    engine.register_chain(cover_chain(
        CHAIN_GATED,
        vec![defend_step_gate()],
        "took_cover",
    ));

    assert_eq!(detection_tick(&mut mgr), 0);
    pick_up_the_vial(&mut mgr, &engine).await;

    assert_eq!(counter(&mgr, "took_cover"), 0);
}

/// The table says the edge was spent, but the player has since walked away and
/// the 1 Hz tick has not caught up. Replaying would credit cover the player is
/// not in.
#[tokio::test]
async fn step_activation_skips_a_cover_set_the_player_has_walked_out_of() {
    let mut mgr = make_mgr([1.0, 0.0, 1.0]);
    let mut engine = ChainEngine::new();
    engine.register_chain(cover_chain(
        CHAIN_GATED,
        vec![defend_step_gate()],
        "took_cover",
    ));

    assert_eq!(detection_tick(&mut mgr), 1);
    mgr.get_entity_mut(PLAYER_EID).unwrap().position = Vector3::new(40.0, 0.0, 0.0);

    pick_up_the_vial(&mut mgr, &engine).await;

    assert_eq!(
        counter(&mgr, "took_cover"),
        0,
        "containment is re-checked against the server-known position before firing"
    );
}

/// An ungated cover chain already ran on the real edge. Replaying it would run
/// its actions a second time, so — as with regions — only mission-gated chains
/// are admitted.
#[tokio::test]
async fn step_activation_does_not_replay_an_ungated_cover_chain() {
    let mut mgr = make_mgr([1.0, 0.0, 1.0]);
    let mut engine = ChainEngine::new();
    engine.register_chain(cover_chain(CHAIN_UNGATED, Vec::new(), "ungated"));

    assert_eq!(detection_tick(&mut mgr), 1);
    pick_up_the_vial(&mut mgr, &engine).await;

    assert_eq!(
        counter(&mgr, "ungated"),
        0,
        "an ungated chain is not replayed (the detection_tick helper here does not \
         dispatch, so 0 means the replay refused it)"
    );
}
