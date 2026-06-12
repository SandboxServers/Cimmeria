//! Mission operations for the CellService.
//!
//! Handles mission accept, abandon, complete, and objective updates.
//! Sends wire-format mission state to the client.
//!
//! Reference: `python/cell/MissionManager.py`

use tokio::sync::mpsc;

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, MISSION_FAILED,
    STATUS_ACTIVE, STATUS_COMPLETED,
};

use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// ── Method indices for mission client methods ────────────────────────────────
// Missionary interface: flat indices 80-84

/// onMissionUpdate(INT32 missionId, INT8 status, INT32 giverName)
const ON_MISSION_UPDATE: u16 = 80;
/// onStepUpdate(INT32 stepId, INT8 status)
const ON_STEP_UPDATE: u16 = 81;
/// onObjectiveUpdate(INT32 objectiveId, INT8 status, INT8 hidden, INT8 optional)
const ON_OBJECTIVE_UPDATE: u16 = 82;

/// Send all active mission state to the client (called during mapLoaded).
///
/// Reference: `python/cell/MissionManager.py:559-574 resend()`
pub async fn resend_missions(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let entity = match space_mgr.get_entity(entity_id) {
        Some(e) => e,
        None => return,
    };

    let messages = entity.missions.serialize_resend();
    for (method_index, args) in messages {
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            })
            .await;
    }
}

/// Advance a mission to a new step: complete old objectives, set new step, load new objectives.
#[tracing::instrument(
    name = "mission.advance_step",
    level = "info",
    skip_all,
    fields(entity_id, mission_id, new_step_id)
)]
pub async fn advance_step(
    entity_id: u32,
    mission_id: i32,
    new_step_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Load new step objectives from the cache before borrowing entity mutably
    let new_objectives: Vec<MissionObjective> = space_mgr
        .get_step_objectives(new_step_id)
        .into_iter()
        .map(|o| MissionObjective {
            objective_id: o.objective_id,
            status: STATUS_ACTIVE,
            hidden: o.is_hidden,
            optional: o.is_optional,
        })
        .collect();

    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => return,
    };

    let mission = match entity.missions.get_mission_mut(mission_id) {
        Some(m) => m,
        None => {
            tracing::warn!(
                entity_id,
                mission_id,
                new_step_id,
                "advance_step: mission not found"
            );
            return;
        }
    };

    // Complete all active objectives in the current step
    let old_objective_ids: Vec<i32> = mission
        .active_objectives
        .iter()
        .filter(|o| o.status != STATUS_COMPLETED)
        .map(|o| o.objective_id)
        .collect();
    for oid in &old_objective_ids {
        mission.complete_objective(*oid);
    }

    let old_step_id = mission.current_step_id;

    // Complete the old step
    if let Some(sid) = old_step_id {
        mission.completed_steps.push(sid);
    }

    // Set the new step
    mission.current_step_id = Some(new_step_id);
    mission.active_objectives = new_objectives.clone();

    tracing::info!(
        entity_id,
        mission_id,
        ?old_step_id,
        new_step_id,
        new_objectives = new_objectives.len(),
        "Mission step advanced"
    );

    // Send onStepUpdate(old_step_id, COMPLETED)
    if let Some(sid) = old_step_id {
        let mut args = Vec::with_capacity(5);
        args.extend_from_slice(&sid.to_le_bytes());
        args.push(STATUS_COMPLETED as u8);
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_STEP_UPDATE,
                args,
            })
            .await;
    }

    // Send onStepUpdate(new_step_id, ACTIVE)
    let mut args = Vec::with_capacity(5);
    args.extend_from_slice(&new_step_id.to_le_bytes());
    args.push(STATUS_ACTIVE as u8);
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_STEP_UPDATE,
            args,
        })
        .await;

    // Send onObjectiveUpdate for each new objective
    for obj in &new_objectives {
        let mut args = Vec::with_capacity(7);
        args.extend_from_slice(&obj.objective_id.to_le_bytes());
        args.push(STATUS_ACTIVE as u8);
        args.push(if obj.hidden { 1 } else { 0 });
        args.push(if obj.optional { 1 } else { 0 });
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_OBJECTIVE_UPDATE,
                args,
            })
            .await;
    }
}

