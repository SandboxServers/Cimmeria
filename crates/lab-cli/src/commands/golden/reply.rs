//! The `lab_golden_record` reply as an exit code, compact JSON and the line
//! `lab golden record` prints. Ports `ConvertTo-GoldenExit`,
//! `ConvertTo-GoldenCompactJson` and `Format-GoldenRecordLine`.

use serde::Serialize;
use serde_json::Value;

use crate::ui::{Mark, Tone, Ui};

/// A JSON value as text: `null` and a missing key are empty, a string is
/// itself, anything else its JSON text (what PowerShell's `"$v"` gives).
pub fn text_of(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// The first line of an error text.
pub fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}

/// `lab_golden_record`'s reply as the command's exit code (README Review
/// outcomes, GD-07): `ok` 0; `exists` 2; `disagree` 6; `not_passing` 1;
/// `cannot_record` and every other code, or none, 7.
pub fn reply_exit(reply: &Value) -> u8 {
    if reply.get("ok").and_then(Value::as_bool) == Some(true) {
        return 0;
    }
    match text_of(reply.get("code")).as_str() {
        "exists" => 2,
        "disagree" => 6,
        "not_passing" => 1,
        _ => 7,
    }
}

/// A reply field that is neither missing, `null` nor an empty string.
fn present<'a>(reply: &'a Value, key: &str) -> Option<&'a Value> {
    reply
        .get(key)
        .filter(|v| !v.is_null() && v.as_str() != Some(""))
}

/// Key order is the PowerShell's: `ok, exit`, then on success `path, rows,
/// events, bytes, runs`, else `code, error, batch`. Absent keys are skipped.
#[derive(Serialize)]
struct Compact<'a> {
    ok: bool,
    exit: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rows: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    events: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    runs: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    batch: Option<&'a str>,
}

/// The reply as one compact JSON object with no nulls. A refusal after the
/// runs names the batch folder they are in.
pub fn compact_json(reply: &Value, exit: u8, batch: Option<&str>) -> String {
    let success = exit == 0;
    let pick = |key: &str, on_success: bool| {
        if on_success == success {
            present(reply, key)
        } else {
            None
        }
    };
    let compact = Compact {
        ok: success,
        exit,
        path: pick("path", true),
        rows: pick("rows", true),
        events: pick("events", true),
        bytes: pick("bytes", true),
        runs: pick("runs", true),
        code: pick("code", false),
        error: pick("error", false),
        batch: if success {
            None
        } else {
            batch.filter(|b| !b.is_empty())
        },
    };
    serde_json::to_string(&compact).unwrap_or_default()
}

/// The one line `lab golden record` prints on success (plain mode).
pub fn record_line(section: &str, reply: &Value) -> String {
    let f = |key: &str| text_of(reply.get(key));
    format!(
        "golden {section}: {} rows, {} events, {} bytes from {} runs; {}",
        f("rows"),
        f("events"),
        f("bytes"),
        f("runs"),
        f("path")
    )
}

