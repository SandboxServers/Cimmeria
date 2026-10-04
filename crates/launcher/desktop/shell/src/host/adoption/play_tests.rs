//! Adoption through the host, then the Play side of the same store: the imported
//! settings binding, prerequisite eligibility and Play admission, on one host and
//! across a restart. Inert fixture content only: no game is ever started.
use super::fixture::*;
use super::*;
use adoption::test_support::CopyFault;
use cimmeria_launcher_engine::{
    effective_settings::{fixtures as settings, LaunchBinding},
    launch,
    migration::LegacySource,
};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, time::Duration};

const SERVERS: [(&str, &str); 2] = [
    ("Second", "https://example.invalid/second"),
    ("First", "https://example.invalid/first"),
];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The fixture seed under a release that carries a signed launcher minimum.
fn release(seed: &[u8]) -> VerifiedRelease {
    let body = serde_json::to_vec(&serde_json::json!({"schema":1,"min_launcher":settings::MINIMUM,
        "seed":{"blob":"seed.zip","size":seed.len(),"sha256":hex(&Sha256::digest(seed))},"patches":[]}))
    .unwrap();
    let signature = hex(&SigningKey::from_bytes(&[0x2a; 32]).sign(&body).to_bytes());
    cimmeria_launcher_engine::catalog::verify_release(&body, signature.as_bytes()).unwrap()
}

/// What a packaged build gives the host, with or without the patch artifact.
fn bundle(host: &mut NativeHost, directory: &Path, patches: bool) {
    let artifact = |name: &str| {
        let path = directory.join(name);
        fs::write(&path, name).unwrap();
        launch::Artifact::open(path, &hex(&Sha256::digest(name))).unwrap()
    };
    host.launch_resources = Some(launch::Resources {
        helper: artifact("launch-worker.exe"),
        client_patches: patches.then(|| artifact("client-patches.dll")),
        graphics: Some(launch::Graphics {
            d3d9: artifact("d3d9.dll"),
            rosetta_x87: None,
        }),
    });
    bundle_prerequisite_helper(host, directory);
}

/// A launcher restart over the same store, optionally as an older build.
fn restart(host: NativeHost, directory: &Path, patches: bool, older: bool) -> NativeHost {
    let root = host.root.clone();
    drop(host);
    let mut host = NativeHost::new(root.clone());
    bundle(&mut host, directory, patches);
    if older {
        let state =
            DesktopState::open_with_compatibility(&root, settings::older_launcher()).unwrap();
        *host.state.lock().unwrap() = Some(Arc::new(Mutex::new(state)));
    }
    host
}

/// What the Play view polls, as it is serialized for the renderer.
fn play_view(host: &NativeHost) -> serde_json::Value {
    serde_json::to_value(
        host.launch_command(LaunchCommand::Inspect { schema_version: 1 })
            .unwrap(),
    )
    .unwrap()
}

fn binding(host: &NativeHost) -> Result<Option<LaunchBinding>, StorageError> {
    host.store()
        .unwrap()
        .lock()
        .unwrap()
        .effective_launch_binding()
}

fn play(id: Uuid, revision: u64, installation: Uuid) -> LaunchCommand {
    LaunchCommand::Play {
        schema_version: 1,
        operation_id: id,
        operation_revision: revision,
        installation_id: installation,
    }
}

/// The one admission checkpoint on disk. It is found there, not through the
/// journal, because later operations replace the Adopt entry.
fn checkpoint(host: &NativeHost) -> PathBuf {
    let mut found = fs::read_dir(&host.root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("adoption-plan-")
        });
    let path = found.next().expect("the checkpoint outlives publication");
    assert!(found.next().is_none());
    path
}

