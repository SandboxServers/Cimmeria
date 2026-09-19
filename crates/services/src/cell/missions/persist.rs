//! Serialization of the cell's authoritative [`MissionInstance`] into the
//! `CellToBaseMsg::MissionUpdate` that the base UPSERTs into `sgw_mission`.
//!
//! Before this module existed every executor arm hand-rolled the message,
//! and every one of them got the objective arrays wrong (Harset H50):
//! accept and `advance_step` wrote `active_objective_ids: vec![step_id]`
//! — the STEP id in the OBJECTIVE array — `complete` wrote both arrays
//! empty, and `complete_objective` sent no `MissionUpdate` at all. The
//! saved row therefore never carried real per-objective state, so after a
//! relog `populate_mission_context` emitted no
//! `mission_<m>_obj_<o>_status` params and every `objective_status`-gated
//! chain fell through `Condition::ObjectiveStatus`'s
//! `unwrap_or("not_active")`.
//!
//! One serializer, read back from the live instance *after* the mutation,
//! is the fix: the emitted message can never disagree with cell state,
//! and an arm that auto-completes its mission (see
//! [`super::progression::complete_objective`]) persists the post-transition
//! status and the bumped `repeats` without the caller having to know.

use tokio::sync::mpsc;

use cimmeria_entity::missions::{MissionInstance, STATUS_COMPLETED};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Build the `MissionUpdate` that persists `mission`'s current state.
///
/// Array semantics, which the hydrate side
/// (`cell::service::base_messages::player_init::mission_restore`) mirrors:
///
/// - `active_objective_ids` — every objective tracked for the *current*
///   step, completed ones included. Hydration re-reads a completed id out
///   of both arrays to decide its `status`, so dropping completed ids here
///   would lose the fact that the objective is still part of this step.
/// - `completed_objective_ids` — the union of `completed_objectives` and
///   any `active_objectives` entry already flipped to `STATUS_COMPLETED`.
///   The union is load-bearing in both directions:
///   [`MissionInstance::complete`] flips every active objective to
///   completed *without* pushing into `completed_objectives`, while
///   [`MissionInstance::complete_objective`] pushes without a
///   contains-check, so the same id can appear twice across an
///   advance-then-complete sequence. Deduped, first-seen order preserved.
/// - `failed_objective_ids` — always empty. Nothing on the cell tracks
///   per-objective failure today; mission-level failure lives in `status`.
pub fn mission_update_msg(player_id: i32, mission: &MissionInstance) -> CellToBaseMsg {
    let active_objective_ids: Vec<i32> = mission
        .active_objectives
        .iter()
        .map(|o| o.objective_id)
        .collect();

    let mut completed_objective_ids: Vec<i32> = Vec::with_capacity(
        mission.completed_objectives.len() + mission.active_objectives.len(),
    );
    let mut push_unique = |id: i32, out: &mut Vec<i32>| {
        if !out.contains(&id) {
            out.push(id);
        }
    };
    for &id in &mission.completed_objectives {
        push_unique(id, &mut completed_objective_ids);
    }
    for obj in &mission.active_objectives {
        if obj.status == STATUS_COMPLETED {
            push_unique(obj.objective_id, &mut completed_objective_ids);
        }
    }

    CellToBaseMsg::MissionUpdate {
        player_id,
        mission_id: mission.mission_id,
        status: mission.status,
        current_step_id: mission.current_step_id,
        completed_step_ids: mission.completed_steps.clone(),
        completed_objective_ids,
        active_objective_ids,
        failed_objective_ids: vec![],
        repeats: mission.repeats,
    }
}

