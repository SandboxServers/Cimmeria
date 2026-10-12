//! Unit tests for [`super`] (`compact.rs`).

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
    let s = Shape::take("client_player_state", args.as_object_mut(), &[]);
    assert!(s.verbose);
    assert_eq!(s.fields, vec!["position", "world_id"]);
    assert_eq!(args, json!({ "x": 1 }));
    let mut csv = json!({ "fields": "a, b" });
    assert_eq!(
        Shape::take("t", csv.as_object_mut(), &[]).fields,
        vec!["a", "b"]
    );
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
    let events = Shape::take("client_events_read", args.as_object_mut(), &[]);
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

/// Images are saved and replaced by a path unless `image: true`.
#[test]
fn images_are_saved_to_a_file_by_default() {
    let mut args = json!({ "image": true });
    let s = Shape::take("lab_screenshot", args.as_object_mut(), &[]);
    assert!(s.image);
    assert_eq!(s.tool, "lab_screenshot");
    assert_eq!(args, json!({}));
    assert!(!Shape::take("lab_screenshot", None, &[]).image);

    let dir = tempfile::tempdir().unwrap();
    // "iVBORw==" is the PNG magic's first four bytes.
    let p = save_image_in(dir.path(), "lab_screenshot", "iVBORw==").unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), vec![0x89, b'P', b'N', b'G']);
    assert!(p
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("lab_screenshot-"));
    assert!(save_image_in(dir.path(), "t", "not base64!").is_err());
}

/// Regression guard (review of #1309): a tool's own `fields` argument
/// (client_wait_event's equality filter) is left with the tool.
#[test]
fn a_tools_own_shape_arguments_are_left_alone() {
    let mut args = json!({ "fields": { "hit_name": "Critical" }, "verbose": true });
    let own = vec!["fields".to_string()];
    let s = Shape::take("client_wait_event", args.as_object_mut(), &own);
    assert!(s.fields.is_empty());
    assert!(s.verbose, "verbose is not the tool's own, so it is taken");
    assert_eq!(args, json!({ "fields": { "hit_name": "Critical" } }));
}

/// The live router: every tool that declares a shaping argument keeps
/// it, which is what call_tool computes from the schemas.
#[test]
fn client_wait_event_declares_its_own_fields() {
    use std::sync::Arc;
    let config = crate::supervisor::SupervisorConfig {
        install_dir: None,
        dll_path: None,
        patches_dll: None,
        helper_path: None,
        bind: "127.0.0.1".into(),
        port: 8770,
        instance: None,
        telemetry: Default::default(),
    };
    let bridge = Arc::new(crate::client::BridgeClient::new("127.0.0.1:1", ""));
    let server =
        crate::server::LabServer::new(Arc::new(crate::supervisor::Supervisor::new(bridge, config)));
    let t = server.tool_router.get("client_wait_event").unwrap();
    let props = t.input_schema["properties"].as_object().unwrap();
    assert!(props.contains_key(FIELDS_ARG));
}

/// Regression guard (review of #1309): probe floats are not rounded;
/// arrays keep empty items in place and mark a cap without a parent
/// key; a bare non-object result is not turned into `{}`.
#[test]
fn probes_keep_exact_floats_and_lists_keep_their_shape() {
    let mut args = json!({});
    let probe = Shape::take("client_batch", args.as_object_mut(), &[]);
    assert_eq!(probe.apply(json!({ "r": 0.0174 }))["r"], 0.0174);
    let read = Shape::take("client_player_state", args.as_object_mut(), &[]);
    assert_eq!(read.apply(json!({ "r": 0.0174 }))["r"], 0.02);

    let v = compact(json!({ "rows": ["a", "", null, "b"] }), 50).unwrap();
    assert_eq!(v["rows"], json!(["a", "", null, "b"]));
    let nested = compact(json!([(0..60).collect::<Vec<_>>()]), 50).unwrap();
    let inner = nested[0].as_array().unwrap();
    assert_eq!(inner.len(), 51);
    assert_eq!(inner[50], json!({ "truncated_total": 60 }));

    let s = Shape::default();
    assert_eq!(s.apply(json!([])), json!([]));
    assert_eq!(s.apply(json!("")), json!(""));
    assert_eq!(s.apply(Value::Null), Value::Null);
}

#[test]
fn old_saved_images_are_pruned() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.png");
    std::fs::write(&old, b"x").unwrap();
    prune_images(dir.path(), std::time::Duration::ZERO);
    assert!(!old.exists());
    let keep = dir.path().join("new.png");
    std::fs::write(&keep, b"x").unwrap();
    prune_images(dir.path(), std::time::Duration::from_secs(3600));
    assert!(keep.exists());
}

#[test]
fn text_results_are_reserialised_on_one_line() {
    let s = Shape::default();
    let out = s.apply_text("{\n  \"a\": 1,\n  \"b\": null\n}").unwrap();
    assert_eq!(out, r#"{"a":1}"#);
    assert!(s.apply_text("client window 800x600").is_none());
}
