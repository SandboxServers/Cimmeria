//! Unit coverage for [`super::lifecycle`]: accept (offer guard, repeat
//! carry-forward) and abandon. Split out of `lifecycle.rs` to keep it under
//! the file-size cap.

use tokio::sync::mpsc;

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, STATUS_ACTIVE, STATUS_COMPLETED,
};

use super::lifecycle::{abandon_mission, accept_mission};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

fn make_objectives() -> Vec<MissionObjective> {
    vec![MissionObjective {
        objective_id: 300,
        status: STATUS_ACTIVE,
        hidden: false,
        optional: false,
    }]
}

#[tokio::test]
async fn accept_sends_three_messages() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

    let (tx, mut rx) = mpsc::channel(16);
    accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;

    // Should get: onMissionUpdate + onStepUpdate + onObjectiveUpdate = 3
    let mut msgs = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        msgs.push(msg);
    }
    assert_eq!(msgs.len(), 3);

    // Verify method indices
    let indices: Vec<u16> = msgs
        .iter()
        .map(|m| match m {
            CellToBaseMsg::EntityMethodCall { method_index, .. } => *method_index,
            _ => panic!("unexpected message"),
        })
        .collect();
    assert_eq!(indices, vec![80, 81, 82]);
}

#[tokio::test]
async fn re_accept_preserves_repeat_counter() {
    // Regression for the Copilot finding on PR #125: `accept_mission` was
    // overwriting the existing MissionInstance with a fresh one (repeats=0),
    // so re-accepting a previously-completed repeatable mission would
    // silently reset the counter -- which then UPSERTed back to 0 on the
    // DB row, defeating the entire #118 fix for repeatable missions.
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    // Declare the mission repeatable (num_repeats=1 → one completion
    // leaves it re-offerable, python parity `repeats > numRepeats`).
    // Without a def the offer guard fails closed and this re-accept
    // would be refused.
    mgr.mission_defs.insert(
        100,
        crate::cell::spawner::MissionDefEntry {
            step_id: 200,
            objectives: vec![],
            is_hidden: false,
            num_repeats: 1,
            can_repeat_on_fail: true,
        },
    );

    // Seed prior state: completed once, repeats == 1.
    {
        let entity = mgr.get_entity_mut(1).unwrap();
        let mut prior = MissionInstance::new(100, 200, vec![]);
        prior.complete(); // sets repeats = 1
        entity.missions.add_mission(prior);
    }

    let (tx, _rx) = mpsc::channel(16);
    accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;

    let m = mgr
        .get_entity(1)
        .unwrap()
        .missions
        .get_mission(100)
        .unwrap();
    assert_eq!(
        m.repeats, 1,
        "re-accept must carry forward prior repeats count"
    );
}

fn make_test_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr
}

fn insert_def(mgr: &mut SpaceManager, mission_id: i32, num_repeats: i32, can_repeat_on_fail: bool) {
    mgr.mission_defs.insert(
        mission_id,
        crate::cell::spawner::MissionDefEntry {
            step_id: 200,
            objectives: vec![],
            is_hidden: false,
            num_repeats,
            can_repeat_on_fail,
        },
    );
}

/// **#411 regression guard.** Re-accepting a COMPLETED mission whose
/// repeat counter is past the def's `num_repeats` cap must be a
/// complete no-op: returns false, mutates nothing, sends nothing.
/// Pre-fix, the accept overwrote the completed instance with a fresh
/// ACTIVE one and the executor persisted status=1 over the saved row —
/// the "completed missions reappear as active after relog" bug.
#[tokio::test]
async fn re_accept_of_completed_mission_at_repeat_cap_is_refused() {
    use cimmeria_entity::missions::MISSION_COMPLETED;

    let mut mgr = make_test_mgr();
    insert_def(&mut mgr, 100, 1, true);
    // Completed twice → repeats == 2 > num_repeats == 1 (python parity:
    // canOffer refuses when `repeats > numRepeats`).
    {
        let entity = mgr.get_entity_mut(1).unwrap();
        let mut prior = MissionInstance::new(100, 200, vec![]);
        prior.complete();
        prior.repeats = 2;
        entity.missions.add_mission(prior);
    }

    let (tx, mut rx) = mpsc::channel(16);
    let accepted = accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;

    assert!(
        !accepted,
        "offer guard must refuse a capped completed mission"
    );
    assert!(
        rx.try_recv().is_err(),
        "a refused accept must not emit any client messages"
    );
    let m = mgr
        .get_entity(1)
        .unwrap()
        .missions
        .get_mission(100)
        .unwrap();
    assert_eq!(m.status, MISSION_COMPLETED, "status must be preserved");
    assert_eq!(m.repeats, 2, "repeat counter must be preserved");
}

