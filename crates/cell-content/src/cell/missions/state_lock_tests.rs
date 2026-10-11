//! TG-DB-12 cell-side lock: refusal, the whole-chain gate and the recovery
//! unlock.

use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_content_engine::actions::Action;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use super::state_lock::{
    gate_mission_chains, is_state_unknown, mark_state_unknown, recover_state, RESTORED_LINE,
    UNAVAILABLE_LINE,
};
use super::{accept_mission, send_mission_update};
use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, LogCapture};

const ENTITY: u32 = 1;
const MISSION: i32 = 622;

fn locked_mgr() -> SpaceManager {
    let mut mgr = make_space_manager();
    mgr.create_entity(ENTITY, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(ENTITY).unwrap().player_id = Some(77);
    mark_state_unknown(ENTITY, &mut mgr);
    mgr
}

fn objectives() -> Vec<MissionObjective> {
    vec![MissionObjective {
        objective_id: 300,
        status: STATUS_ACTIVE,
        hidden: false,
        optional: false,
    }]
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

fn is_feedback(msg: &CellToBaseMsg, text: &str) -> bool {
    let want = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    matches!(msg, CellToBaseMsg::EntityMethodCall { method_index, args, .. }
        if *method_index == ON_PLAYER_COMMUNICATION && *args == want)
}

/// A locked tracker refuses the accept: nothing created, no client frame,
/// no save; one feedback line and the WARN. Without the gate the empty
/// tracker lets the accept through (the #411 overwrite).
#[tokio::test]
async fn locked_accept_is_refused_with_feedback_and_warn() {
    let mut mgr = locked_mgr();
    let capture = LogCapture::install();
    let (tx, mut rx) = mpsc::channel(16);

    let accepted = accept_mission(ENTITY, MISSION, 200, objectives(), &tx, &mut mgr).await;
    send_mission_update(ENTITY, 77, MISSION, "accept", &tx, &mgr).await;

    assert!(!accepted);
    assert!(mgr
        .get_entity(ENTITY)
        .unwrap()
        .missions
        .get_mission(MISSION)
        .is_none());
    let sent = drain(&mut rx);
    assert_eq!(sent.len(), 1, "only the feedback line: {sent:?}");
    assert!(is_feedback(&sent[0], UNAVAILABLE_LINE));
    let row = capture
        .find_event(Level::WARN, "", "saved_missions_unreadable")
        .expect("the refusal WARN");
    assert!(row.has_field("event", "mission_state_unknown"));
    assert!(row.has_field("site", "accept"));
    assert!(row.fields.contains_key("player_id"));
    assert!(row.fields.contains_key("mission_name") || row.fields.contains_key("mission_id"));
}

/// The chain gate drops a chain that changes a mission whole (its counter
/// bump with it) and keeps a chain that doesn't, delays aligned.
#[tokio::test]
async fn locked_tracker_drops_mission_chains_whole() {
    let mgr = locked_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let actions = vec![
        (
            10,
            Action::IncrementCounter {
                counter_name: "c".into(),
                amount: 1,
            },
        ),
        (20, Action::SystemMessage { message_id: 5 }),
        (
            10,
            Action::AcceptMission {
                mission_id: MISSION,
            },
        ),
    ];
    let (kept, delays) = gate_mission_chains(actions, vec![0, 250, 0], ENTITY, &tx, &mgr).await;
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].0, 20);
    assert_eq!(delays, vec![250]);
    let sent = drain(&mut rx);
    assert_eq!(sent.len(), 1);
    assert!(is_feedback(&sent[0], UNAVAILABLE_LINE));
}

/// The recovery unlock: the late missions land, the tracker unlocks, the
/// journal is resent and the player told; a second recovery changes
/// nothing, so progress made after the first is never overwritten.
#[tokio::test]
async fn recovery_restores_unlocks_and_ignores_a_duplicate() {
    let mut mgr = locked_mgr();
    let capture = LogCapture::install();
    let (tx, mut rx) = mpsc::channel(32);

    let mut done = MissionInstance::new(MISSION, 200, objectives());
    done.complete();
    let active = MissionInstance::new(623, 201, objectives());
    assert!(recover_state(ENTITY, vec![done, active], &tx, &mut mgr).await);

    assert!(!is_state_unknown(ENTITY, &mgr));
    let missions = &mgr.get_entity(ENTITY).unwrap().missions;
    assert_eq!(
        missions.get_mission(MISSION).map(|m| m.status),
        Some(MISSION_COMPLETED)
    );
    assert_eq!(
        missions.get_mission(623).map(|m| m.status),
        Some(MISSION_ACTIVE)
    );
    let sent = drain(&mut rx);
    assert!(
        sent.len() >= 2,
        "the active mission's resend plus the feedback line"
    );
    assert!(is_feedback(sent.last().unwrap(), RESTORED_LINE));
    let row = capture
        .find_message(Level::INFO, "Saved missions restored")
        .expect("the recovery INFO");
    assert!(row.has_field("event", "mission_state_recovered"));

    // A duplicate must not replace the live (since-progressed) record.
    let stale = MissionInstance::new(MISSION, 200, objectives());
    assert!(!recover_state(ENTITY, vec![stale], &tx, &mut mgr).await);
    assert_eq!(
        mgr.get_entity(ENTITY)
            .unwrap()
            .missions
            .get_mission(MISSION)
            .map(|m| m.status),
        Some(MISSION_COMPLETED)
    );

    // Unlocked: an accept is judged by the offer guard again, not refused.
    let _ = drain(&mut rx);
    assert!(accept_mission(ENTITY, 999, 200, objectives(), &tx, &mut mgr).await);
}
