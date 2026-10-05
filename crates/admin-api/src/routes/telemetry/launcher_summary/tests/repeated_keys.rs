//! A key written twice. `serde_json` would keep the later value and say
//! nothing; the ingest refuses the request when the repeat is in the
//! envelope and rejects the element when it is inside one.
//!
//! `json!` cannot hold a repeated key, so each body here is the text of a
//! valid request with one piece replaced. In every case the later value is
//! the valid one: an ingest that let the later value win would accept all
//! of them.

use serde_json::Value;

use super::super::dto::Verdict::{Accepted, Duplicate, Rejected};
use super::super::envelope::parse_envelope;
use super::{capture, element, refusal, summary_rows, Env, Harness, REQUEST_ALL};

const REPEATED: &str = "Repeated key";

/// `text` with its one occurrence of `needle` replaced.
fn spliced(text: &str, needle: &str, replacement: &str) -> String {
    assert_eq!(text.matches(needle).count(), 1, "`{needle}` in {text}");
    text.replace(needle, replacement)
}

/// A request around the given element texts.
fn request(elements: &[String]) -> String {
    format!(
        r#"{{"schema_version":1,"client_dropped":{{"overflow":0,"expired":0,"rejected":0}},"summaries":[{}]}}"#,
        elements.join(",")
    )
}

/// **A repeated key in the envelope is a 400.** Each case is the valid
/// request with one key written twice: at the top level, or inside
/// `client_dropped`. The request is refused whole with a static body,
/// writes no row and remembers no id. The control is the request as it was
/// before the edit, posted to the same ingest: accepted as new.
#[test]
fn a_repeated_envelope_key_is_a_400() {
    let _env = Env::install();
    let valid = request(&[element(1).to_string()]);
    let dropped = r#""client_dropped":{"overflow":0,"expired":0,"rejected":0}"#;
    let cases = [
        (
            "schema_version, the later one valid",
            r#""schema_version":1"#,
            r#""schema_version":2,"schema_version":1"#.to_string(),
        ),
        (
            "schema_version, both valid",
            r#""schema_version":1"#,
            r#""schema_version":1,"schema_version":1"#.to_string(),
        ),
        (
            "schema_version, one spelled with an escape",
            r#""schema_version":1"#,
            r#""schema_version":2,"schema_version":1"#.to_string(),
        ),
        (
            "summaries",
            r#""summaries":["#,
            r#""summaries":[],"summaries":["#.to_string(),
        ),
        ("client_dropped", dropped, format!("{dropped},{dropped}")),
        (
            "a counter inside client_dropped",
            r#""overflow":0"#,
            r#""overflow":7,"overflow":0"#.to_string(),
        ),
    ];
    for (name, needle, replacement) in cases {
        let h = Harness::new();
        let body = spliced(&valid, needle, &replacement);
        let (result, rows) = capture(|| h.post(body.as_bytes()));
        let r = refusal(result.expect_err(name));
        assert_eq!(r.status, 400, "{name}: {r:?}");
        assert_eq!(r.body, REPEATED, "{name}");
        assert!(rows.is_empty(), "{name}: {rows:#?}");

        let control = h.post(valid.as_bytes());
        assert_eq!(control.expect(name).results, [Accepted], "{name}: control");
    }
}

/// **A repeated key inside an element rejects that element alone.** Each
/// case is a two-element request whose first element writes one key twice:
/// a shadowed `operation`, a required and an optional key repeated with
/// the same value, a key spelled once with an escape, and a key repeated
/// inside an entry of `phases`. The first element is `rejected`, the
/// sibling is `accepted` and is the only summary written, and the rejected
/// element's id is not remembered.
///
/// The control is the same pair before the edit, on a fresh ingest: both
/// accepted.
#[test]
fn a_repeated_key_inside_an_element_rejects_only_that_element() {
    let _env = Env::install();
    let first = element(1).to_string();
    let sibling = element(2).to_string();
    let cases = [
        (
            "a shadowed operation",
            r#""operation":"install""#,
            r#""operation":"launch","operation":"install""#,
        ),
        (
            "os twice, the same value",
            r#""os":"windows""#,
            r#""os":"windows","os":"windows""#,
        ),
        (
            "an optional key twice",
            r#""duration_ms":81234"#,
            r#""duration_ms":81234,"duration_ms":81234"#,
        ),
        (
            "operation, one spelled with an escape",
            r#""operation":"install""#,
            r#""operation":"repair","operation":"install""#,
        ),
        (
            "phase twice inside a phases entry",
            r#""phase":"starting""#,
            r#""phase":"running","phase":"starting""#,
        ),
        (
            "duration_ms twice inside a phases entry",
            r#""duration_ms":12"#,
            r#""duration_ms":12,"duration_ms":12"#,
        ),
    ];
    for (name, needle, replacement) in cases {
        let control = Harness::new().post(request(&[first.clone(), sibling.clone()]).as_bytes());
        assert_eq!(
            control.expect(name).results,
            [Accepted, Accepted],
            "{name}: control"
        );

        let h = Harness::new();
        let body = request(&[spliced(&first, needle, replacement), sibling.clone()]);
        let (result, rows) = capture(|| h.post(body.as_bytes()));
        assert_eq!(result.expect(name).results, [Rejected, Accepted], "{name}");
        assert_eq!(summary_rows(&rows).len(), 1, "{name}: {rows:#?}");

        // The rejected element's id was not remembered; the sibling's was.
        let again = h.post(request(&[first.clone(), sibling.clone()]).as_bytes());
        assert_eq!(
            again.expect(name).results,
            [Accepted, Duplicate],
            "{name}: after"
        );
    }
}

/// Reading the body with the repeated-key check changes nothing about a
/// body with no repeat: each element is the value `serde_json` builds for
/// it. The golden request covers every field; the second body covers the
/// JSON a valid element never holds (fractions, exponents, negatives, the
/// ends of the integer ranges, `null`, booleans, escapes, nesting, an empty
/// object and an empty array).
#[test]
fn a_body_with_no_repeat_is_read_as_serde_json_reads_it() {
    let odd = r#"{"a":[1,-2,3.5,1e3,-0.0,null,true,false,"x\nyé",{"b":{},"c":[]},18446744073709551615,-9223372036854775808]}"#;
    for (name, body) in [
        ("golden", REQUEST_ALL.to_string()),
        ("odd", request(&[odd.to_string()])),
    ] {
        let plain: Value = serde_json::from_str(&body).unwrap();
        let expected: Vec<Option<Value>> = plain["summaries"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(Some)
            .collect();
        assert!(!expected.is_empty(), "{name}");
        let envelope = parse_envelope(body.as_bytes()).expect(name);
        assert_eq!(envelope.elements, expected, "{name}");
    }
}
