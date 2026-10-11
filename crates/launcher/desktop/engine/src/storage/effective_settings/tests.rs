//! Patch policy on every host. Adoption-backed journeys are in `adopted_tests`
//! and `wine_tests`, which need the macOS-only adoption engine.
use super::*;
use crate::storage::launch::{Artifact, Resources};
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn artifact(root: &Path, name: &str) -> Artifact {
    let path = root.join(name);
    std::fs::write(&path, name).unwrap();
    let hex: String = Sha256::digest(name)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Artifact::open(path, &hex).unwrap()
}

fn bundle(root: &Path, patches: bool) -> Resources {
    Resources {
        helper: artifact(root, "helper.exe"),
        client_patches: patches.then(|| artifact(root, "patches.dll")),
        client_telemetry: None,
        graphics: None,
    }
}

#[test]
fn fresh_install_keeps_the_bundled_patch_contract() {
    let (root, state, _installed) = runtime_setup::tests::fixture_with_runtime([7; 32]);
    let root = root.path().canonicalize().unwrap();
    assert_eq!(state.effective_launch_binding(), Ok(None));
    assert_eq!(
        state.resolve_play_resources(bundle(&root, false)),
        Ok(None),
        "a fresh install is never offered a patchless Play"
    );
    let bundled = bundle(&root, true);
    assert_eq!(
        state.resolve_play_resources(bundled.clone()),
        Ok(Some(bundled))
    );
}

#[test]
fn load_client_patches_off_launches_a_fresh_install_without_the_patches_dll() {
    let (root, mut state, installed) = runtime_setup::tests::fixture_with_runtime([7; 32]);
    let root = root.path().canonicalize().unwrap();
    let full = Resources {
        client_telemetry: Some(artifact(&root, "telemetry.dll")),
        ..bundle(&root, true)
    };
    state.set_game_telemetry(true).unwrap();
    let revision = state.preferences().revision;
    state.save_client_options(vec![], false, revision).unwrap();
    assert!(!state.injection_policy().unwrap().client_patches);
    let off = state
        .resolve_play_resources(full.clone())
        .unwrap()
        .expect("patches off needs no patches DLL");
    assert_eq!(off.client_patches, None);
    // The telemetry DLL follows its own opt-in, not this setting.
    assert_eq!(off.client_telemetry, full.client_telemetry);
    assert_eq!(
        state.resolve_play_resources(bundle(&root, false)),
        Ok(Some(bundle(&root, false))),
        "a bundle without the patches DLL can still Play with patches off"
    );
    // A host that skipped resolution cannot inject the DLL anyway.
    let operations = state.operations().snapshot().revision;
    assert_eq!(
        state
            .admit_launch(
                Uuid::new_v4(),
                operations,
                installed.operation_id,
                full.clone()
            )
            .unwrap_err(),
        IntentError::Operation(ContractError::IdentityConflict)
    );
    assert_eq!(state.operations().snapshot().revision, operations);

    let revision = state.preferences().revision;
    state.save_client_options(vec![], true, revision).unwrap();
    assert_eq!(
        state.resolve_play_resources(full.clone()),
        Ok(Some(full)),
        "patches on keeps the patches DLL"
    );
}

#[test]
fn admission_refuses_resources_that_disagree_with_the_binding() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let on = LaunchBinding {
        client_patches_enabled: true,
    };
    let off = LaunchBinding {
        client_patches_enabled: false,
    };
    let conflict = Err(IntentError::Operation(ContractError::IdentityConflict));
    assert_eq!(
        verify_launch_resources(on, &bundle(&root, false)),
        conflict,
        "patches on must not launch without the artifact"
    );
    assert_eq!(
        verify_launch_resources(off, &bundle(&root, true)),
        conflict,
        "patches off must not inject the artifact"
    );
    verify_launch_resources(off, &bundle(&root, false)).unwrap();
    let with_patches = bundle(&root, true);
    verify_launch_resources(on, &with_patches).unwrap();
    // A replaced artifact is a mismatch against the build's pinned digest.
    std::fs::write(root.join("patches.dll"), b"replacement").unwrap();
    assert_eq!(
        verify_launch_resources(on, &with_patches),
        Err(IntentError::Storage(StorageError::Corrupt))
    );
}
