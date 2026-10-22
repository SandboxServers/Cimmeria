//! Tests for the launcher-row filter and scrub.

use serde_json::json;

use super::dto::*;
use super::launcher_rows::*;

const ADOPTION: &str = "client.launcher.adoption";

fn native(target: &str, level: &str) -> TelemetryEvent {
    serde_json::from_value(json!({
        "type": "client_native", "ts_ms": 1, "seq": 1,
        "target": target, "level": level, "fields": {}
    }))
    .unwrap()
}

fn level_of(event: &TelemetryEvent) -> &str {
    match event {
        TelemetryEvent::ClientNative(row) => &row.level,
        _ => panic!("not a native row"),
    }
}

#[test]
fn only_allowed_client_native_rows_are_kept() {
    let key_dump: TelemetryEvent = serde_json::from_value(json!({
        "type": "key_dump", "ts_ms": 1, "seq": 2, "source_file": "a", "key_b64": "AA=="
    }))
    .unwrap();
    let client_log: TelemetryEvent = serde_json::from_value(json!({
        "type": "client_log", "ts_ms": 1, "seq": 3, "source_file": "a.log",
        "level": "info", "category": "raw", "message": "m"
    }))
    .unwrap();
    let session_meta: TelemetryEvent = serde_json::from_value(json!({
        "type": "session_meta", "ts_ms": 1, "seq": 4, "kind": "start"
    }))
    .unwrap();
    let mut v = vec![
        native(ADOPTION, "info"),
        native("client.native.status", "info"),
        key_dump,
        client_log,
        session_meta,
        native("client.launcher.x1", "info"),
    ];
    assert_eq!(admit_launcher_rows(&mut v, 200), 5);
    assert_eq!(v.len(), 1);
}

#[test]
fn rows_past_the_cap_are_dropped() {
    let mut v: Vec<TelemetryEvent> = (0..250).map(|_| native(ADOPTION, "info")).collect();
    assert_eq!(admit_launcher_rows(&mut v, 200), 50);
    assert_eq!(v.len(), 200);
}

#[test]
fn the_level_is_overwritten() {
    let mut v = vec![
        native(ADOPTION, "FATAL"),
        native(ADOPTION, "WARN"),
        native(ADOPTION, "info\n\u{1b}[31mx"),
    ];
    admit_launcher_rows(&mut v, 200);
    let levels: Vec<&str> = v.iter().map(level_of).collect();
    assert_eq!(levels, ["info", "warn", "info"]);
}

#[test]
fn fields_are_an_allow_list() {
    let mut fields = json!({
        "ip": "1.2.3.4",
        "Account_Name": "gerger",
        "entity_id": 7,
        "nested": {"a": 1},
        "list": [1],
        "h0st": "db.example",
        "detail": "from 10.0.0.5:8081 now",
        "path": "C:\\Users\\bob\\x",
        "unc": "\\\\srv\\share",
        "rel": "Binaries/x.dll",
        "step": "refused\nFORGED=1",
        "uni": "caf\u{e9}",
        "n": 2,
        "long": "x".repeat(300)
    })
    .as_object()
    .unwrap()
    .clone();
    clean_fields(&mut fields);
    for removed in ["ip", "Account_Name", "entity_id", "nested", "list"] {
        assert!(!fields.contains_key(removed), "{removed} kept");
    }
    assert_eq!(fields["h0st"], json!("db.example"));
    assert_eq!(fields["detail"], json!("<removed>"));
    assert_eq!(fields["path"], json!("<removed>"));
    assert_eq!(fields["unc"], json!("<removed>"));
    assert_eq!(fields["rel"], json!("Binaries/x.dll"));
    assert_eq!(fields["step"], json!("refused FORGED=1"));
    assert_eq!(fields["uni"], json!("caf?"));
    assert_eq!(fields["n"], json!(2));
    assert_eq!(fields["long"].as_str().unwrap().len(), 256);

    let mut many = serde_json::Map::new();
    for i in 0..20 {
        many.insert(format!("k{i}"), json!(i));
    }
    clean_fields(&mut many);
    assert_eq!(many.len(), MAX_FIELDS);
}

#[test]
fn scrub_value_cases() {
    let removed = [
        "1.2.3.4:80",
        "[::1]:80",
        "::ffff:1.2.3.4",
        "fe80::1%eth0",
        "see http://x/y",
        "D:/SGW",
        "\\\\srv\\s",
        "/etc/x",
        "\"/etc/x\"",
        "file:C",
        "a@b.c",
        "%USERPROFILE%\\x",
        "x\\Users\\y",
        "/home/y",
        "a/home/b",
        "a/users/b",
        "$HOME/x",
        "ip:1.2.3.4",
        "1.2.3.4:",
        "1.2.3.4.",
        "id=1.2.3.4;",
        "12345678-1234-1234-1234-123456789abc",
        "{12345678-1234-1234-1234-123456789ABC}",
        "0123456789abcdef01234567",
        "<1.2.3.4:5000>",
        "1.2.3.4-1.2.3.9",
        "ip_1.2.3.4",
        "{1.2.3.4}",
        "a|1.2.3.4",
        "010.001.002.003",
    ];
    for value in removed {
        assert_eq!(scrub_value(value), REMOVED, "{value} kept");
    }
    let unchanged = [
        "desktop-0.4.1",
        "hard_link:later.txt; link:(location)",
        ".",
        "Binaries/x.dll",
        "10.0.26300.1",
    ];
    for value in unchanged {
        assert_eq!(scrub_value(value), value, "{value} removed");
    }
}

/// A control character between `c:` and the rest of a path is mapped to a
/// space before the scrub, and the path is still removed (S3).
#[test]
fn control_characters_do_not_hide_a_path() {
    let mut fields = serde_json::Map::new();
    fields.insert("step".into(), json!("c:\u{1}\\srv"));
    clean_fields(&mut fields);
    assert_eq!(fields["step"], json!(REMOVED));
}

#[test]
fn launcher_rows_are_not_named() {
    use super::entity_labels::ENTITY_KEYS;
    for key in ENTITY_KEYS {
        assert!(
            super::launcher_rows::DENIED_KEYS.contains(&key),
            "{key} can name an entity but is not denied"
        );
    }
}

#[test]
fn allowed_target_cases() {
    assert!(allowed_target(ADOPTION));
    for target in [
        "client.launcher.install_result",
        "client.native",
        "Client.launcher.adoption",
        "client.launcher.adoption\n",
        "client.launcher.x1",
    ] {
        assert!(!allowed_target(target), "{target:?} allowed");
    }
}

#[test]
fn launcher_install_target_is_accepted() {
    assert!(allowed_target("client.launcher.install"));
}

#[test]
fn bogus_launcher_target_is_refused() {
    assert!(!allowed_target("client.launcher.bogus"));
}

/// The server list is the canonical file, line for line (C1, D-LT5).
#[test]
fn launcher_targets_match_the_canonical_file() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../launcher-core/src/telemetry/launcher_targets.txt"
    ))
    .unwrap();
    let targets: Vec<&str> = text
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    assert_eq!(targets, LAUNCHER_TARGETS);
}
