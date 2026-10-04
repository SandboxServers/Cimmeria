//! Host journeys over an adopted copy of inert fixture content, reopened as a
//! launcher restart would. Fixture processes only: no game is ever started.
use super::*;
use cimmeria_launcher_engine::{
    effective_settings::fixtures::{self, Legacy},
    mac_wine::PrerequisiteResource,
    OperationState,
};
use sha2::{Digest, Sha256};

const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// A host over the adopted state with a bundle that has no patch artifact at all.
fn reopened(legacy: &Legacy) -> NativeHost {
    let directory = legacy.root.path().canonicalize().unwrap();
    let artifact = |name: &str| {
        let path = directory.join(name);
        std::fs::write(&path, name).unwrap();
        let hex: String = Sha256::digest(name)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        launch::Artifact::open(path, &hex).unwrap()
    };
    let mut host = NativeHost::new(legacy.state_root());
    host.launch_resources = Some(launch::Resources {
        helper: artifact("launch-worker.exe"),
        client_patches: None,
        graphics: Some(launch::Graphics {
            d3d9: artifact("d3d9.dll"),
            rosetta_x87: None,
        }),
    });
    let helper = directory.join("prerequisite-worker.exe");
    std::fs::write(&helper, b"").unwrap();
    host.prerequisite_helper = Some(PrerequisiteResource::open(helper, EMPTY).unwrap());
    host
}

fn inspect(host: &NativeHost) -> LaunchStatus {
    host.launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap()
}

fn play(id: Uuid, revision: u64, installation: Uuid) -> LaunchCommand {
    LaunchCommand::Play {
        schema_version: 1,
        operation_id: id,
        operation_revision: revision,
        installation_id: installation,
    }
}

#[test]
fn native_backed_adoption_is_inspected_without_error_and_never_offered_on_macos() {
    let legacy = Legacy::new(false, true);
    let (before, provenance) = {
        let state = Arc::new(Mutex::new(legacy.open()));
        let before = legacy.source_snapshot();
        (before, fixtures::adopt_native(&legacy, &state))
    };
    let host = reopened(&legacy);
    let status = inspect(&host);
    // Patches are off for this copy, so the bundle without the DLL is complete.
    assert!(status.resources_available);
    assert!(!status.launcher_update_required);
    assert!(
        status.installation_id.is_none(),
        "a Native backend is not playable on macOS"
    );
    assert!(host.install_status().unwrap().runtime_setup.is_none());
    let installation = host
        .store()
        .unwrap()
        .lock()
        .unwrap()
        .installed_content()
        .unwrap()
        .unwrap()
        .intent
        .operation_id;
    // A renderer that names the copy anyway is refused by the platform gate.
    let revision = status.native.operation.revision;
    assert_eq!(
        host.launch_command(play(Uuid::new_v4(), revision, installation))
            .unwrap_err(),
        JobError::CorruptState
    );
    assert_eq!(inspect(&host).native.operation, status.native.operation);

    // Imported settings that no longer verify: Play and prerequisites are simply
    // not offered. Neither status call fails, so the copy stays manageable.
    let record = legacy
        .state_root()
        .join(format!("adoption-{}.json", provenance.work_id));
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&record).unwrap()).unwrap();
    value["plan"]["imported"]["config"]["client_patches"]["enabled"] = true.into();
    std::fs::write(&record, serde_json::to_vec(&value).unwrap()).unwrap();
    let refused = inspect(&host);
    assert!(
        refused.resources_available,
        "refused settings are not a missing bundle resource"
    );
    assert!(refused.installation_id.is_none());
    assert!(!refused.launcher_update_required);
    let install = host.install_status().unwrap();
    assert!(install.runtime_setup.is_none());
    assert!(install.uninstall.is_some());
    assert_eq!(
        host.launch_command(play(Uuid::new_v4(), revision, installation))
            .unwrap_err(),
        JobError::CorruptState
    );
    assert_eq!(
        host.install_command(
            InstallCommand::PrepareRuntime {
                schema_version: 1,
                operation_id: Uuid::new_v4(),
                operation_revision: revision,
                installation_id: installation,
            },
            None
        )
        .unwrap_err(),
        JobError::CorruptState
    );
    assert_eq!(inspect(&host).native.operation, status.native.operation);
    // The signed minimum is still reported for the refused copy.
    let older = super::fixture::reopen_identity(host, true);
    let blocked = inspect(&older);
    assert!(blocked.launcher_update_required);
    assert!(blocked.installation_id.is_none());
    assert_eq!(legacy.source_snapshot(), before);
}

