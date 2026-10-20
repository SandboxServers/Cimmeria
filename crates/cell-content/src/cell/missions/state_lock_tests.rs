//! TG-DB-12 cell-side lock: refusal, the whole-chain gate, the recovery
//! unlock, the pre-load hold and the refusal throttle.
//!
//! The throttle is keyed by entity id in a process-wide table, so every
//! test here uses its own entity id.

use std::time::Instant;

use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_content_engine::actions::Action;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use super::state_lock::{
    admit_refusal_at, gate_mission_chains, is_state_unknown, mark_load_pending, mark_state_unknown,
    recover_state, refuse_if_state_unknown, REFUSAL_WINDOW, RESTORED_LINE, UNAVAILABLE_LINE,
};
use super::{accept_mission, send_mission_update};
use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, LogCapture};

const MISSION: i32 = 622;

fn mgr_with(entity: u32) -> SpaceManager {
    let mut mgr = make_space_manager();
    mgr.create_entity(entity, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(entity).unwrap().player_id = Some(77);
    mgr
}

fn locked_mgr(entity: u32) -> SpaceManager {
    let mut mgr = mgr_with(entity);
    mark_state_unknown(entity, &mut mgr);
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

fn refusal_warns(capture: &crate::test_support::LogCaptureGuard, entity: u32) -> usize {
    let id = entity.to_string();
    capture
        .all()
        .iter()
        .filter(|c| {
            c.level == Level::WARN
                && c.has_field("event", "mission_state_unknown")
                && c.has_field("entity_id", &id)
        })
        .count()
}

/// A locked tracker refuses the accept: nothing created, no client frame,
/// no save; one feedback line and the WARN. Without the gate the empty
/// tracker lets the accept through (the #411 overwrite).
#[tokio::test]
async fn locked_accept_is_refused_with_feedback_and_warn() {
    const E: u32 = 101;
    let mut mgr = locked_mgr(E);
    let capture = LogCapture::install();
    let (tx, mut rx) = mpsc::channel(16);

    let accepted = accept_mission(E, MISSION, 200, objectives(), &tx, &mut mgr).await;
    send_mission_update(E, 77, MISSION, "accept", &tx, &mgr).await;

    assert!(!accepted);
    assert!(mgr
        .get_entity(E)
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
    assert!(row.has_field("suppressed", "0"));
    assert!(row.fields.contains_key("player_id"));
    assert!(row.fields.contains_key("mission_name") || row.fields.contains_key("mission_id"));
}

/// The chain gate drops a chain that changes a mission whole (its counter
/// bump with it) and keeps a chain that doesn't, delays aligned.
#[tokio::test]
async fn locked_tracker_drops_mission_chains_whole() {
    const E: u32 = 102;
    let mgr = locked_mgr(E);
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
    let (kept, delays) = gate_mission_chains(actions, vec![0, 250, 0], E, &tx, &mgr).await;
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].0, 20);
    assert_eq!(delays, vec![250]);
    let sent = drain(&mut rx);
    assert_eq!(sent.len(), 1);
    assert!(is_feedback(&sent[0], UNAVAILABLE_LINE));
}

#[tokio::test]
async fn pending_hydration_drops_letter_turn_in_chain_whole() {
    const E: u32 = 109;
    let mut mgr = mgr_with(E);
    mark_load_pending(E, &mut mgr);
    let (tx, mut rx) = mpsc::channel(8);
    let actions = vec![
        (
            6501,
            Action::IncrementCounter {
                counter_name: "would_be_success".into(),
                amount: 1,
            },
        ),
        (
            6501,
            Action::CompleteMissionWithItem {
                mission_id: 1360,
                expected_step_id: 4038,
                item_id: 3730,
                quantity: 1,
                dialog_id: 4576,
                dialog_set_id: 120101,
                slot: 10,
            },
        ),
    ];
    let (kept, delays) = gate_mission_chains(actions, vec![0, 0], E, &tx, &mgr).await;
    assert!(kept.is_empty());
    assert!(delays.is_empty());
    let sent = drain(&mut rx);
    assert_eq!(sent.len(), 1);
    assert!(is_feedback(&sent[0], UNAVAILABLE_LINE));
}

/// The recovery unlock: the late missions land, the tracker unlocks, the
/// journal is resent and the player told; a second recovery changes
/// nothing, so progress made after the first is never overwritten.
#[tokio::test]
async fn recovery_restores_unlocks_and_ignores_a_duplicate() {
    const E: u32 = 103;
    let mut mgr = locked_mgr(E);
    let capture = LogCapture::install();
    let (tx, mut rx) = mpsc::channel(32);

    let mut done = MissionInstance::new(MISSION, 200, objectives());
    done.complete();
    let active = MissionInstance::new(623, 201, objectives());
    assert!(recover_state(E, vec![done, active], &tx, &mut mgr).await);

    assert!(!is_state_unknown(E, &mgr));
    let missions = &mgr.get_entity(E).unwrap().missions;
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
    assert!(!recover_state(E, vec![stale], &tx, &mut mgr).await);
    assert_eq!(
        mgr.get_entity(E)
            .unwrap()
            .missions
            .get_mission(MISSION)
            .map(|m| m.status),
        Some(MISSION_COMPLETED)
    );

    // Unlocked: an accept is judged by the offer guard again, not refused.
    let _ = drain(&mut rx);
    assert!(accept_mission(E, 999, 200, objectives(), &tx, &mut mgr).await);
}

/// The pre-load hold: a base-created player refuses mission changes until
/// `InitPlayerState` lifts it, with its own reason.
#[tokio::test]
async fn pending_load_refuses_until_marked_loaded() {
    const E: u32 = 104;
    let mut mgr = mgr_with(E);
    mark_load_pending(E, &mut mgr);
    let capture = LogCapture::install();
    let (tx, _rx) = mpsc::channel(16);

    assert!(!accept_mission(E, MISSION, 200, objectives(), &tx, &mut mgr).await);
    assert!(capture
        .find_event(Level::WARN, "", "saved_missions_pending")
        .is_some());

    mgr.get_entity_mut(E).unwrap().missions.mark_loaded();
    assert!(accept_mission(E, MISSION, 200, objectives(), &tx, &mut mgr).await);
}

/// A burst of refusals sends one feedback line and one WARN per window;
/// the next admitted row carries the count swallowed in between.
#[tokio::test]
async fn refusal_burst_is_throttled_per_entity() {
    const E: u32 = 105;
    let mgr = locked_mgr(E);
    let capture = LogCapture::install();
    let (tx, mut rx) = mpsc::channel(32);

    for _ in 0..5 {
        assert!(refuse_if_state_unknown(E, MISSION, "accept", &tx, &mgr).await);
    }
    let sent = drain(&mut rx);
    assert_eq!(sent.len(), 1, "one feedback line per window: {sent:?}");
    assert_eq!(refusal_warns(&capture, E), 1, "one WARN per window");
    assert_eq!(
        admit_refusal_at(E, Instant::now() + REFUSAL_WINDOW),
        Some(4),
        "the next row carries the four refusals swallowed"
    );
}

/// The throttle is per entity: another player's first refusal is never
/// swallowed by this one's window.
#[tokio::test]
async fn refusal_throttle_is_independent_per_entity() {
    const A: u32 = 106;
    const B: u32 = 107;
    let mut mgr = locked_mgr(A);
    mgr.create_entity(B, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.get_entity_mut(B).unwrap().player_id = Some(78);
    mark_state_unknown(B, &mut mgr);
    let capture = LogCapture::install();
    let (tx, mut rx) = mpsc::channel(32);

    assert!(refuse_if_state_unknown(A, MISSION, "accept", &tx, &mgr).await);
    assert!(refuse_if_state_unknown(A, MISSION, "accept", &tx, &mgr).await);
    assert!(refuse_if_state_unknown(B, MISSION, "accept", &tx, &mgr).await);

    let sent = drain(&mut rx);
    assert_eq!(sent.len(), 2, "one line for each player: {sent:?}");
    assert!(sent.iter().any(
        |m| matches!(m, CellToBaseMsg::EntityMethodCall { entity_id, .. } if *entity_id == B)
    ));
    assert_eq!(refusal_warns(&capture, A), 1);
    assert_eq!(refusal_warns(&capture, B), 1);
}
