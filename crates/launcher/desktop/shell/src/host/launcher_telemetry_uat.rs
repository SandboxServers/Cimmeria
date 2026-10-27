//! LT-25 bridge for launcher-telemetry-native-uat.mjs: real emitters, the real ForwardLayer and launcher_batch; one JSON row per line. Temporary state only; nothing is sent.
use super::NativeHost;
use crate::host::ui_event::ui_event;
use crate::host::GameTelemetryCommand;
use crate::telemetry::{record_outcome, ForwardLayer, LauncherRecord, Redaction, FORWARD_CAPACITY};
use cimmeria_launcher_engine::client_setup::login_servers::default_servers;
use cimmeria_launcher_engine::game_telemetry::health::{SharedHealth, TelemetryHealth};
use cimmeria_launcher_engine::game_telemetry::ip_scrub::contains_ip;
use cimmeria_launcher_engine::game_telemetry::launcher_session::SendOutcome;
use cimmeria_launcher_engine::launch::events::launch_not_started_for_test;
use cimmeria_launcher_engine::launch::{Artifact, Resources};
use cimmeria_launcher_engine::NativeCommand;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tracing_subscriber::{layer::SubscriberExt, Registry};
use uuid::Uuid;

fn artifact(root: &std::path::Path, name: &str) -> Artifact {
    let path = root.join(name);
    std::fs::write(&path, b"fixture").unwrap();
    let digest: String = Sha256::digest(b"fixture")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Artifact::open(path, &digest).unwrap()
}

fn host(root: &std::path::Path) -> NativeHost {
    let mut host = NativeHost::new(root.join("state"));
    host.launch_resources = Some(Resources {
        proton: None,
        helper: artifact(root, "helper.exe"),
        client_patches: None,
        client_telemetry: Some(artifact(root, "telemetry.dll")),
        graphics: None,
    });
    host
}

/// True when a string in the value, at any depth, holds an IP literal (LT-40).
fn has_ip(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(s) => contains_ip(s),
        serde_json::Value::Array(items) => items.iter().any(has_ip),
        serde_json::Value::Object(map) => map.values().any(has_ip),
        _ => false,
    }
}

#[test]
#[ignore = "JSON-lines bridge for launcher-telemetry-native-uat.mjs"]
fn uat_bridge_launcher_telemetry() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let host = host(&root);
    let (tx, rx) = std::sync::mpsc::sync_channel(FORWARD_CAPACITY);
    let health: SharedHealth = Arc::new(Mutex::new(TelemetryHealth::default()));
    // A fresh process per run: no other test's callsite interest can disable these.
    tracing::subscriber::with_default(Registry::default().with(ForwardLayer::new(tx)), || {
        tracing::callsite::rebuild_interest_cache();
        // Step 1, startup: the first store open emits launcher_started.
        let snapshot = host
            .dispatch(NativeCommand::Inspect { schema_version: 1 })
            .unwrap();
        host.game_telemetry_command(GameTelemetryCommand::Set {
            schema_version: 1,
            opted_in: true,
        })
        .unwrap();
        // Step 2, UI.
        ui_event(
            "install".into(),
            "install_failed".into(),
            "transport".into(),
            "warn".into(),
        )
        .unwrap();
        // Step 3, settings: re-read the revision, then save.
        let revision = host
            .dispatch(NativeCommand::Inspect { schema_version: 1 })
            .unwrap()
            .preferences
            .revision;
        host.dispatch(NativeCommand::SaveClientOptions {
            schema_version: 1,
            expected_revision: revision,
            login_servers: default_servers(),
            client_patches_enabled: snapshot.preferences.client_patches_enabled,
        })
        .unwrap();
        // Step 4, pipeline recovery: one pipeline_recovered.
        let outcomes = [
            SendOutcome::MintFailed {
                reason: "transport",
            },
            SendOutcome::Dropped,
            SendOutcome::Sent,
        ];
        for (at, outcome) in outcomes.iter().enumerate() {
            record_outcome(&health, outcome, 100 * at as u64);
        }
        // Step 5, launch.
        launch_not_started_for_test(Uuid::new_v4(), "verify_failed");
    });
    let records: Vec<LauncherRecord> = rx.try_iter().collect();
    for (target, event) in [
        ("client.launcher.startup", "launcher_started"),
        ("client.launcher.ui", "install_failed"),
        ("client.launcher.settings", "client_options_saved"),
        ("client.launcher.telemetry", "pipeline_recovered"),
        ("client.launcher.launch", "launch_not_started"),
    ] {
        assert!(
            records.iter().any(|r| {
                r.target == target && r.fields.get("event").and_then(|v| v.as_str()) == Some(event)
            }),
            "missing {event} on {target}"
        );
    }
    assert!(
        !records.iter().any(|r| r.fields.values().any(has_ip)),
        "a record field holds an IP literal"
    );
    let (_, events) = host
        .launcher_batch(&records, &Redaction::from_env())
        .expect("opted in with the module");
    assert_eq!(events.len(), records.len());
    for event in &events {
        println!(
            "LAUNCHER_TELEMETRY_UAT {}",
            serde_json::to_string(event).unwrap()
        );
    }
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
}