/// Accept a mission: create a MissionInstance and send initial state to client.
///
/// Returns `true` when the mission was actually (re-)accepted. Returns
/// `false` when the offer guard refused — the caller must NOT persist a
/// `MissionUpdate` or fire `mission_accepted` in that case, otherwise a
/// refused accept still flips the DB row back to active (#411).
///
/// Offer guard (port of Python `MissionManager.canOffer`,
/// `deprecated/python/cell/MissionManager.py:119-136`):
/// - already ACTIVE → refuse (re-accept would reset progress to step 1 —
///   the chain-1051/1053 "briefing loop" bug class)
/// - FAILED and `!can_repeat_on_fail` → refuse
/// - COMPLETED and `repeats > num_repeats` → refuse
///
/// Chains are expected to gate their `accept_mission` actions on
/// `mission_status eq not_active`; this guard is the server-authoritative
/// backstop so a mis-gated chain (or an event fired before mission state
/// is hydrated) can't resurrect a completed mission as active on relog.
#[tracing::instrument(
    name = "mission.accept",
    level = "info",
    skip_all,
    fields(entity_id, mission_id, step_id, objectives_len = objectives.len(), player_id = tracing::field::Empty),
)]
pub async fn accept_mission(
    entity_id: u32,
    mission_id: i32,
    step_id: i32,
    objectives: Vec<MissionObjective>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // Prior-instance snapshot + def lookup happen against the immutable
    // borrow so the guard can run before any mutation.
    let prior = match space_mgr.get_entity(entity_id) {
        Some(e) => {
            // Backfill the DB player_id for session correlation in SigNoz.
            if let Some(pid) = e.player_id {
                tracing::Span::current().record("player_id", pid);
            }
            e.missions
                .get_mission(mission_id)
                .map(|m| (m.status, m.repeats))
        }
        None => {
            tracing::warn!(
                entity_id,
                mission_id,
                "accept_mission: entity not found — refusing (nothing to mutate, \
                 and persisting an accept for an unknown entity would corrupt \
                 the saved row)"
            );
            return false;
        }
    };
    let def = space_mgr.mission_defs.get(&mission_id);

    // Offer guard — see doc comment. Def-missing falls back fail-closed
    // (treat as non-repeatable) so an unseeded mission can't loop.
    if let Some((status, repeats)) = prior {
        let refusal = match status {
            MISSION_ACTIVE => Some("already active"),
            MISSION_FAILED if !def.is_some_and(|d| d.can_repeat_on_fail) => {
                Some("failed and not repeatable-on-fail")
            }
            MISSION_COMPLETED if repeats > def.map_or(0, |d| d.num_repeats) => {
                Some("completed at repeat cap")
            }
            _ => None,
        };
        if let Some(reason) = refusal {
            tracing::warn!(
                entity_id,
                mission_id,
                status,
                repeats,
                num_repeats = def.map_or(0, |d| d.num_repeats),
                reason,
                "accept_mission: offer refused — mission state preserved"
            );
            return false;
        }
    }

    // Carry forward `repeats` from any prior instance of this mission --
    // `add_mission` overwrites by mission_id, and a fresh `MissionInstance::new`
    // initializes `repeats = 0`. Without this read-then-set, re-accepting a
    // previously-completed repeatable mission would silently reset the counter
    // (which then UPSERTs through `MissionUpdate`), defeating `numRepeats`
    // gating. Spotted by Copilot on PR #125.
    let prior_repeats = prior.map_or(0, |(_, repeats)| repeats);
    // Propagate the mission def's `is_hidden` into the instance. Without
    // this, hidden sub-missions (e.g. mission 682-686 Hallway0N Controllers,
    // is_hidden=true in the seed) appear in the player's mission log because
    // `MissionInstance::new` defaults `is_hidden=false` and `active_missions()`
    // filters on the per-instance flag. Cellblock-only missions affected;
    // visible-by-design missions (681 Mess Hall, 687 Aftermath, etc.) are
    // is_hidden=false in the seed so this is a no-op for them.
    let is_hidden = def.is_some_and(|m| m.is_hidden);
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => return false,
    };
    let mut mission = MissionInstance::new(mission_id, step_id, objectives.clone());
    mission.repeats = prior_repeats;
    mission.is_hidden = is_hidden;
    entity.missions.add_mission(mission);

    tracing::info!(
        entity_id,
        mission_id,
        step_id,
        prior_repeats,
        "Mission accepted"
    );

    // Send onMissionUpdate
    let mut args = Vec::with_capacity(9);
    args.extend_from_slice(&mission_id.to_le_bytes());
    args.push(STATUS_ACTIVE as u8);
    args.extend_from_slice(&0i32.to_le_bytes());
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_MISSION_UPDATE,
            args,
        })
        .await;

    // Send onStepUpdate
    let mut args = Vec::with_capacity(5);
    args.extend_from_slice(&step_id.to_le_bytes());
    args.push(STATUS_ACTIVE as u8);
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_STEP_UPDATE,
            args,
        })
        .await;

    // Send onObjectiveUpdate per objective
    for obj in &objectives {
        let mut args = Vec::with_capacity(7);
        args.extend_from_slice(&obj.objective_id.to_le_bytes());
        args.push(obj.status as u8);
        args.push(if obj.hidden { 1 } else { 0 });
        args.push(if obj.optional { 1 } else { 0 });
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_OBJECTIVE_UPDATE,
                args,
            })
            .await;
    }

    true
}

