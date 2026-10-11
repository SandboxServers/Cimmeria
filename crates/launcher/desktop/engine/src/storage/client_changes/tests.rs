use super::*;
use crate::catalog::verify_release;
use crate::storage::launch::{Artifact, Resources};
use crate::OperationState;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const TITLED: &str = "900-titled-patch";
const CATALOGUED: &str = "001-dialog-portraits";
const UNKNOWN: &str = "999-not-in-the-catalog";

/// A finished install of a signed manifest with three patches: one carrying
/// its own title, one only the built-in catalog knows, one nobody knows. The
/// copy's ledger records only the titled one as applied.
fn installed() -> (tempfile::TempDir, DesktopState) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    state
        .save_preferences(Some(root.path().join("install")), false, 0)
        .unwrap();
    let patch = |id: &str| serde_json::json!({"id":id,"blob":format!("{id}.zip"),"size":1,"sha256":"b".repeat(64)});
    let mut titled = patch(TITLED);
    titled["title"] = "Manifest title".into();
    titled["description"] = "Manifest description.".into();
    let body = serde_json::to_vec(&serde_json::json!({
        "schema":1,
        "seed":{"blob":"seed.zip","size":1,"sha256":"a".repeat(64)},
        "patches":[titled, patch(CATALOGUED), patch(UNKNOWN)],
    }))
    .unwrap();
    let sig = SigningKey::from_bytes(&crate::manifest::DEV_MANIFEST_PRIVKEY).sign(&body);
    let sig: String = sig.to_bytes().iter().map(|b| format!("{b:02x}")).collect();
    let release = verify_release(&body, sig.as_bytes()).unwrap();
    let id = Uuid::new_v4();
    let intent = state
        .admit_install(id, 0, 1, &release, vec![])
        .unwrap()
        .intent;
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Running)
        .unwrap();
    let game = intent.destination.join("game");
    std::fs::create_dir_all(&game).unwrap();
    crate::state::InstalledState {
        seed_sha256: Some("a".repeat(64)),
        seed_adopted: false,
        applied_patches: release
            .manifest()
            .patches
            .iter()
            .filter(|p| p.id == TITLED)
            .map(|p| p.state_key())
            .collect(),
    }
    .save(&game)
    .unwrap();
    for name in [".cimmeria-install.json", "content-ready.json"] {
        atomic::write(&intent.destination, name, &intent).unwrap();
    }
    state.remember_prepared_content().unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Succeeded)
        .unwrap();
    (root, state)
}

fn inspect(state: &DesktopState) -> ClientChangesStatus {
    state
        .client_changes(ClientChangesCommand::Inspect { schema_version: 1 })
        .unwrap()
}

fn row<'a>(status: &'a ClientChangesStatus, title: &str) -> &'a ClientChangeRow {
    status
        .changes
        .iter()
        .find(|row| row.title == title)
        .unwrap_or_else(|| panic!("no row titled {title}: {:#?}", status.changes))
}

#[test]
fn nothing_installed_lists_nothing() {
    let root = tempfile::tempdir().unwrap();
    let state = DesktopState::open(root.path()).unwrap();
    assert_eq!(
        inspect(&state),
        ClientChangesStatus {
            schema_version: 1,
            installed: false,
            changes: vec![],
        }
    );
}

#[test]
fn patch_rows_take_the_manifest_title_then_the_catalog_then_the_id() {
    let (_root, state) = installed();
    let status = inspect(&state);
    assert!(status.installed);
    let patches: Vec<_> = status
        .changes
        .iter()
        .filter(|row| row.group == ChangeGroup::Patch)
        .collect();
    let ids: Vec<_> = patches.iter().map(|r| r.patch_id.as_deref()).collect();
    assert_eq!(ids, [Some(TITLED), Some(CATALOGUED), Some(UNKNOWN)]);

    assert_eq!(patches[0].title, "Manifest title");
    assert_eq!(patches[0].description, "Manifest description.");
    assert_eq!(patches[0].status, ChangeStatus::Applied);

    let (title, description) = core_changes::builtin_description(CATALOGUED).unwrap();
    assert_eq!(patches[1].title, title);
    assert_eq!(patches[1].description, description);
    assert_eq!(patches[1].status, ChangeStatus::Pending);
    assert_eq!(
        patches[1].status_label,
        "applied on the next Install / Update"
    );

    assert_eq!(patches[2].title, UNKNOWN);
    assert!(patches[2].description.contains(UNKNOWN));
    assert_eq!(patches[2].heading, "Patched files");
}

