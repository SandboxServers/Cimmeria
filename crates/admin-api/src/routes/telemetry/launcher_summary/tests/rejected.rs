//! Only the payload gets in. The route is anonymous, so what it refuses is
//! its whole defence: every request here is something other than the
//! schema-1 JSON envelope, and each is refused as a whole with a static
//! body, writes no row and remembers no id.
//!
//! Each refusal is checked beside a control on the same ingest state: the
//! request with only the varied condition put right, which is accepted as
//! new. `envelope` holds the full table of envelope faults; this file is
//! about the kinds of thing a stranger might post instead.

use std::io::Write;

use axum::http::{HeaderMap, HeaderValue};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::{json, Value};

use crate::routes::dev_session::TokenClaims;
use crate::routes::telemetry::replay_ndjson;

use super::super::dto::Verdict::{Accepted, Rejected};
use super::{
    batch, batch_rows, capture, content_type, element, json_headers, phase_rows, refusal,
    summary_rows, Env, Harness,
};

const CONTENT_TYPE: &str = "Content-Type must be application/json";
const NOT_JSON: &str = "Body is not a JSON object";
const MISSING: &str = "Missing a required key";
const UNKNOWN_KEY: &str = "Unknown top-level key";
const VERSION: &str = "Unsupported schema_version";
const COUNT: &str = "summaries must be an array of 1 to 32 elements";

/// One event of the game-telemetry stream, as the launcher writes a line
/// of an upload chunk. `the_game_telemetry_line_is_a_real_upload_event`
/// shows the chunk replay takes it.
const CLIENT_NATIVE_LINE: &str = r#"{"type":"client_native","ts_ms":1,"seq":1,"target":"client.dll.attached","level":"info","fields":{"dll_version":"0.1.0"}}"#;

fn elements(n: u32) -> Vec<Value> {
    (1..=n).map(element).collect()
}

/// The accepted request: one valid element in a valid envelope.
fn valid() -> Value {
    batch(elements(1))
}

fn bytes(body: &Value) -> Vec<u8> {
    body.to_string().into_bytes()
}

fn gzip(body: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(body).unwrap();
    encoder.finish().unwrap()
}

/// Post `body` under `headers` to a fresh ingest and require a refusal of
/// the whole request: `status`, exactly the static `text`, no
/// `Retry-After`, no row of any kind.
///
/// Then post `control` under the launcher's headers to the same ingest.
/// Every element of it is accepted as new and written, which shows that
/// the ingest was working, and that the refused request (which carried the
/// same ids wherever it carried any) left nothing in the dedup set.
fn assert_refused_whole(
    name: &str,
    headers: HeaderMap,
    body: &[u8],
    status: u16,
    text: &str,
    control: &Value,
) {
    let mut h = Harness::new();
    h.headers = headers;
    let (result, rows) = capture(|| h.post(body));
    let r = refusal(result.expect_err(name));
    assert_eq!(r.status, status, "{name}: {r:?}");
    assert_eq!(r.body, text, "{name}");
    assert_eq!(r.retry_after, None, "{name}");
    assert!(rows.is_empty(), "{name}: {rows:#?}");

    let sent = control["summaries"].as_array().unwrap().len();
    h.headers = json_headers();
    let (verdicts, rows) = capture(|| h.verdicts(control));
    assert_eq!(verdicts, vec![Accepted; sent], "{name}: control");
    assert_eq!(summary_rows(&rows).len(), sent, "{name}: control rows");
    assert_eq!(batch_rows(&rows).len(), 1, "{name}: control batch row");
}

