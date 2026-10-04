//! Element validation: an element that breaks a rule is `rejected` in its
//! own position while the valid element beside it is still `accepted`.

use serde_json::{json, Value};

use super::super::dto::Verdict::{self, Accepted, Duplicate, Rejected};
use super::{batch, capture, element, id, summary_rows, Env, Harness};

const NIL: &str = "00000000-0000-0000-0000-000000000000";

/// Post a valid element followed by `candidate` to a fresh ingest.
fn beside_a_control(candidate: Value) -> Vec<Verdict> {
    Harness::new().verdicts(&batch(vec![element(1), candidate]))
}

/// `element(2)` after `mutate`.
fn mutated(mutate: impl FnOnce(&mut Value)) -> Value {
    let mut candidate = element(2);
    mutate(&mut candidate);
    candidate
}

/// `element(2)` with one key replaced.
fn set(key: &str, value: Value) -> Value {
    mutated(|e| e[key] = value)
}

/// `element(2)` with one key removed.
fn without(key: &str) -> Value {
    mutated(|e| {
        e.as_object_mut().unwrap().remove(key);
    })
}

/// `element(2)` with `phases` replaced.
fn phases(value: Value) -> Value {
    set("phases", value)
}

/// The control for everything below: the unmutated pair is accepted twice,
/// so each `rejected` in this file is caused by its one mutation.
#[test]
fn the_unmutated_pair_is_accepted() {
    let _env = Env::install();
    assert_eq!(beside_a_control(element(2)), [Accepted, Accepted]);
    assert_eq!(beside_a_control(mutated(|_| {})), [Accepted, Accepted]);
}

/// Every element-level rule, one mutation each. The first element of each
/// request is valid and stays `accepted`; only the mutated one is
/// `rejected`, and it writes no summary row.
#[test]
fn each_rule_rejects_only_the_element_that_breaks_it() {
    let _env = Env::install();
    let timed = ["starting", "running", "download", "extraction"];
    let thirty_three: Vec<Value> = (0..33)
        .map(|i| json!({ "phase": timed[i % 4], "duration_ms": i }))
        .collect();
    let cases: Vec<(&str, Value)> = vec![
        // Closed objects.
        ("unknown element key", set("install_id", json!("x"))),
        ("element is a string", json!("summary")),
        ("element is null", Value::Null),
        ("element is an array", json!([])),
        // Closed enums.
        ("unknown operation", set("operation", json!("reinstall"))),
        (
            "operation in upper case",
            set("operation", json!("Install")),
        ),
        ("operation is a number", set("operation", json!(1))),
        ("unknown phase", set("phase", json!("content_verify"))),
        ("unknown outcome", set("outcome", json!("ok"))),
        ("unknown error_code", set("error_code", json!("not_a_code"))),
        ("unknown os", set("os", json!("freebsd"))),
        ("unknown arch", set("arch", json!("arm"))),
        // error_code is required on a failure and forbidden otherwise.
        ("failed without error_code", without("error_code")),
        ("error_code on success", set("outcome", json!("succeeded"))),
        ("error_code on a cancel", set("outcome", json!("cancelled"))),
        ("error_code on unknown", set("outcome", json!("unknown"))),
        // An optional key is absent or holds a value; null is neither.
        // (On a success, so the null is not also a missing error_code.)
        (
            "error_code null",
            mutated(|e| {
                e["outcome"] = json!("succeeded");
                e["error_code"] = Value::Null;
            }),
        ),
        ("duration_ms null", set("duration_ms", Value::Null)),
        ("phases null", phases(Value::Null)),
        // Numeric bounds.
        ("duration over cap", set("duration_ms", json!(604_800_001))),
        ("duration negative", set("duration_ms", json!(-1))),
        ("duration fractional", set("duration_ms", json!(1.5))),
        ("duration as text", set("duration_ms", json!("81234"))),
        (
            "duration over i64::MAX",
            set("duration_ms", json!(1u64 << 63)),
        ),
        ("retry over cap", set("retry_count", json!(101))),
        ("retry past u8", set("retry_count", json!(256))),
        ("retry negative", set("retry_count", json!(-1))),
        ("retry over i64::MAX", set("retry_count", json!(u64::MAX))),
        ("retry missing", without("retry_count")),
        // Ids.
        ("nil event_id", set("event_id", json!(NIL))),
        ("nil attempt_id", set("attempt_id", json!(NIL))),
        ("event_id not a UUID", set("event_id", json!("not-a-uuid"))),
        ("event_id empty", set("event_id", json!(""))),
        ("event_id short", set("event_id", json!(&id(0xe5, 2)[..35]))),
        ("event_id a number", set("event_id", json!(7))),
        ("event_id missing", without("event_id")),
        ("attempt_id missing", without("attempt_id")),
        // launcher_version: three components of one to three ASCII digits.
        (
            "version in Arabic-Indic digits",
            set("launcher_version", json!("\u{0660}.\u{0661}.\u{0660}")),
        ),
        (
            "version in fullwidth digits",
            set("launcher_version", json!("\u{ff10}.\u{ff11}.\u{ff10}")),
        ),
        (
            "version 4 digits",
            set("launcher_version", json!("1000.0.0")),
        ),
        ("version 2 parts", set("launcher_version", json!("0.1"))),
        ("version 4 parts", set("launcher_version", json!("0.1.0.0"))),
        ("version letter", set("launcher_version", json!("0.1.x"))),
        ("version sign", set("launcher_version", json!("+1.0.0"))),
        ("version suffix", set("launcher_version", json!("0.1.0-rc"))),
        ("version space", set("launcher_version", json!(" 0.1.0"))),
        ("version empty", set("launcher_version", json!(""))),
        ("version a number", set("launcher_version", json!(1))),
        // phases.
        (
            "duplicate phase",
            phases(json!([
                { "phase": "starting", "duration_ms": 1 },
                { "phase": "starting", "duration_ms": 2 },
            ])),
        ),
        (
            "non-timed phase `none`",
            phases(json!([{ "phase": "none", "duration_ms": 1 }])),
        ),
        (
            "non-timed phase `admission`",
            phases(json!([{ "phase": "admission", "duration_ms": 1 }])),
        ),
        ("33 phases", phases(Value::Array(thirty_three))),
        (
            "phase duration over cap",
            phases(json!([{ "phase": "starting", "duration_ms": 604_800_001 }])),
        ),
        (
            "phase duration negative",
            phases(json!([{ "phase": "starting", "duration_ms": -1 }])),
        ),
        (
            "phase duration missing",
            phases(json!([{ "phase": "starting" }])),
        ),
        (
            "unknown key in a phase",
            phases(json!([{ "phase": "starting", "duration_ms": 1, "bytes": 2 }])),
        ),
        ("phases not an array", phases(json!({ "starting": 1 }))),
    ];
    for (name, candidate) in cases {
        let h = Harness::new();
        let (results, rows) = capture(|| h.verdicts(&batch(vec![element(1), candidate])));
        assert_eq!(results, [Accepted, Rejected], "{name}");
        assert_eq!(summary_rows(&rows).len(), 1, "{name}: {rows:#?}");
    }
}

