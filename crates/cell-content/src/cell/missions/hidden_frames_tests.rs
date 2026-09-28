//! Hidden-mission client-frame gate (#715).
//!
//! The reference `MissionManager.py` never sends `onMissionUpdate`,
//! `onStepUpdate` or `onObjectiveUpdate` for a mission whose def is
//! `isHidden`. These tests drive one hidden mission through every send site
//! in `cell::missions` (accept → complete objective → advance step →
//! complete, plus abandon and the objective-completes-the-mission path) and
//! assert that no frame reaches the base while the state still moves. The
//! control test runs the identical sequence on a visible mission and expects
//! frames at every stage, so a gate that suppressed everything would fail it.

use tokio::sync::mpsc;

use cimmeria_entity::missions::{
    MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE, STATUS_COMPLETED,
};

use super::{
    abandon_mission, accept_mission, advance_step, complete_mission_direct, complete_objective,
    ON_MISSION_UPDATE, ON_OBJECTIVE_UPDATE, ON_STEP_UPDATE,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{MissionDefEntry, MissionObjectiveDef};

const EID: u32 = 1;
const PLAYER_ID: i32 = 4715;
/// Mission 689 (Prison Boot lock gate) is the seeded hidden mission this
/// ticket names; the ids are only labels here, the def is built inline.
const MISSION: i32 = 689;
const FIRST_STEP: i32 = 2200;
const SECOND_STEP: i32 = 2201;

fn make_mgr(is_hidden: bool) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(EID, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(EID).unwrap().player_id = Some(PLAYER_ID);
    mgr.mission_defs.insert(
        MISSION,
        MissionDefEntry {
            step_id: FIRST_STEP,
            objectives: vec![],
            is_hidden,
            num_repeats: 0,
            can_repeat_on_fail: false,
        },
    );
    mgr.step_objectives.insert(
        SECOND_STEP,
        vec![MissionObjectiveDef {
            objective_id: 310,
            is_hidden: false,
            is_optional: false,
        }],
    );
    mgr
}

fn objectives(ids: &[i32]) -> Vec<MissionObjective> {
    ids.iter()
        .map(|&objective_id| MissionObjective {
            objective_id,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        })
        .collect()
}

/// Method indices of every mission frame queued since the last drain.
fn drain_mission_frames(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<u16> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            if matches!(
                method_index,
                ON_MISSION_UPDATE | ON_STEP_UPDATE | ON_OBJECTIVE_UPDATE
            ) {
                out.push(method_index);
            }
        }
    }
    out
}

/// Runs accept → complete objective 300 → advance to `SECOND_STEP` →
/// complete directly, returning the frames each stage queued. Asserts the
/// state moved at every stage, so both the hidden and the control test
/// prove the lifecycle itself is untouched by the gate.
async fn run_lifecycle(mgr: &mut SpaceManager) -> [Vec<u16>; 4] {
    let (tx, mut rx) = mpsc::channel(64);
    let mission = |mgr: &SpaceManager| {
        mgr.get_entity(EID)
            .unwrap()
            .missions
            .get_mission(MISSION)
            .cloned()
            .expect("mission instance must exist")
    };

    assert!(accept_mission(EID, MISSION, FIRST_STEP, objectives(&[300, 301]), &tx, mgr).await);
    let accept = drain_mission_frames(&mut rx);
    assert_eq!(mission(mgr).status, MISSION_ACTIVE);

    assert!(complete_objective(EID, MISSION, 300, &tx, mgr).await);
    let objective = drain_mission_frames(&mut rx);
    let m = mission(mgr);
    assert_eq!(m.status, MISSION_ACTIVE, "301 is still open");
    assert!(
        m.active_objectives
            .iter()
            .any(|o| o.objective_id == 300 && o.status == STATUS_COMPLETED),
        "objective 300 must be completed in state"
    );

    assert!(advance_step(EID, MISSION, SECOND_STEP, &tx, mgr).await);
    let advance = drain_mission_frames(&mut rx);
    let m = mission(mgr);
    assert_eq!(m.current_step_id, Some(SECOND_STEP));
    assert!(m.completed_steps.contains(&FIRST_STEP));

    complete_mission_direct(EID, MISSION, &tx, mgr).await;
    let complete = drain_mission_frames(&mut rx);
    assert_eq!(mission(mgr).status, MISSION_COMPLETED);

    [accept, objective, advance, complete]
}

