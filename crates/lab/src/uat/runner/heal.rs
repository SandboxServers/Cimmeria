//! The heal hint (SV-11, README F15): when a row's first failed step is a
//! world click, target or camera move on a pose (SV-07) that missed in
//! the world, the hint names that pose and the step, so `lab uat -Heal`
//! can recalibrate the pose without parsing the failure's prose.

use serde_json::Value;

use super::HealHint;
use crate::uat::evidence::{RowEvidence, RowResult};
use crate::uat::tier::Role;

/// The world-tool steps a recalibrated pose can fix (`supervisor/world`).
const POSE_STEPS: [&str; 5] = ["hover", "on_screen", "face", "abs_pitch", "abs_yaw"];

/// The camera steps whose failure is a heal case only when the loop ran
/// and missed ("the camera stopped at ..."); a missing readout or an
/// unusable gain is infrastructure, which a recalibrated pose cannot fix.
const CONVERGENCE_STEPS: [&str; 2] = ["abs_pitch", "abs_yaw"];

/// The summary's hint: never on a PASS row, whatever its actions say.
pub(crate) fn summary_heal(ev: &RowEvidence) -> Option<HealHint> {
    if ev.result == RowResult::Pass {
        None
    } else {
        heal_hint(ev)
    }
}

/// The hint for a row, from its first failed required step action; `None`
/// when that action is not a pose click miss, or when no required step
/// failed. An optional action's failure never stopped the row, so it
/// neither gives nor hides a hint.
pub(crate) fn heal_hint(ev: &RowEvidence) -> Option<HealHint> {
    let failed = ev
        .actions
        .iter()
        .find(|a| a.role == Role::Step && !a.ok && !a.optional)?;
    let tool = failed.tool.as_deref()?;
    if !matches!(
        tool,
        "client_world_click" | "client_target" | "client_camera"
    ) {
        return None;
    }
    let pose = failed
        .calls
        .iter()
        .find_map(|c| c.get("resolve")?.get("pose")?.as_str())?;
    let data = failed.result.get("error_data")?;
    let step = data.get("step").and_then(Value::as_str)?;
    if !POSE_STEPS.contains(&step) {
        return None;
    }
    if CONVERGENCE_STEPS.contains(&step) {
        // The world error's own message, else the MCP error's text.
        let message = data
            .get("error")
            .and_then(Value::as_str)
            .or(failed.error.as_deref())
            .unwrap_or_default();
        if !message.contains("stopped at") {
            return None;
        }
    }
    Some(HealHint {
        pose: pose.to_string(),
        step: step.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::uat::evidence::ActionRecord;
    use crate::uat::golden::testkit::row_evidence;
    use crate::uat::tier::Tier;

    /// A step action on `tool`, failed or not, with a pose `resolve` call
    /// when `pose` is set and `error_data` when `step` is set.
    fn step(tool: &str, ok: bool, pose: Option<&str>, step: Option<&str>) -> ActionRecord {
        let mut calls = Vec::new();
        if let Some(pose) = pose {
            calls.push(json!({ "resolve": {
                "tag": "ArmYourself_FrostBody", "entity_id": 100_751,
                "space_id": 3, "pose": pose,
            }}));
        }
        let result = match step {
            Some(s) => json!({ "result": null, "error_data": { "step": s } }),
            None => json!({ "ok": ok }),
        };
        ActionRecord {
            index: 0,
            role: Role::Step,
            label: Some("go".into()),
            kind: "tool".into(),
            requested: format!("@{tool}"),
            tool: Some(tool.into()),
            args: json!({}),
            tier: Some(Tier::N1),
            tier_source: None,
            fallback_used: false,
            host_started_ms: 0,
            elapsed_ms: 0,
            ok,
            error: (!ok).then(|| "missed".to_string()),
            result,
            calls,
            client: None,
            optional: false,
        }
    }

    /// `rec`, marked optional in the spec.
    fn optional(mut rec: ActionRecord) -> ActionRecord {
        rec.optional = true;
        rec
    }

    /// A failed `client_camera` pose step whose world error says `message`.
    fn camera(step: &str, message: &str) -> ActionRecord {
        let mut rec = self::step("client_camera", false, Some("frost"), Some(step));
        rec.result["error_data"]["error"] = json!(message);
        rec.error = Some(format!("client_camera failed at {step}: {message}"));
        rec
    }

    fn row(actions: Vec<ActionRecord>) -> RowEvidence {
        let mut ev = row_evidence("SV-T", RowResult::Fail, &[]);
        ev.actions = actions;
        ev
    }

    #[test]
    fn a_hover_miss_on_a_pose_click_names_the_pose() {
        let ev = row(vec![step(
            "client_world_click",
            false,
            Some("frost"),
            Some("hover"),
        )]);
        assert_eq!(
            heal_hint(&ev),
            Some(HealHint {
                pose: "frost".into(),
                step: "hover".into(),
            })
        );
    }

    #[test]
    fn a_miss_without_a_pose_has_no_hint() {
        let ev = row(vec![step("client_world_click", false, None, Some("hover"))]);
        assert_eq!(heal_hint(&ev), None);
    }

    #[test]
    fn a_mission_clause_failure_has_no_hint() {
        let ev = row(vec![step("client_world_click", true, Some("frost"), None)]);
        assert_eq!(heal_hint(&ev), None);
    }

    #[test]
    fn the_first_failure_decides() {
        let ev = row(vec![
            step("client_lua_eval", false, None, None),
            step("client_world_click", false, Some("frost"), Some("hover")),
        ]);
        assert_eq!(heal_hint(&ev), None);
    }

    #[test]
    fn an_optional_failure_neither_gives_nor_hides_a_hint() {
        // A failed optional step did not stop the row: the pose miss after
        // it is the first real failure.
        let ev = row(vec![
            optional(step("client_lua_eval", false, None, None)),
            step("client_world_click", false, Some("frost"), Some("hover")),
        ]);
        assert_eq!(
            heal_hint(&ev),
            Some(HealHint {
                pose: "frost".into(),
                step: "hover".into(),
            })
        );

        // An optional pose miss before an unrelated required failure is
        // not what failed the row.
        let ev = row(vec![
            optional(step(
                "client_world_click",
                false,
                Some("frost"),
                Some("hover"),
            )),
            step("client_lua_eval", false, None, None),
        ]);
        assert_eq!(heal_hint(&ev), None);
    }

    #[test]
    fn a_pass_row_with_a_failed_optional_pose_click_has_no_heal_key() {
        let miss = || {
            optional(step(
                "client_world_click",
                false,
                Some("frost"),
                Some("hover"),
            ))
        };
        let mut ev = row(vec![miss()]);
        ev.result = RowResult::Pass;
        assert_eq!(summary_heal(&ev), None);
        // The PASS guard alone holds it back, should the action not be
        // marked optional.
        let mut required = miss();
        required.optional = false;
        ev.actions = vec![required];
        assert_eq!(summary_heal(&ev), None);
        assert!(heal_hint(&ev).is_some());
        let summary = super::super::RowSummary {
            section: "s".into(),
            row: "SV-T".into(),
            result: ev.result.as_str().into(),
            reasons: vec![],
            heal: summary_heal(&ev),
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert!(json.get("heal").is_none(), "{json}");
    }

    #[test]
    fn a_camera_convergence_miss_heals_but_a_readout_failure_does_not() {
        for s in ["abs_pitch", "abs_yaw"] {
            let ev = row(vec![camera(
                s,
                "the camera stopped at 12.00 deg, not -28.8 deg, after 6 looks",
            )]);
            assert_eq!(
                heal_hint(&ev),
                Some(HealHint {
                    pose: "frost".into(),
                    step: s.into(),
                }),
                "{s}"
            );
            for infra in ["no camera readout", "camera readout has no usable yaw_gain"] {
                let ev = row(vec![camera(s, infra)]);
                assert_eq!(heal_hint(&ev), None, "{s}: {infra}");
            }
        }
    }

    #[test]
    fn a_non_aiming_tool_has_no_hint() {
        let ev = row(vec![step(
            "client_lua_eval",
            false,
            Some("frost"),
            Some("hover"),
        )]);
        assert_eq!(heal_hint(&ev), None);
    }

    #[test]
    fn a_failure_outside_the_pose_steps_has_no_hint() {
        let ev = row(vec![step(
            "client_world_click",
            false,
            Some("frost"),
            Some("entity_gone"),
        )]);
        assert_eq!(heal_hint(&ev), None);
    }
}
