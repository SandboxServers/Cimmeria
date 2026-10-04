//! Supervised journeys over a real Wine-backed adoption: the pinned Windows
//! archive helper reconstructs inert fixture content in this test's own headless
//! prefix. Prerequisite success is recorded fixture evidence and admission is
//! never dispatched, so no prerequisite installer and no game process runs.
//! Inputs: `CIMMERIA_WINE_HELPER`, `CIMMERIA_WINE_HELPER_SHA256`, and
//! `CIMMERIA_WINE_RUNTIME_TREE` (a verified runtime tree, copied, not downloaded).
use super::fixtures::{self, Legacy, Wine};
use super::*;
use crate::storage::launch::{Artifact, Graphics, Resources};
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use uuid::Uuid;

/// Adopts through Wine, records prerequisite evidence, then closes the state.
async fn adopted(
    client_patches: bool,
    telemetry_opted_in: bool,
) -> (Legacy, Vec<(PathBuf, Vec<u8>)>) {
    let legacy = Legacy::new(client_patches, telemetry_opted_in);
    let state = Arc::new(Mutex::new(legacy.open()));
    let before = legacy.source_snapshot();
    fixtures::adopt_wine(&legacy, &state, &Wine::from_environment()).await;
    fixtures::retire_runtime(&legacy);
    (legacy, before)
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

fn bundle(root: &Path, patches: bool) -> Resources {
    Resources {
        helper: artifact(root, "helper.exe"),
        client_patches: patches.then(|| artifact(root, "patches.dll")),
        graphics: Some(Graphics {
            d3d9: artifact(root, "d3d9.dll"),
            rosetta_x87: None,
        }),
    }
}

#[tokio::test]
#[ignore = "runs the pinned Windows helper headlessly in an isolated Wine prefix"]
async fn wine_adopted_copy_with_patches_off_reopens_prepares_and_admits_play_once() {
    let (legacy, before) = adopted(false, true).await;
    let directory = legacy.root.path().canonicalize().unwrap();
    let mut state = DesktopState::open(&legacy.state_root()).unwrap();
    assert_eq!(
        state.effective_launch_binding(),
        Ok(Some(LaunchBinding {
            client_patches_enabled: false
        }))
    );
    let (installed, _) = state.installed_for_launch().unwrap().unwrap();
    assert!(matches!(
        installed.intent.backend,
        ExtractionBackend::Wine { .. }
    ));
    let installation = installed.intent.operation_id;
    let preferences = state.preferences().clone();
    // The patch artifact is absent from this bundle, and Play does not need it.
    let resources = state
        .resolve_play_resources(bundle(&directory, false))
        .unwrap()
        .unwrap();
    assert!(resources.client_patches.is_none());
    // Not before prerequisites: a published copy alone is not a prepared runtime.
    let revision = state.operations().snapshot().revision;
    assert_eq!(
        state
            .admit_launch(Uuid::new_v4(), revision, installation, resources.clone())
            .unwrap_err(),
        IntentError::Storage(StorageError::Corrupt)
    );
    assert_eq!(state.operations().snapshot().revision, revision);
    assert_eq!(fixtures::record_prepared_runtime(&mut state), installation);
    drop(state);
    let mut state = DesktopState::open(&legacy.state_root()).unwrap();
    let revision = state.operations().snapshot().revision;
    // A host that skipped resolution cannot inject the patch into this copy.
    assert_eq!(
        state
            .admit_launch(
                Uuid::new_v4(),
                revision,
                installation,
                bundle(&directory, true)
            )
            .unwrap_err(),
        IntentError::Operation(ContractError::IdentityConflict)
    );
    assert_eq!(state.operations().snapshot().revision, revision);
    // A bundle that does carry the artifact resolves to the same patchless plan.
    assert_eq!(
        state.resolve_play_resources(bundle(&directory, true)),
        Ok(Some(resources.clone()))
    );
    let id = Uuid::new_v4();
    let admission = state
        .admit_launch(id, revision, installation, resources.clone())
        .unwrap();
    assert!(admission.dispatch);
    assert_eq!(admission.plan.resources.client_patches, None);
    assert!(admission.plan.runtime.is_some());
    let servers: Vec<_> = admission
        .plan
        .installation
        .login_servers
        .iter()
        .map(|s| (s.name.as_str(), s.url.as_str()))
        .collect();
    assert_eq!(servers, fixtures::SERVERS);
    // The identical retry re-resolves to the durable selection and starts nothing.
    let retried = state
        .resolve_play_resources(bundle(&directory, false))
        .unwrap()
        .unwrap();
    let duplicate = state.admit_launch(id, 0, installation, retried).unwrap();
    assert!(!duplicate.dispatch);
    assert_eq!(duplicate.plan, admission.plan);
    assert_eq!(state.launch_plan().unwrap(), Some(admission.plan));
    let revision = state.operations().snapshot().revision;
    assert_eq!(
        state
            .admit_launch(id, 0, installation, bundle(&directory, true))
            .unwrap_err(),
        IntentError::Operation(ContractError::IdentityConflict),
        "a retry cannot change the recorded patch selection"
    );
    assert_eq!(
        state
            .admit_launch(Uuid::new_v4(), revision, installation, resources)
            .unwrap_err(),
        IntentError::Operation(ContractError::Busy)
    );
    assert_eq!(state.operations().snapshot().revision, revision);
    // Nothing about the user was reset, and the old installation was only read.
    let imported = state.legacy_import().unwrap().unwrap();
    assert_eq!(
        imported.identity.install_id.to_string(),
        fixtures::LEGACY_INSTALL_ID
    );
    assert!(imported.config.telemetry.opted_in);
    assert_eq!(imported.config.telemetry.auth_url, fixtures::AUTH_URL);
    assert_eq!(state.preferences(), &preferences);
    assert_eq!(legacy.source_snapshot(), before);
}

#[tokio::test]
#[ignore = "runs the pinned Windows helper headlessly in an isolated Wine prefix"]
async fn wine_adopted_copy_with_patches_on_requires_the_verified_bundled_artifact() {
    let (legacy, before) = adopted(true, false).await;
    let directory = legacy.root.path().canonicalize().unwrap();
    let mut state = DesktopState::open(&legacy.state_root()).unwrap();
    assert_eq!(
        state.effective_launch_binding(),
        Ok(Some(LaunchBinding {
            client_patches_enabled: true
        }))
    );
    let installation = fixtures::record_prepared_runtime(&mut state);
    let revision = state.operations().snapshot().revision;
    // Missing artifact: not offered, and refused if a host submits it anyway.
    let missing = bundle(&directory, false);
    assert_eq!(state.resolve_play_resources(missing.clone()), Ok(None));
    assert_eq!(
        state
            .admit_launch(Uuid::new_v4(), revision, installation, missing)
            .unwrap_err(),
        IntentError::Operation(ContractError::IdentityConflict)
    );
    // Replaced artifact: a mismatch against the digest pinned by the build.
    let bundled = bundle(&directory, true);
    let resources = state
        .resolve_play_resources(bundled.clone())
        .unwrap()
        .unwrap();
    assert_eq!(resources, bundled);
    let patches = directory.join("patches.dll");
    let original = std::fs::read(&patches).unwrap();
    std::fs::write(&patches, b"replacement").unwrap();
    assert_eq!(
        state
            .admit_launch(Uuid::new_v4(), revision, installation, resources.clone())
            .unwrap_err(),
        IntentError::Storage(StorageError::Corrupt)
    );
    assert_eq!(state.operations().snapshot().revision, revision);
    std::fs::write(&patches, original).unwrap();
    let id = Uuid::new_v4();
    let admission = state
        .admit_launch(id, revision, installation, resources.clone())
        .unwrap();
    assert!(admission.dispatch);
    assert_eq!(
        admission.plan.resources.client_patches,
        bundled.client_patches
    );
    let duplicate = state.admit_launch(id, 0, installation, resources).unwrap();
    assert!(!duplicate.dispatch);
    assert_eq!(duplicate.plan, admission.plan);
    assert_eq!(legacy.source_snapshot(), before);
}
