//! Rebuild [`MissionInstance`]s from the persisted `sgw_mission` rows.
//!
//! Split out of the `InitPlayerState` handler as a pure function so the
//! relog half of the mission state machine is testable without standing
//! up the whole world-entry burst — and so a chain-replay test can drive
//! the real accept → persist → hydrate path end to end.
//!
//! Harset H50: this used to hardcode `hidden: false, optional: false` on
//! every restored objective. A restored optional objective therefore
//! counted as required in
//! [`crate::cell::missions::complete_objective`]'s `all_required_complete`
//! check, so a mission whose last open objective was optional could never
//! complete after a relog (live for mission 1200's optional 5399). The
//! flags come back from the mission definition rather than from a new
//! column: they are a property of `resources.mission_objectives`, not of
//! the player's progress, so the seed stays the single source of truth
//! and `sgw_mission` needs no schema change.

use cimmeria_entity::missions::{MissionInstance, MissionObjective, STATUS_ACTIVE, STATUS_COMPLETED};

use crate::cell::messages::SavedMission;
use crate::cell::space_manager::SpaceManager;

/// Rebuild the current step's objective roster.
///
/// **The roster comes from the step definition, not from
/// `saved.active_objective_ids`.** That is deliberate and is what makes
/// this self-healing: every `sgw_mission` row written before H50 holds
/// the *step* id in the objective array, and a saved-array-driven
/// hydration would turn that into a required, ACTIVE pseudo-objective
/// that no `complete_objective` can ever satisfy (`MissionInstance::
/// complete_objective` matches by id). `all_required_complete` would then
/// be permanently false and the auto-complete path bricked for every
/// player who is mid-mission at deploy time. Reconstructing from
/// `resources.mission_objectives` repairs those rows on first login —
/// and since the repo does not do DB migrations, hydration is the only
/// lever available.
///
/// The saved arrays are used purely as a *status overlay*: an id in
/// `completed_objective_ids` comes back `STATUS_COMPLETED`, everything
/// else `STATUS_ACTIVE`.
///
/// Fallback: if the step isn't in the cache (unseeded step, or a
/// completed/failed row with `current_step_id = NULL`) the saved array is
/// used verbatim with `(false, false)` flags — the pre-H50 behaviour —
/// so an unseeded mission degrades rather than losing its roster.
fn restore_objectives(space_mgr: &SpaceManager, saved: &SavedMission) -> Vec<MissionObjective> {
    let status_of = |oid: i32| {
        if saved.completed_objective_ids.contains(&oid) {
            STATUS_COMPLETED
        } else {
            STATUS_ACTIVE
        }
    };

    let defs = saved
        .current_step_id
        .map(|sid| space_mgr.get_step_objectives(sid))
        .unwrap_or_default();

    if defs.is_empty() {
        if !saved.active_objective_ids.is_empty() {
            tracing::warn!(
                mission_id = saved.mission_id,
                current_step_id = ?saved.current_step_id,
                objectives = saved.active_objective_ids.len(),
                "Restoring mission objectives without a step definition — \
                 hidden/optional flags default to false"
            );
        }
        return saved
            .active_objective_ids
            .iter()
            .map(|&objective_id| MissionObjective {
                objective_id,
                status: status_of(objective_id),
                hidden: false,
                optional: false,
            })
            .collect();
    }

    // Anything in the saved array that the step doesn't define is either
    // the pre-H50 step-id corruption or an objective the seed has since
    // dropped. Either way it must not join the roster, or it becomes a
    // required objective that can never be completed.
    for &oid in &saved.active_objective_ids {
        if !defs.iter().any(|d| d.objective_id == oid) {
            tracing::warn!(
                mission_id = saved.mission_id,
                current_step_id = ?saved.current_step_id,
                objective_id = oid,
                "Saved objective id is not defined for the current step — \
                 dropping it from the restored roster (pre-H50 rows stored the \
                 step id here)"
            );
        }
    }

    defs.into_iter()
        .map(|d| MissionObjective {
            objective_id: d.objective_id,
            status: status_of(d.objective_id),
            hidden: d.is_hidden,
            optional: d.is_optional,
        })
        .collect()
}

