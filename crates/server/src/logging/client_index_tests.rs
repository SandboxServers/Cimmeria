//! Ingest round trip for the `cimmeria-client` SigNoz index: an uploaded
//! client event, replayed by the real admin-api ingest path, reaches the
//! OTLP exporter behind the client log layer, with the client resource and
//! its per-record attributes, and does not reach the server's exporter.
//!
//! The providers here are the production ones minus the network: the
//! resource comes from [`otel::client_resource`], the bridge is the same
//! `OpenTelemetryTracingBridge`, and the per-layer filters are the ones
//! `init_logging` installs. Only the exporter is a recorder.

use std::sync::{Arc, Mutex};

use opentelemetry::logs::AnyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::logs::{LogBatch, LogExporter, SdkLoggerProvider};
use opentelemetry_sdk::Resource;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;

use cimmeria_admin_api::routes::dev_session::TokenClaims;
use cimmeria_admin_api::routes::telemetry::replay_ndjson;

use super::filters::{otel_client_log_filter, otel_server_log_filter};
use crate::otel;

/// One exported log record, flattened for assertions.
#[derive(Debug, Clone)]
struct Exported {
    target: String,
    body: String,
    attrs: Vec<(String, String)>,
}

impl Exported {
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

fn text(v: &AnyValue) -> String {
    match v {
        AnyValue::String(s) => s.as_str().to_string(),
        AnyValue::Boolean(b) => b.to_string(),
        AnyValue::Int(i) => i.to_string(),
        other => format!("{other:?}"),
    }
}

/// A `LogExporter` that keeps what it is given: the records and the
/// resource the provider hands it, which is what goes on the wire.
#[derive(Debug, Clone, Default)]
struct Recorder {
    records: Arc<Mutex<Vec<Exported>>>,
    resource: Arc<Mutex<Vec<(String, String)>>>,
}

impl LogExporter for Recorder {
    async fn export(&self, batch: LogBatch<'_>) -> OTelSdkResult {
        let mut out = self.records.lock().unwrap();
        for (rec, _scope) in batch.iter() {
            out.push(Exported {
                target: rec.target().map(|t| t.to_string()).unwrap_or_default(),
                body: rec.body().map(text).unwrap_or_default(),
                attrs: rec
                    .attributes_iter()
                    .map(|(k, v)| (k.as_str().to_string(), text(v)))
                    .collect(),
            });
        }
        Ok(())
    }

    fn set_resource(&mut self, resource: &Resource) {
        *self.resource.lock().unwrap() = resource
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_string()))
            .collect();
    }
}

fn provider(resource: Resource) -> (SdkLoggerProvider, Recorder) {
    let rec = Recorder::default();
    let p = SdkLoggerProvider::builder()
        .with_simple_exporter(rec.clone())
        .with_resource(resource)
        .build();
    (p, rec)
}

fn lab_claims() -> TokenClaims {
    TokenClaims {
        iss: "cimmeria-server".into(),
        sub: "cimmeria-lab".into(),
        sid: "session-rt".into(),
        iat: 0,
        exp: i64::MAX,
        scope: vec!["telemetry.write".into()],
        kind: Some("lab".into()),
    }
}

/// **The round trip.** A lab session uploads a DLL event and a tailed game
/// log line; both reach the client exporter under `service.name =
/// cimmeria-client` with `cimmeria.source = client`, tagged lab, with the
/// DLL event name as the body. The server exporter sees neither, and still
/// sees the server's own row.
///
/// Reverting the client exclusion in `routes_to_server` puts the client
/// rows in the server exporter too and fails this.
#[test]
fn a_replayed_client_event_reaches_the_client_index_with_its_attributes() {
    let (server_p, server_rec) = provider(
        Resource::builder()
            .with_service_name("cimmeria-server")
            .build(),
    );
    let (client_p, client_rec) = provider(otel::client_resource("colo", "ingest-box", "abc123"));

    let subscriber = tracing_subscriber::registry()
        .with(OpenTelemetryTracingBridge::new(&server_p).with_filter(otel_server_log_filter()))
        .with(OpenTelemetryTracingBridge::new(&client_p).with_filter(otel_client_log_filter()));

    let ndjson = concat!(
        r#"{"type":"client_native","ts_ms":1,"seq":1,"target":"client.dll.attached","level":"info","fields":{"dll_version":"0.1.0","session_id":"s"}}"#,
        "\n",
        r#"{"type":"client_log","ts_ms":2,"seq":2,"source_file":"sgw.log","level":"warn","category":"raw","message":"tail line"}"#,
        "\n",
    );
    tracing::subscriber::with_default(subscriber, || {
        replay_ndjson(&lab_claims(), ndjson).expect("valid chunk");
        tracing::warn!(target: "cimmeria_server::client_index_probe", "server row");
    });
    client_p.force_flush().unwrap();
    server_p.force_flush().unwrap();

    let resource = client_rec.resource.lock().unwrap().clone();
    let res = |k: &str| {
        resource
            .iter()
            .find(|(rk, _)| rk == k)
            .map(|(_, v)| v.clone())
    };
    assert_eq!(res("service.name").as_deref(), Some("cimmeria-client"));
    assert_eq!(res("cimmeria.source").as_deref(), Some("client"));
    assert_eq!(res("cimmeria.deploy_env").as_deref(), Some("colo"));
    assert_eq!(res("cimmeria.ingest_host").as_deref(), Some("ingest-box"));
    assert_eq!(res("host.name"), None, "host.name would name the server");

    let client = client_rec.records.lock().unwrap().clone();
    assert_eq!(client.len(), 2, "{client:#?}");
    let dll = &client[0];
    assert_eq!(dll.target, "client.native");
    assert_eq!(dll.body, "client.dll.attached");
    assert_eq!(dll.attr("client_target"), Some("client.dll.attached"));
    assert_eq!(dll.attr("cimmeria.session_kind"), Some("lab"));
    assert_eq!(dll.attr("lab"), Some("true"));
    assert_eq!(dll.attr("session_id"), Some("session-rt"));
    assert_eq!(dll.attr("dll_version"), Some("0.1.0"));
    let tail = &client[1];
    assert_eq!(tail.target, "launcher.client_log");
    assert_eq!(tail.attr("cimmeria.session_kind"), Some("lab"));

    let server = server_rec.records.lock().unwrap().clone();
    assert!(
        server.iter().all(|r| !otel::is_client_target(&r.target)),
        "a client row reached cimmeria-server: {server:#?}"
    );
    assert!(
        server.iter().any(|r| r.body == "server row"),
        "the server's own row must still reach cimmeria-server: {server:#?}"
    );
}

/// The predicate the routing hangs on: each client target and its dotted
/// children, never a sibling that only shares the prefix, and never the
/// ingest's own rows or the key dump.
#[test]
fn is_client_target_matches_the_replay_targets_only() {
    for t in otel::CLIENT_TARGETS {
        assert!(otel::is_client_target(t), "{t}");
        assert!(otel::is_client_target(&format!("{t}.child")), "{t}.child");
    }
    for t in [
        "client",
        "client.nativex",
        "launcher",
        "launcher.ingest",
        "launcher.bundle",
        "launcher.key_dump",
        "cimmeria_admin_api::routes::telemetry",
    ] {
        assert!(!otel::is_client_target(t), "{t}");
    }
}
