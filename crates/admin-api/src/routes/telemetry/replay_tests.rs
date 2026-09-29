//! What a replayed row carries: the session's identity and kind from the
//! token, the DLL's event name as the body, and the lifted correlation
//! keys only when the DLL sent them. Captured with a local field-recording
//! layer (this crate has no `LogCapture`); like the OTLP bridge it sees
//! only the event's own fields, so a field asserted here is one SigNoz
//! can filter on.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde_json::json;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

use crate::routes::dev_session::TokenClaims;

use super::dto::ClientNativeEvent;
use super::replay::{
    replay_client_native, replay_ndjson, replay_ndjson_gated, LiftedFields, ReplayError,
};

#[derive(Debug, Clone)]
struct Row {
    target: String,
    level: tracing::Level,
    fields: BTreeMap<String, String>,
}

#[derive(Clone, Default)]
struct Rows(Arc<Mutex<Vec<Row>>>);

struct FieldText<'a>(&'a mut BTreeMap<String, String>);

impl Visit for FieldText<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{value:?}"));
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_string());
    }
}

impl<S: Subscriber> Layer<S> for Rows {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let mut fields = BTreeMap::new();
        event.record(&mut FieldText(&mut fields));
        self.0.lock().unwrap().push(Row {
            target: event.metadata().target().to_string(),
            level: *event.metadata().level(),
            fields,
        });
    }
}

fn capture(f: impl FnOnce()) -> Vec<Row> {
    let rows = Rows::default();
    let sub = tracing_subscriber::registry().with(rows.clone());
    tracing::subscriber::with_default(sub, f);
    let out = rows.0.lock().unwrap().clone();
    out
}

fn claims(kind: Option<&str>) -> TokenClaims {
    TokenClaims {
        iss: "cimmeria-server".into(),
        sub: "install-1".into(),
        sid: "session-1".into(),
        iat: 0,
        exp: i64::MAX,
        scope: vec!["telemetry.write".into()],
        kind: kind.map(str::to_string),
    }
}

fn native(level: &str, fields: serde_json::Value) -> ClientNativeEvent {
    ClientNativeEvent {
        ts_ms: 1_700_000_000_000,
        seq: 7,
        target: "client.dispatch.method_dropped".into(),
        level: level.into(),
        fields: fields.as_object().cloned().unwrap_or_default(),
    }
}

/// A lab session's DLL row carries the lab tag, the token's identity, the
/// event name as its body and the lifted method index, at the DLL's level.
#[test]
fn a_lab_client_row_carries_kind_identity_and_lifted_keys() {
    let rows = capture(|| {
        replay_client_native(
            &claims(Some("lab")),
            native("warn", json!({ "method_index": 41, "account_id": 6 })),
        );
    });
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.target, "client.native");
    assert_eq!(r.level, tracing::Level::WARN);
    assert_eq!(r.fields["cimmeria.session_kind"], "lab");
    assert_eq!(r.fields["lab"], "true");
    assert_eq!(r.fields["session_id"], "session-1");
    assert_eq!(r.fields["install_id"], "install-1");
    assert_eq!(r.fields["client_target"], "client.dispatch.method_dropped");
    assert_eq!(r.fields["client_level"], "warn");
    assert_eq!(r.fields["message"], "client.dispatch.method_dropped");
    assert_eq!(r.fields["method_index"], "41");
    assert_eq!(r.fields["account_id"], "6");
    assert!(r.fields["fields"].contains("\"method_index\":41"));
}

/// A player session is tagged `player`, and a key the DLL did not send is
/// absent rather than a `0` sentinel that would match every row.
#[test]
fn a_player_row_omits_the_keys_the_dll_did_not_send() {
    let rows = capture(|| replay_client_native(&claims(None), native("info", json!({}))));
    let r = &rows[0];
    assert_eq!(r.fields["cimmeria.session_kind"], "player");
    assert_eq!(r.fields["lab"], "false");
    for absent in [
        "account_id",
        "player_id",
        "method_index",
        "level_name",
        "dll_version",
        "fingerprint_usable",
    ] {
        assert!(!r.fields.contains_key(absent), "{absent} must be omitted");
    }
}

/// A level string the server does not know is replayed at INFO, keeping
/// the raw string, so a DLL typo never drops the row.
#[test]
fn an_unknown_level_is_replayed_at_info_with_the_raw_string() {
    let rows = capture(|| replay_client_native(&claims(None), native("verbose", json!({}))));
    assert_eq!(rows[0].level, tracing::Level::INFO);
    assert_eq!(rows[0].fields["client_level"], "verbose");
}

