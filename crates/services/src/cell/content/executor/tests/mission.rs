//! Mission executor coverage — accept / complete, plus the
//! completion-event gate that must fire only on a real ACTIVE→COMPLETED
//! transition.

use super::*;

/// `Action::CompleteMission` against a mission already in MISSION_FAILED
/// must NOT fire `OnMissionCompleted` chains. The executor's gate looks
/// at the prior status — only a real active→completed transition fires
/// the dispatcher. Without this guard, "failing" a mission then later
/// "completing" it (e.g., from a manual seed action or chain authoring
/// mistake) would re-fire downstream chains like 1105's auto-accept of
/// the next mission, producing a spurious mission grant the player
/// didn't earn.
///
/// Detection shape: register a `OnMissionCompleted` chain whose only
/// action is `IncrementCounter`. The counter mutation is observable on
/// the entity without needing to wire up the full base-side mission
/// dispatch. The gate's positive (ACTIVE→COMPLETED fires) path is
/// already covered by `fire_mission_completed_runs_matched_chain_actions`
/// in event_dispatch::mission::tests.
#[tokio::test]
async fn complete_mission_action_against_failed_mission_does_not_fire_completion_event() {
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;
    use cimmeria_entity::missions::{MissionInstance, MISSION_FAILED};

    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(42);
        // Seed mission 687 in MISSION_FAILED state. Pre-fix the gate
        // read prior_status.is_some() and would treat this as
        // "transitioned" since it's not None.
        let mut m = MissionInstance::new(687, 2113, vec![]);
        m.fail();
        assert_eq!(m.status, MISSION_FAILED);
        e.missions.add_mission(m);
    }
    mgr.connect_entity(1);

    // Chain: mission_completed 687 → increment_counter (detectable
    // mutation on the entity we control). Same trigger shape as
    // production chain 1105's 687→688 auto-accept.
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 1105,
        name: "test_complete_chain".to_string(),
        enabled: true,
        trigger: Trigger::OnMissionCompleted { mission_id: 687 },
        conditions: Vec::new(),
        actions: vec![Action::IncrementCounter {
            counter_name: "completion_fired".to_string(),
            amount: 1,
        }],
        priority: 0,
    });

    let (tx, _rx) = mpsc::channel(64);
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(9000, Action::CompleteMission { mission_id: 687 })],
    };
    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    // The counter must NOT have been touched — fire_mission_completed
    // should have been skipped because the prior status was MISSION_FAILED,
    // not MISSION_ACTIVE.
    let entity = mgr.get_entity(1).expect("entity must still exist");
    assert!(
        !entity.counters.contains_key("completion_fired"),
        "fire_mission_completed must NOT fire on FAILED→COMPLETED \
         transition; counter was incremented, meaning chain 1105 \
         spuriously ran",
    );
}

/// `Action::AcceptMission` happy path: a known mission def with the player
/// having no prior instance inserts a fresh ACTIVE mission. Also exercises
/// the accept-side Discord `mission_accepted` seam (no-op without a runtime,
/// but the name-capture path runs). The complement of the offer-refused
/// branch — without this, the accepted path had no executor-level coverage.
#[tokio::test]
async fn accept_mission_action_inserts_active_instance() {
    use crate::cell::spawner::{MissionDefEntry, MissionObjectiveDef};
    use cimmeria_entity::missions::MISSION_ACTIVE;

    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(42);
        // Cached name the accept seam reads when attributing the emit.
        e.character_name = Some("Teal'c".into());
    }
    mgr.connect_entity(1);
    mgr.mission_defs.insert(
        700,
        MissionDefEntry {
            step_id: 2100,
            objectives: vec![MissionObjectiveDef {
                objective_id: 3000,
                is_hidden: false,
                is_optional: false,
            }],
            is_hidden: false,
            num_repeats: 0,
            can_repeat_on_fail: false,
        },
    );

    let (tx, _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(9000, Action::AcceptMission { mission_id: 700 })],
    };
    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let m = mgr
        .get_entity(1)
        .unwrap()
        .missions
        .get_mission(700)
        .expect("AcceptMission must insert a mission instance");
    assert_eq!(
        m.status, MISSION_ACTIVE,
        "a freshly accepted mission is ACTIVE"
    );
}

/// `Action::CompleteMission` happy path: an ACTIVE mission transitions to
/// COMPLETED. This is the positive complement of
/// `complete_mission_action_against_failed_mission_does_not_fire_completion_event`
/// and exercises the complete-side `mission_completed` seam (the
/// `transitioned_from_active` branch that the failure test deliberately
/// avoids).
#[tokio::test]
async fn complete_mission_action_against_active_mission_marks_completed() {
    use cimmeria_entity::missions::{MissionInstance, MISSION_ACTIVE, MISSION_COMPLETED};

    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(42);
        e.character_name = Some("Teal'c".into());
        let m = MissionInstance::new(700, 2100, vec![]);
        assert_eq!(m.status, MISSION_ACTIVE, "new mission starts ACTIVE");
        e.missions.add_mission(m);
    }
    mgr.connect_entity(1);

    let (tx, _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(9000, Action::CompleteMission { mission_id: 700 })],
    };
    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let m = mgr
        .get_entity(1)
        .unwrap()
        .missions
        .get_mission(700)
        .expect("mission instance must still exist after completion");
    assert_eq!(
        m.status, MISSION_COMPLETED,
        "an ACTIVE mission must transition to COMPLETED"
    );
}