/// Read `mission_id` back off the cell entity and persist its current
/// state. Call this *after* the mutation, never before.
///
/// A missing entity or a mission the tracker doesn't know about is a
/// no-op: persisting a `MissionUpdate` we can't source from live cell
/// state is how #411 ("completed missions reappear as active after
/// relog") happened, so the silent-skip is deliberate — but it is logged,
/// because reaching here means a caller mutated something that isn't
/// there.
pub async fn send_mission_update(
    entity_id: u32,
    player_id: i32,
    mission_id: i32,
    site: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let msg = match space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.missions.get_mission(mission_id))
    {
        Some(m) => mission_update_msg(player_id, m),
        None => {
            tracing::warn!(
                entity_id,
                player_id,
                mission_id,
                site,
                "send_mission_update: no live mission instance — skipping persist \
                 (mission progress for this action will not survive relog)"
            );
            return;
        }
    };

    if let Err(e) = tx.send(msg).await {
        tracing::error!(
            entity_id, player_id, mission_id, site, error = %e,
            "MissionUpdate send to base failed -- mission progress not persisted"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::missions::{MissionObjective, STATUS_ACTIVE};

    fn obj(objective_id: i32, status: i8, optional: bool) -> MissionObjective {
        MissionObjective {
            objective_id,
            status,
            hidden: false,
            optional,
        }
    }

    fn arrays(msg: &CellToBaseMsg) -> (Vec<i32>, Vec<i32>, Vec<i32>, i8) {
        match msg {
            CellToBaseMsg::MissionUpdate {
                active_objective_ids,
                completed_objective_ids,
                completed_step_ids,
                status,
                ..
            } => (
                active_objective_ids.clone(),
                completed_objective_ids.clone(),
                completed_step_ids.clone(),
                *status,
            ),
            other => panic!("expected MissionUpdate, got {other:?}"),
        }
    }

    /// The H50 bug shape: the objective array must carry OBJECTIVE ids,
    /// never the step id, and a partially-completed step must persist
    /// both which objectives are tracked and which of them are done.
    #[test]
    fn partially_completed_step_serializes_both_objective_arrays() {
        let mut m = MissionInstance::new(
            742,
            2504,
            vec![
                obj(2913, STATUS_ACTIVE, false),
                obj(2914, STATUS_ACTIVE, false),
                obj(2915, STATUS_ACTIVE, false),
            ],
        );
        m.complete_objective(2913);

        let (active, completed, _, status) = arrays(&mission_update_msg(77, &m));
        assert_eq!(
            active,
            vec![2913, 2914, 2915],
            "every objective of the current step must persist, completed ones \
             included — pre-fix this array held the STEP id (2504)",
        );
        assert_eq!(
            completed,
            vec![2913],
            "the completed objective must persist so `objective_status 742 2913 \
             eq completed` still matches after a relog",
        );
        assert_eq!(status, 1, "mission stays active with 2 objectives open");
    }

    /// `MissionInstance::complete` flips active objectives to completed
    /// without pushing them into `completed_objectives`. The union is
    /// what keeps them in `completed_objective_ids`.
    #[test]
    fn complete_flips_active_objectives_into_the_completed_array() {
        let mut m = MissionInstance::new(
            688,
            2356,
            vec![obj(2734, STATUS_ACTIVE, false), obj(4647, STATUS_ACTIVE, true)],
        );
        m.complete();
        assert!(
            m.completed_objectives.is_empty(),
            "precondition: complete() does not touch completed_objectives",
        );

        let (active, completed, steps, status) = arrays(&mission_update_msg(77, &m));
        assert_eq!(active, vec![2734, 4647]);
        assert_eq!(
            completed,
            vec![2734, 4647],
            "a completed mission must persist its objectives as completed — \
             pre-fix the complete arm wrote both arrays empty",
        );
        assert_eq!(steps, vec![2356]);
        assert_eq!(status, 2);
    }

    /// `complete_objective` pushes without a contains-check, and
    /// `advance_step` then `complete` can flip the same id again. The
    /// serialized array must not carry the duplicate into the DB row.
    #[test]
    fn duplicate_completions_are_deduped() {
        let mut m = MissionInstance::new(641, 2121, vec![obj(999, STATUS_ACTIVE, false)]);
        m.complete_objective(999);
        m.complete_objective(999);
        assert_eq!(
            m.completed_objectives,
            vec![999, 999],
            "precondition: the in-memory list really does hold the duplicate",
        );

        let (_, completed, _, _) = arrays(&mission_update_msg(77, &m));
        assert_eq!(completed, vec![999]);
    }

    /// Objectives from a step the player has advanced past live only in
    /// `completed_objectives`; they must stay in the completed array even
    /// though `active_objectives` has been replaced wholesale.
    #[test]
    fn objectives_carried_across_a_step_boundary_stay_completed() {
        let mut m = MissionInstance::new(688, 2356, vec![obj(2734, STATUS_ACTIVE, false)]);
        m.complete_objective(2734);
        // What `progression::advance_step` does to the instance.
        m.completed_steps.push(2356);
        m.current_step_id = Some(80688);
        m.active_objectives = vec![obj(4647, STATUS_ACTIVE, false)];

        let (active, completed, steps, _) = arrays(&mission_update_msg(77, &m));
        assert_eq!(active, vec![4647], "only the new step's objectives are active");
        assert_eq!(
            completed,
            vec![2734],
            "the prior step's objective must survive the step boundary — this is \
             what `objective_status 688 2734 eq completed` reads after a relog",
        );
        assert_eq!(steps, vec![2356]);
    }
}