/// **#715 regression guard.** A hidden mission's whole lifecycle runs with
/// zero mission frames. Remove the `suppress_hidden_mission_frames` call
/// from any one site and that stage's vector is non-empty.
#[tokio::test]
async fn hidden_mission_lifecycle_sends_no_client_frames() {
    let mut mgr = make_mgr(true);
    let [accept, objective, advance, complete] = run_lifecycle(&mut mgr).await;

    assert!(
        mgr.get_entity(EID)
            .unwrap()
            .missions
            .get_mission(MISSION)
            .unwrap()
            .is_hidden
    );
    assert_eq!(
        accept,
        Vec::<u16>::new(),
        "accept must not announce a hidden mission"
    );
    assert_eq!(
        objective,
        Vec::<u16>::new(),
        "objective completion must be silent"
    );
    assert_eq!(advance, Vec::<u16>::new(), "step advance must be silent");
    assert_eq!(
        complete,
        Vec::<u16>::new(),
        "direct completion must be silent"
    );
}

/// Control: the same sequence on a visible mission still sends every frame
/// family, so the gate keys on `is_hidden` and not on anything broader.
#[tokio::test]
async fn visible_mission_lifecycle_still_sends_client_frames() {
    let mut mgr = make_mgr(false);
    let [accept, objective, advance, complete] = run_lifecycle(&mut mgr).await;

    assert_eq!(
        accept,
        vec![
            ON_MISSION_UPDATE,
            ON_STEP_UPDATE,
            ON_OBJECTIVE_UPDATE,
            ON_OBJECTIVE_UPDATE
        ]
    );
    assert_eq!(objective, vec![ON_OBJECTIVE_UPDATE]);
    // 301 force-completed, old step completed, new step active, 310 active.
    assert_eq!(
        advance,
        vec![
            ON_OBJECTIVE_UPDATE,
            ON_STEP_UPDATE,
            ON_STEP_UPDATE,
            ON_OBJECTIVE_UPDATE
        ]
    );
    assert_eq!(
        complete,
        vec![ON_OBJECTIVE_UPDATE, ON_STEP_UPDATE, ON_MISSION_UPDATE]
    );
}

/// The last required objective completing a hidden mission goes through
/// `complete_objective`'s own completion branch (step + mission frames),
/// a different send site from `complete_mission_direct`.
#[tokio::test]
async fn hidden_mission_completed_by_its_last_objective_is_silent() {
    let mut mgr = make_mgr(true);
    let (tx, mut rx) = mpsc::channel(64);
    assert!(accept_mission(EID, MISSION, FIRST_STEP, objectives(&[300]), &tx, &mut mgr).await);
    assert!(complete_objective(EID, MISSION, 300, &tx, &mut mgr).await);

    assert_eq!(drain_mission_frames(&mut rx), Vec::<u16>::new());
    let m = mgr
        .get_entity(EID)
        .unwrap()
        .missions
        .get_mission(MISSION)
        .unwrap();
    assert_eq!(m.status, MISSION_COMPLETED, "state still completes");
}

/// Abandoning a hidden mission (chain action or GM) still removes it, but
/// sends no `onMissionUpdate`: the client never had a journal row for it.
#[tokio::test]
async fn hidden_mission_abandon_removes_without_a_frame() {
    let mut mgr = make_mgr(true);
    let (tx, mut rx) = mpsc::channel(64);
    assert!(accept_mission(EID, MISSION, FIRST_STEP, objectives(&[300]), &tx, &mut mgr).await);

    assert!(abandon_mission(EID, MISSION, &tx, &mut mgr).await);

    assert_eq!(drain_mission_frames(&mut rx), Vec::<u16>::new());
    assert!(mgr
        .get_entity(EID)
        .unwrap()
        .missions
        .get_mission(MISSION)
        .is_none());
}

/// The suppression is visible in SigNoz: one DEBUG event per call site with
/// `reason=hidden_mission`, the mission id and the player id.
#[tokio::test]
async fn suppressed_frames_log_a_debug_event_with_mission_and_player() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_mgr(true);
    let (tx, _rx) = mpsc::channel(64);
    assert!(accept_mission(EID, MISSION, FIRST_STEP, objectives(&[300]), &tx, &mut mgr).await);

    let event = capture
        .find_event(
            Level::DEBUG,
            "mission client frames suppressed",
            "hidden_mission",
        )
        .unwrap_or_else(|| panic!("no suppression event; captured: {:#?}", capture.all()));
    assert!(event.has_field("mission_id", &MISSION.to_string()));
    assert!(event.has_field("site", "accept"));
    assert!(
        event.has_field("player_id", &PLAYER_ID.to_string()),
        "player_id must ride the event; captured: {event:#?}"
    );
}
