//! A real, natively reconstructed adoption of inert fixture content, reopened.
//! Its backend is Native, which macOS Play admission refuses by design, so these
//! tests stop at the settings and minimum gates. `wine_tests` carries admission.
use super::fixtures::{self, Legacy};
use super::*;
use crate::storage::launch::{Artifact, Resources};
use crate::{OperationKind, OperationState};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use uuid::Uuid;

struct Adopted {
    legacy: Legacy,
    provenance: adoption::Provenance,
    before: Vec<(PathBuf, Vec<u8>)>,
}

/// Imports, adopts and closes the state, as a launcher restart would.
fn adopted(client_patches: bool, telemetry_opted_in: bool) -> Adopted {
    let legacy = Legacy::new(client_patches, telemetry_opted_in);
    let state = Arc::new(Mutex::new(legacy.open()));
    let before = legacy.source_snapshot();
    let provenance = fixtures::adopt_native(&legacy, &state);
    Adopted {
        legacy,
        provenance,
        before,
    }
}

fn bundle(root: &Path, patches: bool) -> Resources {
    let artifact = |name: &str| {
        let path = root.join(name);
        std::fs::write(&path, name).unwrap();
        let hex: String = Sha256::digest(name)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        Artifact::open(path, &hex).unwrap()
    };
    Resources {
        helper: artifact("helper.exe"),
        client_patches: patches.then(|| artifact("patches.dll")),
        graphics: None,
    }
}

/// Rewrites one JSON state file in place and returns its original bytes.
fn edit(path: &Path, change: impl FnOnce(&mut Value)) -> Vec<u8> {
    let original = std::fs::read(path).unwrap();
    let mut value: Value = serde_json::from_slice(&original).unwrap();
    change(&mut value);
    std::fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    original
}

/// Applies the edits, requires every settings consumer to refuse without a
/// trace, then restores the files and requires the reviewed settings back.
fn refused(
    state: &mut DesktopState,
    directory: &Path,
    what: &str,
    edits: &[(&Path, &dyn Fn(&mut Value))],
) {
    let originals: Vec<_> = edits
        .iter()
        .map(|(path, change)| (*path, edit(path, change)))
        .collect();
    assert!(
        state.effective_launch_binding().is_err(),
        "editing {what} must not change or keep the effective patch setting"
    );
    // Neither bundle shape is offered, and admission leaves the journal alone.
    for patches in [false, true] {
        assert!(state
            .resolve_play_resources(bundle(directory, patches))
            .is_err());
    }
    let revision = state.operations().snapshot().revision;
    let (installed, _) = state.installed_for_launch().unwrap().unwrap();
    assert!(matches!(
        state.admit_launch(
            Uuid::new_v4(),
            revision,
            installed.intent.operation_id,
            bundle(directory, true)
        ),
        Err(IntentError::Storage(_))
    ));
    assert_eq!(state.operations().snapshot().revision, revision);
    for (path, original) in originals {
        std::fs::write(path, original).unwrap();
    }
    assert!(
        state.effective_launch_binding().unwrap().is_some(),
        "restoring {what} restores the reviewed settings"
    );
}