/// Abandon a mission: remove it and send removal to client.
#[tracing::instrument(
    name = "mission.abandon",
    level = "info",
    skip_all,
    fields(entity_id, mission_id, player_id = tracing::field::Empty)
)]
pub async fn abandon_mission(
    entity_id: u32,
    mission_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => return,
    };
    if let Some(pid) = entity.player_id {
        tracing::Span::current().record("player_id", pid);
    }

    if entity.missions.remove_mission(mission_id).is_some() {
        tracing::info!(entity_id, mission_id, "Mission abandoned");

        // Send onMissionUpdate with status=completed (removes from client log)
        let mut args = Vec::with_capacity(9);
        args.extend_from_slice(&mission_id.to_le_bytes());
        args.push(STATUS_COMPLETED as u8);
        args.extend_from_slice(&0i32.to_le_bytes());
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_MISSION_UPDATE,
                args,
            })
            .await;
    }
}

/// Complete a mission objective and check if the mission advances.
#[tracing::instrument(
    name = "mission.complete_objective",
    level = "info",
    skip_all,
    fields(entity_id, mission_id, objective_id)
)]
pub async fn complete_objective(
    entity_id: u32,
    mission_id: i32,
    objective_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => return,
    };

    let mission = match entity.missions.get_mission_mut(mission_id) {
        Some(m) => m,
        None => return,
    };

    if !mission.complete_objective(objective_id) {
        return;
    }

    tracing::debug!(entity_id, mission_id, objective_id, "Objective completed");

    // Send onObjectiveUpdate with completed status
    let mut args = Vec::with_capacity(7);
    args.extend_from_slice(&objective_id.to_le_bytes());
    args.push(STATUS_COMPLETED as u8);
    args.push(0); // hidden
    args.push(0); // optional
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_OBJECTIVE_UPDATE,
            args,
        })
        .await;

    // Check if all objectives are completed → advance mission
    let all_required_complete = mission
        .active_objectives
        .iter()
        .filter(|o| !o.optional)
        .all(|o| o.status == STATUS_COMPLETED);

    if all_required_complete {
        mission.complete();

        // Send onStepUpdate completed
        if let Some(&step_id) = mission.completed_steps.last() {
            let mut args = Vec::with_capacity(5);
            args.extend_from_slice(&step_id.to_le_bytes());
            args.push(STATUS_COMPLETED as u8);
            let _ = tx
                .send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: ON_STEP_UPDATE,
                    args,
                })
                .await;
        }

        // Send onMissionUpdate completed
        let mut args = Vec::with_capacity(9);
        args.extend_from_slice(&mission_id.to_le_bytes());
        args.push(MISSION_ACTIVE as u8); // Status sent as "completed" removal
        args.extend_from_slice(&0i32.to_le_bytes());
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_MISSION_UPDATE,
                args,
            })
            .await;

        tracing::info!(entity_id, mission_id, "Mission completed!");
    }
}

