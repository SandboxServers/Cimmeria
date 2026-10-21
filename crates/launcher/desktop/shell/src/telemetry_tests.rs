//! Tests for the launcher's tracing layers (C9). They use `with_default`, never
//! `init`, because `init` sets the global subscriber.
use crate::telemetry::*;
use std::io::Write;
use tracing::subscriber::with_default;
use tracing::{debug, info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

#[test]
fn only_adoption_info_and_above_is_forwarded() {
    let (tx, rx) = std::sync::mpsc::sync_channel(16);
    with_default(Registry::default().with(ForwardLayer::new(tx)), || {
        info!(target: "adoption", step = "mode_chosen", decision_outcome = "copy", "adoption step");
        debug!(target: "adoption", "x");
        warn!(target: "cimmeria_launcher_engine", path = "C:/x", "other");
        warn!(
            target: "adoption",
            reason = ?Some(1),
            path = %"later.txt",
            total = 2u64,
            "adoption refused: an entry cannot be adopted safely"
        );
    });
    let records: Vec<LauncherRecord> = rx.try_iter().collect();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].level, "info");
    assert_eq!(records[0].fields["step"], "mode_chosen");
    assert!(records[0].ts_ms > 0);
    assert_eq!(records[1].level, "warn");
    assert_eq!(records[1].fields["path"], "later.txt");
    assert_eq!(records[1].fields["total"], 2);
    assert!(records[1].fields["message"]
        .as_str()
        .unwrap_or_default()
        .contains("adoption refused"));
}

#[test]
fn a_full_channel_drops_instead_of_blocking() {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    with_default(Registry::default().with(ForwardLayer::new(tx)), || {
        for _ in 0..3 {
            info!(target: "adoption", step = "copy", "adoption step");
        }
    });
    assert_eq!(rx.try_iter().count(), 1);
}

#[test]
fn the_log_writer_writes_the_home_folder_as_a_tilde() {
    let mut writer = Redacting {
        inner: Vec::new(),
        redaction: Redaction {
            home: Some(r"C:\Users\alice".into()),
            user: None,
        },
    };
    writer
        .write_all(r#"C:\Users\alice\AppData\x and "C:\\Users\\alice\\y""#.as_bytes())
        .unwrap();
    let out = String::from_utf8(writer.inner).unwrap();
    assert!(out.contains(r"~\AppData\x"));
    assert!(out.contains(r"~\\y"));
    assert!(!out.contains("alice"));
}

#[test]
fn file_layer_writes_a_launcher_log() {
    let dir = tempfile::tempdir().unwrap();
    let redaction = Redaction {
        home: Some(r"C:\Users\alice".into()),
        user: None,
    };
    let (layer, guard) = file_layer(dir.path(), redaction).unwrap();
    with_default(Registry::default().with(layer), || {
        warn!(target: "adoption", path = r"C:\Users\alice\x", "adoption refused");
    });
    drop(guard);
    let files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    let name = files[0].file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("launcher.") && name.ends_with(".log"));
    let text = std::fs::read_to_string(&files[0]).unwrap();
    assert!(text.contains("adoption refused"));
    assert!(text.contains(r"~\\x"));
    assert!(!text.contains("alice"));
}
