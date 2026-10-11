//! The heal hint (SV-11, README F15): when a row's first failed step is a
//! world click, target or camera move on a pose (SV-07) that missed in
//! the world, the hint names that pose and the step, so `lab uat -Heal`
//! can recalibrate the pose without parsing the failure's prose.

use serde_json::Value;

use super::HealHint;
use crate::uat::evidence::RowEvidence;
use crate::uat::tier::Role;

/// The world-tool steps a recalibrated pose can fix (`supervisor/world`).
const POSE_STEPS: [&str; 5] = ["hover", "on_screen", "face", "abs_pitch", "abs_yaw"];

/// The hint for a row, from its first failed step action; `None` when that
/// action is not a pose click miss, or when no step failed.
pub(crate) fn heal_hint(ev: &RowEvidence) -> Option<HealHint> {
    let failed = ev.actions.iter().find(|a| a.role == Role::Step && !a.ok)?;
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
    let step = failed
        .result
        .get("error_data")
        .and_then(|d| d.get("step"))
        .and_then(Value::as_str)?;
    POSE_STEPS.contains(&step).then(|| HealHint {
        pose: pose.to_string(),
        step: step.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::uat::evidence::{ActionRecord, RowResult};
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
        }
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