/// Fancy mode: a green check and the counts, then the path in cyan.
pub fn record_line_fancy(ui: &Ui, section: &str, reply: &Value) -> String {
    let f = |key: &str| text_of(reply.get(key));
    let head = format!(
        "{} golden {section}: {} rows, {} events from {} runs",
        ui.paint(Tone::Ok, ui.mark(Mark::Pass)),
        f("rows"),
        f("events"),
        f("runs")
    );
    format!("{head}\n  {}", ui.paint(Tone::Cmd, &f("path")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::strip_ansi;
    use serde_json::json;

    const BATCH_DIR: &str =
        r"C:\Users\someone\AppData\Local\cimmeria-lab\uat-runs\batch-20261011-101500";

    fn ok_reply() -> Value {
        json!({"ok": true,
               "path": r"C:\Users\someone\source\projects\Cimmeria\docs\guides\uat-specs\golden\first-session.json",
               "rows": 7, "events": 312, "bytes": 21456, "runs": 5})
    }

    // PS L115
    #[test]
    fn ok_is_exit_0() {
        assert_eq!(reply_exit(&json!({"ok": true, "path": "p", "rows": 7})), 0);
    }

    // PS L116-L119 (one check per row of the table; loops count once)
    #[test]
    fn each_refusal_code_maps_to_its_exit_code() {
        for (code, exit) in [
            ("exists", 2),
            ("disagree", 6),
            ("not_passing", 1),
            ("cannot_record", 7),
            ("cannot_diff", 7),
            ("no_golden", 7),
            ("", 7),
        ] {
            let reply = json!({"ok": false, "code": code, "error": "x"});
            assert_eq!(reply_exit(&reply), exit, "code '{code}'");
        }
    }

    // PS L120
    #[test]
    fn a_refusal_without_a_code_is_7() {
        assert_eq!(reply_exit(&json!({"ok": false})), 7);
    }

    // PS L126
    #[test]
    fn a_success_stays_under_300_characters() {
        let text = compact_json(&ok_reply(), 0, None);
        assert!(text.len() < 300, "{} chars", text.len());
    }

    // PS L127
    #[test]
    fn a_success_has_no_nulls() {
        assert!(!compact_json(&ok_reply(), 0, None).contains("null"));
        let sparse =
            json!({"ok": true, "path": "p", "rows": 7, "events": null, "bytes": "", "runs": 5});
        assert_eq!(
            compact_json(&sparse, 0, None),
            r#"{"ok":true,"exit":0,"path":"p","rows":7,"runs":5}"#
        );
    }

    // PS L129
    #[test]
    fn a_success_carries_rows_events_bytes_and_runs() {
        let v: Value = serde_json::from_str(&compact_json(&ok_reply(), 0, None)).unwrap();
        assert_eq!(v["ok"], json!(true));
        assert_eq!(v["exit"], json!(0));
        assert_eq!(
            (&v["rows"], &v["events"], &v["bytes"], &v["runs"]),
            (&json!(7), &json!(312), &json!(21456), &json!(5))
        );
    }

    // PS L132
    #[test]
    fn a_refusal_carries_its_code_and_error() {
        let reply =
            json!({"ok": false, "code": "disagree", "error": "run b differs: FS-P3 event 4"});
        let v: Value = serde_json::from_str(&compact_json(&reply, 6, None)).unwrap();
        assert_eq!(v["ok"], json!(false));
        assert_eq!(v["exit"], json!(6));
        assert_eq!(v["code"], json!("disagree"));
        assert!(v["error"].as_str().unwrap().starts_with("run b"));
    }

    // PS L133
    #[test]
    fn a_refusal_has_no_nulls_and_no_success_fields() {
        let reply = json!({"ok": false, "code": "disagree", "error": "run b differs", "path": "p"});
        let text = compact_json(&reply, 6, None);
        assert!(!text.contains("null"));
        assert!(!text.contains("path"));
    }

    // PS L135
    #[test]
    fn a_missing_error_is_left_out_not_null() {
        let reply = json!({"ok": false, "code": "cannot_record", "error": null});
        let text = compact_json(&reply, 7, None);
        assert!(!text.contains("null"));
        assert!(!text.contains("error"));
    }

    // PS L136-L139 (loop: one check)
    #[test]
    fn a_refusal_names_the_batch_folder() {
        for (code, exit) in [("disagree", 6u8), ("not_passing", 1), ("cannot_record", 7)] {
            let reply = json!({"ok": false, "code": code, "error": "x"});
            let v: Value =
                serde_json::from_str(&compact_json(&reply, exit, Some(BATCH_DIR))).unwrap();
            assert_eq!(v["exit"], json!(exit), "{code}");
            assert_eq!(v["batch"], json!(BATCH_DIR), "{code}");
        }
    }

    // PS L140
    #[test]
    fn a_success_names_the_golden_path_not_the_batch() {
        assert!(!compact_json(&ok_reply(), 0, Some(BATCH_DIR)).contains("batch"));
    }

    // PS L141
    #[test]
    fn no_batch_folder_is_left_out_not_empty() {
        let reply = json!({"ok": false, "code": "disagree", "error": "x"});
        assert!(!compact_json(&reply, 6, Some("")).contains("batch"));
        assert!(!compact_json(&reply, 6, None).contains("batch"));
    }

    // New: the key order is part of the contract (guide: ok, exit, path, rows, events, bytes, runs).
    #[test]
    fn compact_json_key_order_is_fixed() {
        let reply =
            json!({"ok": true, "path": "p", "rows": 7, "events": 312, "bytes": 21456, "runs": 5});
        assert_eq!(
            compact_json(&reply, 0, None),
            r#"{"ok":true,"exit":0,"path":"p","rows":7,"events":312,"bytes":21456,"runs":5}"#
        );
        let bad = json!({"ok": false, "code": "disagree", "error": "e"});
        assert_eq!(
            compact_json(&bad, 6, Some("B")),
            r#"{"ok":false,"exit":6,"code":"disagree","error":"e","batch":"B"}"#
        );
    }

    // PS L143
    #[test]
    fn the_record_line() {
        let reply = ok_reply();
        let path = reply["path"].as_str().unwrap();
        assert_eq!(
            record_line("first-session", &reply),
            format!("golden first-session: 7 rows, 312 events, 21456 bytes from 5 runs; {path}")
        );
    }

    // New: fancy mode.
    #[test]
    fn the_fancy_record_line_has_a_check_and_the_path_on_its_own_line() {
        let ui = Ui {
            color: true,
            fancy: true,
            width: None,
        };
        let text = record_line_fancy(&ui, "first-session", &ok_reply());
        let path = ok_reply()["path"].as_str().unwrap().to_string();
        assert_eq!(
            strip_ansi(&text),
            format!("✓ golden first-session: 7 rows, 312 events from 5 runs\n  {path}")
        );
        assert!(text.contains("\x1b[36m"), "the path is cyan");
    }
}
