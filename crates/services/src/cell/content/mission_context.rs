//! Helpers for populating the content engine [`ExecutionContext`] with
//! per-entity mission and step state.
//!
//! Every event-firing path needs to expose mission state to chain conditions
//! (e.g., `mission_622_status == "active"`). This module centralizes that
//! population so the dispatch sites stay focused on event-specific params.

use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::missions::{
    MISSION_ACTIVE, MISSION_COMPLETED, MISSION_NOT_ACTIVE, STATUS_COMPLETED,
};

/// Populate per-stat current/max into the context as `stat_<id>_cur` and
/// `stat_<id>_max` numeric params. Read by `Condition::StatBelowMax` to
/// gate consumable chains against full-stat fizzles.
///
/// Currently only invoked from the `OnItemUse` dispatch site
/// (consumable-use is the only chain class that needs stat state for
/// gating). Generalize to other dispatch sites if/when more conditions
/// learn to read stat values.
pub(super) fn populate_stats_context(entity: &CellEntity, ctx: &mut ExecutionContext) {
    for (stat_id, stat) in entity.stats.iter() {
        ctx.set_param(format!("stat_{}_cur", stat_id), serde_json::json!(stat.cur));
        ctx.set_param(format!("stat_{}_max", stat_id), serde_json::json!(stat.max));
    }
}

