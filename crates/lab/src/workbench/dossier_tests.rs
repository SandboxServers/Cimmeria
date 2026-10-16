use serde_json::{json, Value};

use super::{
    build_dossier, checked_client_row, matching_messages, read_messages, DossierInput, Sample,
};
use crate::uat::invoke::{BoxedOutcome, ServerInvoker, ToolOutcome};
use crate::workbench::context::CaptureContext;

fn sample(value: Value) -> Result<Sample, String> {
    Ok(Sample { at_ms: 950, value })
}

fn fixture() -> DossierInput {
    let client_context = CaptureContext {
        schema_version: 1,
        instance: "default".into(),
        client_pid: 1,
        client_fingerprint: "fixture".into(),
        player_entity_id: Some(7),
        selected_entity_id: Some(42),
        server_session_id: Some("session-B".into()),
        host_ms: 950,
        monotonic_ms: 950,
        server_offset_ms: None,
        source: vec!["client".into()],
        gaps: vec![],
    };
    DossierInput {
        instance: "default".into(),
        client_pid: Some(1),
        entity_id: 42,
        selected_before: sample(json!(42)),
        selected_after: sample(json!(42)),
        client: sample(json!({
            "entity_id": 42, "actor": "0x1234", "rendered": true,
            "targetable": true, "mob_id": 900,
            "position": { "x": 100.0, "y": 300.0, "z": 200.0 },
        })),
        client_context: Some(client_context),
        sessions: sample(json!({ "sessions": [
            { "entity_id": 7, "session": "session-B" }
        ] })),
        server: sample(json!({
            "entity_id": 42, "template_id": 901,
            "health_cur": 10, "health_max": 20, "spawn_id": 15,
            "position": [3.0, 2.0, 1.0],
        })),
        witnesses: Err("server_witnesses timed out".into()),
        messages: sample(json!([])),
        log_tail_truncated: false,
        now_ms: 1000,
    }
}

