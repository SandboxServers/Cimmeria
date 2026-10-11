//! Mission lifecycle: accept and abandon.

use tokio::sync::mpsc;

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, MISSION_FAILED,
    MISSION_NOT_ACTIVE, STATUS_ACTIVE, STATUS_COMPLETED,
};

use super::{
    send_mission_frame, suppress_hidden_mission_frames, ON_MISSION_UPDATE, ON_OBJECTIVE_UPDATE,
    ON_STEP_UPDATE,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{EntityNames, SpaceManager};

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
    // TG-DB-12: an unreadable saved state reads as "never taken" here, so
    // the offer guard below would wave a completed mission through.
    if super::state_lock::refuse_if_state_unknown(entity_id, mission_id, "accept", tx, space_mgr)
        .await
    {
        return false;
    }
    // Rule 5 identity and the world, read before the mutable borrow below.
    let who = space_mgr.player_identity(entity_id);
    let world = space_mgr.get_entity_world_name(entity_id);
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
                event = "mission_accept_refused",
                reason = "entity_missing",
                entity_id, // nt:id-only the entity is gone, nothing left to name
                mission_id,
                mission_name = cimmeria_names::book().mission(mission_id),
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
            MISSION_ACTIVE => Some("already_active"),
            MISSION_FAILED if !def.is_some_and(|d| d.can_repeat_on_fail) => {
                Some("failed_not_repeatable")
            }
            MISSION_COMPLETED if repeats > def.map_or(0, |d| d.num_repeats) => {
                Some("repeat_cap_reached")
            }
            _ => None,
        };
        if let Some(reason) = refusal {
            tracing::warn!(
                event = "mission_accept_refused",
                reason,
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                world = world.as_deref(),
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                mission_id,
                mission_name = cimmeria_names::book().mission(mission_id),
                status,
                repeats,
                num_repeats = def.map_or(0, |d| d.num_repeats),
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
        None => {
            tracing::warn!(
                event = "mission_accept_refused",
                reason = "entity_missing",
                entity_id, // nt:id-only the entity is gone, nothing left to name
                mission_id,
                mission_name = cimmeria_names::book().mission(mission_id),
                "accept_mission: entity vanished before the accept was applied"
            );
            return false;
        }
    };
    let player_id = entity.player_id;
    let mut mission = MissionInstance::new(mission_id, step_id, objectives.clone());
    mission.repeats = prior_repeats;
    mission.is_hidden = is_hidden;
    entity.missions.add_mission(mission);

    {
        let names = cimmeria_names::book();
        tracing::info!(
            event = "mission_accepted",
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            world = world.as_deref(),
            entity_id,
            entity_name = EntityNames::of(entity).entity_name,
            mission_id,
            mission_name = names.mission(mission_id),
            step_id,
            step_name = names.mission_step(step_id),
            prior_repeats,
            "Mission accepted"
        );
    }

    // Hidden missions are accepted (state above, persistence and the
    // `mission_accepted` event in the caller) but never announced (#715).
    if suppress_hidden_mission_frames(is_hidden, entity_id, player_id, mission_id, "accept") {
        return true;
    }

    // Send onMissionUpdate
    let mut args = Vec::with_capacity(9);
    args.extend_from_slice(&mission_id.to_le_bytes());
    args.push(STATUS_ACTIVE as u8);
    args.extend_from_slice(&0i32.to_le_bytes());
    send_mission_frame(
        tx,
        entity_id,
        ON_MISSION_UPDATE,
        args,
        "onMissionUpdate",
        mission_id,
        &who,
    )
    .await;

    // Send onStepUpdate
    let mut args = Vec::with_capacity(5);
    args.extend_from_slice(&step_id.to_le_bytes());
    args.push(STATUS_ACTIVE as u8);
    send_mission_frame(
        tx,
        entity_id,
        ON_STEP_UPDATE,
        args,
        "onStepUpdate",
        mission_id,
        &who,
    )
    .await;

    // Send onObjectiveUpdate per objective
    for obj in &objectives {
        let mut args = Vec::with_capacity(7);
        args.extend_from_slice(&obj.objective_id.to_le_bytes());
        args.push(obj.status as u8);
        args.push(if obj.hidden { 1 } else { 0 });
        args.push(if obj.optional { 1 } else { 0 });
        send_mission_frame(
            tx,
            entity_id,
            ON_OBJECTIVE_UPDATE,
            args,
            "onObjectiveUpdate",
            mission_id,
            &who,
        )
        .await;
    }

    true
}

