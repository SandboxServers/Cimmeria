//! TG-PIPE-17: what labd exports must carry no local path, user or machine
//! name, token, lease free text, or anything the `OTEL_*` environment named.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use opentelemetry::logs::{AnyValue, LogRecord as _, Logger as _, LoggerProvider as _};
use opentelemetry::Key;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_http::{Bytes, HttpClient, HttpError, Request, Response};
use opentelemetry_sdk::logs::{InMemoryLogExporter, SdkLogRecord, SdkLoggerProvider};
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::Layer as _;

use super::logger::is_dropped_field;
use super::redact::{HOST_PLACEHOLDER, PATH_PLACEHOLDER, SECRET_PLACEHOLDER, USER_PLACEHOLDER};
use super::{
    filter, headers, init, init_with, resource, RedactingProvider, Redactor, ENDPOINT_ENV,
    PROTOCOL_ENV, SERVICE_NAME,
};

fn redactor() -> Redactor {
    Redactor::new(["inst-3".to_string(), "p2".to_string()], Some("xavier"))
}

/// Every string an exported record carries: body and attribute values.
fn strings(record: &SdkLogRecord) -> Vec<String> {
    fn walk(v: &AnyValue, out: &mut Vec<String>) {
        match v {
            AnyValue::String(s) => out.push(s.as_str().to_string()),
            AnyValue::ListAny(l) => l.iter().for_each(|v| walk(v, out)),
            AnyValue::Map(m) => m.values().for_each(|v| walk(v, out)),
            other => out.push(format!("{other:?}")),
        }
    }
    let mut out = Vec::new();
    if let Some(b) = record.body() {
        walk(b, &mut out);
    }
    for (_, v) in record.attributes_iter() {
        walk(v, &mut out);
    }
    out
}

fn attr(record: &SdkLogRecord, key: &str) -> Option<String> {
    record
        .attributes_iter()
        .find(|(k, _)| k.as_str() == key)
        .map(|(_, v)| match v {
            AnyValue::String(s) => s.as_str().to_string(),
            other => format!("{other:?}"),
        })
}

/// The production pipeline: a tracing row through the appender bridge over
/// a [`RedactingProvider`], into an in-memory exporter.
fn export_rows(emit: impl FnOnce()) -> Vec<SdkLogRecord> {
    let exporter = InMemoryLogExporter::default();
    let provider = SdkLoggerProvider::builder()
        .with_simple_exporter(exporter.clone())
        .build();
    let layer =
        OpenTelemetryTracingBridge::new(&RedactingProvider::new(provider.clone(), redactor()));
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, emit);
    provider.force_flush().expect("flush");
    exporter
        .get_emitted_logs()
        .expect("emitted logs")
        .into_iter()
        .map(|l| l.record)
        .collect()
}