#[test]
fn same_numeric_player_on_another_server_session_never_joins() {
    let mut input = fixture();
    input.sessions = sample(json!({ "sessions": [
        { "entity_id": 7, "session": "session-A" }
    ] }));
    let dossier = build_dossier(input);
    assert_eq!(dossier["data"]["client"]["mob_id"]["value"], Value::Null);
    assert_eq!(
        dossier["data"]["server"]["template_id"]["value"],
        Value::Null
    );
    assert_eq!(dossier["data"]["server"]["witnesses"]["value"], Value::Null);
    assert_eq!(dossier["data"]["position_delta"]["value"], Value::Null);
    assert_eq!(dossier["data"]["recent_messages"]["value"], Value::Null);
    assert!(dossier["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["code"] == "server_session_mismatch"));
}

#[test]
fn target_slot_metadata_from_another_entity_is_never_attributed_to_selected_id() {
    let mut input = fixture();
    input.client.as_mut().unwrap().value["slot_entity_id"] = json!(43);
    input.client.as_mut().unwrap().value["targetable"] = json!(true);
    input.client.as_mut().unwrap().value["mob_id"] = json!(943);
    input.client.as_mut().unwrap().value["name"] = json!("entity 43");
    let dossier = build_dossier(input);
    assert_eq!(dossier["context"]["current"], true);
    for field in ["targetable", "mob_id", "name"] {
        assert_eq!(dossier["data"]["client"][field]["value"], Value::Null);
        assert_eq!(dossier["data"]["client"][field]["at_ms"], Value::Null);
    }
    assert!(dossier["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["source"] == "client.target_slot" && g["code"] == "slot_metadata_unbound"));
}

#[test]
fn position_delta_age_uses_the_older_of_client_and_server_samples() {
    let mut input = fixture();
    input.client.as_mut().unwrap().at_ms = 990;
    input.server.as_mut().unwrap().at_ms = 100;
    let dossier = build_dossier(input);
    assert_eq!(
        dossier["data"]["position_delta"]["value"]["distance_m"],
        0.0
    );
    assert_eq!(dossier["data"]["position_delta"]["at_ms"], 100);
    assert_eq!(dossier["data"]["position_delta"]["age_ms"], 900);
}

#[test]
fn server_binding_requires_same_instance_selected_id_player_and_unique_roster_row() {
    for input in [
        {
            let mut f = fixture();
            f.client_context.as_mut().unwrap().instance = "other".into();
            f
        },
        {
            let mut f = fixture();
            f.client_context.as_mut().unwrap().selected_entity_id = Some(43);
            f
        },
        {
            let mut f = fixture();
            f.client_context.as_mut().unwrap().player_entity_id = Some(8);
            f
        },
        {
            let mut f = fixture();
            f.sessions = sample(json!({ "sessions": [
            { "entity_id": 7, "session": "session-B" },
            { "entity_id": 7, "session": "session-B" }
        ] }));
            f
        },
        {
            let mut f = fixture();
            f.sessions = sample(json!({ "sessions": [
                { "entity_id": 7, "session": "session-B" },
                { "entity_id": 8, "session": "session-B" }
            ] }));
            f
        },
        {
            let mut f = fixture();
            f.sessions = sample(json!({ "sessions": [
                { "entity_id": 7, "session": "session-B" },
                { "entity_id": 7, "session": "session-C" }
            ] }));
            f
        },
    ] {
        let dossier = build_dossier(input);
        assert_eq!(
            dossier["data"]["server"]["template_id"]["value"],
            Value::Null
        );
        assert_eq!(dossier["data"]["position_delta"]["value"], Value::Null);
        assert!(dossier["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["code"] == "server_session_mismatch"));
    }
}

#[test]
fn absent_client_session_binding_fails_closed() {
    let mut input = fixture();
    input.client_context = None;
    let dossier = build_dossier(input);
    assert_eq!(
        dossier["data"]["server"]["template_id"]["value"],
        Value::Null
    );
    assert!(dossier["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["code"] == "server_session_unbound"));
}

#[test]
fn stale_client_process_capture_cannot_bind_the_server() {
    let mut input = fixture();
    input.client_pid = Some(2);
    let dossier = build_dossier(input);
    assert_eq!(
        dossier["data"]["server"]["template_id"]["value"],
        Value::Null
    );
    assert!(dossier["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["code"] == "server_session_mismatch"));
}

#[test]
fn client_table_requires_matching_embedded_id_and_stable_pointer() {
    let good =
        json!({"entities": {"details": [{"id": 42, "id_field_matches": true, "ptr": "0x1234"}]}});
    assert!(checked_client_row(&good, 42, 0x1234).is_ok());
    for row in [
        json!({"id": 42, "id_field_matches": false, "ptr": "0x1234"}),
        json!({"id": 42, "id_field_matches": true, "ptr": "0x9999"}),
    ] {
        let table = json!({"entities": {"details": [row]}});
        assert!(checked_client_row(&table, 42, 0x1234).is_err());
    }
}

#[test]
fn actor_pose_failure_has_a_client_gap() {
    let mut input = fixture();
    input.client.as_mut().unwrap().value["pose_error"] = json!("actor read failed");
    input.client.as_mut().unwrap().value["position"] = Value::Null;
    let dossier = build_dossier(input);
    assert_eq!(
        dossier["data"]["client"]["position_ue3"]["value"],
        Value::Null
    );
    assert!(dossier["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["code"] == "pose_unavailable"));
}

struct FakeLog;

impl ServerInvoker for FakeLog {
    fn url(&self) -> &str {
        "fake"
    }

    fn call<'a>(&'a self, name: &'a str, args: Value) -> BoxedOutcome<'a> {
        assert_eq!(name, "server_log_tail");
        let limit = args["limit"].as_u64().unwrap() as usize;
        Box::pin(async move {
            let entries: Vec<Value> = (0..101)
                .map(|n| {
                    json!({
                        "fields": { "entity_id": if n == 0 { 42 } else { 99 } }, "message": n
                    })
                })
                .collect();
            let entries = entries[101usize.saturating_sub(limit)..].to_vec();
            ToolOutcome {
                ok: true,
                json: json!({"total_buffered": 101, "entries": entries}),
                error: None,
                images: vec![],
                error_data: None,
            }
        })
    }
}

#[tokio::test]
async fn log_tail_keeps_structured_match_at_row_101_and_signals_truncation() {
    let (sample, truncated) = read_messages(&FakeLog, 42).await.unwrap();
    assert_eq!(sample.value.as_array().unwrap().len(), 1);
    assert_eq!(sample.value[0]["message"], 0);
    assert!(!truncated);
    let entries: Vec<Value> = (0..101)
        .map(|n| {
            json!({
                "fields": { "entity_id": if n == 0 { 42 } else { 99 } }, "message": n
            })
        })
        .collect();
    let outcome = ToolOutcome {
        ok: true,
        json: json!({"total_buffered": 101, "entries": entries}),
        error: None,
        images: vec![],
        error_data: None,
    };
    let (_, truncated) = matching_messages(outcome, 42).unwrap();
    assert!(!truncated);
    let outcome = ToolOutcome {
        ok: true,
        json: json!({"total_buffered": 101,
        "entries": [{"fields": {"entity_id": 42}, "message": "newest only"}]}),
        error: None,
        images: vec![],
        error_data: None,
    };
    assert!(matching_messages(outcome, 42).unwrap().1);
    let mut input = fixture();
    input.log_tail_truncated = true;
    let dossier = build_dossier(input);
    assert!(dossier["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["code"] == "log_tail_truncated"));
}

#[test]
fn unbound_client_mob_and_server_template_keep_distinct_sources() {
    let dossier = build_dossier(fixture());
    assert_eq!(dossier["context"]["current"], true);
    assert_eq!(dossier["data"]["client"]["mob_id"]["value"], Value::Null);
    assert_eq!(
        dossier["data"]["client"]["mob_id"]["source"],
        "client.unit_api"
    );
    assert_eq!(dossier["data"]["server"]["template_id"]["value"], 901);
    assert_eq!(
        dossier["data"]["server"]["template_id"]["source"],
        "server_entity_get"
    );
    assert_eq!(dossier["data"]["server"]["template_id"]["age_ms"], 50);
    assert!(dossier["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["code"] == "slot_metadata_unbound"));
    assert_eq!(
        dossier["data"]["position_delta"]["value"]["distance_m"],
        0.0
    );
    assert_eq!(
        dossier["data"]["server"]["witnessed_by"]["value"],
        Value::Null
    );
    assert!(dossier["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["source"] == "server.witnesses"));
}

#[test]
fn missing_server_is_unknown_not_empty_or_guessed() {
    let mut input = fixture();
    input.server = Err("endpoint offline".into());
    let dossier = build_dossier(input);
    assert_eq!(
        dossier["data"]["server"]["template_id"]["value"],
        Value::Null
    );
    assert_eq!(
        dossier["data"]["server"]["position_m"]["value"],
        Value::Null
    );
    assert_eq!(dossier["data"]["position_delta"]["value"], Value::Null);
    assert!(dossier["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["detail"] == "endpoint offline"));
}

#[test]
fn target_change_invalidates_every_entity_field() {
    let mut input = fixture();
    input.selected_after = sample(json!(43));
    let dossier = build_dossier(input);
    assert_eq!(dossier["context"]["current"], false);
    assert_eq!(dossier["context"]["selected_entity_id"]["value"], 43);
    assert_eq!(dossier["data"]["client"]["actor"]["value"], Value::Null);
    assert_eq!(
        dossier["data"]["server"]["template_id"]["value"],
        Value::Null
    );
    assert_eq!(dossier["data"]["recent_messages"]["value"], Value::Null);
    assert!(dossier["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["code"] == "target_changed"));
}

#[test]
fn wrong_entity_id_does_not_join_on_name_or_template() {
    let mut input = fixture();
    input.server = sample(json!({ "entity_id": 77, "template_id": 900, "name": "same" }));
    let dossier = build_dossier(input);
    assert_eq!(
        dossier["data"]["server"]["template_id"]["value"],
        Value::Null
    );
    assert!(dossier["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["code"] == "id_mismatch"));
}
