//! Admission guards exercise public native entry points and persisted signed bytes.
use super::*;
use crate::{
    catalog::verify_release,
    launcher_compatibility::{CompatibilityPolicy, Identity, KnownRelease, MinimumStatus},
    OperationState,
};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use uuid::Uuid;
const OWN: &str = "launcher-20261002-aaaaaaa";
const SAME_DAY: &str = "launcher-20261002-bbbbbbb";
const NEXT_DAY: &str = "launcher-20261003-ccccccc";
fn policy(known: bool) -> CompatibilityPolicy {
    CompatibilityPolicy::new(
        Identity::from_parts("0.1.0", None, Some(OWN), Some("1000")),
        if known {
            vec![KnownRelease {
                tag: SAME_DAY.into(),
                published_at: 2000,
            }]
        } else {
            vec![]
        },
    )
}
fn release(minimum: &str) -> crate::catalog::VerifiedRelease {
    let body = serde_json::to_vec(&serde_json::json!({"schema":1,"min_launcher":minimum,"seed":{"blob":"seed.zip","size":1,"sha256":"a".repeat(64)},"patches":[]})).unwrap();
    let signature = SigningKey::from_bytes(&crate::manifest::DEV_MANIFEST_PRIVKEY).sign(&body);
    let hex: String = signature
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    verify_release(&body, hex.as_bytes()).unwrap()
}
fn tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(root: &Path, at: &Path, result: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in std::fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            result.push((
                path.strip_prefix(root).unwrap().into(),
                if path == root.join("state/launcher.lock") {
                    // Windows byte-range locks reject a second-handle read even
                    // in this process. The lock has no persisted payload.
                    assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
                    vec![]
                } else if path.is_file() {
                    std::fs::read(&path).unwrap()
                } else {
                    vec![]
                },
            ));
            if path.is_dir() {
                walk(root, &path, result);
            }
        }
    }
    let mut result = vec![];
    walk(root, root, &mut result);
    result.sort();
    result
}
#[test]
fn signed_minimum_blocks_direct_install_without_any_durable_mutation() {
    for minimum in [NEXT_DAY, SAME_DAY] {
        let root = tempfile::tempdir().unwrap();
        let mut state =
            DesktopState::open_with_compatibility(&root.path().join("state"), policy(true))
                .unwrap();
        state
            .save_preferences(Some(root.path().join("install")), false, 0)
            .unwrap();
        let before = tree(root.path());
        let result = state.admit_install(Uuid::new_v4(), 0, 1, &release(minimum), vec![]);
        assert!(matches!(result, Err(IntentError::LauncherTooOld)));
        assert_eq!(tree(root.path()), before);
        assert_eq!(state.operations().snapshot().revision, 0);
    }
}
fn installed(
    minimum: &str,
) -> (
    tempfile::TempDir,
    DesktopState,
    InstallIntent,
    launch::Resources,
) {
    let root = tempfile::tempdir().unwrap();
    // An unstamped development build can prepare this installation. The reopened
    // older release must enforce its own identity against the retained evidence.
    let mut state = DesktopState::open_with_compatibility(
        &root.path().join("state"),
        CompatibilityPolicy::new(Identity::from_parts("0.1.0", None, None, None), vec![]),
    )
    .unwrap();
    state
        .save_preferences(Some(root.path().join("install")), false, 0)
        .unwrap();
    let id = Uuid::new_v4();
    let intent = state
        .admit_install(id, 0, 1, &release(minimum), vec![])
        .unwrap()
        .intent;
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Running)
        .unwrap();
    std::fs::create_dir(&intent.destination).unwrap();
    for name in [".cimmeria-install.json", "content-ready.json"] {
        atomic::write(&intent.destination, name, &intent).unwrap();
    }
    state.remember_prepared_content().unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Succeeded)
        .unwrap();
    let helper = root.path().canonicalize().unwrap().join("helper.exe");
    std::fs::write(&helper, b"inert fixture").unwrap();
    let hash: String = Sha256::digest(b"inert fixture")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let resources = launch::Resources {
        helper: launch::Artifact::open(helper, &hash).unwrap(),
        client_patches: None,
        graphics: None,
    };
    (root, state, intent, resources)
}
#[test]
fn offline_reopen_launch_blocks_cached_minimum_even_before_legacy_index_migration() {
    for (minimum, remove_index) in [(NEXT_DAY, false), (NEXT_DAY, true), (SAME_DAY, true)] {
        let (root, state, installed, resources) = installed(minimum);
        drop(state);
        if remove_index {
            std::fs::remove_file(root.path().join("state/installed-content.json")).unwrap();
        }
        let mut state =
            DesktopState::open_with_compatibility(&root.path().join("state"), policy(true))
                .unwrap();
        let before = tree(root.path());
        assert!(state
            .installed_launcher_minimum()
            .unwrap()
            .unwrap()
            .blocks());
        let revision = state.operations().snapshot().revision;
        assert!(matches!(
            state.admit_launch(Uuid::new_v4(), revision, installed.operation_id, resources),
            Err(IntentError::LauncherTooOld)
        ));
        assert_eq!(tree(root.path()), before);
        assert_eq!(state.operations().snapshot().revision, revision);
    }
}
#[test]
fn offline_unknown_same_day_is_explicit_and_stale_or_tampered_evidence_fails_closed() {
    let (root, state, installed, resources) = installed(SAME_DAY);
    drop(state);
    let mut state =
        DesktopState::open_with_compatibility(&root.path().join("state"), policy(false)).unwrap();
    assert_eq!(
        state.installed_launcher_minimum().unwrap(),
        Some(MinimumStatus::UnknownSameDay {
            required: SAME_DAY.into()
        })
    );
    // A different, validly signed manifest must not replace the retained identity.
    state
        .save_release_evidence(installed.operation_id, &release(OWN))
        .unwrap();
    let before = tree(root.path());
    assert_eq!(
        state.installed_launcher_minimum(),
        Err(StorageError::Corrupt)
    );
    let revision = state.operations().snapshot().revision;
    assert!(matches!(
        state.admit_launch(
            Uuid::new_v4(),
            revision,
            installed.operation_id,
            resources.clone()
        ),
        Err(IntentError::Storage(StorageError::Corrupt))
    ));
    assert_eq!(tree(root.path()), before);
    std::fs::write(
        root.path().join(format!(
            "state/release-evidence-{}.bin",
            installed.operation_id
        )),
        b"tampered",
    )
    .unwrap();
    let before = tree(root.path());
    assert!(state
        .admit_launch(Uuid::new_v4(), revision, installed.operation_id, resources)
        .is_err());
    assert_eq!(tree(root.path()), before);
}

