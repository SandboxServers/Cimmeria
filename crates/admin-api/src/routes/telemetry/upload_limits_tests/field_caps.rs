//! The string caps on uploaded rows.

use serde_json::{json, Value};

use crate::routes::telemetry::dto::{ClientLogEvent, ClientNativeEvent, TelemetryEvent};
use crate::routes::telemetry::field_caps::{
    cap_event, cap_string, marker, DROPPED_KEYS_FIELD, MAX_FIELD_KEYS, MAX_FIELD_VALUE_BYTES,
    MAX_MESSAGE_BYTES,
};

#[test]
fn a_long_message_is_cut_with_a_marker() {
    let original = "x".repeat(100 * 1024);
    let mut ev = TelemetryEvent::ClientLog(ClientLogEvent {
        ts_ms: 0,
        seq: 0,
        source_file: "f".repeat(1000),
        level: "info".into(),
        category: "raw".into(),
        packet_no: None,
        message: original.clone(),
    });
    cap_event(&mut ev);
    let TelemetryEvent::ClientLog(e) = ev else {
        unreachable!()
    };
    let tail = marker(original.len());
    assert!(
        e.message.ends_with(&tail),
        "{}",
        &e.message[e.message.len() - 64..]
    );
    assert_eq!(e.message.len(), MAX_MESSAGE_BYTES + tail.len());
    assert!(e.source_file.ends_with(&marker(1000)));
    assert_eq!(e.level, "info", "a short value is untouched");
}

/// A cut never splits a character.
#[test]
fn a_cut_lands_on_a_character_boundary() {
    let mut s = "é".repeat(MAX_MESSAGE_BYTES);
    assert!(cap_string(&mut s, MAX_MESSAGE_BYTES - 1));
    assert!(s.ends_with(&marker(2 * MAX_MESSAGE_BYTES)));
}

/// **A small nested value becomes its JSON text.** A dense array well
/// under the 2 KiB value cap would otherwise stay parsed, costing many
/// times its text in memory; scalars are left alone.
#[test]
fn a_small_nested_value_becomes_its_json_text() {
    let mut fields = serde_json::Map::new();
    fields.insert("ids".into(), json!([1, 2, 3]));
    fields.insert("pos".into(), json!({ "x": 1 }));
    fields.insert("n".into(), json!(7));
    let mut ev = TelemetryEvent::ClientNative(ClientNativeEvent {
        ts_ms: 0,
        seq: 0,
        target: "client.lua.pcall".into(),
        level: "info".into(),
        fields,
    });
    cap_event(&mut ev);
    let TelemetryEvent::ClientNative(e) = ev else {
        unreachable!()
    };
    assert_eq!(e.fields["ids"], Value::String("[1,2,3]".into()));
    assert_eq!(e.fields["pos"], Value::String(r#"{"x":1}"#.into()));
    assert_eq!(e.fields["n"], json!(7), "scalars stay as they are");
}

#[test]
fn a_fields_bag_is_capped_in_keys_and_values() {
    let mut fields = serde_json::Map::new();
    for i in 0..100 {
        fields.insert(format!("k{i:03}"), json!(i));
    }
    fields.insert("k000".into(), json!("v".repeat(10_000)));
    fields.insert("k001".into(), json!({ "deep": "w".repeat(10_000) }));
    let mut ev = TelemetryEvent::ClientNative(ClientNativeEvent {
        ts_ms: 0,
        seq: 0,
        target: "client.lua.pcall".into(),
        level: "info".into(),
        fields,
    });
    cap_event(&mut ev);
    let TelemetryEvent::ClientNative(e) = ev else {
        unreachable!()
    };
    assert_eq!(e.fields.len(), MAX_FIELD_KEYS + 1);
    assert_eq!(e.fields[DROPPED_KEYS_FIELD], json!(100 - MAX_FIELD_KEYS));
    for key in ["k000", "k001"] {
        let Value::String(v) = &e.fields[key] else {
            panic!("{key} should be a capped string")
        };
        assert!(v.len() <= MAX_FIELD_VALUE_BYTES + 40, "{key}: {}", v.len());
        assert!(v.contains("...[truncated, "), "{key}");
    }
}