#[test]
fn labd_redacts_local_paths() {
    let exe = r"C:\Users\xavier\AppData\Local\cimmeria-lab\instances\inst-3\SGW.exe";
    let rows = export_rows(|| {
        tracing::info!(
            target: "lab.client",
            event = "launched",
            instance = "inst-3",
            path = exe,
            // `?` on a PathBuf: Debug doubles each backslash.
            install_dir = ?std::path::PathBuf::from(exe),
            "lab client launched from C:\\Users\\xavier\\Games\\SGW\\SGW.exe"
        );
    });
    assert_eq!(rows.len(), 1, "one row exported");
    let row = &rows[0];
    assert_eq!(attr(row, "path").as_deref(), Some("inst-3"));
    assert_eq!(attr(row, "install_dir").as_deref(), Some("\"inst-3\""));
    // Fields that are not paths pass through.
    assert_eq!(attr(row, "event").as_deref(), Some("launched"));
    for s in strings(row) {
        assert!(!s.contains(r"C:\"), "a path survived: {s}");
        assert!(!s.contains("Users"), "a profile path survived: {s}");
        assert!(
            !s.to_ascii_lowercase().contains("xavier"),
            "the user name survived: {s}"
        );
    }
    assert!(
        strings(row)
            .iter()
            .any(|s| s == &format!("lab client launched from {PATH_PLACEHOLDER}")),
        "the message keeps its words and loses the path: {:?}",
        strings(row)
    );
}

#[test]
fn labd_drops_an_event_name_holding_a_path() {
    // tracing names a dependency's event after its absolute registry path.
    let exporter = InMemoryLogExporter::default();
    let provider = SdkLoggerProvider::builder()
        .with_simple_exporter(exporter.clone())
        .build();
    let logger = RedactingProvider::new(provider.clone(), redactor()).logger("");
    let mut rec = logger.create_log_record();
    rec.set_event_name(r"event C:\Users\xavier\.cargo\registry\src\rmcp\lib.rs:10");
    logger.emit(rec);
    let mut rec = logger.create_log_record();
    rec.set_event_name("event crates/lab/src/main.rs:10");
    logger.emit(rec);
    provider.force_flush().expect("flush");
    let names: Vec<_> = exporter
        .get_emitted_logs()
        .expect("logs")
        .into_iter()
        .map(|l| l.record.event_name())
        .collect();
    assert_eq!(names, vec![None, Some("event crates/lab/src/main.rs:10")]);
}

#[test]
fn redact_reduces_each_path_shape() {
    let r = redactor();
    let cases: &[(&str, &str)] = &[
        // Per-instance profile, any separator style.
        (
            r"C:\Users\xavier\AppData\Local\cimmeria-lab\instances\p2\profile",
            "p2",
        ),
        ("C:/Users/xavier/x/instances/p2/profile", "p2"),
        // Debug form of a PathBuf.
        (
            r#"Some("C:\\Users\\xavier\\x\\inst-3\\SGW.exe")"#,
            r#"Some("inst-3")"#,
        ),
        // Verbatim prefix belongs to the path.
        (r"\\?\C:\Users\xavier\x\inst-3\a.dll", "inst-3"),
        // A per-instance file name.
        (r"D:\logs\cimmeria-client-native-p2.log", "p2"),
        // An unlisted instance under `instances\`.
        (r"C:\Users\xavier\x\instances\p5\profile\Documents", "p5"),
        // No instance: the placeholder. Spaces stay inside the path, the
        // prose after its file name does not.
        (
            r"cannot open C:\Program Files (x86)\Stargate Worlds\SGW.exe for reading",
            "cannot open <path> for reading",
        ),
        (r"see (C:\x\labd.log).", "see (<path>)."),
        // Two paths in one string; the second's drive letter is not eaten.
        (r"copy C:\a\b to D:\c\inst-3\d", "copy <path> to inst-3"),
        // A path ends at a field separator.
        (r"dir=C:\x\y, port=8770", "dir=<path>, port=8770"),
        // UNC share.
        (r"\\nas\share\xavier\f.txt", PATH_PLACEHOLDER),
        // Not paths: a URL, a regex escape, a plain sentence.
        ("http://127.0.0.1:8770/mcp", "http://127.0.0.1:8770/mcp"),
        (r"pattern \\d+ matched", r"pattern \\d+ matched"),
        ("lease expired", "lease expired"),
        // The user name anywhere else.
        ("owner XAVIER asked", "owner <user> asked"),
    ];
    for (input, want) in cases {
        assert_eq!(r.redact(input), *want, "input: {input}");
    }
    assert_eq!(USER_PLACEHOLDER, "<user>");
}

#[test]
fn redact_leaves_short_user_names_alone() {
    let r = Redactor::new(Vec::new(), Some("ab"));
    assert_eq!(r.redact("a cab ride"), "a cab ride");
}

#[test]
fn redact_keeps_a_spaced_profile_folder_whole() {
    // USERNAME is `Steve`, but the profile folder is `Steve Cady`: cutting the
    // path at the first space would ship the surname.
    let r = Redactor::new(Vec::new(), Some("Steve"));
    let cases: &[(&str, &str)] = &[
        (r"root C:\Users\Steve Cady", "root <path>"),
        (r#"dir="C:\\Users\\Steve Cady", ok"#, r#"dir="<path>", ok"#),
        (
            r"C:\Users\Steve Cady\AppData\x.log missing",
            "<path> missing",
        ),
        (r"C:\Users\Steve Cady is not writable", "<path>"),
    ];
    for (input, want) in cases {
        let got = r.redact(input);
        assert_eq!(got, *want, "input: {input}");
    }
    for (input, _) in &cases[..2] {
        assert!(!r.redact(input).contains("Cady"), "surname leaked: {input}");
    }
}

#[test]
fn redact_scrubs_the_machine_name_and_lab_tokens() {
    let token = "0123456789abcdef0123456789abcdef";
    let vars = vars(&[
        ("USERNAME", "xavier"),
        ("COMPUTERNAME", "WS-XAVIER-01"),
        ("CIMMERIA_LAB_DAEMON_TOKEN", token),
        ("CIMMERIA_LAB_MCP_TOKEN", "short"),
    ]);
    let r = Redactor::from_vars(&vars);
    assert_eq!(
        r.redact("connected to ws-xavier-01 as xavier"),
        format!("connected to {HOST_PLACEHOLDER} as {USER_PLACEHOLDER}")
    );
    assert_eq!(
        r.redact(&format!("Authorization: Bearer {token}")),
        format!("Authorization: Bearer {SECRET_PLACEHOLDER}")
    );
    // A value too short to be a real token is not scrubbed.
    assert_eq!(r.redact("a short note"), "a short note");
}

#[test]
fn labd_drops_lease_free_text_and_credentials() {
    let rows = export_rows(|| {
        tracing::warn!(target: "lab.lease", event = "force", owner = "jane-agent",
            purpose = "probe jane's quest", prev_owner = "bob", prev_purpose = "bob's run",
            reason = "jane needs it", "lab lease taken over");
        tracing::warn!(target: "lab.uat", event = "wait_server_expired",
            reason = "timed out", bearer_token = "abc", "wait_server timed out");
    });
    assert_eq!(rows.len(), 2);
    for key in ["owner", "purpose", "prev_owner", "prev_purpose", "reason"] {
        assert_eq!(attr(&rows[0], key), None, "lab.lease {key} exported");
    }
    assert_eq!(attr(&rows[0], "event").as_deref(), Some("force"));
    // `reason` is only free text on lease rows.
    assert_eq!(attr(&rows[1], "reason").as_deref(), Some("timed out"));
    assert_eq!(attr(&rows[1], "bearer_token"), None);
    assert!(is_dropped_field("CIMMERIA_LAB_DAEMON_TOKEN", false));
    assert!(!is_dropped_field("reason", false));
}

// --- init(): the real pipeline -------------------------------------------

/// The process environment is shared by every test in this binary under
/// `cargo test`; the ones that set `OTEL_*` or read the endpoint hold this.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Sets process variables for one test and removes them after it.
struct EnvVars(Vec<&'static str>);

impl EnvVars {
    fn set(pairs: &[(&'static str, &str)]) -> Self {
        for (k, v) in pairs {
            std::env::set_var(k, v);
        }
        Self(pairs.iter().map(|(k, _)| *k).collect())
    }
}

impl Drop for EnvVars {
    fn drop(&mut self) {
        for k in &self.0 {
            std::env::remove_var(k);
        }
    }
}

/// The `OTEL_*` values another tool might set user-wide.
const LEAKY_OTEL: [(&str, &str); 5] = [
    ("OTEL_RESOURCE_ATTRIBUTES", "leak.user=SECRETRES"),
    ("OTEL_SERVICE_NAME", "leaked-service"),
    ("OTEL_EXPORTER_OTLP_HEADERS", "x-leak=SECRETHDR"),
    ("OTEL_EXPORTER_OTLP_LOGS_HEADERS", "x-leak-logs=SECRETLOGS"),
    // Would make a protocol-less HTTP builder refuse to build.
    ("OTEL_EXPORTER_OTLP_PROTOCOL", "grpc"),
];

fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |k| map.get(k).cloned()
}

/// An OTLP/HTTP collector that keeps each request.
#[derive(Debug, Default)]
struct Capture(Mutex<Vec<Request<Bytes>>>);

#[async_trait::async_trait]
impl HttpClient for Capture {
    async fn send_bytes(&self, request: Request<Bytes>) -> Result<Response<Bytes>, HttpError> {
        self.0.lock().unwrap().push(request);
        Ok(Response::new(Bytes::new()))
    }
}

fn contains(hay: &[u8], needle: &str) -> bool {
    hay.windows(needle.len()).any(|w| w == needle.as_bytes())
}

#[test]
fn init_is_off_without_an_endpoint() {
    let _l = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::remove_var(ENDPOINT_ENV);
    assert!(init().is_none(), "no endpoint, no exporter");
    let _v = EnvVars::set(&[(ENDPOINT_ENV, "  ")]);
    assert!(init().is_none(), "a blank endpoint is unset");
    assert!(init_with(&vars(&[]), None).is_none());
}

#[test]
fn resource_ignores_otel_resource_attributes() {
    let _l = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _v = EnvVars::set(&LEAKY_OTEL);
    let res = resource("dev", "inst-3");
    let mut keys: Vec<String> = res.iter().map(|(k, _)| k.as_str().to_string()).collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "cimmeria.deploy_env",
            "cimmeria.lab.instance",
            "cimmeria.source",
            "deployment.environment",
            "service.name",
            "service.version",
        ]
    );
    let get = |k: &'static str| res.get(&Key::from_static_str(k)).map(|v| v.to_string());
    assert_eq!(get("service.name").as_deref(), Some(SERVICE_NAME));
    assert_eq!(get("cimmeria.lab.instance").as_deref(), Some("inst-3"));
}

#[test]
fn grpc_interceptor_clears_merged_metadata() {
    // The SDK merges OTEL_EXPORTER_OTLP_HEADERS into the request's metadata
    // before it calls the interceptor (tonic/mod.rs `build_channel`).
    let mut req = tonic::Request::new(());
    req.metadata_mut()
        .insert("x-leak", "SECRETHDR".parse().unwrap());
    req.metadata_mut()
        .insert("authorization", "Bearer SECRET".parse().unwrap());
    let req = headers::clear_metadata(req).expect("never refuses");
    assert!(req.metadata().is_empty(), "{:?}", req.metadata());
}

#[test]
fn init_pipeline_redacts_and_sends_nothing_from_otel_env() {
    let _l = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _v = EnvVars::set(&LEAKY_OTEL);
    let token = "0123456789abcdef0123456789abcdef";
    let lab = vars(&[
        (ENDPOINT_ENV, "http://127.0.0.1:9/v1/logs"),
        (PROTOCOL_ENV, "http/protobuf"),
        ("CIMMERIA_LAB_INSTANCE", "inst-3"),
        ("USERNAME", "xavier"),
        ("COMPUTERNAME", "WS-XAVIER-01"),
        ("CIMMERIA_LAB_DAEMON_TOKEN", token),
    ]);
    let capture = Arc::new(Capture::default());
    let (layer, guard) =
        init_with(&lab, Some(capture.clone() as Arc<dyn HttpClient>)).expect("endpoint set");
    let subscriber = tracing_subscriber::registry().with(layer.with_filter(filter()));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(target: "lab.client", event = "launched",
            exe = r"C:\Users\xavier\AppData\Local\cimmeria-lab\instances\inst-3\SGW.exe",
            "lab client on WS-XAVIER-01 launched from C:\\Users\\Steve Cady\\Games\\SGW.exe");
        tracing::info!(target: "lab.lease", event = "acquire", owner = "jane-agent",
            purpose = "probe jane's quest", "lab lease acquired");
        tracing::warn!(target: "lab.auth", "rejected bearer {token}");
    });
    drop(guard); // shutdown flushes the batch

    let sent = capture.0.lock().unwrap();
    assert!(
        !sent.is_empty(),
        "the batch was exported through init()'s client"
    );
    for req in sent.iter() {
        let names: Vec<&str> = req.headers().keys().map(|k| k.as_str()).collect();
        assert!(
            names
                .iter()
                .all(|n| ["content-type", "content-encoding"].contains(n)),
            "an extra header was sent: {names:?}"
        );
    }
    let body: Vec<u8> = sent.iter().flat_map(|r| r.body().to_vec()).collect();
    for wanted in [
        SERVICE_NAME,
        "inst-3",
        "lab lease acquired",
        HOST_PLACEHOLDER,
    ] {
        assert!(
            contains(&body, wanted),
            "{wanted:?} missing from the export"
        );
    }
    for leak in [
        "SECRETRES",
        "leak.user",
        "leaked-service",
        "SECRETHDR",
        "SECRETLOGS",
        "xavier",
        "XAVIER",
        "Cady",
        "jane",
        token,
        r"C:\",
        "Users",
    ] {
        assert!(!contains(&body, leak), "{leak:?} was exported");
    }
}