/// Lifting takes each key only at its JSON type: a string `account_id`
/// is not guessed into a number.
#[test]
fn lifted_fields_read_only_well_typed_keys() {
    let f = json!({
        "account_id": "6",
        "player_id": 9,
        "level_name": "Castle_CellBlock",
        "dll_version": "0.1.0+abc",
        "usable": true,
    });
    let l = LiftedFields::from_fields(f.as_object().unwrap());
    assert_eq!(l.account_id, None);
    assert_eq!(l.player_id, Some(9));
    assert_eq!(l.level_name.as_deref(), Some("Castle_CellBlock"));
    assert_eq!(l.dll_version.as_deref(), Some("0.1.0+abc"));
    assert_eq!(l.fingerprint_usable, Some(true));
}

/// The chunk path: every line is replayed with the session's kind, the
/// launcher's own event types included, and blank lines are skipped.
#[test]
fn replay_ndjson_tags_every_event_type_with_the_session_kind() {
    let ndjson = concat!(
        r#"{"type":"client_native","ts_ms":1,"seq":1,"target":"client.dll.attached","level":"info","fields":{"dll_version":"0.1.0"}}"#,
        "\n\n",
        r#"{"type":"client_log","ts_ms":2,"seq":2,"source_file":"a","level":"info","category":"raw","message":"m"}"#,
        "\n",
        r#"{"type":"session_meta","ts_ms":3,"seq":3,"kind":"start","fields":{}}"#,
        "\n",
    );
    let mut counts = None;
    let rows = capture(|| counts = Some(replay_ndjson(&claims(Some("lab")), ndjson)));
    let counts = counts.unwrap().unwrap();
    assert_eq!((counts.parsed, counts.accepted), (3, 3));
    let targets: Vec<_> = rows.iter().map(|r| r.target.as_str()).collect();
    assert_eq!(
        targets,
        [
            "client.native",
            "launcher.client_log",
            "launcher.session_meta"
        ]
    );
    for r in &rows {
        assert_eq!(r.fields["cimmeria.session_kind"], "lab", "{}", r.target);
    }
    assert_eq!(rows[0].fields["dll_version"], "0.1.0");
}

/// A malformed line refuses the chunk and names the line, as the handler
/// did before the replay moved here.
#[test]
fn replay_ndjson_reports_the_first_bad_line() {
    let err = replay_ndjson(&claims(None), "\n{\"type\":\"nope\"}\n").unwrap_err();
    assert!(matches!(err, ReplayError { line: 2, .. }), "{err:?}");
}

/// A governor rollup lifts its summarized target and count, so SigNoz can
/// sum `rollup_count` by `rollup_target`; a plain `count` elsewhere is not
/// mistaken for one.
#[test]
fn a_rollup_lifts_its_target_and_count() {
    let mut e = native(
        "info",
        json!({ "rollup_target": "client.lua.pcall", "count": 2059, "reason": "hot_stream" }),
    );
    e.target = "client.telemetry.rollup".into();
    let rows = capture(|| replay_client_native(&claims(Some("lab")), e));
    assert_eq!(rows[0].fields["rollup_target"], "client.lua.pcall");
    assert_eq!(rows[0].fields["rollup_count"], "2059");

    let rows =
        capture(|| replay_client_native(&claims(None), native("info", json!({ "count": 3 }))));
    assert!(!rows[0].fields.contains_key("rollup_count"));
}

/// Events the session budget refuses are parsed and counted, never
/// replayed; the rest replay in order.
#[test]
fn a_gated_replay_counts_what_it_suppresses() {
    let line = |seq: u32, level: &str| {
        format!(
            r#"{{"type":"client_native","ts_ms":1,"seq":{seq},"target":"client.lua.pcall","level":"{level}"}}"#
        )
    };
    let ndjson = [line(1, "debug"), line(2, "debug"), line(3, "warn")].join(
        "
",
    );
    let mut counts = None;
    let rows = capture(|| {
        counts = Some(replay_ndjson_gated(
            &claims(None),
            &ndjson,
            |ev| matches!(ev, super::dto::TelemetryEvent::ClientNative(e) if e.level == "warn"),
        ))
    });
    let counts = counts.unwrap().unwrap();
    assert_eq!(
        (counts.parsed, counts.accepted, counts.suppressed),
        (3, 1, 2)
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].level, tracing::Level::WARN);
}
