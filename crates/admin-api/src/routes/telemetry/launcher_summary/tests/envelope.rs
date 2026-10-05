//! Envelope validation: a request that is not the schema-1 envelope is a
//! 400 with a static body, and writes nothing.

use serde_json::{json, Value};

use super::super::dto::Verdict::{Accepted, Duplicate, Rejected};
use super::super::MAX_SUMMARY_BODY_BYTES;
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

/// **A request refused for `client_dropped` remembers no id.** Two valid
/// summaries under a missing or malformed `client_dropped` are a 400 that
/// writes no row, and the same two summaries in a valid envelope are then
/// `accepted`, not `duplicate`: the refusal came before the dedup set was
/// touched. The control is the third post, the valid request again, whose
/// summaries are `duplicate`: the set does remember what was accepted.
#[test]
fn a_request_refused_for_client_dropped_remembers_no_id() {
    let _env = Env::install();
    let valid = batch(elements(2));
    let faulty = |mutate: fn(&mut Value)| {
        let mut body = valid.clone();
        mutate(&mut body);
        bytes(&body)
    };
    let cases: Vec<(&str, Vec<u8>, &str)> = vec![
        (
            "missing",
            faulty(|b| {
                b.as_object_mut().unwrap().remove("client_dropped");
            }),
            MISSING,
        ),
        (
            "null",
            faulty(|b| b["client_dropped"] = Value::Null),
            DROPPED,
        ),
        (
            "a counter as text",
            faulty(|b| b["client_dropped"]["expired"] = json!("0")),
            DROPPED,
        ),
        (
            "a counter past u16",
            faulty(|b| b["client_dropped"]["overflow"] = json!(65_536)),
            DROPPED,
        ),
        (
            "a counter missing",
            faulty(|b| b["client_dropped"] = json!({ "overflow": 0, "expired": 0 })),
            DROPPED,
        ),
        (
            "an array",
            faulty(|b| b["client_dropped"] = json!([0, 0, 0])),
            DROPPED,
        ),
    ];
    for (name, body, text) in cases {
        let h = Harness::new();
        let (result, rows) = capture(|| h.post(&body));
        let r = refusal(result.expect_err(name));
        assert_eq!((r.status, r.body.as_str()), (400, text), "{name}");
        assert!(rows.is_empty(), "{name}: {rows:#?}");

        assert_eq!(h.verdicts(&valid), [Accepted, Accepted], "{name}");
        assert_eq!(
            h.verdicts(&valid),
            [Duplicate, Duplicate],
            "{name}: control"
        );
    }
}

/// **Deep nesting is a 400, not a stack overflow.** 40,000 unclosed `[`
/// as the whole body, and in each place the envelope holds a value: a
/// top-level key the route knows, one it does not, an element, and an
/// element past the 33 that are kept. Where the route builds the value,
/// `serde_json` stops at 128 levels, closed nest or not; where it only
/// skips the value (the unknown key, the late element) the skip does not
/// recurse, and the body ends unclosed. Each is "not JSON".
///
/// A nest that closes under a skipped key is skipped whole, and the request
/// is then refused for the key. The control is a nest of 100 levels as an
/// element: it parses, so the request is a 200 in which that element is
/// `rejected` and the valid one beside it `accepted`.
#[test]
fn deep_nesting_is_a_400() {
    let _env = Env::install();
    let open = "[".repeat(40_000);
    let closed = format!("{}{}", "[".repeat(20_000), "]".repeat(20_000));
    let dropped = r#""client_dropped":{"overflow":0,"expired":0,"rejected":0}"#;
    let valid = element(1).to_string();
    let filler = "0,".repeat(40);
    let cases = [
        ("the whole body", open.clone()),
        ("the whole body, closed", closed.clone()),
        (
            "client_dropped",
            format!(r#"{{"schema_version":1,"client_dropped":{open}"#),
        ),
        (
            "an unknown key",
            format!(r#"{{"schema_version":1,{dropped},"extra":{open}"#),
        ),
        (
            "an element",
            format!(r#"{{"schema_version":1,{dropped},"summaries":[{valid},{open}"#),
        ),
        (
            "an element, closed",
            format!(r#"{{"schema_version":1,{dropped},"summaries":[{valid},{closed}]}}"#),
        ),
        (
            "an element past the kept ones",
            format!(r#"{{"schema_version":1,{dropped},"summaries":[{filler}{open}"#),
        ),
    ];
    for (name, body) in cases {
        assert!(body.len() <= MAX_SUMMARY_BODY_BYTES, "{name}");
        let h = Harness::new();
        let (result, rows) = capture(|| h.post(body.as_bytes()));
        let r = refusal(result.expect_err(name));
        assert_eq!((r.status, r.body.as_str()), (400, NOT_JSON), "{name}");
        assert!(rows.is_empty(), "{name}: {rows:#?}");
    }

    let body =
        format!(r#"{{"schema_version":1,{dropped},"summaries":[{valid}],"extra":{closed}}}"#);
    let r = refusal(Harness::new().post(body.as_bytes()).unwrap_err());
    assert_eq!((r.status, r.body.as_str()), (400, UNKNOWN_KEY));

    let shallow = format!("{}{}", "[".repeat(100), "]".repeat(100));
    let body = format!(r#"{{"schema_version":1,{dropped},"summaries":[{shallow},{valid}]}}"#);
    let control = Harness::new().post(body.as_bytes());
    assert_eq!(control.expect("control").results, [Rejected, Accepted]);
}

/// **30,000 elements inside the body cap are refused by their count.** The
/// array starts with a valid summary and goes on with 29,999 two-byte
/// elements. The request is the count rule's 400 and no element was typed:
/// nothing is written, and the valid summary is accepted as new when it is
/// then posted alone (the control, which also shows it was valid).
#[test]
fn thirty_thousand_small_elements_are_refused_by_the_count_rule() {
    let _env = Env::install();
    let valid = element(1);
    let body = format!(
        r#"{{"schema_version":1,"client_dropped":{{"overflow":0,"expired":0,"rejected":0}},"summaries":[{valid}{}]}}"#,
        ",0".repeat(29_999)
    );
    assert!(body.len() <= MAX_SUMMARY_BODY_BYTES, "{}", body.len());
    let parsed: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed["summaries"].as_array().unwrap().len(), 30_000);

    let h = Harness::new();
    let (result, rows) = capture(|| h.post(body.as_bytes()));
    let r = refusal(result.expect_err("30,000 elements"));
    assert_eq!((r.status, r.body.as_str()), (400, COUNT));
    assert!(rows.is_empty(), "{rows:#?}");

    assert_eq!(h.verdicts(&batch(vec![valid])), [Accepted], "control");
}