#[tokio::test]
#[ignore = "runs the pinned Windows helper headlessly in an isolated Wine prefix"]
async fn wine_adopted_copy_reopens_offers_prerequisites_and_admits_one_play() {
    let legacy = Legacy::new(false, true);
    let (before, provenance) = {
        let state = Arc::new(Mutex::new(legacy.open()));
        let before = legacy.source_snapshot();
        let wine = fixtures::Wine::from_environment();
        (before, fixtures::adopt_wine(&legacy, &state, &wine).await)
    };
    // Without the private runtime copy, the retained prerequisite and Play
    // workers below stop at their resource claim: Wine is not started again.
    fixtures::retire_runtime(&legacy);
    let host = reopened(&legacy);
    let preferences = inspect(&host).native.preferences;

    // Terminal Adopt success is offered prerequisites, and Play is not offered yet.
    let status = host.install_status().unwrap();
    let installation = status
        .runtime_setup
        .expect("adopted Wine copy is prerequisite-eligible");
    let waiting = inspect(&host);
    assert!(waiting.resources_available);
    assert!(waiting.installation_id.is_none());
    let revision = status.native.operation.revision;
    assert_eq!(
        host.launch_command(play(Uuid::new_v4(), revision, installation))
            .unwrap_err(),
        JobError::CorruptState
    );

    // Admission through the host, retained once; a cancelled attempt stays eligible.
    let setup = Uuid::new_v4();
    let prepare = || InstallCommand::PrepareRuntime {
        schema_version: 1,
        operation_id: setup,
        operation_revision: revision,
        installation_id: installation,
    };
    host.install_command(prepare(), None).unwrap();
    host.install_command(prepare(), None).unwrap();
    host.install_command(
        InstallCommand::Cancel {
            schema_version: 1,
            operation_id: setup,
        },
        None,
    )
    .unwrap();
    let mut result = host
        .runtime_worker
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .result
        .clone();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while result.borrow().is_none() {
            result.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    let cancelled = host.install_status().unwrap();
    assert_eq!(
        cancelled.native.operation.operation.as_ref().unwrap().state,
        OperationState::Cancelled
    );
    assert_eq!(cancelled.runtime_setup, Some(installation));
    assert!(!legacy.state_root().join("game-prefixes").exists());

    // Imported settings that stop verifying withdraw the offer; restoring the
    // record restores it.
    let record = legacy
        .state_root()
        .join(format!("adoption-{}.json", provenance.work_id));
    let published = std::fs::read(&record).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&published).unwrap();
    value["plan"]["imported"]["config"]["client_patches"]["enabled"] = true.into();
    std::fs::write(&record, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(host.install_status().unwrap().runtime_setup.is_none());
    std::fs::write(&record, &published).unwrap();
    assert_eq!(
        host.install_status().unwrap().runtime_setup,
        Some(installation)
    );

    // Successful prerequisite evidence is recorded, not run.
    fixtures::record_prepared_runtime(&mut host.store().unwrap().lock().unwrap());
    let ready = inspect(&host);
    assert_eq!(ready.installation_id, Some(installation));
    assert!(ready.resources_available);
    assert!(!ready.launcher_update_required);
    // With everything else in place, refused settings alone withhold Play.
    std::fs::write(&record, serde_json::to_vec(&value).unwrap()).unwrap();
    let withheld = inspect(&host);
    assert!(withheld.installation_id.is_none());
    assert!(withheld.resources_available);
    assert_eq!(
        host.launch_command(play(
            Uuid::new_v4(),
            ready.native.operation.revision,
            installation
        ))
        .unwrap_err(),
        JobError::CorruptState
    );
    std::fs::write(&record, &published).unwrap();
    assert_eq!(inspect(&host).native.operation, ready.native.operation);

    // One Play: the identical retry neither admits nor dispatches again.
    let id = Uuid::new_v4();
    let request = || play(id, ready.native.operation.revision, installation);
    host.launch_command(request()).unwrap();
    host.launch_command(request()).unwrap();
    assert_eq!(
        host.launch_worker
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .operation_id(),
        id
    );
    let plan = host
        .store()
        .unwrap()
        .lock()
        .unwrap()
        .launch_plan()
        .unwrap()
        .unwrap();
    assert_eq!(plan.id, id);
    assert_eq!(plan.resources.client_patches, None);
    let servers: Vec<_> = plan
        .installation
        .login_servers
        .iter()
        .map(|s| (s.name.as_str(), s.url.as_str()))
        .collect();
    assert_eq!(servers, fixtures::SERVERS);
    assert_eq!(
        host.launch_command(play(
            Uuid::new_v4(),
            inspect(&host).native.operation.revision,
            installation
        ))
        .unwrap_err(),
        JobError::Busy
    );
    // No prefix or runtime exists here: the retained worker records NotStarted.
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while inspect(&host).observation != Some(launch::Observation::NotStarted) {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let after = host.launch_command(request()).unwrap();
    assert_eq!(after.observation, Some(launch::Observation::NotStarted));
    assert_eq!(after.native.preferences, preferences);

    let store = host.store().unwrap();
    let imported = store.lock().unwrap().legacy_import().unwrap().unwrap();
    assert_eq!(
        imported.identity.install_id.to_string(),
        fixtures::LEGACY_INSTALL_ID
    );
    assert!(imported.config.telemetry.opted_in);
    assert_eq!(imported.config.telemetry.auth_url, fixtures::AUTH_URL);
    assert_eq!(legacy.source_snapshot(), before);
}
