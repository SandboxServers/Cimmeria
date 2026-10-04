//! The shared wire fixtures. The server's ingest tests read the same files, so
//! JSON is compared as values, never as bytes.
use super::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeSet, fmt::Debug};

fn fixture(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}
fn request_all() -> Value {
    fixture(include_str!("../fixtures/request-all.json"))
}

fn names<T: Serialize>(values: impl IntoIterator<Item = T>) -> BTreeSet<String> {
    values
        .into_iter()
        .map(|value| {
            serde_json::to_value(value)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

#[test]
fn request_all_round_trips_through_the_strict_wire_types() {
    let golden = request_all();
    let typed: SummaryRequest = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(typed.summaries.len(), 27);
    assert_eq!(serde_json::to_value(&typed).unwrap(), golden);
    // The engine's own serialization respects the body limit with room to spare.
    assert!(serde_json::to_vec(&typed).unwrap().len() <= MAX_BODY_BYTES);
}

#[test]
fn every_enum_variant_occurs_in_the_golden_fixtures() {
    let typed: SummaryRequest = serde_json::from_value(request_all()).unwrap();
    let rows = &typed.summaries;
    assert_eq!(
        names(rows.iter().map(|row| row.operation)),
        names(SummaryOperation::ALL)
    );
    assert_eq!(
        names(rows.iter().map(|row| row.phase)),
        names(SummaryPhase::ALL)
    );
    assert_eq!(
        names(rows.iter().map(|row| row.outcome)),
        names(SummaryOutcome::ALL)
    );
    assert_eq!(
        names(rows.iter().filter_map(|row| row.error_code)),
        names(SummaryErrorCode::ALL)
    );
    assert_eq!(names(rows.iter().map(|row| row.os)), names(SummaryOs::ALL));
    assert_eq!(
        names(rows.iter().map(|row| row.arch)),
        names(SummaryArch::ALL)
    );
    assert_eq!(
        names(
            rows.iter()
                .flat_map(|row| row.phases.iter().flatten())
                .map(|entry| entry.phase)
        ),
        names(TimedPhase::ALL)
    );
    let response: SummaryResponse =
        serde_json::from_value(fixture(include_str!("../fixtures/response-mixed.json"))).unwrap();
    assert_eq!(names(response.results.iter()), names(SummaryResult::ALL));
}

#[test]
fn wire_names_are_exactly_the_contract_lists() {
    fn listed<T: Serialize + Copy>(all: &[T], expected: &str) {
        let expected: BTreeSet<String> = expected.split(" | ").map(str::to_owned).collect();
        assert_eq!(names(all.iter().copied()), expected);
        assert_eq!(all.len(), expected.len(), "a variant shares a wire name");
    }
    listed(
        SummaryOperation::ALL,
        "install | prepare_runtime | repair | uninstall | launch",
    );
    listed(
        SummaryPhase::ALL,
        "none | platform_check | compatibility_check | catalog_fetch | manifest_verify | destination_check | admission | starting | running | download | extraction",
    );
    listed(
        TimedPhase::ALL,
        "starting | running | download | extraction",
    );
    listed(
        SummaryOutcome::ALL,
        "succeeded | failed | cancelled | unknown",
    );
    listed(
        SummaryErrorCode::ALL,
        "unspecified | platform_unavailable | launcher_too_old | invalid_directory | manifest_unavailable | manifest_invalid | signing_key_unavailable | state_invalid | local_io | destination_unavailable | install_failed | content_invalid | rosetta_required | runtime_unavailable | prerequisite_failed | launch_not_started | launch_early_exit | launch_exit_nonzero",
    );
    listed(SummaryOs::ALL, "windows | macos | linux");
    listed(SummaryArch::ALL, "x86_64 | aarch64");
    listed(SummaryResult::ALL, "accepted | duplicate | rejected");
    // Every timed phase is also a place an attempt can end.
    for phase in TimedPhase::ALL {
        assert_eq!(
            names([SummaryPhase::from(*phase)]),
            names([*phase]),
            "{phase:?}"
        );
    }
}

#[test]
fn mint_request_matches_the_golden_body() {
    let install_id = Uuid::parse_str("00000000-0000-4000-8000-0000000000aa").unwrap();
    let body = MintRequest::new(install_id, LauncherVersion::new((0, 1, 0)));
    assert_eq!(
        serde_json::to_value(body).unwrap(),
        fixture(include_str!("../fixtures/mint-request.json"))
    );
}

#[test]
fn mixed_fixture_types_its_valid_rows_and_refuses_the_invalid_one() {
    let request = fixture(include_str!("../fixtures/request-mixed.json"));
    let rows = request["summaries"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    let first: Summary = serde_json::from_value(rows[0].clone()).unwrap();
    let second: Summary = serde_json::from_value(rows[1].clone()).unwrap();
    assert_eq!(first.event_id, second.event_id, "the duplicate pair");
    assert!(serde_json::from_value::<Summary>(rows[2].clone()).is_err());
    let response: SummaryResponse =
        serde_json::from_value(fixture(include_str!("../fixtures/response-mixed.json"))).unwrap();
    assert_eq!(
        response.results,
        [
            SummaryResult::Accepted,
            SummaryResult::Duplicate,
            SummaryResult::Rejected
        ]
    );
}

// One change at a time to a row the positive control accepts.
fn refused<T: DeserializeOwned + Debug>(valid: &Value, changes: &[(&str, Value)]) {
    serde_json::from_value::<T>(valid.clone()).expect("positive control");
    for (key, value) in changes {
        let mut changed = valid.clone();
        match value {
            Value::Null => changed.as_object_mut().unwrap().remove(*key),
            value => changed
                .as_object_mut()
                .unwrap()
                .insert((*key).to_owned(), value.clone()),
        };
        assert!(
            serde_json::from_value::<T>(changed).is_err(),
            "{key} = {value} must be refused"
        );
    }
}

#[test]
fn strict_types_refuse_every_rule_the_server_enforces() {
    let golden = request_all();
    // A failed row carrying every optional field.
    let failed = golden["summaries"][10].clone();
    assert_eq!(failed["outcome"], "failed");
    assert!(failed["phases"].is_array() && failed["duration_ms"].is_u64());
    refused::<Summary>(
        &failed,
        &[
            ("surprise", json!(1)),
            ("error_code", Value::Null),
            ("error_code", json!("disk_on_fire")),
            ("operation", json!("reinstall")),
            ("phase", json!("content_verify")),
            ("outcome", json!("timed_out")),
            ("os", json!("plan9")),
            ("arch", json!("x86")),
            ("event_id", json!("00000000-0000-0000-0000-000000000000")),
            ("attempt_id", json!("00000000-0000-0000-0000-000000000000")),
            ("event_id", json!("not-a-uuid")),
            ("duration_ms", json!(604_800_001_u64)),
            ("duration_ms", json!(-1)),
            ("duration_ms", json!(1.5)),
            ("retry_count", json!(101)),
            ("retry_count", Value::Null),
            ("launcher_version", json!("1.2")),
            ("launcher_version", json!("1.2.3.4")),
            ("launcher_version", json!("1.2.x")),
            ("launcher_version", json!("1234.0.0")),
            ("launcher_version", json!("1..3")),
            ("launcher_version", json!("١.٢.٣")),
            (
                "phases",
                json!([{"phase": "starting", "duration_ms": 1}, {"phase": "starting", "duration_ms": 2}]),
            ),
            ("phases", json!([{"phase": "admission", "duration_ms": 1}])),
            (
                "phases",
                json!([{"phase": "running", "duration_ms": 604_800_001_u64}]),
            ),
            (
                "phases",
                json!([{"phase": "running", "duration_ms": 1, "extra": 0}]),
            ),
        ],
    );
    // An error code is forbidden unless the outcome is `failed`.
    let succeeded = golden["summaries"][18].clone();
    assert_eq!(succeeded["outcome"], "succeeded");
    refused::<Summary>(&succeeded, &[("error_code", json!("unspecified"))]);

    let many: Vec<Value> = (0..33).map(|_| failed.clone()).collect();
    refused::<SummaryRequest>(
        &golden,
        &[
            ("schema_version", json!(2)),
            ("summaries", json!([])),
            ("summaries", Value::Array(many)),
            ("client_dropped", Value::Null),
            (
                "client_dropped",
                json!({"overflow": 65536, "expired": 0, "rejected": 0}),
            ),
            (
                "client_dropped",
                json!({"overflow": 0, "expired": 0, "rejected": 0, "other": 0}),
            ),
            ("extra", json!(true)),
        ],
    );
    refused::<SummaryResponse>(
        &json!({"results": ["accepted"]}),
        &[("results", json!(["stored"])), ("more", json!(1))],
    );
}

#[test]
fn bounded_values_saturate_instead_of_leaving_their_range() {
    assert_eq!(
        Millis::saturating(Duration::from_secs(8 * 24 * 60 * 60)),
        Millis::MAX
    );
    assert_eq!(Millis::saturating(Duration::from_millis(12)).get(), 12);
    let mut retries = RetryCount::ZERO;
    for _ in 0..300 {
        retries = retries.next();
    }
    assert_eq!(retries.get(), 100);
    assert_eq!(
        serde_json::to_value(LauncherVersion::new((1000, 65535, 7))).unwrap(),
        json!("999.999.7")
    );
}
