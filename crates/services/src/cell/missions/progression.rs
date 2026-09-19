//! Mission progression: advance step, complete objective, complete direct.

use tokio::sync::mpsc;

use cimmeria_entity::missions::{MissionObjective, STATUS_ACTIVE, STATUS_COMPLETED};

use super::{ON_MISSION_UPDATE, ON_OBJECTIVE_UPDATE, ON_STEP_UPDATE};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Advance a mission to a new step: complete old objectives, set new step, load new objectives.
///
/// Returns `true` only when the step was actually activated — i.e. the entity
/// and the mission instance both existed. Callers use that to gate work that
/// must only happen on a real activation: H52's step-activation region replay
/// would otherwise re-fire `enter_region` for a step that never became
/// current.
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
) -> bool {
    // Ordering seam. Region / cover triggers are EDGE events: if the player is
    // already inside when this step activates, the edge has already been spent
    // and the step's chain never sees it (2026-09-18: take-cover fired 1 s
    // before step 2144). Record what is already true at activation so that
    // shape is visible instead of inferred from timestamps.
    //
    // H52 closes the `enter_region` half of that: the callers that own the
    // ChainEngine replay those volumes through
    // `content::event_dispatch::step_activation` once this call returns
    // `true`. `cover_sets` below is still only diagnostic — the cover edge
    // belongs to the Cellblock lane (objective 2484).
    if let Some(e) = space_mgr.get_entity(entity_id) {
        let world = space_mgr
            .get_entity_world_name(entity_id)
            .unwrap_or_default();
        let regions_inside: Vec<&str> = space_mgr
            .regions_for_world(&world)
            .into_iter()
            .filter(|r| {
                crate::cell::playtest_friction::region_contains_xz(
                    &r.points,
                    e.position.x,
                    e.position.z,
                )
            })
            .map(|r| r.tag.as_str())
            .collect();
        let cover_sets = space_mgr
            .cover_detection
            .current_sets(e.entity_id, std::time::Instant::now());
        tracing::debug!(
            target: "mission.step_context",
            entity_id,
            mission_id,
            new_step_id,
            x = e.position.x,
            y = e.position.y,
            z = e.position.z,
            ?regions_inside,
            ?cover_sets,
            crouched = e.state_field & crate::cell::cell_methods::combatant::BSF_CROUCHING != 0,
            in_combat = !e.threatened_mobs.is_empty(),
            "mission step activating -- state already true here will NOT re-fire as an edge trigger"
        );
    }

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
        None => return false,
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
            return false;
        }
    };

    // Complete all active objectives in the current step
    let old_objectives: Vec<MissionObjective> = mission
        .active_objectives
        .iter()
        .filter(|o| o.status != STATUS_COMPLETED)
        .cloned()
        .collect();
    for obj in &old_objectives {
        mission.complete_objective(obj.objective_id);
        // The incoming step transition carries no objective status, so the
        // completion must be reported before the old step disappears. The
        // frame carries the objective's real flags — the reference
        // MissionManager sends them even on completed frames. Reporting the
        // objectives first diverges deliberately from MissionManager.py's
        // step-before-objective order (:851-862), so the checkmarks land
        // while the old step is still the client's current step.
        let mut args = Vec::with_capacity(7);
        args.extend_from_slice(&obj.objective_id.to_le_bytes());
        args.push(STATUS_COMPLETED as u8);
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
    crate::cell::player_journal::note(
        entity_id,
        crate::cell::player_journal::kinds::STEP_ADVANCE,
        format!("mission={mission_id} step={old_step_id:?}->{new_step_id}"),
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

    true
}

/// Complete a mission objective and check if the mission advances.
///
/// Returns `true` only when the objective was actually flipped — i.e. it
/// was on the current step's roster. Callers use that to gate the
/// `MissionUpdate` persist: emitting one for a no-op would write the
/// unchanged state back and make a regression guard unable to tell a
/// working executor arm from a dead one.
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
) -> bool {
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(
                entity_id,
                mission_id,
                objective_id,
                "complete_objective: entity not found"
            );
            return false;
        }
    };

    let mission = match entity.missions.get_mission_mut(mission_id) {
        Some(m) => m,
        None => {
            tracing::warn!(
                entity_id,
                mission_id,
                objective_id,
                "complete_objective: mission not tracked"
            );
            return false;
        }
    };

    if !mission.complete_objective(objective_id) {
        tracing::warn!(
            entity_id,
            mission_id,
            objective_id,
            current_step_id = ?mission.current_step_id,
            "complete_objective: objective not on the current step's roster — no-op"
        );
        return false;
    }

    tracing::debug!(entity_id, mission_id, objective_id, "Objective completed");

    // Send onObjectiveUpdate with completed status. `hidden`/`optional`
    // ride the frame from the objective's own flags, not hardcoded zeroes
    // — the client's journal renders an optional objective differently,
    // and this frame is the only place it learns the flag outside the
    // login resend.
    let (hidden, optional) = mission
        .active_objectives
        .iter()
        .find(|o| o.objective_id == objective_id)
        .map(|o| (o.hidden, o.optional))
        .unwrap_or((false, false));
    let mut args = Vec::with_capacity(7);
    args.extend_from_slice(&objective_id.to_le_bytes());
    args.push(STATUS_COMPLETED as u8);
    args.push(u8::from(hidden));
    args.push(u8::from(optional));
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_OBJECTIVE_UPDATE,
            args,
        })
        .await;

    // Check if all objectives are completed → advance mission
    let required_count = mission
        .active_objectives
        .iter()
        .filter(|o| !o.optional)
        .count();
    let all_required_complete = mission
        .active_objectives
        .iter()
        .filter(|o| !o.optional)
        .all(|o| o.status == STATUS_COMPLETED);

    // Vacuous truth. `.all()` over an empty filter is `true`, so on a step
    // whose objectives are ALL optional the first completion of any one of
    // them ends the mission. 37 seeded steps have that shape. This is not
    // new — `advance_step` has always loaded the real `is_optional` from
    // `resources.mission_objectives` — but H50 makes it reachable on the
    // restore path too, where the flags used to be forced to `false`. Left
    // as-is deliberately (matching the fresh path is the defensible
    // behaviour and changing it is outside this packet), but logged so a
    // surprise completion in UAT is attributable instead of mysterious.
    if all_required_complete && required_count == 0 {
        tracing::warn!(
            entity_id,
            mission_id,
            objective_id,
            current_step_id = ?mission.current_step_id,
            optional_count = mission.active_objectives.len(),
            "mission auto-completed on a step with no required objectives —              `all_required_complete` was vacuously true"
        );
    }

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

        // Send onMissionUpdate completed. The byte is `STATUS_COMPLETED`,
        // matching `complete_mission_direct`'s frame. It used to read
        // `MISSION_ACTIVE` with a comment about "completed removal" — both
        // constants are 1, so the wire was accidentally right while the
        // source lied about which enum it meant.
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

        tracing::info!(entity_id, mission_id, "Mission completed!");
        crate::cell::player_journal::note(
            entity_id,
            crate::cell::player_journal::kinds::MISSION_COMPLETE,
            format!("mission={mission_id}"),
        );
    }

    true
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

    // Objectives still open here were never completed through play -- the
    // chain is about to force them. Report before the force-complete hides it.
    let never_completed: Vec<(i32, bool)> = mission
        .active_objectives
        .iter()
        .filter(|o| o.status == STATUS_ACTIVE)
        .map(|o| (o.objective_id, o.optional))
        .collect();
    crate::cell::playtest_friction::objectives_never_completed(
        entity_id,
        mission_id,
        &never_completed,
    );

    // Complete all objectives. The flags ride along so the wire frames
    // below can report them (the client renders optional objectives
    // differently); they are captured before the mutation because
    // `complete_objective` borrows the instance mutably.
    let objectives: Vec<(i32, bool, bool)> = mission
        .active_objectives
        .iter()
        .map(|o| (o.objective_id, o.hidden, o.optional))
        .collect();
    for (oid, _, _) in &objectives {
        mission.complete_objective(*oid);
    }
    mission.complete();

    let step_id = mission.completed_steps.last().copied();

    tracing::info!(entity_id, mission_id, "Mission completed directly");
    crate::cell::player_journal::note(
        entity_id,
        crate::cell::player_journal::kinds::MISSION_COMPLETE,
        format!("mission={mission_id} direct"),
    );

    // Send objective updates
    for (oid, hidden, optional) in &objectives {
        let mut args = Vec::with_capacity(7);
        args.extend_from_slice(&oid.to_le_bytes());
        args.push(STATUS_COMPLETED as u8);
        args.push(u8::from(*hidden));
        args.push(u8::from(*optional));
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
    use crate::cell::missions::lifecycle::accept_mission;

    fn make_objectives() -> Vec<MissionObjective> {
        vec![MissionObjective {
            objective_id: 300,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }]
    }

    #[tokio::test]
    async fn complete_objective_completes_mission() {
        let mut mgr = SpaceManager::new(1);
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

    /// A step transition must report every old-step objective it silently
    /// completes as a completed-status onObjectiveUpdate before the step
    /// disappears — otherwise the last objective of a multi-objective AND-gate
    /// resolved via AdvanceStep never reaches the client as completed.
    #[tokio::test]
    async fn advance_step_reports_completed_old_step_objectives() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

        let (tx, mut rx) = mpsc::channel(16);
        accept_mission(
            1,
            100,
            200,
            vec![
                // Hidden + optional: the completion frame must preserve these
                // flags, not collapse them to the completed path's 0/0.
                MissionObjective {
                    objective_id: 300,
                    status: STATUS_ACTIVE,
                    hidden: true,
                    optional: true,
                },
                MissionObjective {
                    objective_id: 301,
                    status: STATUS_ACTIVE,
                    hidden: false,
                    optional: false,
                },
            ],
            &tx,
            &mut mgr,
        )
        .await;
        // Drain accept's initial messages: mission + step + 2 objectives.
        while rx.try_recv().is_ok() {}
        assert!(
            rx.try_recv().is_err(),
            "drain must empty the accept messages"
        );

        advance_step(1, 100, 201, &tx, &mut mgr).await;

        let mut msgs = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            msgs.push(msg);
        }

        // 2 completed-objective updates, then onStepUpdate(old, COMPLETED),
        // then onStepUpdate(new, ACTIVE). The objective updates must precede
        // the step updates so the client sees them while the old step is still
        // current.
        assert_eq!(
            msgs.len(),
            4,
            "advance_step must report each old-step objective it completes"
        );

        let decode_id = |args: &[u8]| i32::from_le_bytes([args[0], args[1], args[2], args[3]]);

        // First objective: real flags carried through (hidden=1, optional=1).
        match &msgs[0] {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } => {
                assert_eq!(*method_index, ON_OBJECTIVE_UPDATE);
                assert_eq!(decode_id(args), 300);
                assert_eq!(args[4], STATUS_COMPLETED as u8);
                assert_eq!(args[5], 1, "hidden flag must be preserved");
                assert_eq!(args[6], 1, "optional flag must be preserved");
            }
            _ => panic!("expected onObjectiveUpdate"),
        }
        match &msgs[1] {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } => {
                assert_eq!(*method_index, ON_OBJECTIVE_UPDATE);
                assert_eq!(decode_id(args), 301);
                assert_eq!(args[4], STATUS_COMPLETED as u8);
                assert_eq!(args[5], 0);
                assert_eq!(args[6], 0);
            }
            _ => panic!("expected onObjectiveUpdate at index 1"),
        }
        match &msgs[2] {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } => {
                assert_eq!(*method_index, ON_STEP_UPDATE);
                assert_eq!(decode_id(args), 200);
                assert_eq!(args[4], STATUS_COMPLETED as u8);
            }
            _ => panic!("expected onStepUpdate"),
        }
        match &msgs[3] {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } => {
                assert_eq!(*method_index, ON_STEP_UPDATE);
                assert_eq!(decode_id(args), 201);
                assert_eq!(args[4], STATUS_ACTIVE as u8);
            }
            _ => panic!("expected onStepUpdate"),
        }
    }
}