/// Complete a mission directly (all objectives + step + mission update).
///
/// Used by the content engine when a chain action completes a mission
/// without stepping through individual objectives.
#[tracing::instrument(
    name = "mission.complete_direct",
    level = "info",
    skip_all,
    fields(entity_id, mission_id, player_id = tracing::field::Empty)
)]
pub async fn complete_mission_direct(
    entity_id: u32,
    mission_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => return,
    };
    if let Some(pid) = entity.player_id {
        tracing::Span::current().record("player_id", pid);
    }

    let mission = match entity.missions.get_mission_mut(mission_id) {
        Some(m) => m,
        None => {
            tracing::warn!(
                entity_id,
                mission_id,
                "complete_mission_direct: mission not found"
            );
            return;
        }
    };

    // Complete all objectives
    let objective_ids: Vec<i32> = mission
        .active_objectives
        .iter()
        .map(|o| o.objective_id)
        .collect();
    for oid in &objective_ids {
        mission.complete_objective(*oid);
    }
    mission.complete();

    let step_id = mission.completed_steps.last().copied();

    tracing::info!(entity_id, mission_id, "Mission completed directly");

    // Send objective updates
    for oid in &objective_ids {
        let mut args = Vec::with_capacity(7);
        args.extend_from_slice(&oid.to_le_bytes());
        args.push(STATUS_COMPLETED as u8);
        args.push(0); // hidden
        args.push(0); // optional
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_OBJECTIVE_UPDATE,
                args,
            })
            .await;
    }

    // Send step completed
    if let Some(sid) = step_id {
        let mut args = Vec::with_capacity(5);
        args.extend_from_slice(&sid.to_le_bytes());
        args.push(STATUS_COMPLETED as u8);
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_STEP_UPDATE,
                args,
            })
            .await;
    }

    // Send mission completed
    let mut args = Vec::with_capacity(9);
    args.extend_from_slice(&mission_id.to_le_bytes());
    args.push(STATUS_COMPLETED as u8);
    args.extend_from_slice(&0i32.to_le_bytes());
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_MISSION_UPDATE,
            args,
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
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
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
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

    fn make_test_mgr() -> super::super::space_manager::SpaceManager {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        mgr
    }

    fn insert_def(
        mgr: &mut super::super::space_manager::SpaceManager,
        mission_id: i32,
        num_repeats: i32,
        can_repeat_on_fail: bool,
    ) {
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
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
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

    #[tokio::test]
    async fn complete_objective_completes_mission() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

        let (tx, mut rx) = mpsc::channel(16);
        accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;
        while rx.try_recv().is_ok() {}

        complete_objective(1, 100, 300, &tx, &mut mgr).await;

        // Should get: onObjectiveUpdate(completed) + onStepUpdate(completed) + onMissionUpdate
        let mut msgs = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            msgs.push(msg);
        }
        assert_eq!(msgs.len(), 3);

        // First: objective completed
        match &msgs[0] {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } => {
                assert_eq!(*method_index, 82); // onObjectiveUpdate
                assert_eq!(args[4], STATUS_COMPLETED as u8);
            }
            _ => panic!("unexpected"),
        }
    }

    #[tokio::test]
    async fn resend_sends_active_missions() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

        let (tx, mut rx) = mpsc::channel(16);
        accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;
        while rx.try_recv().is_ok() {}

        resend_missions(1, &tx, &mgr).await;

        let mut msgs = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            msgs.push(msg);
        }
        // 1 mission × (1 update + 1 step + 1 objective) = 3
        assert_eq!(msgs.len(), 3);
    }
}
