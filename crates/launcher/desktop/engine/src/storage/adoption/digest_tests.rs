//! An adoption in flight across the preferences schema 1 -> 2 upgrade.
use super::*;
use sha2::{Digest, Sha256};

const WORK: &str = "6f1c2a4e-3b5d-4e7f-8a9b-0c1d2e3f4a5b";
const OWNER: &str = "1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d";
const LEGACY: &str = "9e8d7c6b-5a4f-4e3d-8c2b-1a0f9e8d7c6b";

fn bytes(value: u8) -> String {
    format!("[{}]", vec![value.to_string(); 32].join(","))
}

/// A plan exactly as the schema-1 build serialized it: compact, in struct
/// field order, with four-field v1 preferences.
fn v1_plan() -> String {
    let servers = r#"[{"name":"Cimmeria","url":"http://login.example:8081"}]"#;
    let identity = format!(
        r#"{{"schema_version":1,"install_id":"{LEGACY}","machine_id":"machine","first_seen_ms":1,"created_by_launcher_version":"0.9.0"}}"#
    );
    let config = format!(
        r#"{{"schema_version":1,"install_path":"/legacy/game","manifest_url":"https://example.test/manifest.json","login_servers":{servers},"telemetry":{{"opted_in":false,"prompt_answered":false,"auth_url":"http://login.example:8081/api"}},"client_patches":{{"enabled":true,"dll_override":null}}}}"#
    );
    let preferences = |revision: u32, directory: &str| {
        format!(
            r#"{{"schema_version":1,"revision":{revision},"install_directory":{directory},"launcher_summary_consent":true}}"#
        )
    };
    let fs_identity = r#"{"device":1,"inode":2,"length":3,"modified_ns":4}"#;
    [
        r#"{"schema_version":1"#.to_string(),
        format!(
            r#""installation":{{"schema_version":1,"operation_id":"{OWNER}","preferences_revision":3,"destination":"/new/Stargate Worlds","manifest_digest":{},"login_servers":{servers}}}"#,
            bytes(7)
        ),
        format!(
            r#""provenance":{{"work_id":"{WORK}","legacy_install_id":"{LEGACY}","import_digest":"confirmation","reference_digest":{},"setup_policy_version":1}}"#,
            bytes(8)
        ),
        format!(
            r#""imported":{{"source":{{"launcher_directory":"/legacy/launcher","game_directory":"/legacy/game"}},"identity":{identity},"config":{config},"ledger":{{"applied_patches":[],"seed_sha256":null,"seed_adopted":false}},"confirmation":"confirmation"}}"#
        ),
        format!(r#""before":{}"#, preferences(3, r#""/legacy/game""#)),
        format!(r#""after":{}"#, preferences(4, r#""/new/Stargate Worlds""#)),
        format!(
            r#""report":{{"preview_handle":"{WORK}","source":"/legacy/game","destination":"/new/Stargate Worlds","release_digest":{},"imported_identity":{identity},"requested_config":{config},"files":[{{"path":"a.txt","source_path":null,"classification":"matched"}}],"user_data_remains_in_source":true,"game_telemetry_available":false}}"#,
            bytes(7)
        ),
        r#""choices":{"normalize_managed_files":true,"accept_unavailable_game_telemetry":true,"old_game_closed":true}"#.to_string(),
        format!(r#""source_digest":{}"#, bytes(9)),
        format!(r#""stage_identity":{fs_identity}"#),
        format!(r#""destination_identity":{fs_identity}}}"#),
    ]
    .join(",")
}

#[test]
fn a_plan_journaled_before_the_upgrade_keeps_its_digest() {
    let old = v1_plan();
    let plan: Plan = serde_json::from_str(&old).unwrap();
    // It reads as schema 2 with the default client options...
    assert_eq!(plan.before.schema_version, 2);
    assert!(plan.after.client_patches_enabled && plan.after.login_servers.is_empty());
    // ...and still hashes to what the old build journaled.
    let recorded: [u8; 32] = Sha256::digest(old.as_bytes()).into();
    assert_eq!(plan.digest().unwrap(), recorded);
}

/// Adoption reads its records through macOS-only file identity checks, so the
/// persisted half runs where adoption does.
#[cfg(target_os = "macos")]
#[test]
fn recovery_accepts_a_plan_journaled_before_the_upgrade() {
    // The persisted pair as the old build left it: the plan checkpoint and an
    // Adopt operation carrying the digest of those exact bytes.
    let old = v1_plan();
    let recorded: [u8; 32] = Sha256::digest(old.as_bytes()).into();
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    let id: Uuid = WORK.parse().unwrap();
    std::fs::write(
        state.state_root().join(format!("adoption-plan-{id}.json")),
        &old,
    )
    .unwrap();
    let revision = state.operations().snapshot().revision;
    state
        .operations_mut()
        .unwrap()
        .begin(id, OperationKind::Adopt, recorded, revision)
        .unwrap();
    let interrupted = interrupted(&state, id).unwrap();
    assert_eq!(interrupted.directory, PathBuf::from("/new/Stargate Worlds"));
}

#[test]
fn a_changed_client_option_changes_the_digest() {
    let plan: Plan = serde_json::from_str(&v1_plan()).unwrap();
    let base = plan.digest().unwrap();
    let mut patches_off = plan.clone();
    patches_off.after.client_patches_enabled = false;
    let mut servers = plan.clone();
    servers.before.login_servers = plan.installation.login_servers.clone();
    let mut consent = plan.clone();
    consent.after.launcher_summary_consent = false;
    let digests = [
        base,
        patches_off.digest().unwrap(),
        servers.digest().unwrap(),
        consent.digest().unwrap(),
    ];
    for (i, a) in digests.iter().enumerate() {
        for b in &digests[i + 1..] {
            assert_ne!(a, b);
        }
    }
}
