//! Compact tool results at the MCP edge.
//!
//! Lab-driving agents pay for every result byte on every later turn, so the
//! results an MCP client sees are compact by default:
//!
//! - one-line JSON (no pretty-printing);
//! - null, empty-string, empty-array and empty-object fields left out (a
//!   `false` stays: it answers a question);
//! - floats rounded to 2 decimals;
//! - arrays capped at [`LIST_CAP`] items, with `<key>_total` and
//!   `truncated: true` beside a capped one;
//! - the per-step native trail (`native_steps`, `trail`) left out: the
//!   summary fields (`native_level`, `native_tier` / `tier`, `native_pass` /
//!   `counts_as_native_pass`) stay.
//!
//! `verbose: true` on any call returns the full result, and `fields: [..]`
//! keeps only those top-level keys (dotted paths reach one level down,
//! `position.x`). Both arguments are taken out in `call_tool` before the
//! tool sees its arguments.
//!
//! The UAT runner calls the router in-process (`server::uat`) and never
//! passes through here, so its clause grading reads the full results.

use serde_json::{Map, Value};

/// Items kept per array.
pub const LIST_CAP: usize = 50;

/// The argument that turns compaction off.
pub const VERBOSE_ARG: &str = "verbose";
/// The argument that projects the result onto some top-level keys.
pub const FIELDS_ARG: &str = "fields";

/// Keys that hold a per-step native trail (left out unless verbose).
const TRAIL_KEYS: [&str; 2] = ["native_steps", "trail"];

/// Tools whose top-level list is a drained cursor read: a cap there would
/// drop events the cursor already moved past, so their own `max` governs.
const UNCAPPED: [&str; 4] = [
    "client_events_read",
    "client_chat_log",
    "client_combat_log",
    "client_wait_event",
];

/// How one call's result is shaped.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shape {
    pub verbose: bool,
    pub fields: Vec<String>,
    /// No array cap (cursor readers).
    pub uncapped: bool,
}

impl Shape {
    /// Take `verbose` and `fields` out of a call's arguments.
    pub fn take(tool: &str, args: Option<&mut Map<String, Value>>) -> Self {
        let mut shape = Shape {
            uncapped: UNCAPPED.contains(&tool),
            ..Default::default()
        };
        let Some(args) = args else { return shape };
        if let Some(v) = args.remove(VERBOSE_ARG) {
            shape.verbose = v.as_bool().unwrap_or(false);
        }
        if let Some(f) = args.remove(FIELDS_ARG) {
            shape.fields = match f {
                Value::Array(a) => a
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect(),
                Value::String(s) => s.split(',').map(|s| s.trim().to_string()).collect(),
                _ => Vec::new(),
            };
            shape.fields.retain(|f| !f.is_empty());
        }
        shape
    }

    /// Shape one JSON result. Fields that match nothing are named in
    /// `fields_missing`, with the keys the result does have in
    /// `fields_available`, so a wrong guess costs one call, not a puzzle.
    pub fn apply(&self, v: Value) -> Value {
        let mut v = if self.fields.is_empty() {
            v
        } else {
            let mut p = project(&v, &self.fields);
            let missing: Vec<&String> = self
                .fields
                .iter()
                .filter(|f| {
                    let mut parts = f.splitn(2, '.');
                    let head = parts.next().unwrap_or_default();
                    match parts.next() {
                        None => p.get(head).is_none(),
                        Some(rest) => p.get(head).and_then(|h| h.get(rest)).is_none(),
                    }
                })
                .collect();
            if !missing.is_empty() {
                p["fields_missing"] = serde_json::json!(missing);
                if let Some(o) = v.as_object() {
                    let mut keys: Vec<&String> = o.keys().collect();
                    keys.sort();
                    p["fields_available"] = serde_json::json!(keys);
                }
            }
            p
        };
        if self.verbose {
            return v;
        }
        let cap = if self.uncapped { usize::MAX } else { LIST_CAP };
        // The top level keeps its empty lists and objects: `windows: []`
        // answers "nothing open", where a missing key reads as no answer
        // (client_ui_state came back `{}` once, 2026-10-10).
        if let Value::Object(o) = &mut v {
            let keep: Vec<(String, Value)> = o
                .iter()
                .filter(|(_, x)| {
                    matches!(x, Value::Array(a) if a.is_empty())
                        || matches!(x, Value::Object(m) if m.is_empty())
                })
                .map(|(k, x)| (k.clone(), x.clone()))
                .collect();
            let mut out = compact(v, cap).unwrap_or(Value::Object(Map::new()));
            if let Value::Object(m) = &mut out {
                for (k, x) in keep {
                    m.entry(k).or_insert(x);
                }
            }
            return out;
        }
        compact(v, cap).unwrap_or(Value::Object(Map::new()))
    }

