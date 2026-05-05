//! Helpers for populating the content engine [`ExecutionContext`] with
//! per-entity mission and step state.
//!
//! Every event-firing path needs to expose mission state to chain conditions
//! (e.g., `mission_622_status == "active"`). This module centralizes that
//! population so the dispatch sites stay focused on event-specific params.

use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::missions::{MISSION_ACTIVE, MISSION_COMPLETED, MISSION_NOT_ACTIVE};

/// Populate mission status and step status context params from entity state.
///
/// Step status semantics (chain conditions read these via `step_status`):
///
/// - **`active`** — the step is the mission's current step.
/// - **`completed`** — the step appears in `completed_steps` (the player has
///   advanced past it).
/// - **`not_active`** — anything else, surfaced via the evaluator's
///   `unwrap_or("not_active")` fallback. This includes both "step never
///   reached" (mission not yet at this step) and "mission never accepted".
///   Chains that need to distinguish "passed this step" from "never reached
///   it" should compare against `completed` rather than relying on
///   `not_active`.
pub(super) fn populate_mission_context(entity: &CellEntity, ctx: &mut ExecutionContext) {
    for mission in entity.missions.all_missions() {
        let status_str = match mission.status {
            MISSION_NOT_ACTIVE => "not_active",
            MISSION_ACTIVE => "active",
            MISSION_COMPLETED => "completed",
            _ => "not_active",
        };
        ctx.set_param(
            format!("mission_{}_status", mission.mission_id),
            serde_json::json!(status_str),
        );

        // Mark every completed step before the active one so chains can gate on
        // "step has already been passed" without needing to enumerate every
        // possible follow-on step. Order matters: the active-step write below
        // takes precedence if (somehow) both lists overlapped, since the
        // active step is the source of truth.
        for completed_step_id in &mission.completed_steps {
            ctx.set_param(
                format!(
                    "mission_{}_step_{}_status",
                    mission.mission_id, completed_step_id
                ),
                serde_json::json!("completed"),
            );
        }

        if let Some(step_id) = mission.current_step_id {
            ctx.set_param(
                format!("mission_{}_step_{}_status", mission.mission_id, step_id),
                serde_json::json!("active"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_common::math::Vector3;
    use cimmeria_common::types::{EntityId, SpaceId};
    use cimmeria_entity::missions::{MissionInstance, MissionObjective, STATUS_ACTIVE};

    fn make_entity_with_mission(mission: MissionInstance) -> CellEntity {
        let mut entity = CellEntity::new(EntityId(1), SpaceId(100), Vector3::zero());
        entity.missions.add_mission(mission);
        entity
    }

    /// A mission that's been advanced past its initial step must mark that
    /// step as `completed`, not leave it at the `not_active` fallback. This
    /// is the load-bearing fix for the Marsh-quest loop: chains that need
    /// to gate "after the player advanced past step 2121" can now write
    /// `step_status 641 2121 eq completed` instead of leaning on
    /// `not_active`, which is also true *before* the mission is accepted.
    #[test]
    fn populates_completed_step_status_for_advanced_past_step() {
        let mut mission = MissionInstance::new(
            641,
            2121,
            vec![MissionObjective {
                objective_id: 1,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        );
        // Simulate advancing past step 2121 to step 3563.
        mission.completed_steps.push(2121);
        mission.current_step_id = Some(3563);

        let entity = make_entity_with_mission(mission);
        let mut ctx = ExecutionContext::new();
        populate_mission_context(&entity, &mut ctx);

        assert_eq!(
            ctx.params
                .get("mission_641_step_2121_status")
                .and_then(|v| v.as_str()),
            Some("completed"),
            "advanced-past step must populate as `completed`",
        );
        assert_eq!(
            ctx.params
                .get("mission_641_step_3563_status")
                .and_then(|v| v.as_str()),
            Some("active"),
            "current step must populate as `active`",
        );
    }

    /// If a step somehow appears in both `completed_steps` and as the
    /// `current_step_id`, the active write must win — it's set last in the
    /// populator on purpose so a transient state-machine glitch doesn't
    /// freeze a chain at "completed" while the player is genuinely on
    /// that step.
    #[test]
    fn active_step_overrides_stale_completed_entry() {
        let mut mission = MissionInstance::new(
            641,
            2121,
            vec![MissionObjective {
                objective_id: 1,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        );
        // Pathological state: step 2121 listed as both completed and current.
        mission.completed_steps.push(2121);

        let entity = make_entity_with_mission(mission);
        let mut ctx = ExecutionContext::new();
        populate_mission_context(&entity, &mut ctx);

        assert_eq!(
            ctx.params
                .get("mission_641_step_2121_status")
                .and_then(|v| v.as_str()),
            Some("active"),
            "active step write must take precedence over stale completed entry",
        );
    }

    /// A mission with no completed steps and an active current step
    /// must populate only the active entry — no stray `completed`
    /// params for steps the player never reached.
    #[test]
    fn does_not_populate_unreached_steps() {
        let mission = MissionInstance::new(
            641,
            2121,
            vec![MissionObjective {
                objective_id: 1,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        );

        let entity = make_entity_with_mission(mission);
        let mut ctx = ExecutionContext::new();
        populate_mission_context(&entity, &mut ctx);

        assert_eq!(
            ctx.params
                .get("mission_641_step_2121_status")
                .and_then(|v| v.as_str()),
            Some("active"),
        );
        // Step 3563 is a real step ID for mission 641 but the player hasn't
        // reached it yet — the param must be absent so the evaluator's
        // `unwrap_or("not_active")` fallback applies.
        assert!(
            !ctx.params.contains_key("mission_641_step_3563_status"),
            "unreached steps must not be populated",
        );
    }
}
