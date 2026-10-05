//! The golden wire fixtures shared with the desktop launcher's exporter
//! (`crates/launcher/desktop/engine/src/storage/launcher_summary/fixtures/`).
//! The launcher proves it produces these bodies; this proves the server
//! accepts them and answers as the launcher expects. Compared as JSON
//! values, never as bytes.

use std::collections::BTreeSet;

use serde_json::Value;

use super::super::dto::{Arch, ErrorCode, Operation, Os, Outcome, Phase, TimedPhase, Verdict};
use super::{
    batch_rows, capture, phase_rows, summary_rows, Env, Harness, Row, REQUEST_ALL,
    REQUEST_INSTALL_FAILURE, REQUEST_MIXED, RESPONSE_MIXED,
};

/// What a row should hold for an optional JSON key: its text, or nothing.
fn optional(element: &Value, key: &str) -> Option<String> {
    element.get(key).map(|v| match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}

fn field(row: &Row, key: &str) -> Option<String> {
    row.fields.get(key).cloned()
}

/// The wire keys of a summary that a `launcher_summary` row repeats.
const SUMMARY_KEYS: [&str; 11] = [
    "event_id",
    "attempt_id",
    "operation",
    "phase",
    "outcome",
    "error_code",
    "duration_ms",
    "retry_count",
    "launcher_version",
    "os",
    "arch",
];

/// `(attempt_id, phase, duration_ms)` for each `phases` entry of
/// `elements`, in order: the `launcher_phase` rows they must produce.
fn sent_phases(elements: &[Value]) -> Vec<(String, String, String)> {
    elements
        .iter()
        .flat_map(|element| {
            let attempt = element["attempt_id"].as_str().unwrap().to_string();
            let entries = element.get("phases").and_then(Value::as_array);
            entries.into_iter().flatten().map(move |entry| {
                (
                    attempt.clone(),
                    entry["phase"].as_str().unwrap().to_string(),
                    entry["duration_ms"].to_string(),
                )
            })
        })
        .collect()
}

/// The same three values from the `launcher_phase` rows that were written.
fn emitted_phases(rows: &[Row]) -> Vec<(String, String, String)> {
    phase_rows(rows)
        .iter()
        .map(|row| {
            (
                row.fields["attempt_id"].clone(),
                row.fields["phase"].clone(),
                row.fields["duration_ms"].clone(),
            )
        })
        .collect()
}

/// **`request-all.json`.** Every element is `accepted`, and the rows are
/// the request: one `launcher_summary` row per element carrying exactly
/// that element's values (an absent optional key is an absent field), one
/// `launcher_phase` row per `phases` entry in order, and one batch row
/// with the request's `client_dropped` counters.
#[test]
fn every_element_of_request_all_is_accepted_and_emitted_as_sent() {
    let _env = Env::install();
    let request: Value = serde_json::from_str(REQUEST_ALL).unwrap();
    let elements = request["summaries"].as_array().unwrap();
    assert_eq!(elements.len(), 27, "the fixture changed size");

    let h = Harness::new();
    let (response, rows) = capture(|| h.post(REQUEST_ALL.as_bytes()));
    let results = response.expect("a 200").results;
    assert_eq!(results, vec![Verdict::Accepted; elements.len()]);

    let summaries = summary_rows(&rows);
    assert_eq!(summaries.len(), elements.len());
    for (element, row) in elements.iter().zip(&summaries) {
        let id = element["event_id"].as_str().unwrap();
        for key in SUMMARY_KEYS {
            assert_eq!(field(row, key), optional(element, key), "{id}: {key}");
        }
        assert_eq!(
            row.fields.contains_key("duration_bucket"),
            element.get("duration_ms").is_some(),
            "{id}: a bucket exactly when there is a duration"
        );
    }

    let expected_phases = sent_phases(elements);
    assert_eq!(expected_phases.len(), 14, "the fixture changed size");
    assert_eq!(emitted_phases(&rows), expected_phases);

    let batches = batch_rows(&rows);
    assert_eq!(batches.len(), 1);
    let batch = &batches[0].fields;
    assert_eq!(batch["accepted"], "27");
    assert_eq!(batch["duplicate"], "0");
    assert_eq!(batch["rejected"], "0");
    assert_eq!(batch["client_dropped_overflow"], "65535");
    assert_eq!(batch["client_dropped_expired"], "0");
    assert_eq!(batch["client_dropped_rejected"], "7");
    assert_eq!(rows.len(), 27 + 14 + 1, "no other row is written");
}

/// The fixture exercises every value of every wire enum, so "every element
/// is accepted" above covers the whole closed set. A value added to an enum
/// here without a fixture element fails this, which is the prompt to add
/// it on the launcher's side too.
#[test]
fn request_all_covers_every_value_of_every_enum() {
    let request: Value = serde_json::from_str(REQUEST_ALL).unwrap();
    let elements = request["summaries"].as_array().unwrap();
    let seen = |key: &str| -> BTreeSet<&str> {
        elements
            .iter()
            .filter_map(|e| e.get(key).and_then(Value::as_str))
            .collect()
    };
    let timed: BTreeSet<&str> = elements
        .iter()
        .filter_map(|e| e.get("phases").and_then(Value::as_array))
        .flatten()
        .filter_map(|entry| entry["phase"].as_str())
        .collect();

    fn all<T: Copy>(values: &[T], as_str: fn(T) -> &'static str) -> BTreeSet<&'static str> {
        values.iter().map(|v| as_str(*v)).collect()
    }
    assert_eq!(seen("operation"), all(Operation::ALL, Operation::as_str));
    assert_eq!(seen("phase"), all(Phase::ALL, Phase::as_str));
    assert_eq!(seen("outcome"), all(Outcome::ALL, Outcome::as_str));
    assert_eq!(seen("error_code"), all(ErrorCode::ALL, ErrorCode::as_str));
    assert_eq!(seen("os"), all(Os::ALL, Os::as_str));
    assert_eq!(seen("arch"), all(Arch::ALL, Arch::as_str));
    assert_eq!(timed, all(TimedPhase::ALL, TimedPhase::as_str));
    // Every timed phase is also a phase, spelled the same.
    assert!(timed.is_subset(&all(Phase::ALL, Phase::as_str)));
}

/// **`request-install-failure.json`.** The body the launcher's real
/// install worker produced for one failed install (the engine's
/// `install_worker/export_tests.rs` proves it still produces it). This is
/// the other half of that proof: the server accepts that body and writes
/// it down as sent, as one failed install with the phases it timed.
///
/// What a failed install is stays pinned here (`install`, `failed`,
/// `install_failed`). Where it ended and which phases it timed are the
/// engine's to decide, so those are read from the file: the row must repeat
/// them, whatever they are.
#[test]
fn the_recorded_install_failure_is_accepted_and_emitted_as_recorded() {
    let _env = Env::install();
    let request: Value = serde_json::from_str(REQUEST_INSTALL_FAILURE).unwrap();
    let elements = request["summaries"].as_array().unwrap();
    assert_eq!(elements.len(), 1, "the fixture is one attempt");
    let element = &elements[0];
    let expected_phases = sent_phases(elements);
    // An attempt the launcher watched from admission always timed at least
    // `starting`; with none, the phase rows below would be unproven.
    assert!(
        !expected_phases.is_empty(),
        "the fixture lost its phase timings"
    );

    let h = Harness::new();
    let (response, rows) = capture(|| h.post(REQUEST_INSTALL_FAILURE.as_bytes()));
    assert_eq!(response.expect("a 200").results, [Verdict::Accepted]);

    let summaries = summary_rows(&rows);
    assert_eq!(summaries.len(), 1);
    let row = summaries[0];
    assert_eq!(row.fields["operation"], "install");
    assert_eq!(row.fields["outcome"], "failed");
    assert_eq!(row.fields["error_code"], "install_failed");
    assert_eq!(row.fields["phase"], element["phase"].as_str().unwrap());
    for key in SUMMARY_KEYS {
        assert_eq!(field(row, key), optional(element, key), "{key}");
    }

    assert_eq!(emitted_phases(&rows), expected_phases);
    let batches = batch_rows(&rows);
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].fields["accepted"], "1");
    assert_eq!(
        rows.len(),
        1 + expected_phases.len() + 1,
        "no other row is written"
    );
}

/// **`request-mixed.json`.** On a fresh process the real response is
/// `response-mixed.json`: accepted, duplicate (the second element repeats
/// the first's `event_id`), rejected (the third has an unknown
/// `error_code`). The launcher feeds the same response file to its
/// exporter.
#[test]
fn the_response_to_request_mixed_is_response_mixed() {
    let _env = Env::install();
    let h = Harness::new();
    let (response, rows) = capture(|| h.post(REQUEST_MIXED.as_bytes()));
    let response = serde_json::to_value(response.expect("a 200")).unwrap();
    let expected: Value = serde_json::from_str(RESPONSE_MIXED).unwrap();
    assert_eq!(response, expected);

    assert_eq!(summary_rows(&rows).len(), 1);
    assert_eq!(phase_rows(&rows).len(), 3, "the accepted element's phases");
    let batch = &batch_rows(&rows)[0].fields;
    assert_eq!(
        (
            &*batch["accepted"],
            &*batch["duplicate"],
            &*batch["rejected"]
        ),
        ("1", "1", "1")
    );
}