    /// Shape a text block: JSON is re-serialised compactly; anything else
    /// is left as it is.
    pub fn apply_text(&self, text: &str) -> Option<String> {
        let v: Value = serde_json::from_str(text).ok()?;
        let out = self.apply(v);
        Some(if self.verbose {
            serde_json::to_string_pretty(&out).unwrap_or_else(|_| out.to_string())
        } else {
            out.to_string()
        })
    }
}

/// Tools whose results are long enough that `fields` / `verbose` are worth
/// a line in their schema. Every tool accepts both; the server
/// instructions say so once.
const ADVERTISED: [&str; 12] = [
    "client_window_read",
    "client_inventory",
    "client_ui_state",
    "lab_client_status",
    "client_player_state",
    "client_drag_drop",
    "client_events_read",
    "client_chat_log",
    "client_combat_log",
    "client_entity_find",
    "client_hotbar",
    "lab_crash_report",
];

/// Strip what the schema generator adds but no caller needs, from every
/// tool's input schema: `$schema`, `"default": null`, integer `format` and
/// `minimum: 0`, `["T", "null"]` types (an optional argument is already
/// optional by not being `required`), `anyOf [X, {type: null}]` wrappers,
/// and line breaks inside descriptions. Every tool schema is re-sent on
/// every agent turn, so this is paid per turn, per tool.
pub fn slim_schema(v: &mut Value) {
    match v {
        Value::Object(o) => {
            o.remove("$schema");
            if o.get("default").is_some_and(Value::is_null) {
                o.remove("default");
            }
            if o.get("format").and_then(Value::as_str).is_some_and(|f| {
                matches!(
                    f,
                    "uint"
                        | "uint8"
                        | "uint16"
                        | "uint32"
                        | "uint64"
                        | "int32"
                        | "int64"
                        | "double"
                        | "float"
                )
            }) {
                o.remove("format");
            }
            if o.get("minimum").and_then(Value::as_f64) == Some(0.0) {
                o.remove("minimum");
            }
            if let Some(Value::Array(t)) = o.get("type") {
                let non_null: Vec<Value> = t.iter().filter(|x| *x != "null").cloned().collect();
                if non_null.len() == 1 {
                    o.insert("type".into(), non_null[0].clone());
                }
            }
            if let Some(Value::Array(any)) = o.get("anyOf") {
                let non_null: Vec<Value> = any
                    .iter()
                    .filter(|x| x.get("type").is_none_or(|t| t != "null"))
                    .cloned()
                    .collect();
                if non_null.len() == 1 {
                    o.remove("anyOf");
                    if let Value::Object(inner) = &non_null[0] {
                        for (k, x) in inner {
                            o.entry(k.clone()).or_insert_with(|| x.clone());
                        }
                    }
                }
            }
            if let Some(Value::String(d)) = o.get_mut("description") {
                if d.contains('\n') {
                    *d = d.split_whitespace().collect::<Vec<_>>().join(" ");
                }
            }
            for x in o.values_mut() {
                slim_schema(x);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(slim_schema),
        _ => {}
    }
}

/// [`slim_schema`] over every tool.
pub fn slim(tools: Vec<rmcp::model::Tool>) -> Vec<rmcp::model::Tool> {
    tools
        .into_iter()
        .map(|mut t| {
            let mut schema = Value::Object((*t.input_schema).clone());
            slim_schema(&mut schema);
            if let Value::Object(m) = schema {
                t.input_schema = std::sync::Arc::new(m);
            }
            t
        })
        .collect()
}

/// Add `verbose` and `fields` to the heavy tools' schemas.
pub fn advertise(tools: Vec<rmcp::model::Tool>) -> Vec<rmcp::model::Tool> {
    tools
        .into_iter()
        .map(|mut t| {
            if !ADVERTISED.contains(&t.name.as_ref()) {
                return t;
            }
            let mut schema = (*t.input_schema).clone();
            schema
                .entry("type")
                .or_insert_with(|| Value::String("object".into()));
            let props = schema
                .entry("properties")
                .or_insert_with(|| Value::Object(Map::new()));
            if let Some(p) = props.as_object_mut() {
                p.insert(
                    FIELDS_ARG.into(),
                    serde_json::json!({ "type": "array", "items": { "type": "string" },
                        "description": "Keep only these top-level keys (a.b for one level down)." }),
                );
                p.insert(
                    VERBOSE_ARG.into(),
                    serde_json::json!({ "type": "boolean",
                        "description": "Full result: nulls, long lists, step trails." }),
                );
            }
            t.input_schema = std::sync::Arc::new(schema);
            t
        })
        .collect()
}

/// Keep only `fields` of `v` (an object). `a.b` keeps key `b` of object
/// `a`. Unknown names are ignored, so a misspelt field yields `{}` rather
/// than an error.
pub fn project(v: &Value, fields: &[String]) -> Value {
    let Some(obj) = v.as_object() else {
        return v.clone();
    };
    let mut out = Map::new();
    for f in fields {
        match f.split_once('.') {
            None => {
                if let Some(x) = obj.get(f) {
                    out.insert(f.clone(), x.clone());
                }
            }
            Some((head, rest)) => {
                if let Some(x) = obj.get(head).and_then(|x| x.get(rest)) {
                    let slot = out
                        .entry(head.to_string())
                        .or_insert_with(|| Value::Object(Map::new()));
                    if let Some(m) = slot.as_object_mut() {
                        m.insert(rest.to_string(), x.clone());
                    }
                }
            }
        }
    }
    Value::Object(out)
}

/// Round a float to 2 decimals; integers pass through.
fn round_number(n: &serde_json::Number) -> Value {
    if n.is_f64() {
        if let Some(f) = n.as_f64() {
            let r = (f * 100.0).round() / 100.0;
            // A whole number prints without the trailing `.0`.
            if r.fract() == 0.0 && r.abs() < 9.0e15 {
                return Value::from(r as i64);
            }
            return serde_json::Number::from_f64(r)
                .map(Value::Number)
                .unwrap_or(Value::Null);
        }
    }
    Value::Number(n.clone())
}

/// Compact `v`; `None` when it compacts to nothing (null, `""`, `[]`, `{}`).
pub fn compact(v: Value, cap: usize) -> Option<Value> {
    match v {
        Value::Null => None,
        Value::String(s) if s.is_empty() => None,
        Value::Number(n) => Some(round_number(&n)),
        Value::Array(a) => {
            let items: Vec<Value> = a
                .into_iter()
                .take(cap)
                .filter_map(|x| compact(x, cap).or(Some(Value::Null)))
                .collect();
            (!items.is_empty()).then_some(Value::Array(items))
        }
        Value::Object(o) => {
            let mut out = Map::new();
            let mut truncated = false;
            for (k, x) in o {
                if TRAIL_KEYS.contains(&k.as_str()) {
                    continue;
                }
                if let Value::Array(a) = &x {
                    if a.len() > cap {
                        out.insert(format!("{k}_total"), Value::from(a.len()));
                        truncated = true;
                    }
                }
                if let Some(c) = compact(x, cap) {
                    out.insert(k, c);
                }
            }
            if truncated {
                out.insert("truncated".into(), Value::Bool(true));
            }
            (!out.is_empty()).then_some(Value::Object(out))
        }
        other => Some(other),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn defaults_are_omitted_and_false_is_kept() {
        let v = json!({ "a": null, "b": "", "c": [], "d": {}, "e": false, "f": 0,
                        "g": { "h": null } });
        assert_eq!(compact(v, LIST_CAP).unwrap(), json!({ "e": false, "f": 0 }));
    }

    #[test]
    fn floats_round_to_two_decimals_and_whole_floats_print_as_integers() {
        let v = json!({ "x": 1234.56789, "y": -0.004, "z": 3.0, "n": 7 });
        assert_eq!(
            compact(v, LIST_CAP).unwrap(),
            json!({ "x": 1234.57, "y": 0, "z": 3, "n": 7 })
        );
    }

    #[test]
    fn long_lists_are_capped_with_a_total_and_a_flag() {
        let v = json!({ "rows": (0..60).collect::<Vec<_>>() });
        let c = compact(v, 50).unwrap();
        assert_eq!(c["rows"].as_array().unwrap().len(), 50);
        assert_eq!(c["rows_total"], 60);
        assert_eq!(c["truncated"], true);
        let short = compact(json!({ "rows": [1, 2] }), 50).unwrap();
        assert!(short.get("truncated").is_none());
    }

    #[test]
    fn the_native_trail_is_dropped_but_the_summary_stays() {
        let v = json!({ "native_level": "native_cegui", "native_tier": "N1",
                        "native_pass": true, "counts_as_native_pass": true, "tier": "N1",
                        "native_steps": [{ "step": "press" }], "trail": ["x"] });
        let c = compact(v, LIST_CAP).unwrap();
        assert!(c.get("native_steps").is_none() && c.get("trail").is_none());
        for k in [
            "native_level",
            "native_tier",
            "native_pass",
            "counts_as_native_pass",
            "tier",
        ] {
            assert!(c.get(k).is_some(), "{k} kept");
        }
    }

    #[test]
    fn verbose_and_fields_are_taken_out_of_the_arguments() {
        let mut args = json!({ "verbose": true, "fields": ["position", "world_id"], "x": 1 });
        let s = Shape::take("client_player_state", args.as_object_mut());
        assert!(s.verbose);
        assert_eq!(s.fields, vec!["position", "world_id"]);
        assert_eq!(args, json!({ "x": 1 }));
        let mut csv = json!({ "fields": "a, b" });
        assert_eq!(Shape::take("t", csv.as_object_mut()).fields, vec!["a", "b"]);
    }

    #[test]
    fn fields_project_top_level_and_dotted_keys() {
        let v = json!({ "position": { "x": 1.0, "y": 2.0 }, "world_id": 7, "stats": {} });
        let p = project(&v, &["world_id".into(), "position.x".into(), "nope".into()]);
        assert_eq!(p, json!({ "world_id": 7, "position": { "x": 1.0 } }));
    }

    #[test]
    fn verbose_keeps_everything_and_cursor_readers_are_uncapped() {
        let v = json!({ "a": null, "native_steps": [1], "x": 1.23456 });
        let verbose = Shape {
            verbose: true,
            ..Default::default()
        };
        assert_eq!(verbose.apply(v.clone()), v);
        let mut args = json!({});
        let events = Shape::take("client_events_read", args.as_object_mut());
        let big = json!({ "events": (0..80).collect::<Vec<_>>() });
        assert_eq!(events.apply(big)["events"].as_array().unwrap().len(), 80);
    }

    /// Regression guard (2026-10-10, bank UAT): `client_ui_state` came back
    /// `{}` because every top-level list was empty, and `fields: ["text"]`
    /// on a window read returned `{}` with no hint.
    #[test]
    fn top_level_empties_stay_and_missing_fields_are_named() {
        let s = Shape::default();
        let v = json!({ "windows": [], "dialog": null, "chat": {}, "nested": { "x": [] } });
        assert_eq!(s.apply(v), json!({ "windows": [], "chat": {} }));
        let f = Shape {
            fields: vec!["text".into(), "title".into()],
            ..Default::default()
        };
        let out = f.apply(json!({ "title": "Lance", "texts": ["hi"], "buttons": [] }));
        assert_eq!(out["title"], "Lance");
        assert_eq!(out["fields_missing"], json!(["text"]));
        assert_eq!(
            out["fields_available"],
            json!(["buttons", "texts", "title"])
        );
    }

    #[test]
    fn schemas_lose_generator_noise_but_keep_meaning() {
        let mut s = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "required": ["from"],
            "properties": {
                "steps": { "default": null, "format": "uint32", "minimum": 0,
                           "type": ["integer", "null"], "description": "Cursor steps\nbetween." },
                "point": { "anyOf": [{ "$ref": "#/$defs/PointArg" }, { "type": "null" }],
                           "description": "A point." },
                "min": { "type": "integer", "minimum": 3 },
                "kind": { "type": ["string", "integer"] }
            }
        });
        slim_schema(&mut s);
        assert!(s.get("$schema").is_none());
        assert_eq!(
            s["properties"]["steps"],
            json!({ "type": "integer", "description": "Cursor steps between." })
        );
        assert_eq!(
            s["properties"]["point"],
            json!({ "$ref": "#/$defs/PointArg", "description": "A point." })
        );
        assert_eq!(s["properties"]["min"]["minimum"], 3, "a real bound stays");
        assert_eq!(
            s["properties"]["kind"]["type"],
            json!(["string", "integer"])
        );
        assert_eq!(s["required"], json!(["from"]));
    }

    #[test]
    fn text_results_are_reserialised_on_one_line() {
        let s = Shape::default();
        let out = s.apply_text("{\n  \"a\": 1,\n  \"b\": null\n}").unwrap();
        assert_eq!(out, r#"{"a":1}"#);
        assert!(s.apply_text("client window 800x600").is_none());
    }
}
