//! TG-PIPE-17: what labd exports must carry no local path or user name.

use opentelemetry::logs::{AnyValue, LogRecord as _, Logger as _, LoggerProvider as _};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::logs::{InMemoryLogExporter, SdkLogRecord, SdkLoggerProvider};
use tracing_subscriber::layer::SubscriberExt as _;

use super::redact::{PATH_PLACEHOLDER, USER_PLACEHOLDER};
use super::{RedactingProvider, Redactor};

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
