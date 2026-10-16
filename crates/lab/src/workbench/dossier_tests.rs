use serde_json::{json, Value};

use super::{build_dossier, DossierInput, Sample};

fn sample(value: Value) -> Result<Sample, String> {
    Ok(Sample { at_ms: 950, value })
}

fn fixture() -> DossierInput {
    DossierInput {
        instance: "default".into(),
        entity_id: 42,
        selected_before: sample(json!(42)),
        selected_after: sample(json!(42)),
        client: sample(json!({
            "entity_id": 42, "actor": "0x1234", "rendered": true,
            "targetable": true, "mob_id": 900,
            "position": { "x": 100.0, "y": 300.0, "z": 200.0 },
        })),
        server: sample(json!({
            "entity_id": 42, "template_id": 901,
            "health_cur": 10, "health_max": 20, "spawn_id": 15,
            "position": [3.0, 2.0, 1.0],
        })),
        witnesses: Err("server_witnesses timed out".into()),
        messages: sample(json!([])),
        now_ms: 1000,
    }
}

#[test]
fn mismatched_client_mob_and_server_template_keep_distinct_sources() {
    let dossier = build_dossier(fixture());
    assert_eq!(dossier["context"]["current"], true);
    assert_eq!(dossier["data"]["client"]["mob_id"]["value"], 900);
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