/// The valid body under each content type that is not `application/json`,
/// and under none: a 415, whatever the body says. The control is the same
/// body as `application/json`.
#[test]
fn a_content_type_other_than_json_is_a_415() {
    let _env = Env::install();
    let body = bytes(&valid());
    let cases = [
        ("no Content-Type", None),
        ("empty", Some("")),
        ("text/plain", Some("text/plain")),
        (
            "text/plain with a charset",
            Some("text/plain; charset=utf-8"),
        ),
        ("text/json", Some("text/json")),
        ("NDJSON", Some("application/x-ndjson")),
        ("gzip", Some("application/gzip")),
        ("octet-stream", Some("application/octet-stream")),
        ("a form", Some("application/x-www-form-urlencoded")),
        ("multipart", Some("multipart/form-data; boundary=x")),
        ("a +json suffix", Some("application/vnd.api+json")),
        ("a longer subtype", Some("application/json5")),
        ("a bare word", Some("json")),
        ("a wildcard", Some("*/*")),
        ("another parameter", Some("application/json; boundary=x")),
        (
            "a second parameter",
            Some("application/json; charset=utf-8; q=1"),
        ),
        ("an empty parameter", Some("application/json;")),
        ("a list", Some("application/json, text/plain")),
    ];
    for (name, value) in cases {
        assert_refused_whole(
            name,
            content_type(value),
            &body,
            415,
            CONTENT_TYPE,
            &valid(),
        );
    }

    // Two `Content-Type` headers, the first of them the right one.
    let mut two = json_headers();
    two.append(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain"),
    );
    assert_refused_whole("two headers", two, &body, 415, CONTENT_TYPE, &valid());
}

/// The spellings of `application/json` that are accepted: the bare type,
/// and the type with a `charset` parameter, in any case and spacing.
#[test]
fn json_with_or_without_a_charset_is_accepted() {
    let _env = Env::install();
    for value in [
        "application/json",
        "application/json; charset=utf-8",
        "application/json;charset=UTF-8",
        "Application/JSON ; Charset=\"utf-8\"",
    ] {
        let mut h = Harness::new();
        h.headers = content_type(Some(value));
        assert_eq!(h.verdicts(&valid()), [Accepted], "{value}");
    }
}

/// Bodies that are not the envelope, each sent as `application/json`: a
/// 400. The control is the valid envelope (for the 33-element request, the
/// same request with 32).
#[test]
fn a_body_that_is_not_the_envelope_is_a_400() {
    let _env = Env::install();
    let valid_bytes = bytes(&valid());
    let wrapped = |open: &str, close: &str| {
        let mut body = open.as_bytes().to_vec();
        body.extend_from_slice(&valid_bytes);
        body.extend_from_slice(close.as_bytes());
        body
    };
    let edited = |mutate: &dyn Fn(&mut Value)| {
        let mut body = valid();
        mutate(&mut body);
        bytes(&body)
    };
    let gzipped = gzip(&valid_bytes);
    assert_eq!(gzipped[..2], [0x1f, 0x8b], "the gzip magic bytes");

    let cases: Vec<(&str, Vec<u8>, &str)> = vec![
        ("empty body", Vec::new(), NOT_JSON),
        ("whitespace", b" \r\n".to_vec(), NOT_JSON),
        (
            "not JSON",
            b"operation=install&outcome=failed".to_vec(),
            NOT_JSON,
        ),
        ("XML", b"<summaries/>".to_vec(), NOT_JSON),
        // The valid envelope, one level down.
        ("a JSON array", wrapped("[", "]"), NOT_JSON),
        ("an empty array", b"[]".to_vec(), NOT_JSON),
        (
            "a JSON string",
            bytes(&json!(valid().to_string())),
            NOT_JSON,
        ),
        ("a JSON number", b"1".to_vec(), NOT_JSON),
        ("JSON null", b"null".to_vec(), NOT_JSON),
        ("JSON true", b"true".to_vec(), NOT_JSON),
        (
            "the envelope twice",
            wrapped("", &valid().to_string()),
            NOT_JSON,
        ),
        ("the envelope then text", wrapped("", "x"), NOT_JSON),
        (
            "the envelope after a BOM",
            wrapped("\u{feff}", ""),
            NOT_JSON,
        ),
        // The valid envelope, gzipped as the chunk upload's bodies are.
        ("gzip", gzipped, NOT_JSON),
        // One line parses as a JSON object, so it fails on its keys.
        (
            "a game-telemetry event",
            CLIENT_NATIVE_LINE.as_bytes().to_vec(),
            MISSING,
        ),
        (
            "a game-telemetry NDJSON line",
            format!("{CLIENT_NATIVE_LINE}\n").into_bytes(),
            MISSING,
        ),
        (
            "two game-telemetry NDJSON lines",
            format!("{CLIENT_NATIVE_LINE}\n{CLIENT_NATIVE_LINE}\n").into_bytes(),
            NOT_JSON,
        ),
        ("an empty object", b"{}".to_vec(), MISSING),
        (
            "an unknown top-level key",
            edited(&|b| b["install_id"] = json!("x")),
            UNKNOWN_KEY,
        ),
        (
            "the envelope with a game-telemetry key",
            edited(&|b| b["type"] = json!("client_native")),
            UNKNOWN_KEY,
        ),
        (
            "schema_version 2",
            edited(&|b| b["schema_version"] = json!(2)),
            VERSION,
        ),
        (
            "schema_version as text",
            edited(&|b| b["schema_version"] = json!("1")),
            VERSION,
        ),
        ("0 elements", bytes(&batch(Vec::new())), COUNT),
    ];
    for (name, body, text) in cases {
        assert_refused_whole(name, json_headers(), &body, 400, text, &valid());
    }
    assert_refused_whole(
        "33 elements",
        json_headers(),
        &bytes(&batch(elements(33))),
        400,
        COUNT,
        &batch(elements(32)),
    );
}

