//! Envelope validation: a request that is not the schema-1 envelope is a
//! 400 with a static body, and writes nothing.

use serde_json::{json, Value};

use super::super::dto::Verdict::Accepted;
use super::{batch, capture, element, refusal, Env, Harness};

const NOT_JSON: &str = "Body is not a JSON object";
const MISSING: &str = "Missing a required key";
const UNKNOWN_KEY: &str = "Unknown top-level key";
const VERSION: &str = "Unsupported schema_version";
const DROPPED: &str = "Invalid client_dropped";
const COUNT: &str = "summaries must be an array of 1 to 32 elements";

fn elements(n: u32) -> Vec<Value> {
    (1..=n).map(element).collect()
}

fn bytes(body: &Value) -> Vec<u8> {
    body.to_string().into_bytes()
}

/// A valid one-element request after `mutate`.
fn edited(mutate: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut body = batch(elements(1));
    mutate(&mut body);
    bytes(&body)
}

fn set(key: &str, value: Value) -> Vec<u8> {
    edited(|b| b[key] = value)
}

fn without(key: &str) -> Vec<u8> {
    edited(|b| {
        b.as_object_mut().unwrap().remove(key);
    })
}

/// A valid request with one `client_dropped` counter replaced.
fn dropped(counter: &str, value: Value) -> Vec<u8> {
    edited(|b| b["client_dropped"][counter] = value)
}

/// The controls for the faults below: the unedited request is a 200, the
/// most a request may carry (32 elements) is a 200, and the top of the
/// `client_dropped` range is a 200.
#[test]
fn the_unedited_envelope_and_its_bounds_are_accepted() {
    let _env = Env::install();
    let post = |body: Vec<u8>| Harness::new().post(&body).expect("a 200").results;

    assert_eq!(post(edited(|_| {})), [Accepted]);
    assert_eq!(post(bytes(&batch(elements(32)))), vec![Accepted; 32]);
    let top = json!({ "overflow": 65_535, "expired": 65_535, "rejected": 65_535 });
    assert_eq!(post(set("client_dropped", top)), [Accepted]);
}

/// Each envelope fault, one edit each from the accepted request above.
/// None reaches validation, so none writes a row, a batch row included.
#[test]
fn an_envelope_fault_is_a_400_with_a_static_body() {
    let _env = Env::install();
    let truncated = bytes(&batch(elements(1)))[..40].to_vec();
    // An integer above `i64::MAX`, spliced in as text.
    let huge = String::from_utf8(dropped("overflow", json!("__HUGE__")))
        .unwrap()
        .replace("\"__HUGE__\"", "9223372036854775808")
        .into_bytes();
    let cases: Vec<(&str, Vec<u8>, &str)> = vec![
        ("0 elements", bytes(&batch(Vec::new())), COUNT),
        ("33 elements", bytes(&batch(elements(33))), COUNT),
        ("summaries an object", set("summaries", json!({})), COUNT),
        ("summaries null", set("summaries", Value::Null), COUNT),
        (
            "unknown top-level key",
            set("install_id", json!("x")),
            UNKNOWN_KEY,
        ),
        ("summaries missing", without("summaries"), MISSING),
        ("client_dropped missing", without("client_dropped"), MISSING),
        ("schema_version missing", without("schema_version"), MISSING),
        ("schema_version 2", set("schema_version", json!(2)), VERSION),
        ("schema_version 0", set("schema_version", json!(0)), VERSION),
        (
            "schema_version text",
            set("schema_version", json!("1")),
            VERSION,
        ),
        (
            "schema_version 1.0",
            set("schema_version", json!(1.0)),
            VERSION,
        ),
        (
            "schema_version null",
            set("schema_version", Value::Null),
            VERSION,
        ),
        (
            "dropped past u16",
            dropped("overflow", json!(65_536)),
            DROPPED,
        ),
        ("dropped negative", dropped("expired", json!(-1)), DROPPED),
        (
            "dropped fractional",
            dropped("rejected", json!(0.5)),
            DROPPED,
        ),
        ("dropped as text", dropped("rejected", json!("0")), DROPPED),
        ("dropped over i64::MAX", huge, DROPPED),
        ("dropped unknown key", dropped("lost", json!(0)), DROPPED),
        (
            "dropped key missing",
            set("client_dropped", json!({ "overflow": 0, "expired": 0 })),
            DROPPED,
        ),
        ("dropped null", set("client_dropped", Value::Null), DROPPED),
        ("empty body", Vec::new(), NOT_JSON),
        ("not JSON", b"not json".to_vec(), NOT_JSON),
        ("truncated JSON", truncated, NOT_JSON),
        ("a JSON array", b"[]".to_vec(), NOT_JSON),
        ("invalid UTF-8", vec![b'{', 0xff, b'}'], NOT_JSON),
    ];
    for (name, body, text) in cases {
        let h = Harness::new();
        let (result, rows) = capture(|| h.post(&body));
        let r = refusal(result.expect_err(name));
        assert_eq!(r.status, 400, "{name}: {r:?}");
        assert_eq!(r.body, text, "{name}");
        assert_eq!(r.retry_after, None, "{name}");
        assert!(rows.is_empty(), "{name}: {rows:#?}");
    }
}