/// Python-parity companion: a completed mission still UNDER the cap
/// (`repeats <= num_repeats`) stays re-offerable. Keeps the guard from
/// over-blocking legitimately repeatable missions.
#[tokio::test]
async fn re_accept_of_completed_mission_below_repeat_cap_is_allowed() {
    let mut mgr = make_test_mgr();
    insert_def(&mut mgr, 100, 1, true);
    {
        let entity = mgr.get_entity_mut(1).unwrap();
        let mut prior = MissionInstance::new(100, 200, vec![]);
        prior.complete(); // repeats = 1, not > num_repeats = 1
        entity.missions.add_mission(prior);
    }

    let (tx, _rx) = mpsc::channel(16);
    let accepted = accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;

    assert!(accepted, "below the cap the mission must be re-offerable");
    let m = mgr
        .get_entity(1)
        .unwrap()
        .missions
        .get_mission(100)
        .unwrap();
    assert_eq!(m.status, MISSION_ACTIVE);
    assert_eq!(m.repeats, 1, "repeat counter carries forward on re-accept");
}

/// Re-accepting a mission that is already ACTIVE must be refused —
/// otherwise a re-fired grant chain resets in-flight progress back to
/// the def's first step (the chain-1051/1053 "briefing loop" bug class,
/// previously only guarded by chain-side conditions).
#[tokio::test]
async fn re_accept_of_active_mission_is_refused_and_preserves_progress() {
    let mut mgr = make_test_mgr();
    insert_def(&mut mgr, 100, 1, true);

    let (tx, mut rx) = mpsc::channel(16);
    let accepted = accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;
    assert!(accepted, "fresh accept must succeed");
    while rx.try_recv().is_ok() {}

    // Simulate progress past the first step.
    {
        let entity = mgr.get_entity_mut(1).unwrap();
        let m = entity.missions.get_mission_mut(100).unwrap();
        m.current_step_id = Some(999);
    }

    let accepted = accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;
    assert!(!accepted, "an active mission must not be re-offered");
    assert!(rx.try_recv().is_err(), "refusal must not emit messages");
    let m = mgr
        .get_entity(1)
        .unwrap()
        .missions
        .get_mission(100)
        .unwrap();
    assert_eq!(
        m.current_step_id,
        Some(999),
        "in-flight progress must survive the refused re-accept"
    );
}

/// FAILED missions follow `can_repeat_on_fail` (python parity): false →
/// permanently refused; true → re-offerable.
#[tokio::test]
async fn re_accept_of_failed_mission_respects_can_repeat_on_fail() {
    use cimmeria_entity::missions::MISSION_FAILED;

    let mut mgr = make_test_mgr();
    insert_def(&mut mgr, 100, 5, false);
    {
        let entity = mgr.get_entity_mut(1).unwrap();
        let mut prior = MissionInstance::new(100, 200, vec![]);
        prior.fail();
        entity.missions.add_mission(prior);
    }

    let (tx, _rx) = mpsc::channel(16);
    let accepted = accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;
    assert!(
        !accepted,
        "failed + can_repeat_on_fail=false must refuse the re-offer"
    );
    let m = mgr
        .get_entity(1)
        .unwrap()
        .missions
        .get_mission(100)
        .unwrap();
    assert_eq!(m.status, MISSION_FAILED);

    // Same state with a repeat-on-fail def → allowed.
    insert_def(&mut mgr, 100, 5, true);
    let accepted = accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;
    assert!(
        accepted,
        "failed + can_repeat_on_fail=true must allow the re-offer"
    );
}