/// Turn the DB rows into live mission instances.
///
/// Array semantics mirror [`crate::cell::missions::mission_update_msg`]:
/// `active_objective_ids` is the full objective roster of the current
/// step (completed entries included). Ids that appear *only* in
/// `completed_objective_ids` belong to steps the player has advanced
/// past; they stay in `MissionInstance::completed_objectives`, which is
/// where `populate_mission_context`'s second loop reads them.
pub(crate) fn build_restored_missions(
    saved: &[SavedMission],
    space_mgr: &SpaceManager,
) -> Vec<MissionInstance> {
    saved
        .iter()
        .map(|saved| {
            let objectives = restore_objectives(space_mgr, saved);

            let mut mission = MissionInstance::new(
                saved.mission_id,
                saved.current_step_id.unwrap_or(0),
                objectives,
            );
            mission.status = saved.status;
            mission.completed_steps = saved.completed_step_ids.clone();
            mission.completed_objectives = saved.completed_objective_ids.clone();
            // Without this, `complete()` on a re-accepted repeatable
            // mission post-relog would jump from 0 -> 1 instead of
            // N -> N+1, defeating the numRepeats cap. (#118)
            mission.repeats = saved.repeats;
            // `MissionInstance::new` defaults `is_hidden` to false while
            // `accept_mission` sets it from the def, so without this a
            // hidden sub-mission (682-686, the Hallway0N Controllers)
            // leaked into the player's mission log on the first relog —
            // `MissionManager::active_missions` filters on the
            // per-instance flag.
            mission.is_hidden = space_mgr
                .mission_defs
                .get(&saved.mission_id)
                .is_some_and(|d| d.is_hidden);

            mission
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::spawner::{MissionDefEntry, MissionObjectiveDef};

    const MISSION: i32 = 742;
    const STEP: i32 = 2504;

    fn saved(active: Vec<i32>, completed: Vec<i32>) -> SavedMission {
        SavedMission {
            mission_id: MISSION,
            status: 1,
            current_step_id: Some(STEP),
            completed_step_ids: vec![2502, 2503],
            completed_objective_ids: completed,
            active_objective_ids: active,
            failed_objective_ids: vec![],
            repeats: 0,
        }
    }

    /// A space manager whose step cache knows step 2504's three devices,
    /// with 2915 marked hidden+optional so the flag carry is observable.
    fn mgr_with_defs() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        mgr.step_objectives.insert(
            STEP,
            vec![
                MissionObjectiveDef {
                    objective_id: 2913,
                    is_hidden: false,
                    is_optional: false,
                },
                MissionObjectiveDef {
                    objective_id: 2914,
                    is_hidden: false,
                    is_optional: false,
                },
                MissionObjectiveDef {
                    objective_id: 2915,
                    is_hidden: true,
                    is_optional: true,
                },
            ],
        );
        mgr.mission_defs.insert(
            MISSION,
            MissionDefEntry {
                step_id: 2502,
                objectives: vec![],
                is_hidden: false,
                num_repeats: 0,
                can_repeat_on_fail: false,
            },
        );
        mgr
    }

    /// The H50 flag carry: `hidden`/`optional` come from the step
    /// definition, not from a hardcoded `false`. Reverting this makes
    /// 2915 come back required, and
    /// `progression::complete_objective`'s `all_required_complete` then
    /// waits forever on an objective the player never has to do.
    #[test]
    fn restores_hidden_and_optional_from_the_step_definition() {
        let mgr = mgr_with_defs();
        let restored = build_restored_missions(&[saved(vec![2913, 2914, 2915], vec![])], &mgr);

        let objs = &restored[0].active_objectives;
        assert_eq!(objs.len(), 3);
        assert_eq!(
            (objs[0].hidden, objs[0].optional),
            (false, false),
            "2913 is a plain required objective"
        );
        assert_eq!(
            (objs[2].hidden, objs[2].optional),
            (true, true),
            "2915 must come back hidden+optional — pre-H50 both were hardcoded false",
        );
    }

    /// An id in both arrays is a completed objective of the *current*
    /// step; it stays on the roster (so `all_required_complete` still
    /// sees it) but with `STATUS_COMPLETED`.
    #[test]
    fn objective_in_both_arrays_restores_as_completed_and_stays_on_the_roster() {
        let mgr = mgr_with_defs();
        let restored = build_restored_missions(&[saved(vec![2913, 2914, 2915], vec![2913])], &mgr);
        let m = &restored[0];

        assert_eq!(
            m.active_objectives.len(),
            3,
            "the completed objective must not be dropped from the step roster — \
             `all_required_complete` iterates this list",
        );
        assert_eq!(m.active_objectives[0].status, STATUS_COMPLETED);
        assert_eq!(m.active_objectives[1].status, STATUS_ACTIVE);
        assert_eq!(m.completed_objectives, vec![2913]);
    }

    /// An id that appears only in `completed_objective_ids` belongs to a
    /// step the player advanced past. It must not be resurrected onto
    /// the active roster (that would make it count toward the current
    /// step's completion), but it must survive in
    /// `completed_objectives` so `objective_status … eq completed`
    /// still matches — that is the `688 / 2734` shape.
    #[test]
    fn objective_from_a_prior_step_stays_completed_only() {
        let mgr = mgr_with_defs();
        let restored = build_restored_missions(&[saved(vec![2913], vec![2913, 2734])], &mgr);
        let m = &restored[0];

        assert!(
            !m.active_objectives.iter().any(|o| o.objective_id == 2734),
            "a prior step's objective must not rejoin the current roster",
        );
        assert!(
            m.completed_objectives.contains(&2734),
            "prior-step objective must survive hydration"
        );
    }

    /// The self-heal. Every `sgw_mission` row written before H50 holds
    /// the STEP id in `active_objective_ids`. Hydrating that verbatim
    /// yields a required, ACTIVE objective with a step id that nothing
    /// can complete — `all_required_complete` stays false forever and
    /// the mission can never auto-advance. Def-driven reconstruction
    /// drops it and restores the real roster.
    #[test]
    fn legacy_row_holding_the_step_id_is_repaired_from_the_step_definition() {
        let mgr = mgr_with_defs();
        // What every pre-H50 accept/advance_step wrote.
        let restored = build_restored_missions(&[saved(vec![STEP], vec![])], &mgr);
        let m = &restored[0];

        let ids: Vec<i32> = m
            .active_objectives
            .iter()
            .map(|o| o.objective_id)
            .collect();
        assert_eq!(
            ids,
            vec![2913, 2914, 2915],
            "the roster must be rebuilt from resources.mission_objectives",
        );
        assert!(
            !ids.contains(&STEP),
            "the step id must not survive as a pseudo-objective — nothing can \
             ever complete it, so the mission would be permanently stuck",
        );
    }

    /// With no step definition cached (unseeded step, or a completed row
    /// whose `current_step_id` is NULL) the saved array is used verbatim
    /// rather than emptying the roster.
    #[test]
    fn falls_back_to_the_saved_array_when_the_step_has_no_definition() {
        let mgr = mgr_with_defs();
        let mut row = saved(vec![999_999], vec![]);
        row.current_step_id = Some(4242); // not in the cache

        let restored = build_restored_missions(&[row], &mgr);
        let objs = &restored[0].active_objectives;
        assert_eq!(objs.len(), 1);
        assert_eq!(objs[0].objective_id, 999_999);
        assert_eq!((objs[0].hidden, objs[0].optional), (false, false));
    }

    /// `repeats`, `status`, `completed_steps` and the mission-level
    /// `is_hidden` all have to survive; `is_hidden` is the one that had
    /// no restore at all before H50.
    #[test]
    fn restores_mission_level_fields_including_is_hidden() {
        let mut mgr = mgr_with_defs();
        if let Some(d) = mgr.mission_defs.get_mut(&MISSION) {
            d.is_hidden = true;
        }
        let mut row = saved(vec![2913], vec![]);
        row.repeats = 3;
        row.status = 2;

        let restored = build_restored_missions(&[row], &mgr);
        let m = &restored[0];
        assert_eq!(m.repeats, 3);
        assert_eq!(m.status, 2);
        assert_eq!(m.completed_steps, vec![2502, 2503]);
        assert!(
            m.is_hidden,
            "a hidden mission must stay hidden across a relog — otherwise it \
             appears in the player's log on next login",
        );
    }

    /// The mission-1200 bug shape, end to end through the progression
    /// helper: a restored optional objective must not be treated as
    /// required. With `optional` hardcoded to `false` at hydration,
    /// completing the last *required* objective leaves
    /// `all_required_complete` false — the mission can never finish and
    /// the player is stuck.
    #[tokio::test]
    async fn restored_optional_objective_does_not_block_mission_completion() {
        use cimmeria_entity::missions::MISSION_COMPLETED;
        use tokio::sync::mpsc;

        let mut mgr = mgr_with_defs();
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

        // Relog state: 2913 and 2914 (required) open, 2915 hidden+optional open.
        let restored = build_restored_missions(&[saved(vec![2913, 2914, 2915], vec![])], &mgr);
        assert!(
            restored[0].active_objectives[2].optional,
            "precondition: 2915 restored as optional",
        );
        mgr.get_entity_mut(1)
            .unwrap()
            .missions
            .add_mission(restored.into_iter().next().unwrap());

        let (tx, _rx) = mpsc::channel(64);
        crate::cell::missions::complete_objective(1, MISSION, 2913, &tx, &mut mgr).await;
        assert_ne!(
            mgr.get_entity(1).unwrap().missions.get_mission(MISSION).unwrap().status,
            MISSION_COMPLETED,
            "2914 is still open — the mission must not complete yet",
        );
        crate::cell::missions::complete_objective(1, MISSION, 2914, &tx, &mut mgr).await;

        assert_eq!(
            mgr.get_entity(1).unwrap().missions.get_mission(MISSION).unwrap().status,
            MISSION_COMPLETED,
            "both required objectives are done; the still-open optional 2915 must \
             not hold the mission open — pre-H50 it was restored as required and did",
        );
    }
}
