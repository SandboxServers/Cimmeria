//! Mission action handlers: accept/advance, complete, advance step, abandon,
//! complete objective.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{MissionObjective, STATUS_ACTIVE};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Look up a mission's current `repeats` count from the cell-side player
/// instance. Returns 0 when the entity or mission isn't tracked — the cell
/// is the authoritative source for this value, so a missing entry means
/// "no completions yet" and 0 is the correct default.
fn mission_repeats(space_mgr: &SpaceManager, entity_id: u32, mission_id: i32) -> i32 {
    space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.missions.get_mission(mission_id))
        .map(|m| m.repeats)
        .unwrap_or(0)
}

/// `Action::AcceptMission` and `Action::AdvanceMission` — identical handling
/// (insert/refresh the mission instance, persist via `MissionUpdate`, fire
/// the `mission_accepted` follow-up event).
pub(super) async fn accept_or_advance(
    mission_id: i32,
    entity_id: u32,
    player_id: i32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    tracing::info!(
        entity_id,
        mission_id,
        chain_id,
        "Content: accepting mission"
    );
    if let Some(def) = space_mgr.mission_defs.get(&mission_id) {
        let step_id = def.step_id;
        let objectives: Vec<MissionObjective> = def
            .objectives
            .iter()
            .map(|o| MissionObjective {
                objective_id: o.objective_id,
                status: STATUS_ACTIVE,
                hidden: o.is_hidden,
                optional: o.is_optional,
            })
            .collect();
        crate::cell::missions::accept_mission(
            entity_id, mission_id, step_id, objectives, tx, space_mgr,
        )
        .await;
        // Read repeats AFTER the helper runs — for a re-accept of
        // a previously-completed repeatable mission, the count
        // restored from DB is what should round-trip back, not 0.
        let repeats = mission_repeats(space_mgr, entity_id, mission_id);
        if let Err(e) = tx
            .send(CellToBaseMsg::MissionUpdate {
                player_id,
                mission_id,
                status: 1,
                current_step_id: Some(step_id),
                completed_step_ids: vec![],
                completed_objective_ids: vec![],
                active_objective_ids: vec![step_id],
                failed_objective_ids: vec![],
                repeats,
            })
            .await
        {
            tracing::error!(
                entity_id, player_id, mission_id, step_id,
                chain_id, error = %e,
                "MissionUpdate (accept) send to base failed -- mission progress not persisted"
            );
        }
        // Fire the follow-up `mission_accepted` event so chains
        // tied to mission start can run their setup work
        // (e.g., chain 1097 highlighting Cellblock_WoodenCrate
        // for mission 687). The in-process entity mutation is
        // already committed even if MissionUpdate failed to
        // persist, so the chain's view of mission state is
        // valid regardless.
        crate::cell::content::event_dispatch::fire_mission_accepted(
            entity_id, player_id, mission_id, engine, tx, space_mgr,
        )
        .await;
    } else {
        tracing::warn!(
            mission_id,
            chain_id,
            "No mission_defs entry — cannot accept mission"
        );
    }
}

/// `Action::CompleteMission` — mark the mission complete and persist.
pub(super) async fn complete(
    mission_id: i32,
    entity_id: u32,
    player_id: i32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    tracing::info!(
        entity_id,
        mission_id,
        chain_id,
        "Content: completing mission"
    );
    crate::cell::missions::complete_mission_direct(entity_id, mission_id, tx, space_mgr).await;
    // Read repeats AFTER complete_mission_direct so we capture
    // the post-bump value (`MissionInstance::complete` increments).
    let repeats = mission_repeats(space_mgr, entity_id, mission_id);
    if let Err(e) = tx
        .send(CellToBaseMsg::MissionUpdate {
            player_id,
            mission_id,
            status: 2,
            current_step_id: None,
            completed_step_ids: vec![],
            completed_objective_ids: vec![],
            active_objective_ids: vec![],
            failed_objective_ids: vec![],
            repeats,
        })
        .await
    {
        tracing::error!(
            entity_id, player_id, mission_id, chain_id, error = %e,
            "MissionUpdate (complete) send to base failed -- mission completion not persisted"
        );
    }
}

/// `Action::AdvanceStep` — move a mission to a new step and persist.
pub(super) async fn advance_step(
    mission_id: i32,
    step_id: i32,
    entity_id: u32,
    player_id: i32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    tracing::info!(
        entity_id,
        mission_id,
        step_id,
        chain_id,
        "Content: advancing step"
    );
    crate::cell::missions::advance_step(entity_id, mission_id, step_id, tx, space_mgr).await;
    let repeats = mission_repeats(space_mgr, entity_id, mission_id);
    if let Err(e) = tx
        .send(CellToBaseMsg::MissionUpdate {
            player_id,
            mission_id,
            status: 1,
            current_step_id: Some(step_id),
            completed_step_ids: vec![],
            completed_objective_ids: vec![],
            active_objective_ids: vec![step_id],
            failed_objective_ids: vec![],
            repeats,
        })
        .await
    {
        tracing::error!(
            entity_id, player_id, mission_id, step_id,
            chain_id, error = %e,
            "MissionUpdate (advance step) send to base failed -- step progress not persisted"
        );
    }
}

/// `Action::AbandonMission` — drop the mission from the player's tracker.
pub(super) async fn abandon(
    mission_id: i32,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    tracing::info!(
        entity_id,
        mission_id,
        chain_id,
        "Content: abandoning mission"
    );
    crate::cell::missions::abandon_mission(entity_id, mission_id, tx, space_mgr).await;
}

/// `Action::CompleteObjective` — mark a single objective complete.
pub(super) async fn complete_objective(
    mission_id: i32,
    objective_id: i32,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    tracing::info!(
        entity_id,
        mission_id,
        objective_id,
        chain_id,
        "Content: complete objective"
    );
    crate::cell::missions::complete_objective(entity_id, mission_id, objective_id, tx, space_mgr)
        .await;
}