/// Entity-missing accepts must return false so callers don't persist a
/// `MissionUpdate` for a player whose mission state was never loaded —
/// pre-fix, that persisted status=1 over the saved row and resurrected
/// completed missions on the next relog (#411).
#[tokio::test]
async fn accept_with_missing_entity_is_refused() {
    let mut mgr = make_test_mgr();
    insert_def(&mut mgr, 100, 1, true);

    let (tx, mut rx) = mpsc::channel(16);
    let accepted = accept_mission(999, 100, 200, make_objectives(), &tx, &mut mgr).await;

    assert!(!accepted, "unknown entity must refuse the accept");
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn abandon_removes_mission() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

    let (tx, mut rx) = mpsc::channel(16);
    accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;
    // Drain accept messages
    while rx.try_recv().is_ok() {}

    abandon_mission(1, 100, &tx, &mut mgr).await;

    // Should get onMissionUpdate with completed status
    let msg = rx.try_recv().unwrap();
    match msg {
        CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } => {
            assert_eq!(method_index, 80);
            assert_eq!(args[4], STATUS_COMPLETED as u8);
        }
        _ => panic!("unexpected"),
    }

    // Mission should be gone
    assert_eq!(mgr.get_entity(1).unwrap().missions.count(), 0);
}
/// The first captured row whose `event` field is `name`.
fn event_row(
    capture: &crate::test_support::LogCaptureGuard,
    name: &str,
) -> crate::test_support::Captured {
    capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", name))
        .unwrap_or_else(|| panic!("no `{name}` row; captured: {:#?}", capture.all()))
}

/// TG-MIS-01: accept and abandon rows carry `event`, the player's numeric
/// `player_id` (not `"Some(..)"`) and the world name, so SigNoz can filter
/// a player's mission lifecycle without joining on the entity id.
#[tokio::test]
async fn accept_and_abandon_rows_carry_event_and_player_identity() {
    use crate::test_support::LogCapture;

    let capture = LogCapture::install();
    let mut mgr = make_test_mgr();
    mgr.get_entity_mut(1).unwrap().player_id = Some(7101);
    let (tx, _rx) = mpsc::channel(16);

    assert!(accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await);
    let accepted = event_row(&capture, "mission_accepted");
    assert!(accepted.has_field("player_id", "7101"), "{accepted:#?}");
    assert!(accepted.has_field("world", "Agnos"), "{accepted:#?}");

    // A second accept of the now-active mission is the refusal row.
    assert!(!accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await);
    let refused = event_row(&capture, "mission_accept_refused");
    assert!(
        refused.has_field("reason", "already_active"),
        "{refused:#?}"
    );
    assert!(refused.has_field("player_id", "7101"), "{refused:#?}");

    assert!(abandon_mission(1, 100, &tx, &mut mgr).await);
    let abandoned = event_row(&capture, "mission_abandoned");
    assert!(abandoned.has_field("player_id", "7101"), "{abandoned:#?}");
    assert!(abandoned.has_field("world", "Agnos"), "{abandoned:#?}");
}

/// TG-MIS-01: an abandon for an entity the cell no longer holds used to
/// return without a row. It now WARNs with `reason=entity_missing`.
#[tokio::test]
async fn abandon_with_missing_entity_warns_entity_missing() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_test_mgr();
    let (tx, _rx) = mpsc::channel(16);

    assert!(!abandon_mission(999, 100, &tx, &mut mgr).await);
    let row = capture
        .find_event(
            Level::WARN,
            "abandon_mission: entity not found",
            "entity_missing",
        )
        .unwrap_or_else(|| panic!("no entity_missing WARN; captured: {:#?}", capture.all()));
    assert!(
        row.has_field("event", "mission_abandon_refused"),
        "{row:#?}"
    );
}