/// An integer too large for any machine integer (above `u64::MAX`, so well
/// above `i64::MAX`) rejects its element and nothing else. `json!` cannot
/// hold such a number, so it is spliced into the body text; the same body
/// with `0` spliced in is the control.
#[test]
fn an_integer_beyond_u64_rejects_only_its_element() {
    let _env = Env::install();
    for key in ["duration_ms", "retry_count"] {
        let body = batch(vec![element(1), set(key, json!("__HUGE__"))]).to_string();
        let splice = |literal: &str| body.replace("\"__HUGE__\"", literal).into_bytes();

        let results = Harness::new().post(&splice("0")).expect("control");
        assert_eq!(results.results, [Accepted, Accepted], "{key}");
        let results = Harness::new()
            .post(&splice("18446744073709551616"))
            .expect("still a 200");
        assert_eq!(results.results, [Accepted, Rejected], "{key}");
    }
}

/// The bounds themselves are inside the range: 0 and the maximum of each
/// field are accepted, and so are an empty `phases` and a summary with no
/// optional key at all.
#[test]
fn the_bounds_of_each_field_are_accepted() {
    let _env = Env::install();
    let cases: Vec<(&str, Value)> = vec![
        ("duration 0", set("duration_ms", json!(0))),
        ("duration max", set("duration_ms", json!(604_800_000))),
        ("retry 100", set("retry_count", json!(100))),
        ("version max", set("launcher_version", json!("999.999.999"))),
        ("phases empty", phases(json!([]))),
        (
            "phase duration max",
            phases(json!([{ "phase": "extraction", "duration_ms": 604_800_000 }])),
        ),
        (
            "no optional key",
            json!({
                "event_id": id(0xe5, 2),
                "attempt_id": id(0xa5, 2),
                "operation": "launch",
                "phase": "running",
                "outcome": "unknown",
                "retry_count": 0,
                "launcher_version": "0.0.0",
                "os": "linux",
                "arch": "aarch64",
            }),
        ),
    ];
    for (name, candidate) in cases {
        assert_eq!(beside_a_control(candidate), [Accepted, Accepted], "{name}");
    }
}

/// An id is compared and emitted as its parsed value: the braced, URN,
/// upper-case and hyphenated spellings of one UUID are the same id as its
/// simple spelling, so the second element is a `duplicate`, and the row
/// carries the canonical lower-case hyphenated form whichever arrived.
#[test]
fn two_spellings_of_one_id_are_one_id() {
    let _env = Env::install();
    let canonical = "000000e7-0000-4000-8000-0000000000ab";
    let simple = "000000e70000400080000000000000ab";
    for first in [
        "{000000e7-0000-4000-8000-0000000000ab}",
        "urn:uuid:000000e7-0000-4000-8000-0000000000ab",
        "000000E7-0000-4000-8000-0000000000AB",
        canonical,
    ] {
        let h = Harness::new();
        let body = batch(vec![
            set("event_id", json!(first)),
            set("event_id", json!(simple)),
        ]);
        let (results, rows) = capture(|| h.verdicts(&body));
        assert_eq!(results, [Accepted, Duplicate], "{first}");
        let summaries = summary_rows(&rows);
        assert_eq!(summaries.len(), 1, "{first}");
        assert_eq!(summaries[0].fields["event_id"], canonical, "{first}");
    }

    // Control: a different id in the braced spelling is not a duplicate.
    let body = batch(vec![
        set("event_id", json!("{000000e7-0000-4000-8000-0000000000ac}")),
        set("event_id", json!(simple)),
    ]);
    assert_eq!(Harness::new().verdicts(&body), [Accepted, Accepted]);
}
