use serde_json::{json, Value};

use super::mission::{build_mission, MissionInput, Sample};

fn sample(value: Value) -> Result<Sample, String> {
    Ok(Sample { at_ms: 900, value })
}

fn fixture() -> MissionInput {
    MissionInput {
        instance: "default".into(),
        mission_id: 622,
        definition: sample(json!({"mission_id": 622, "mission_label": "Arm Yourself!"})),
        steps: sample(json!([
            {"step_id": 2113, "mission_id": 622, "step_display_log_text": "Search Frost"},
            {"step_id": 80623, "mission_id": 622, "step_display_log_text": "Search the Guard"},
        ])),
        chains: sample(json!([{"chain_id": 1003, "scope_type": "mission", "scope_id": 622}])),
        server: sample(json!({"mission_id": 622, "current_step_id": 80623, "status": 1})),
        wire: Err("tap unbound".into()),
        client: Err("client mission state unavailable".into()),
        ui: sample(json!({"mission_tracker": [
            {"kind": "mission", "text": "Arm Yourself!"},
            {"kind": "step", "text": "Search Frost"},
            {"kind": "objective", "text": "Search Frost's body"},
        ]})),
        now_ms: 1000,
    }
}

#[test]
fn persisted_step_and_visible_tracker_disagree_without_rewriting_state() {
    let model = build_mission(fixture());
    assert_eq!(model["data"]["server"]["value"]["current_step_id"], 80623);
    assert_eq!(
        model["data"]["matched_tracker"]["step_text"],
        "Search Frost"
    );
    assert_eq!(
        model["data"]["active_step"]["description"],
        "Search the Guard"
    );
    assert_eq!(model["data"]["step_disagreement"], true);
    assert_eq!(model["data"]["server"]["source"], "sgw_mission");
    assert_eq!(model["data"]["ui"]["age_ms"], 100);
    assert!(model["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["code"] == "step_disagreement"));
}

#[test]
fn missing_definition_is_unknown_and_tracker_cannot_bind_to_id() {
    let mut input = fixture();
    input.definition = Err("database disconnected".into());
    let model = build_mission(input);
    assert!(model["data"]["definition"]["value"].is_null());
    assert!(model["data"]["matched_tracker"].is_null());
    assert!(model["data"]["step_disagreement"].is_null());
    assert_eq!(model["data"]["server"]["value"]["current_step_id"], 80623);
    assert!(model["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["source"] == "resources.missions"));
}

#[test]
fn missing_bound_server_state_is_unknown_even_when_tracker_has_text() {
    let mut input = fixture();
    input.server = Err("trusted client-to-server session binding unavailable".into());
    let model = build_mission(input);
    assert!(model["data"]["server"]["value"].is_null());
    assert_eq!(
        model["data"]["matched_tracker"]["step_text"],
        "Search Frost"
    );
    assert!(model["data"]["active_step"]["id"].is_null());
    assert!(model["data"]["step_disagreement"].is_null());
}

#[test]
fn duplicate_visible_titles_do_not_attribute_a_tracker_step() {
    let mut input = fixture();
    input.ui = sample(json!({"mission_tracker": [
        {"kind": "mission", "text": "Arm Yourself!"},
        {"kind": "step", "text": "Search Frost"},
        {"kind": "mission", "text": "Arm Yourself!"},
        {"kind": "step", "text": "Search the Guard"},
    ]}));
    let model = build_mission(input);
    assert!(model["data"]["matched_tracker"].is_null());
    assert!(model["data"]["step_disagreement"].is_null());
}

#[test]
fn completed_mission_does_not_report_a_stale_step_as_active() {
    let mut input = fixture();
    input.server.as_mut().unwrap().value["status"] = json!(2);
    let model = build_mission(input);
    assert_eq!(model["data"]["server"]["value"]["current_step_id"], 80623);
    assert!(model["data"]["active_step"]["id"].is_null());
    assert!(model["data"]["step_disagreement"].is_null());
}
