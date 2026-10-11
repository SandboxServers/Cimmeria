//! TG-MIS-03 guards: a step activation logs one `mission.step_anchor` row.
//!
//! Both tests advance through the **executor arm**, so deleting the
//! `log_step_anchors` call in `fire_step_activation_regions` leaves no row
//! and fails them.

use std::collections::HashMap;

use tokio::sync::mpsc;

use cimmeria_common::EntityId;
use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine, ResolvedActions};
use cimmeria_content_engine::conditions::{ComparisonOp, Condition, StepStatusValue};
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::missions::{MissionInstance, MissionObjective, STATUS_ACTIVE};

use super::*;
use crate::test_support::{LogCapture, LogCaptureGuard};

/// Spelled out here because the emitter writes it as a literal (so the
/// server's target scanner sees it); a rename has to change both.
const TARGET: &str = "mission.step_anchor";

const PLAYER_EID: u32 = 1;
const ANCHOR_EID: u32 = 2;
const PLAYER_ID: i32 = 100;
const MISSION: i32 = 1360;
const STEP_ONE: i32 = 4037;
const STEP_TWO: i32 = 4038;
const OBJ: i32 = 7001;
/// The reserved `0x7000_xxxx` test range; `0x7005_3xxx` is this packet's.
const FIRING_CHAIN: i64 = 0x7005_3000;
const ADVANCER: i64 = 0x7005_3001;

/// One world, the player on the mission's first step, and (when
/// `with_anchor`) a tagged NPC 3 m away that the player's client already sees.
fn make_mgr(with_anchor: bool) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(PLAYER_EID, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(PLAYER_EID).unwrap();
    e.is_player = true;
    e.player_id = Some(PLAYER_ID);
    e.missions.add_mission(MissionInstance::new(
        MISSION,
        STEP_ONE,
        vec![MissionObjective {
            objective_id: OBJ,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }],
    ));
    if with_anchor {
        mgr.create_entity(ANCHOR_EID, "Castle", [3.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.get_entity_mut(ANCHOR_EID).unwrap().tag = Some("Frost_Terminal".to_string());
        mgr.get_entity_mut(PLAYER_EID)
            .unwrap()
            .witnesses
            .insert(EntityId(ANCHOR_EID as i32));
    }
    mgr.connect_entity(PLAYER_EID);
    mgr
}

fn advancer(tag: &str) -> Chain {
    Chain {
        id: ADVANCER,
        name: "test: step anchor advancer".to_string(),
        enabled: true,
        trigger: Trigger::OnInteractTag {
            entity_tag: tag.to_string(),
        },
        conditions: vec![Condition::StepStatus {
            mission_id: MISSION,
            step_id: STEP_TWO,
            operator: ComparisonOp::Eq,
            expected_status: StepStatusValue::Active,
        }],
        actions: vec![Action::CompleteObjective {
            mission_id: MISSION,
            objective_id: OBJ,
        }],
        action_delays: Vec::new(),
        priority: 0,
        once: false,
    }
}

/// Advance the player to `STEP_TWO` through the executor, the production
/// path that calls `fire_step_activation_regions`.
async fn advance(mgr: &mut SpaceManager, engine: &ChainEngine) {
    let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(8192);
    let resolved = ResolvedActions {
        actions: vec![(
            FIRING_CHAIN,
            Action::AdvanceStep {
                mission_id: MISSION,
                step_id: STEP_TWO,
            },
        )],
        action_delays: vec![0],
        params: HashMap::new(),
    };
    executor::execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, mgr, engine).await;
}

fn anchor_row(capture: &LogCaptureGuard) -> HashMap<String, String> {
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == TARGET)
        .collect();
    assert_eq!(
        rows.len(),
        1,
        "one step activation logs exactly one {TARGET} row; captured: {:#?}",
        capture.all()
    );
    assert_eq!(
        rows[0].level,
        tracing::Level::INFO,
        "{TARGET} is an INFO row; OTEL_FILTER pins it at info"
    );
    rows[0].fields.clone()
}

/// The step-4037 shape: a step no chain advances logs `advancers = 0`, with
/// identity and bare ids, the moment it activates.
#[tokio::test]
async fn ungated_step_logs_zero_advancers() {
    let mut mgr = make_mgr(false);
    let engine = ChainEngine::new();
    let capture = LogCapture::install();

    advance(&mut mgr, &engine).await;

    let row = anchor_row(&capture);
    assert_eq!(row.get("event").map(String::as_str), Some("step_anchor"));
    assert_eq!(row.get("advancers").map(String::as_str), Some("0"));
    assert_eq!(row.get("anchors").map(String::as_str), Some("0"));
    assert_eq!(row.get("player_id").map(String::as_str), Some("100"));
    assert_eq!(row.get("mission_id").map(String::as_str), Some("1360"));
    assert_eq!(row.get("step_id").map(String::as_str), Some("4038"));
    assert_eq!(row.get("world").map(String::as_str), Some("Castle"));
}

/// An advancer's tag resolves to the live entity with its distance and
/// witness state; a tag with no entity (#1341's shape) is listed unresolved.
#[tokio::test]
async fn advancer_anchor_resolves_or_is_listed_unresolved() {
    let mut mgr = make_mgr(true);
    let mut engine = ChainEngine::new();
    engine.register_chain(advancer("Frost_Terminal"));
    let capture = LogCapture::install();

    advance(&mut mgr, &engine).await;

    let row = anchor_row(&capture);
    assert_eq!(row.get("advancers").map(String::as_str), Some("1"));
    assert_eq!(row.get("anchors").map(String::as_str), Some("1"));
    assert_eq!(row.get("anchor_ids").map(String::as_str), Some("2"));
    assert_eq!(
        row.get("anchor_tags").map(String::as_str),
        Some("Frost_Terminal")
    );
    assert_eq!(
        row.get("anchor_in_witness_set").map(String::as_str),
        Some("true")
    );
    assert_eq!(row.get("anchor_distance").map(String::as_str), Some("3.0"));
    assert_eq!(
        row.get("advancer_chain_ids").map(String::as_str),
        Some(ADVANCER.to_string().as_str())
    );
    // A test-range chain id has no seed name, so its slot is the placeholder.
    assert_eq!(
        row.get("advancer_chain_names").map(String::as_str),
        Some("-")
    );
    assert_eq!(row.get("unresolved_count").map(String::as_str), Some("0"));
    assert_eq!(row.get("unresolved_anchors").map(String::as_str), Some(""));

    // The same advancer, but nothing in the space carries its tag.
    let mut mgr = make_mgr(false);
    let mut engine = ChainEngine::new();
    engine.register_chain(advancer("Never_Spawned"));
    drop(capture);
    let capture = LogCapture::install();

    advance(&mut mgr, &engine).await;

    let row = anchor_row(&capture);
    assert_eq!(row.get("advancers").map(String::as_str), Some("1"));
    assert_eq!(row.get("anchors").map(String::as_str), Some("0"));
    assert_eq!(row.get("unresolved_count").map(String::as_str), Some("1"));
    assert_eq!(
        row.get("unresolved_anchors").map(String::as_str),
        Some("Never_Spawned")
    );
}
