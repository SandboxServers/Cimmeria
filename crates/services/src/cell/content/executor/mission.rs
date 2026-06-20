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
        let accepted = crate::cell::missions::accept_mission(
            entity_id, mission_id, step_id, objectives, tx, space_mgr,
        )
        .await;
        // Offer refused (already active / completed at repeat cap /
        // failed non-repeatable) or entity missing. Persisting the
        // MissionUpdate anyway would UPSERT status=1 over the saved
        // row — exactly the "completed missions reappear as active
        // after relog" corruption (#411). Skip the follow-up
        // `mission_accepted` event too: no acceptance happened.
        if !accepted {
            tracing::info!(
                entity_id,
                mission_id,
                chain_id,
                "Content: accept_mission refused by offer guard — skipping persist + follow-up event"
            );
            return;
        }
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

        // Discord gameplay-channel. Mission defs carry no name cell-side, so
        // the mission id is the identifier; the player name comes from the
        // entity's InitPlayerState-cached value.
        let character_name = space_mgr
            .get_entity(entity_id)
            .and_then(|e| e.character_name.clone())
            .unwrap_or_else(|| format!("entity:{entity_id}"));
        cimmeria_discord::emit_mission_accepted(character_name, mission_id, None);
    } else {
        tracing::warn!(
            mission_id,
            chain_id,
            "No mission_defs entry — cannot accept mission"
        );
    }
}

/// `Action::CompleteMission` — mark the mission complete and persist,
/// then fire the `mission_completed` follow-up event so chains hooked
/// on completion (e.g., chain 1105's auto-accept of 688 when 687
/// completes) actually run. Mirrors `accept_or_advance`'s firing of
/// `fire_mission_accepted`.
pub(super) async fn complete(
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
        "Content: completing mission"
    );
    // Snapshot the prior status BEFORE `complete_mission_direct` flips
    // it. We only fire the `mission_completed` follow-up event on a
    // real active→completed transition — running this action against
    // an already-completed mission must be a wire/no-op, and running
    // it against a FAILED mission must not fire either (a failure
    // being "completed" via this action would otherwise re-fire
    // completion-driven chains like the auto-accept of the next
    // mission, which never legitimately follows a failure).
    use cimmeria_entity::missions::MISSION_ACTIVE;
    let prior_status = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.missions.get_mission(mission_id))
        .map(|m| m.status);
    let transitioned_from_active = prior_status == Some(MISSION_ACTIVE);

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
    // Fire the `mission_completed` chain-engine event only when the
    // mission was MISSION_ACTIVE before this call — that's the only
    // legitimate transition into MISSION_COMPLETED. Already-completed,
    // failed, or untracked missions all skip the event; otherwise a
    // chain like 1105 (`mission_completed 687 → accept_mission 688`)
    // could re-fire on retries or fire spuriously when a failure is
    // converted to a completion.
    if transitioned_from_active {
        crate::cell::content::event_dispatch::fire_mission_completed(
            entity_id, player_id, mission_id, engine, tx, space_mgr,
        )
        .await;

        // Discord gameplay-channel — only on the real active→completed
        // transition (guarded by `transitioned_from_active`), so retries and
        // failure-conversions don't post.
        let character_name = space_mgr
            .get_entity(entity_id)
            .and_then(|e| e.character_name.clone())
            .unwrap_or_else(|| format!("entity:{entity_id}"));
        cimmeria_discord::emit_mission_completed(character_name, mission_id, None);
    } else {
        tracing::debug!(
            entity_id,
            mission_id,
            ?prior_status,
            "Content: complete called on non-active mission — skipping mission_completed event"
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

#[cfg(test)]
mod offer_guard_tests {
    //! #411 regression guards at the executor boundary: a refused (or
    //! entity-less) `accept_or_advance` must NOT persist a `MissionUpdate`
    //! — pre-fix, the handler sent `status=1` to the base unconditionally,
    //! so any re-fired grant chain UPSERTed "active" over a saved
    //! completed row and the mission resurrected in the quest log on the
    //! next relog.

    use super::*;
    use crate::cell::spawner::MissionDefEntry;
    use cimmeria_entity::missions::{MissionInstance, MISSION_COMPLETED};
    use tokio::sync::mpsc;

    fn make_mgr_with_def() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.mission_defs.insert(
            622,
            MissionDefEntry {
                step_id: 2113,
                objectives: vec![],
                is_hidden: false,
                num_repeats: 1,
                can_repeat_on_fail: true,
            },
        );
        mgr
    }

    fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
        let mut msgs = Vec::new();
        while let Ok(m) = rx.try_recv() {
            msgs.push(m);
        }
        msgs
    }

    fn mission_updates(msgs: &[CellToBaseMsg]) -> Vec<i8> {
        msgs.iter()
            .filter_map(|m| match m {
                CellToBaseMsg::MissionUpdate { status, .. } => Some(*status),
                _ => None,
            })
            .collect()
    }

    /// Refused offer (completed mission at the repeat cap) → no
    /// `MissionUpdate` reaches the base, entity state untouched.
    #[tokio::test]
    async fn refused_accept_does_not_persist_mission_update() {
        let mut mgr = make_mgr_with_def();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        {
            let entity = mgr.get_entity_mut(1).unwrap();
            let mut prior = MissionInstance::new(622, 2113, vec![]);
            prior.complete();
            prior.repeats = 2; // past num_repeats = 1
            entity.missions.add_mission(prior);
        }
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(32);

        accept_or_advance(622, 1, 100, 9999, &tx, &mut mgr, &engine).await;

        let msgs = drain(&mut rx);
        assert!(
            mission_updates(&msgs).is_empty(),
            "refused accept must not send MissionUpdate (would UPSERT \
             status=1 over the saved completed row); got {msgs:?}"
        );
        let m = mgr
            .get_entity(1)
            .unwrap()
            .missions
            .get_mission(622)
            .unwrap();
        assert_eq!(m.status, MISSION_COMPLETED, "completed status preserved");
    }

    /// Entity missing entirely (e.g. an event fired against a player whose
    /// cell entity is gone) → nothing to verify against, so nothing may be
    /// persisted. Pre-fix this path still sent `MissionUpdate status=1
    /// repeats=0`, silently corrupting the saved row.
    #[tokio::test]
    async fn missing_entity_does_not_persist_mission_update() {
        let mut mgr = make_mgr_with_def();
        // No entity created.
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(32);

        accept_or_advance(622, 1, 100, 9999, &tx, &mut mgr, &engine).await;

        let msgs = drain(&mut rx);
        assert!(
            mission_updates(&msgs).is_empty(),
            "entity-less accept must not persist anything; got {msgs:?}"
        );
    }

    /// Happy-path companion pinning the success contract: a fresh accept
    /// still persists exactly one `MissionUpdate` with status=1. Guards
    /// against the refusal gate accidentally swallowing legitimate accepts.
    #[tokio::test]
    async fn fresh_accept_still_persists_mission_update() {
        let mut mgr = make_mgr_with_def();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(32);

        accept_or_advance(622, 1, 100, 9999, &tx, &mut mgr, &engine).await;

        let msgs = drain(&mut rx);
        assert_eq!(
            mission_updates(&msgs),
            vec![1],
            "fresh accept must persist exactly one MissionUpdate(status=1)"
        );
    }
}