/// A gzip body that says so (`Content-Encoding: gzip`) is still a 400: the
/// route does not inflate anything. The control is the same body, not
/// compressed, under the same headers.
#[test]
fn a_gzip_body_is_not_inflated() {
    let _env = Env::install();
    let mut headers = json_headers();
    headers.insert(
        axum::http::header::CONTENT_ENCODING,
        HeaderValue::from_static("gzip"),
    );
    let body = bytes(&valid());

    assert_refused_whole(
        "gzip with Content-Encoding",
        headers.clone(),
        &gzip(&body),
        400,
        NOT_JSON,
        &valid(),
    );
    let mut h = Harness::new();
    h.headers = headers;
    assert_eq!(h.verdicts(&valid()), [Accepted], "control");
}

/// The line used above is game telemetry and not a made-up string: the
/// chunk upload's replay takes it and writes it as a `client.native` row.
#[test]
fn the_game_telemetry_line_is_a_real_upload_event() {
    let claims = TokenClaims {
        iss: "cimmeria-server".into(),
        sub: "install-1".into(),
        sid: "session-1".into(),
        iat: 0,
        exp: i64::MAX,
        scope: vec!["telemetry.write".into()],
        kind: None,
    };
    let (counts, rows) = capture(|| replay_ndjson(&claims, &format!("{CLIENT_NATIVE_LINE}\n")));
    let counts = counts.expect("the upload replay accepts the line");
    assert_eq!((counts.parsed, counts.accepted), (1, 1));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].target, "client.native");
}

/// The same event as an element of an otherwise valid envelope is
/// `rejected` in its own position, and the valid elements on either side of
/// it are still accepted and written. Nothing of the event reaches a row.
#[test]
fn a_game_telemetry_event_as_an_element_is_rejected_alone() {
    let _env = Env::install();
    let event: Value = serde_json::from_str(CLIENT_NATIVE_LINE).unwrap();
    let h = Harness::new();

    let (verdicts, rows) = capture(|| h.verdicts(&batch(vec![element(1), event, element(3)])));
    assert_eq!(verdicts, [Accepted, Rejected, Accepted]);
    assert_eq!(summary_rows(&rows).len(), 2);
    assert_eq!(phase_rows(&rows).len(), 4, "two phases per valid element");
    let batch_row = &batch_rows(&rows)[0].fields;
    assert_eq!(batch_row["accepted"], "2");
    assert_eq!(batch_row["rejected"], "1");
    for row in &rows {
        for needle in ["client_native", "client.dll.attached", "dll_version"] {
            assert!(!row.mentions(needle), "{needle} in {row:#?}");
        }
    }
}