fn preserved(host: &NativeHost, source: &LegacySource) {
    let store = host.store().unwrap();
    let kept = store.lock().unwrap().legacy_import().unwrap().unwrap();
    assert_eq!(
        kept.identity.install_id.to_string(),
        settings::LEGACY_INSTALL_ID
    );
    assert!(kept.config.telemetry.opted_in);
    assert_eq!(kept.config.telemetry.auth_url, settings::AUTH_URL);
    assert_eq!(kept.source, *source);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn host_published_copy_binds_its_reviewed_settings_and_play_polling_never_waits_on_the_copy()
{
    let mut f = Fixture::new(serde_json::json!({}));
    let _server = f.serve().await;
    let directory = f.root.path().canonicalize().unwrap();
    bundle(&mut f.host, &directory, false);
    let (location, destination) = (f.location(), f.destination());
    let before = source_bytes(&f.source);
    let imported = fs::read(f.host.root.join("legacy-import.json")).unwrap();
    f.host
        .begin_adoption_preview(location, release(&f.seed))
        .unwrap();
    let review = reviewed(&f.host).await;
    let (resume, held) = std::sync::mpsc::channel();
    f.fault(CopyFault::Hold(held));
    f.host.adoption_command(confirmation(&review)).unwrap();
    until(&f.host, "the running copy", Duration::from_secs(10), |s| {
        operation(s) == Some((OperationKind::Adopt, OperationState::Running))
    })
    .await;

    // The copy is paused mid-flight. The Play and installation views keep being
    // answered, offer nothing, and report no failure.
    let Fixture {
        host,
        source,
        root: _root,
        ..
    } = f;
    let host = Arc::new(host);
    for _ in 0..2 {
        let reader = host.clone();
        let (view, install) = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::task::spawn_blocking(move || {
                (play_view(&reader), reader.install_status().unwrap())
            }),
        )
        .await
        .expect("Play polling must not wait for a running copy")
        .unwrap();
        assert!(view["installation_id"].is_null());
        assert_eq!(view["launcher_update_required"], false);
        assert_eq!(install.runtime_setup, None);
    }
    resume.send(()).unwrap();
    let done = until(&host, "publication", Duration::from_secs(30), |status| {
        status.activity == Activity::Idle
    })
    .await;
    assert_eq!(done.last_error, None);
    let host = Arc::into_inner(host).expect("no reader outlives its poll");

    // Without a restart, the records this host published satisfy the settings
    // binding. The admission checkpoint outlives publication and its cleanup.
    let reviewed_off = Ok(Some(LaunchBinding {
        client_patches_enabled: false,
    }));
    assert_eq!(binding(&host), reviewed_off);
    assert_eq!(plans(&host), 1);
    assert!(!host.root.join("adoption-artifacts").exists());
    let published = |host: &NativeHost| {
        let view = play_view(host);
        // Patches were reviewed off, so a bundle without the DLL is complete.
        assert_eq!(view["resources_available"], true);
        assert_eq!(view["launcher_update_required"], false);
        // The portable backend is Native, which macOS never plays or prepares.
        assert!(view["installation_id"].is_null());
        let install = host.install_status().unwrap();
        assert_eq!(install.runtime_setup, None);
        assert_eq!(install.uninstall.unwrap().directory, destination);
    };
    published(&host);
    let host = restart(host, &directory, false, false);
    assert_eq!(binding(&host), reviewed_off);
    published(&host);

    // Losing the checkpoint refuses the imported settings. Both views still
    // answer, nothing is offered, and the bundle is not blamed.
    let checkpoint = checkpoint(&host);
    let saved = fs::read(&checkpoint).unwrap();
    fs::remove_file(&checkpoint).unwrap();
    assert!(binding(&host).is_err());
    let refused = play_view(&host);
    assert_eq!(refused["resources_available"], true);
    assert!(refused["installation_id"].is_null());
    assert_eq!(refused["launcher_update_required"], false);
    let install = host.install_status().unwrap();
    assert_eq!(install.runtime_setup, None);
    assert_eq!(install.uninstall.unwrap().directory, destination);
    // The signed minimum is the owner's: an older build still reports it.
    let older = restart(host, &directory, false, true);
    let blocked = play_view(&older);
    assert_eq!(blocked["launcher_update_required"], true);
    assert!(blocked["installation_id"].is_null());
    fs::write(&checkpoint, saved).unwrap();
    let host = restart(older, &directory, false, false);
    assert_eq!(binding(&host), reviewed_off);

    preserved(&host, &source);
    assert_eq!(source_bytes(&source), before);
    assert_eq!(
        fs::read(host.root.join("legacy-import.json")).unwrap(),
        imported
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_recovered_copy_carries_the_same_settings_binding() {
    let mut f = Fixture::new(serde_json::json!({"client_patches": {"enabled": true}}));
    let _server = f.serve().await;
    let directory = f.root.path().canonicalize().unwrap();
    bundle(&mut f.host, &directory, true);
    f.host
        .begin_adoption_preview(f.location(), release(&f.seed))
        .unwrap();
    let review = reviewed(&f.host).await;
    f.fault(CopyFault::AfterPromotion);
    f.host.adoption_command(confirmation(&review)).unwrap();
    let failed = until(
        &f.host,
        "the interrupted copy",
        Duration::from_secs(30),
        |s| s.activity == Activity::Idle,
    )
    .await;
    let Some(Reconciliation::Copy { operation_id, .. }) = failed.reconciliation else {
        panic!("the promoted copy awaits recovery: {failed:?}");
    };
    // Promoted but not yet recorded as installed: there is nothing to bind, and
    // neither view fails.
    assert_eq!(binding(&f.host), Ok(None));
    let waiting = play_view(&f.host);
    assert!(waiting["installation_id"].is_null());
    assert_eq!(waiting["launcher_update_required"], false);
    assert_eq!(f.host.install_status().unwrap().runtime_setup, None);
    f.host
        .adoption_command(AdoptionCommand::Recover {
            schema_version: 1,
            operation_id,
            operation_revision: failed.native.operation.revision,
            confirmed: true,
        })
        .unwrap();
    assert_eq!(
        binding(&f.host),
        Ok(Some(LaunchBinding {
            client_patches_enabled: true
        }))
    );
    assert_eq!(play_view(&f.host)["resources_available"], true);
}

const SLOW: Duration = Duration::from_secs(600);

/// Review, confirm and publish through the host with the pinned Wine helper,
/// then restart and take the copy to one admitted Play.
async fn wine_journey(patches: bool) {
    let mut f = Fixture::new(serde_json::json!({"client_patches": {"enabled": patches}}));
    let helper = super::wine_tests::helper().expect("CIMMERIA_WINE_HELPER names the pinned helper");
    super::wine_tests::seed_runtime(&f.host.root);
    let _server = f.serve().await;
    f.host.adoption_fixture.as_mut().unwrap().helper = Some(helper.clone());
    let directory = f.root.path().canonicalize().unwrap();
    // Start from a bundle that carries no patch artifact.
    bundle(&mut f.host, &directory, false);
    let before = source_bytes(&f.source);
    let imported = fs::read(f.host.root.join("legacy-import.json")).unwrap();
    f.host
        .begin_adoption_preview(f.location(), release(&f.seed))
        .unwrap();
    let review = until(&f.host, "the Wine-extracted review", SLOW, |status| {
        assert_eq!(status.last_error, None, "preparation failed");
        status.review.is_some()
    })
    .await
    .review
    .unwrap();
    f.host.adoption_command(confirmation(&review)).unwrap();
    let done = until(&f.host, "publication", SLOW, |status| {
        status.activity == Activity::Idle
    })
    .await;
    assert_eq!(done.last_error, None);
    let preferences = done.native.preferences;

    // No restart: the copy the host just published is prerequisite-eligible, and
    // Play waits for those prerequisites.
    let installation = f
        .host
        .install_status()
        .unwrap()
        .runtime_setup
        .expect("a Wine-backed adopted copy is offered prerequisites");
    let installed = f
        .host
        .store()
        .unwrap()
        .lock()
        .unwrap()
        .installed_content()
        .unwrap()
        .unwrap();
    assert_eq!(installed.intent.operation_id, installation);
    assert_eq!(installed.intent.backend, helper.backend());
    assert_eq!(
        binding(&f.host),
        Ok(Some(LaunchBinding {
            client_patches_enabled: patches
        }))
    );
    let waiting = play_view(&f.host);
    assert!(waiting["installation_id"].is_null());
    assert_eq!(waiting["resources_available"], !patches);

    // Without the private runtime copy, the retained Play worker below stops at
    // its resource claim: Wine is not started again.
    fs::rename(
        f.host.root.join("runtimes"),
        directory.join("retired-runtimes"),
    )
    .unwrap();
    let Fixture {
        host,
        source,
        root: _root,
        ..
    } = f;
    let mut host = restart(host, &directory, false, false);
    assert_eq!(
        host.install_status().unwrap().runtime_setup,
        Some(installation)
    );
    // Successful prerequisite evidence is recorded, not run.
    settings::record_prepared_runtime(&mut host.store().unwrap().lock().unwrap());
    if patches {
        // Patches were reviewed on: the bundle without the artifact cannot play
        // this copy, and admission is refused before the journal moves.
        let lacking = play_view(&host);
        assert_eq!(lacking["resources_available"], false);
        assert!(lacking["installation_id"].is_null());
        let revision = lacking["native"]["operation"]["revision"].as_u64().unwrap();
        assert_eq!(
            host.launch_command(play(Uuid::new_v4(), revision, installation))
                .unwrap_err(),
            JobError::PlatformUnavailable
        );
        assert_eq!(play_view(&host)["native"], lacking["native"]);
        host = restart(host, &directory, true, false);
    }
    let ready = play_view(&host);
    assert_eq!(ready["installation_id"], serde_json::json!(installation));
    assert_eq!(ready["resources_available"], true);
    assert_eq!(ready["launcher_update_required"], false);
    let revision = ready["native"]["operation"]["revision"].as_u64().unwrap();

    // With everything else in place, refused settings alone withhold Play and
    // prerequisites; restoring the checkpoint restores both.
    let checkpoint = checkpoint(&host);
    let saved = fs::read(&checkpoint).unwrap();
    fs::remove_file(&checkpoint).unwrap();
    let withheld = play_view(&host);
    assert!(withheld["installation_id"].is_null());
    assert_eq!(withheld["resources_available"], true);
    assert_eq!(host.install_status().unwrap().runtime_setup, None);
    // A missing record is reported as unreadable state; nothing is admitted.
    assert_eq!(
        host.launch_command(play(Uuid::new_v4(), revision, installation))
            .unwrap_err(),
        JobError::Io
    );
    fs::write(&checkpoint, saved).unwrap();
    assert_eq!(play_view(&host), ready);

    // One Play: the identical retry neither admits nor dispatches again.
    let id = Uuid::new_v4();
    host.launch_command(play(id, revision, installation))
        .unwrap();
    host.launch_command(play(id, revision, installation))
        .unwrap();
    assert_eq!(
        host.launch_worker
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .operation_id(),
        id
    );
    let store = host.store().unwrap();
    let plan = store.lock().unwrap().launch_plan().unwrap().unwrap();
    assert_eq!(plan.id, id);
    assert_eq!(plan.resources.client_patches.is_some(), patches);
    let servers: Vec<_> = plan
        .installation
        .login_servers
        .iter()
        .map(|s| (s.name.as_str(), s.url.as_str()))
        .collect();
    assert_eq!(servers, SERVERS);
    // No runtime or prefix exists here: the retained worker records NotStarted.
    tokio::time::timeout(Duration::from_secs(5), async {
        while store.lock().unwrap().launch_observation().unwrap()
            != Some(launch::Observation::NotStarted)
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the Play worker settles without a runtime");
    assert!(!host.root.join("game-prefixes").exists());
    // The settled attempt is still the one admitted above: the retry added none.
    let settled = play_view(&host);
    assert_eq!(
        settled["native"]["operation"]["operation"]["id"],
        serde_json::json!(id)
    );
    assert_eq!(
        settled["native"]["operation"]["operation"]["kind"],
        "launch"
    );

    assert_eq!(
        host.adoption_status().unwrap().native.preferences,
        preferences
    );
    preserved(&host, &source);
    assert_eq!(source_bytes(&source), before);
    assert_eq!(
        fs::read(host.root.join("legacy-import.json")).unwrap(),
        imported
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "runs the pinned Windows helper headlessly in an isolated Wine prefix"]
async fn wine_copy_adopted_through_the_host_with_patches_off_reaches_one_play() {
    wine_journey(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "runs the pinned Windows helper headlessly in an isolated Wine prefix"]
async fn wine_copy_adopted_through_the_host_with_patches_on_requires_the_verified_artifact() {
    wine_journey(true).await;
}