#[test]
fn reopened_adoption_keeps_reviewed_settings_identity_consent_and_source() {
    let a = adopted(false, true);
    let state = DesktopState::open(&a.legacy.state_root()).unwrap();
    assert_eq!(
        state.effective_launch_binding(),
        Ok(Some(LaunchBinding {
            client_patches_enabled: false
        }))
    );
    assert!(state
        .installed_launcher_minimum()
        .unwrap()
        .is_some_and(|minimum| !minimum.blocks()));
    let (installed, _) = state.installed_for_launch().unwrap().unwrap();
    let servers: Vec<_> = installed
        .intent
        .login_servers
        .iter()
        .map(|s| (s.name.as_str(), s.url.as_str()))
        .collect();
    assert_eq!(servers, fixtures::SERVERS, "reviewed order, not sorted");
    // Imported identity, game-telemetry answer and auth URL are untouched, and
    // the exact legacy JSON is retained, unknown fields included.
    let imported = state.legacy_import().unwrap().unwrap();
    assert_eq!(
        imported.identity.install_id.to_string(),
        fixtures::LEGACY_INSTALL_ID
    );
    assert_eq!(imported.identity.install_id, a.provenance.legacy_install_id);
    assert!(imported.config.telemetry.opted_in);
    assert!(imported.config.telemetry.prompt_answered);
    assert_eq!(imported.config.telemetry.auth_url, fixtures::AUTH_URL);
    let record: Value = serde_json::from_slice(
        &std::fs::read(a.legacy.state_root().join("legacy-import.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        record["sources"]["config"].as_str().unwrap().as_bytes(),
        a.legacy.config
    );
    // Launcher summary consent is a separate choice that adoption never grants.
    assert!(!state.preferences().launcher_summary_consent);
    assert_eq!(a.legacy.source_snapshot(), a.before);
}

#[test]
fn patch_setting_is_bound_beyond_the_adoption_record() {
    let a = adopted(false, true);
    let root = a.legacy.state_root();
    let directory = a.legacy.root.path().canonicalize().unwrap();
    let mut state = DesktopState::open(&root).unwrap();
    // Supersede the Adopt operation, so the journal digest no longer covers the
    // plan and each remaining record has to hold on its own.
    let later = Uuid::new_v4();
    let revision = state.operations().snapshot().revision;
    state
        .operations_mut()
        .unwrap()
        .begin(later, OperationKind::Launch, [9; 32], revision)
        .unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(later, OperationState::Failed)
        .unwrap();
    assert!(state.effective_launch_binding().unwrap().is_some());

    let record = root.join(format!("adoption-{}.json", a.provenance.work_id));
    let checkpoint = root.join(format!("adoption-plan-{}.json", a.provenance.work_id));
    let import = root.join("legacy-import.json");
    let index = root.join("installed-content.json");
    let enable = |plan: &mut Value| {
        plan["imported"]["config"]["client_patches"]["enabled"] = Value::Bool(true);
        plan["report"]["requested_config"]["client_patches"]["enabled"] = Value::Bool(true);
    };
    let on = Value::Bool(true);
    refused(
        &mut state,
        &directory,
        "the published record alone",
        &[(record.as_path(), &|v: &mut Value| enable(&mut v["plan"]))],
    );
    refused(
        &mut state,
        &directory,
        "the record and its checkpoint together",
        &[
            (record.as_path(), &|v: &mut Value| enable(&mut v["plan"])),
            (checkpoint.as_path(), &|v: &mut Value| enable(v)),
        ],
    );
    refused(
        &mut state,
        &directory,
        "record, checkpoint and the retained import's parsed settings",
        &[
            (record.as_path(), &|v: &mut Value| enable(&mut v["plan"])),
            (checkpoint.as_path(), &|v: &mut Value| enable(v)),
            (import.as_path(), &|v: &mut Value| {
                v["imported"]["config"]["client_patches"]["enabled"] = on.clone()
            }),
        ],
    );
    refused(
        &mut state,
        &directory,
        "all three plus the retained exact JSON, leaving the old digest",
        &[
            (record.as_path(), &|v: &mut Value| enable(&mut v["plan"])),
            (checkpoint.as_path(), &|v: &mut Value| enable(v)),
            (import.as_path(), &|v: &mut Value| {
                v["imported"]["config"]["client_patches"]["enabled"] = on.clone();
                let text = v["sources"]["config"].as_str().unwrap();
                assert!(text.contains("\"enabled\": false"));
                v["sources"]["config"] =
                    Value::String(text.replacen("\"enabled\": false", "\"enabled\": true", 1));
            }),
        ],
    );
    refused(
        &mut state,
        &directory,
        "the reviewed telemetry acceptance",
        &[
            (record.as_path(), &|v: &mut Value| {
                v["plan"]["choices"]["accept_unavailable_game_telemetry"] = Value::Bool(false)
            }),
            (checkpoint.as_path(), &|v: &mut Value| {
                v["choices"]["accept_unavailable_game_telemetry"] = Value::Bool(false)
            }),
        ],
    );
    refused(
        &mut state,
        &directory,
        "the import digest named by the installed index",
        &[(index.as_path(), &|v: &mut Value| {
            v["adoption"]["import_digest"] = Value::String("forged".into())
        })],
    );
    // Not a setting the import or the owner intent restates: only the admission
    // checkpoint still says what the user reviewed.
    refused(
        &mut state,
        &directory,
        "a reviewed report field in the published record alone",
        &[(record.as_path(), &|v: &mut Value| {
            v["plan"]["report"]["user_data_remains_in_source"] = Value::Bool(false)
        })],
    );
    let custom = Value::String("C:\\custom.dll".into());
    refused(
        &mut state,
        &directory,
        "a custom patch DLL claimed after review",
        &[
            (record.as_path(), &|v: &mut Value| {
                v["plan"]["imported"]["config"]["client_patches"]["dll_override"] = custom.clone()
            }),
            (checkpoint.as_path(), &|v: &mut Value| {
                v["imported"]["config"]["client_patches"]["dll_override"] = custom.clone()
            }),
        ],
    );
    assert_eq!(a.legacy.source_snapshot(), a.before);
}

#[test]
fn checkpoint_removal_and_the_journal_digest_both_refuse_the_plan() {
    let a = adopted(true, false);
    let root = a.legacy.state_root();
    let state = DesktopState::open(&root).unwrap();
    assert_eq!(
        state.effective_launch_binding(),
        Ok(Some(LaunchBinding {
            client_patches_enabled: true
        }))
    );
    // Adopt is still the journal's latest operation: a consistent rewrite of the
    // record and checkpoint, with the import left alone, breaks its digest.
    let record = root.join(format!("adoption-{}.json", a.provenance.work_id));
    let checkpoint = root.join(format!("adoption-plan-{}.json", a.provenance.work_id));
    let saved = [
        edit(&record, |v| {
            v["plan"]["report"]["user_data_remains_in_source"] = Value::Bool(false)
        }),
        edit(&checkpoint, |v| {
            v["report"]["user_data_remains_in_source"] = Value::Bool(false)
        }),
    ];
    assert_eq!(state.effective_launch_binding(), Err(StorageError::Corrupt));
    std::fs::write(&record, &saved[0]).unwrap();
    std::fs::write(&checkpoint, &saved[1]).unwrap();
    assert!(state.effective_launch_binding().unwrap().is_some());
    std::fs::remove_file(&checkpoint).unwrap();
    assert!(state.effective_launch_binding().is_err());
}

#[test]
fn signed_minimum_is_reported_even_when_imported_settings_are_refused() {
    let a = adopted(false, true);
    let root = a.legacy.state_root();
    let directory = a.legacy.root.path().canonicalize().unwrap();
    let record = root.join(format!("adoption-{}.json", a.provenance.work_id));
    edit(&record, |v| {
        v["plan"]["imported"]["config"]["client_patches"]["enabled"] = Value::Bool(true)
    });
    // A current launcher: the copy is refused for its settings, not its minimum.
    let mut state = DesktopState::open(&root).unwrap();
    assert!(state.effective_launch_binding().is_err());
    assert!(state
        .installed_launcher_minimum()
        .unwrap()
        .is_some_and(|minimum| !minimum.blocks()));
    let (installed, _) = state.installed_for_launch().unwrap().unwrap();
    let installation = installed.intent.operation_id;
    let revision = state.operations().snapshot().revision;
    assert_eq!(
        state
            .admit_launch(
                Uuid::new_v4(),
                revision,
                installation,
                bundle(&directory, false)
            )
            .unwrap_err(),
        IntentError::Storage(StorageError::Corrupt)
    );
    drop(state);
    // An older launcher: the signed minimum is still reported and is the refusal.
    let mut state =
        DesktopState::open_with_compatibility(&root, fixtures::older_launcher()).unwrap();
    assert!(state.effective_launch_binding().is_err());
    assert!(state
        .installed_launcher_minimum()
        .unwrap()
        .is_some_and(|minimum| minimum.blocks()));
    assert_eq!(
        state
            .admit_launch(
                Uuid::new_v4(),
                revision,
                installation,
                bundle(&directory, false)
            )
            .unwrap_err(),
        IntentError::LauncherTooOld
    );
    assert_eq!(state.operations().snapshot().revision, revision);
    assert_eq!(a.legacy.source_snapshot(), a.before);
}

#[test]
fn native_backed_copy_resolves_resources_but_is_not_admitted_on_macos() {
    let a = adopted(false, true);
    let directory = a.legacy.root.path().canonicalize().unwrap();
    let mut state = DesktopState::open(&a.legacy.state_root()).unwrap();
    // Patches off: the artifact is dropped when bundled and not needed when absent.
    for patches in [false, true] {
        let resolved = state
            .resolve_play_resources(bundle(&directory, patches))
            .unwrap()
            .unwrap();
        assert!(resolved.client_patches.is_none());
    }
    let (installed, _) = state.installed_for_launch().unwrap().unwrap();
    let revision = state.operations().snapshot().revision;
    // A host that skipped resolution cannot inject the patch into this copy.
    assert_eq!(
        state
            .admit_launch(
                Uuid::new_v4(),
                revision,
                installed.intent.operation_id,
                bundle(&directory, true)
            )
            .unwrap_err(),
        IntentError::Operation(ContractError::IdentityConflict)
    );
    // The platform backend gate, after settings and resources were accepted.
    assert_eq!(
        state
            .admit_launch(
                Uuid::new_v4(),
                revision,
                installed.intent.operation_id,
                bundle(&directory, false)
            )
            .unwrap_err(),
        IntentError::Storage(StorageError::Corrupt)
    );
    assert_eq!(state.operations().snapshot().revision, revision);
}