/// Abandon a mission: remove it, save the abandon, and send the removal to
/// the client.
///
/// Only an active mission is abandoned: a completed, failed or not-active
/// one is refused with an INFO row and nothing is saved or sent (CS-08
/// review R1; python's `abandon()` is `fail()`, active only).
///
/// Returns `true` only when a mission instance was actually removed. Callers
/// use that to gate the `mission_abandoned` content event (H54): abandoning a
/// mission the player does not hold is a no-op, and firing the event for it
/// would repaint an offer on every stray `abandonMission` the client sends.
///
/// The save is a `MissionUpdate` at `MISSION_NOT_ACTIVE` with no step and no
/// objectives, carrying the mission's `repeats`. The base deletes the row
/// when `repeats` is 0 and keeps a not-active row otherwise, so the mission
/// does not come back after a relog and a repeatable one keeps its count
/// (#1315, #118). Every abandon path (the player's `abandonMission`, the
/// `abandon_mission` chain action, `gmMissionClear` / `gmMissionAbandon`)
/// comes through here, so none of them persists on its own.
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
) -> bool {
    if super::state_lock::refuse_if_state_unknown(entity_id, mission_id, "abandon", tx, space_mgr)
        .await
    {
        return false;
    }
    let who = space_mgr.player_identity(entity_id);
    let world = space_mgr.get_entity_world_name(entity_id);
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(
                event = "mission_abandon_refused",
                reason = "entity_missing",
                entity_id, // nt:id-only the entity is gone, nothing left to name
                mission_id,
                mission_name = cimmeria_names::book().mission(mission_id),
                "abandon_mission: entity not found — nothing removed or saved"
            );
            return false;
        }
    };
    if let Some(pid) = entity.player_id {
        tracing::Span::current().record("player_id", pid);
    }
    let player_id = entity.player_id;

    if let Some(removed) = entity.missions.abandon_mission(mission_id) {
        tracing::info!(
            event = "mission_abandoned",
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            world = world.as_deref(),
            entity_id,
            entity_name = EntityNames::of(entity).entity_name,
            mission_id,
            mission_name = cimmeria_names::book().mission(mission_id),
            "Mission abandoned"
        );

        // Persist first, hidden or not: the saved row must not reload as
        // active either way.
        if let Some(pid) = player_id {
            let saved = CellToBaseMsg::MissionUpdate {
                player_id: pid,
                mission_id,
                status: MISSION_NOT_ACTIVE,
                current_step_id: None,
                completed_step_ids: vec![],
                completed_objective_ids: vec![],
                active_objective_ids: vec![],
                failed_objective_ids: vec![],
                repeats: removed.repeats,
            };
            if let Err(e) = tx.send(saved).await {
                tracing::error!(
                    entity_id,
                    entity_name = EntityNames::of(entity).entity_name,
                    player_id = pid,
                    // A player's entity name is its character name.
                    player_name = EntityNames::of(entity).entity_name,
                    mission_id,
                    mission_name = cimmeria_names::book().mission(mission_id),
                    error = %e,
                    "abandon_mission: MissionUpdate send to base failed -- the abandon \
                     is not saved and the mission returns after a relog"
                );
            }
        }

        // The client never saw a hidden mission, so there is no journal
        // row to remove (#715). The removal itself still counts.
        if suppress_hidden_mission_frames(
            removed.is_hidden,
            entity_id,
            player_id,
            mission_id,
            "abandon",
        ) {
            return true;
        }

        // Send onMissionUpdate with status=completed (removes from client log)
        let mut args = Vec::with_capacity(9);
        args.extend_from_slice(&mission_id.to_le_bytes());
        args.push(STATUS_COMPLETED as u8);
        args.extend_from_slice(&0i32.to_le_bytes());
        send_mission_frame(
            tx,
            entity_id,
            ON_MISSION_UPDATE,
            args,
            "onMissionUpdate",
            mission_id,
            &who,
        )
        .await;
        return true;
    }

    if let Some(status) = entity.missions.get_mission(mission_id).map(|m| m.status) {
        tracing::info!(
            event = "mission_abandon_refused",
            reason = "not_active",
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            world = world.as_deref(),
            entity_id,
            entity_name = EntityNames::of(entity).entity_name,
            mission_id,
            mission_name = cimmeria_names::book().mission(mission_id),
            status,
            "abandon_mission: only an active mission can be abandoned; the record and its \
             saved row are left as they are"
        );
        return false;
    }
    tracing::debug!(
        entity_id,
        entity_name = EntityNames::of(entity).entity_name,
        mission_id,
        mission_name = cimmeria_names::book().mission(mission_id),
        "abandon_mission: mission not tracked — no removal, no mission_abandoned event"
    );
    false
}