#[test]
fn direct_install_retains_legacy_unknown_and_malformed_exemptions() {
    for minimum in [SAME_DAY, "bad-minimum", OWN, ""] {
        let root = tempfile::tempdir().unwrap();
        let mut state =
            DesktopState::open_with_compatibility(&root.path().join("state"), policy(false))
                .unwrap();
        state
            .save_preferences(Some(root.path().join("install")), false, 0)
            .unwrap();
        let admission = state
            .admit_install(Uuid::new_v4(), 0, 1, &release(minimum), vec![])
            .unwrap();
        assert!(admission.dispatch);
        assert_eq!(
            state.cached_install_release().unwrap().digest(),
            release(minimum).digest()
        );
    }
}

#[test]
fn signed_minimum_blocks_game_update_before_evidence_or_plan_publication() {
    let (root, state, owner, _resources) = installed(OWN);
    drop(state);
    let mut state =
        DesktopState::open_with_compatibility(&root.path().join("state"), policy(true)).unwrap();
    let before = tree(root.path());
    let target = release(NEXT_DAY);
    let result = state.admit_update(update::Request {
        id: Uuid::new_v4(),
        operation_revision: state.operations().snapshot().revision,
        installation_id: owner.operation_id,
        expected_current: owner.release_identity(),
        target: &target,
        confirmed: true,
    });
    assert!(matches!(result, Err(IntentError::LauncherTooOld)));
    assert_eq!(tree(root.path()), before);
}
