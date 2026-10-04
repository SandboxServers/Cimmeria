//! Nothing the caller sent comes back: not in a response body, not in a
//! log record on any target. The marker carries a newline and a forged
//! log line, which is what an echo into a plain-text sink would cost.

use axum::response::IntoResponse;
use axum::Json;
use serde_json::{json, Value};

use super::super::dto::{SummaryError, SummaryResponse};
use super::{batch, bearer, body_text, capture, element, Env, Harness, Row};

const MARKER: &str = "ZZMARKER\nlevel=ERROR msg=forged";
/// The part of the marker that survives any escaping of the newline.
const NEEDLE: &str = "ZZMARKER";

/// The response body as the client would read it, for a 200 or a refusal.
fn response_text(result: Result<SummaryResponse, SummaryError>) -> String {
    body_text(match result {
        Ok(response) => Json(response).into_response(),
        Err(refusal) => refusal.into_response(),
    })
}

fn assert_clean(name: &str, text: &str, rows: &[Row]) {
    assert!(!text.contains(NEEDLE), "{name}: response {text:?}");
    for row in rows {
        assert!(!row.mentions(NEEDLE), "{name}: {row:#?}");
    }
}

/// The marker in each position of an element (an enum, a key, an id, the
/// version, a phase name, a phase key). Each request also carries a valid
/// element, so the capture is known to be live: it holds that element's
/// rows and the batch row, and none of them holds the marker.
#[test]
fn a_marker_in_an_element_reaches_no_row_and_no_response() {
    let _env = Env::install();
    let with = |mutate: &dyn Fn(&mut Value)| {
        let mut e = element(2);
        mutate(&mut e);
        e
    };
    let cases: Vec<(&str, Value)> = vec![
        ("operation", with(&|e| e["operation"] = json!(MARKER))),
        ("phase", with(&|e| e["phase"] = json!(MARKER))),
        ("outcome", with(&|e| e["outcome"] = json!(MARKER))),
        ("error_code", with(&|e| e["error_code"] = json!(MARKER))),
        ("os", with(&|e| e["os"] = json!(MARKER))),
        ("arch", with(&|e| e["arch"] = json!(MARKER))),
        ("event_id", with(&|e| e["event_id"] = json!(MARKER))),
        ("attempt_id", with(&|e| e["attempt_id"] = json!(MARKER))),
        ("version", with(&|e| e["launcher_version"] = json!(MARKER))),
        ("element key", with(&|e| e[MARKER] = json!(1))),
        ("unknown key value", with(&|e| e["note"] = json!(MARKER))),
        (
            "phase name",
            with(&|e| e["phases"][0]["phase"] = json!(MARKER)),
        ),
        ("phase key", with(&|e| e["phases"][0][MARKER] = json!(1))),
        ("the element itself", json!(MARKER)),
    ];
    for (name, candidate) in cases {
        let h = Harness::new();
        let (result, rows) = capture(|| h.post_json(&batch(vec![element(1), candidate])));
        let text = response_text(result);
        assert_eq!(text, r#"{"results":["accepted","rejected"]}"#, "{name}");
        // Not vacuous: the valid element's summary and phases, and the
        // batch row, were captured.
        assert_eq!(rows.len(), 4, "{name}: {rows:#?}");
        assert_clean(name, &text, &rows);
    }
}

/// The marker in each position of the envelope and of the raw body: every
/// one is a refusal with a body that does not hold it, and no row at all.
#[test]
fn a_marker_in_the_envelope_reaches_no_row_and_no_response() {
    let _env = Env::install();
    let with = |mutate: &dyn Fn(&mut Value)| {
        let mut body = batch(vec![element(1)]);
        mutate(&mut body);
        body.to_string().into_bytes()
    };
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("top-level key", with(&|b| b[MARKER] = json!(1))),
        ("top-level value", with(&|b| b["note"] = json!(MARKER))),
        (
            "schema_version",
            with(&|b| b["schema_version"] = json!(MARKER)),
        ),
        (
            "client_dropped",
            with(&|b| b["client_dropped"] = json!(MARKER)),
        ),
        (
            "dropped key",
            with(&|b| b["client_dropped"][MARKER] = json!(0)),
        ),
        (
            "dropped value",
            with(&|b| b["client_dropped"]["expired"] = json!(MARKER)),
        ),
        ("summaries", with(&|b| b["summaries"] = json!(MARKER))),
        ("raw text", MARKER.as_bytes().to_vec()),
        ("broken JSON", format!("{{\"{MARKER}\": ").into_bytes()),
    ];
    for (name, body) in cases {
        let h = Harness::new();
        let (result, rows) = capture(|| h.post(&body));
        assert!(result.is_err(), "{name} must be refused");
        let text = response_text(result);
        assert!(!text.is_empty(), "{name}");
        assert_clean(name, &text, &rows);
        assert!(rows.is_empty(), "{name}: {rows:#?}");
    }
}

/// The marker as the token (a header cannot hold the newline, so only the
/// printable part): a 401 whose body does not repeat it, and no row.
#[test]
fn a_marker_as_the_token_reaches_no_row_and_no_response() {
    let _env = Env::install();
    for token in [
        NEEDLE.to_string(),
        format!("{NEEDLE}.{NEEDLE}"),
        format!("e30.{NEEDLE}"),
    ] {
        let mut h = Harness::new();
        h.headers = bearer(&token);
        let (result, rows) = capture(|| h.post_json(&batch(vec![element(1)])));
        assert!(result.is_err(), "{token}");
        let text = response_text(result);
        assert_clean(&token, &text, &rows);
    }
}

/// The capture these tests rely on does see a marker when one is logged,
/// and `mentions` finds it in a key, a value or a target.
#[test]
fn the_capture_would_see_an_echo() {
    let (_, rows) = capture(|| {
        tracing::warn!(echo = MARKER, "value");
        tracing::debug!(target: "ZZMARKER.target", "target");
    });
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.mentions(NEEDLE)), "{rows:#?}");
}