#[test]
fn rows_keep_the_core_group_order() {
    let (_root, state) = installed();
    let groups: Vec<_> = inspect(&state).changes.iter().map(|r| r.group).collect();
    let mut sorted = groups.clone();
    sorted.sort_by_key(|g| *g as u8);
    assert_eq!(groups, sorted);
    assert_eq!(groups.first(), Some(&ChangeGroup::Setup));
    assert_eq!(groups.last(), Some(&ChangeGroup::Server));
    let json = serde_json::to_value(row(&inspect(&state), "Cooked game data")).unwrap();
    assert_eq!(json["group"], "server");
    assert_eq!(json["status"], "during_play");
    assert_eq!(json["patch_id"], serde_json::Value::Null);
}

fn artifact(root: &Path, name: &str) -> Artifact {
    let path = root.join(name);
    std::fs::write(&path, name).unwrap();
    let hex: String = Sha256::digest(name)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Artifact::open(path, &hex).unwrap()
}

/// A bundle carrying both DLLs, so what Play drops is the policy's doing.
fn full_bundle(root: &Path) -> Resources {
    Resources {
        helper: artifact(root, "helper.exe"),
        client_patches: Some(artifact(root, "patches.dll")),
        client_telemetry: Some(artifact(root, "telemetry.dll")),
        graphics: None,
    }
}

/// The launch rows must say what `resolve_play_resources` would load, in every
/// setting the player can change. The patches preference is flipped too: a
/// panel that read it directly would disagree with Play, which (until LX-04)
/// ignores it on a fresh install.
#[test]
fn dll_rows_match_what_play_would_inject() {
    let (root, mut state) = installed();
    let bundle_root = root.path().join("bundle");
    std::fs::create_dir_all(&bundle_root).unwrap();
    let bundle_root = bundle_root.canonicalize().unwrap();
    for patches_pref in [true, false] {
        for telemetry in [false, true] {
            let revision = state.preferences().revision;
            state
                .save_client_options(vec![], patches_pref, revision)
                .unwrap();
            state.set_game_telemetry(telemetry).unwrap();
            let play = state
                .resolve_play_resources(full_bundle(&bundle_root))
                .unwrap()
                .expect("the bundle has the patch artifact");
            let status = inspect(&state);
            let on = |title: &str| row(&status, title).status == ChangeStatus::OnLaunch;
            let case = format!("patches preference {patches_pref}, telemetry {telemetry}");
            assert_eq!(
                on("Client patches DLL"),
                play.client_patches.is_some(),
                "{case}"
            );
            assert_eq!(on("Telemetry"), play.client_telemetry.is_some(), "{case}");
            assert_eq!(on("Telemetry"), telemetry, "{case}");
        }
    }
}

#[test]
fn rejects_unknown_fields_and_versions() {
    let root = tempfile::tempdir().unwrap();
    let state = DesktopState::open(root.path()).unwrap();
    assert_eq!(
        state.client_changes(ClientChangesCommand::Inspect { schema_version: 2 }),
        Err(ClientChangesError::Storage(StorageError::UnsupportedSchema))
    );
    assert!(serde_json::from_str::<ClientChangesCommand>(
        r#"{"command":"inspect","schema_version":1,"path":"/other"}"#
    )
    .is_err());
}

#[test]
fn errors_serialize_as_the_flat_codes_the_bridge_accepts() {
    for (error, code) in [
        (ClientChangesError::Unavailable, "unavailable"),
        (ClientChangesError::Storage(StorageError::Io), "io"),
        (
            ClientChangesError::Storage(StorageError::UnsupportedSchema),
            "unsupported_schema",
        ),
    ] {
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            serde_json::json!(code)
        );
    }
}