/// Populate per-entity counter values into the context as `counter_<name>`
/// numeric params. Read by `Condition::Counter` (the loader maps DB
/// rows with `condition_type='counter'` and `target_key=<name>` onto
/// this).
///
/// Called from `populate_mission_context` so every mission-aware
/// dispatcher (notably `fire_entity_death`) sees up-to-date counter
/// values when evaluating completion chains. Mutated by
/// `Action::IncrementCounter` and `Action::ResetCounter` in the
/// executor.
pub(super) fn populate_counters_context(entity: &CellEntity, ctx: &mut ExecutionContext) {
    for (name, value) in &entity.counters {
        ctx.set_param(format!("counter_{}", name), serde_json::json!(*value));
    }
}

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
    // Counters travel alongside missions — completion chains for
    // multi-step kill sequences read counter values to know when to
    // fire. Populate them here so every mission-aware dispatcher gets
    // them without each having to remember a separate call.
    populate_counters_context(entity, ctx);

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

        // Per-objective status. `Condition::ObjectiveStatus` reads
        // `mission_{m}_obj_{o}_status`; without this loop the evaluator's
        // `unwrap_or("not_active")` fallback masked completed objectives,
        // so chains gated on `objective_status eq completed` could never
        // fire and chains gated on `neq completed` always fired (latent
        // bug surfaced by mission 688 chains 1109/1111).
        for obj in &mission.active_objectives {
            let status_str = if obj.status == STATUS_COMPLETED {
                "completed"
            } else {
                "active"
            };
            ctx.set_param(
                format!(
                    "mission_{}_obj_{}_status",
                    mission.mission_id, obj.objective_id
                ),
                serde_json::json!(status_str),
            );
        }
        // Objectives that were completed before the current step (e.g.
        // multi-step missions where prior steps' objectives stay
        // tracked) live in `completed_objectives` only. Mark them
        // `completed` if they aren't already in `active_objectives`
        // (active_objectives is the source of truth for the current
        // step's objectives, so don't overwrite a present entry).
        let active_ids: std::collections::HashSet<i32> = mission
            .active_objectives
            .iter()
            .map(|o| o.objective_id)
            .collect();
        for completed_id in &mission.completed_objectives {
            if active_ids.contains(completed_id) {
                continue;
            }
            ctx.set_param(
                format!("mission_{}_obj_{}_status", mission.mission_id, completed_id),
                serde_json::json!("completed"),
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

    /// Per-objective status population is what `Condition::ObjectiveStatus`
    /// (`mission_{m}_obj_{o}_status`) reads. Without this loop the
    /// evaluator's `unwrap_or("not_active")` fallback would mask
    /// completed objectives, breaking gates of the form
    /// `objective_status eq completed` (chain 1109's primary check).
    #[test]
    fn populates_per_objective_status_active_and_completed() {
        let mut mission = MissionInstance::new(
            688,
            2356,
            vec![
                MissionObjective {
                    objective_id: 2734,
                    status: STATUS_ACTIVE,
                    hidden: false,
                    optional: false,
                },
                MissionObjective {
                    objective_id: 4647,
                    status: STATUS_ACTIVE,
                    hidden: false,
                    optional: true,
                },
            ],
        );
        // Simulate completing the required objective (terminal interact).
        mission.complete_objective(2734);

        let entity = make_entity_with_mission(mission);
        let mut ctx = ExecutionContext::new();
        populate_mission_context(&entity, &mut ctx);

        assert_eq!(
            ctx.params
                .get("mission_688_obj_2734_status")
                .and_then(|v| v.as_str()),
            Some("completed"),
            "completed objective must populate as `completed` so chain 1109's \
             `objective_status 688 2734 eq completed` gate matches",
        );
        assert_eq!(
            ctx.params
                .get("mission_688_obj_4647_status")
                .and_then(|v| v.as_str()),
            Some("active"),
            "still-active optional objective must populate as `active`",
        );
    }

    /// Objectives in `completed_objectives` that are no longer in
    /// `active_objectives` (e.g. a previous step's objectives carried
    /// forward) must still surface as `completed` — otherwise chains
    /// that gate on a prior step's objective would lose state once the
    /// step advanced.
    #[test]
    fn populates_completed_only_objectives_carried_across_steps() {
        let mut mission = MissionInstance::new(
            641,
            2121,
            vec![MissionObjective {
                objective_id: 999,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        );
        // A different objective from a prior step is in the
        // completed-only list.
        mission.completed_objectives.push(2734);

        let entity = make_entity_with_mission(mission);
        let mut ctx = ExecutionContext::new();
        populate_mission_context(&entity, &mut ctx);

        assert_eq!(
            ctx.params
                .get("mission_641_obj_2734_status")
                .and_then(|v| v.as_str()),
            Some("completed"),
            "objective in completed_objectives but not in active_objectives \
             must still populate as `completed`",
        );
    }

    /// `populate_counters_context` writes one `counter_<name>` numeric
    /// param per entry in `entity.counters`. `Condition::Counter` reads
    /// these to gate completion chains for kill-counter missions like
    /// the Mess Hall (counter `messhall_kills`) and Hallway05 (counter
    /// `hallway05_kills`).
    #[test]
    fn populate_counters_context_writes_counter_keys() {
        let mut entity = CellEntity::new(EntityId(1), SpaceId(100), Vector3::zero());
        entity.counters.insert("messhall_kills".to_string(), 1);
        entity.counters.insert("hallway05_kills".to_string(), 0);

        let mut ctx = ExecutionContext::new();
        populate_counters_context(&entity, &mut ctx);

        assert_eq!(
            ctx.params
                .get("counter_messhall_kills")
                .and_then(|v| v.as_i64()),
            Some(1),
            "non-zero counter must populate as its current value",
        );
        assert_eq!(
            ctx.params
                .get("counter_hallway05_kills")
                .and_then(|v| v.as_i64()),
            Some(0),
            "zero-valued counter must populate explicitly (not be elided) — \
             a `Counter::Eq 0` condition would otherwise see the missing-key \
             default and could miss the genuinely-zero case",
        );
    }

    /// Empty `entity.counters` must not pollute the context — chains
    /// that don't use counters should see a clean slate.
    #[test]
    fn populate_counters_context_empty_when_no_counters() {
        let entity = CellEntity::new(EntityId(1), SpaceId(100), Vector3::zero());
        let mut ctx = ExecutionContext::new();
        populate_counters_context(&entity, &mut ctx);
        assert!(
            ctx.params.is_empty(),
            "no counters → no params written; got {:?}",
            ctx.params
        );
    }

    /// `populate_stats_context` mirrors mission population but for
    /// `entity.stats`. `Condition::StatBelowMax` reads `stat_<id>_cur`
    /// and `stat_<id>_max` to gate consumable chains.
    #[test]
    fn populate_stats_context_writes_cur_and_max_for_each_stat() {
        use cimmeria_entity::stats::{Stat, HEALTH};

        let mut entity = CellEntity::new(EntityId(1), SpaceId(100), Vector3::zero());
        // StatList is initialized with all default stats; overwrite HEALTH
        // so we have known values to assert against.
        if let Some(s) = entity.stats.get_mut(HEALTH) {
            *s = Stat::new(0, 250, 1000, 0, 250, 1000);
        }

        let mut ctx = ExecutionContext::new();
        populate_stats_context(&entity, &mut ctx);

        let cur_key = format!("stat_{}_cur", HEALTH);
        let max_key = format!("stat_{}_max", HEALTH);
        assert_eq!(
            ctx.params.get(&cur_key).and_then(|v| v.as_i64()),
            Some(250),
            "stat cur must populate verbatim — `Condition::StatBelowMax` reads it",
        );
        assert_eq!(
            ctx.params.get(&max_key).and_then(|v| v.as_i64()),
            Some(1000),
        );
    }
}
