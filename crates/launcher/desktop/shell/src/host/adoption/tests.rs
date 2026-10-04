//! Production host journey and admission against an isolated filesystem and a
//! signed loopback origin.
use super::fixture::*;
use super::*;
use std::{fs, time::Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reviewed_copy_publishes_once_and_preserves_source_import_identity_and_consent() {
    let mut f = Fixture::new(serde_json::json!({}));
    let server = f.serve().await;
    let before = source_bytes(&f.source);
    let imported = fs::read(f.host.root.join("legacy-import.json")).unwrap();
    let idle = f.host.adoption_command(INSPECT).unwrap();
    assert_eq!(idle.activity, Activity::Idle);
    assert_eq!(
        idle.imported,
        Some(Imported {
            launcher_directory: f.source.launcher_directory.clone(),
            game_directory: f.source.game_directory.clone(),
            blocker: None,
        })
    );
    let started = f.begin().unwrap();
    assert_eq!(started.activity, Activity::Preparing);
    assert!(started.cancellable);
    let review = reviewed(&f.host).await;
    // The journal advanced when preparation was admitted. A review that kept the
    // request's revision could never be confirmed.
    let current = f.host.adoption_status().unwrap().native;
    assert!(review.operation_revision > idle.native.operation.revision);
    assert_eq!(review.operation_revision, current.operation.revision);
    assert_eq!(review.preferences_revision, current.preferences.revision);
    assert_eq!(review.source, f.source.game_directory);
    assert_eq!(review.destination, f.destination());
    assert_eq!(
        review.release.seed_sha256,
        f.release().manifest().seed.sha256
    );
    // Matched: the executable and the login server list the old launcher wrote.
    assert_eq!(
        serde_json::to_value(&review.counts).unwrap(),
        serde_json::json!({"matched": 2, "known_transform": 0, "modified": 1, "missing": 0, "extra": 2})
    );
    assert_eq!(
        serde_json::to_value(&review.differences).unwrap(),
        serde_json::json!([
            {"path": "later.txt", "source_path": "later.txt", "classification": "modified"},
            {"path": "launcher-installed.json", "source_path": "launcher-installed.json", "classification": "extra"},
            {"path": "unknown.dll", "source_path": "unknown.dll", "classification": "extra"}
        ])
    );
    assert_eq!(review.differences_omitted, 0);
    assert_eq!(
        review
            .login_servers
            .iter()
            .map(|server| server.name.as_str())
            .collect::<Vec<_>>(),
        ["Second", "First"]
    );
    assert!(!review.client_patches_enabled);
    assert!(review.game_telemetry_opted_in && !review.game_telemetry_available);
    assert!(review.requires_normalization && review.requires_telemetry_acceptance);
    assert!(review.user_data_remains_in_source);
    assert!(!f.destination().exists());

    // Refused confirmations leave the reviewed preview held and unchanged.
    let refuse = |change: fn(&mut AdoptionCommand)| {
        let mut request = confirmation(&review);
        change(&mut request);
        let error = f.host.adoption_command(request).unwrap_err();
        let status = f.host.adoption_status().unwrap();
        assert_eq!(status.activity, Activity::Review);
        assert_eq!(status.review.unwrap().preview_handle, review.preview_handle);
        assert_eq!(status.native.operation.revision, review.operation_revision);
        assert!(!f.destination().exists());
        error
    };
    assert_eq!(
        refuse(|request| {
            if let AdoptionCommand::Confirm {
                operation_revision, ..
            } = request
            {
                *operation_revision -= 1;
            }
        }),
        AdoptionError::StaleRevision
    );
    assert_eq!(
        refuse(|request| {
            if let AdoptionCommand::Confirm {
                normalize_managed_files,
                ..
            } = request
            {
                *normalize_managed_files = false;
            }
        }),
        AdoptionError::ConsentRequired
    );
    assert_eq!(
        refuse(|request| {
            if let AdoptionCommand::Confirm {
                accept_unavailable_game_telemetry,
                ..
            } = request
            {
                *accept_unavailable_game_telemetry = false;
            }
        }),
        AdoptionError::ConsentRequired
    );
    assert_eq!(
        refuse(|request| {
            if let AdoptionCommand::Confirm {
                old_game_closed, ..
            } = request
            {
                *old_game_closed = false;
            }
        }),
        AdoptionError::ConsentRequired
    );
    assert_eq!(
        refuse(|request| {
            if let AdoptionCommand::Confirm { confirmed, .. } = request {
                *confirmed = false;
            }
        }),
        AdoptionError::ConsentRequired
    );
    assert_eq!(
        refuse(|request| {
            if let AdoptionCommand::Confirm { preview_handle, .. } = request {
                *preview_handle = Uuid::new_v4();
            }
        }),
        AdoptionError::ReviewUnavailable
    );

    let request = confirmation(&review);
    let accepted = f.host.adoption_command(request.clone()).unwrap();
    assert!(accepted.review.is_none());
    // A repeated or replayed confirmation can never start a second copy.
    assert!(matches!(
        f.host.adoption_command(request.clone()),
        Err(AdoptionError::Busy | AdoptionError::ReviewUnavailable)
    ));
    let done = until(&f.host, "publication", Duration::from_secs(30), |status| {
        status.activity == Activity::Idle
    })
    .await;
    assert_eq!(done.last_error, None);
    assert_eq!(
        operation(&done),
        Some((OperationKind::Adopt, OperationState::Succeeded))
    );
    assert_eq!(
        done.completed,
        Some(Completed {
            directory: f.destination()
        })
    );
    assert_eq!(done.preparations, Vec::<Uuid>::new());
    assert_eq!(
        f.host.adoption_command(request).unwrap_err(),
        AdoptionError::ReviewUnavailable
    );
    assert_eq!(f.revision(), done.native.operation.revision);
    assert_eq!(plans(&f.host), 1);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);

    // The copy holds the signed bytes: the edit and the unknown file stay behind.
    let game = f.destination().join("game");
    assert_eq!(fs::read(game.join("later.txt")).unwrap(), b"later entry");
    assert!(!game.join("unknown.dll").exists());
    assert!(game.join("Working/Binaries/SGW.exe").is_file());
    assert_eq!(source_bytes(&f.source), before);
    assert_eq!(
        fs::read(f.host.root.join("legacy-import.json")).unwrap(),
        imported
    );
    assert!(!f.host.root.join("adoption-artifacts").exists());
    assert_eq!(
        done.native.preferences.install_directory,
        Some(f.destination())
    );
    assert!(!done.native.preferences.launcher_summary_consent);
    // Desktop ownership of the new copy only: uninstall targets the destination.
    let target = f.host.install_status().unwrap().uninstall.unwrap();
    assert_eq!(target.directory, f.destination());
    let store = f.host.store().unwrap();
    let kept = store.lock().unwrap().legacy_import().unwrap().unwrap();
    assert!(kept.config.telemetry.opted_in);
    assert!(!kept.config.client_patches.enabled);
    assert_eq!(
        kept.identity.install_id.to_string(),
        "72a8a13b-2a4e-4ea0-b5ba-5ba3cf0a619d"
    );
    drop(store);

    // Reopening restores the published result without any retained worker.
    let Fixture { root, host, .. } = f;
    let path = host.root.clone();
    drop(host);
    let reopened = NativeHost::new(path).adoption_status().unwrap();
    assert_eq!(reopened.completed, done.completed);
    assert_eq!(reopened.activity, Activity::Idle);
    drop(root);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_imports_and_existing_destinations_are_refused_before_any_work() {
    for (config, expected) in [
        (
            serde_json::json!({"manifest_url": "https://example.invalid/custom/manifest.json"}),
            AdoptionError::UnsupportedCatalog,
        ),
        (
            serde_json::json!({"client_patches": {"enabled": true, "dll_override": "C:\\custom.dll"}}),
            AdoptionError::UnsupportedConfiguration,
        ),
    ] {
        let mut f = Fixture::new(config);
        let _server = f.serve().await;
        let status = f.host.adoption_status().unwrap();
        assert_eq!(status.imported.unwrap().blocker, Some(expected));
        assert_eq!(f.host.adoption_choice_allowed().unwrap_err(), expected);
        assert_eq!(f.begin().unwrap_err(), expected);
        assert_eq!(
            f.host.adoption_status().unwrap().native.operation.operation,
            None
        );
    }
    let mut f = Fixture::new(serde_json::json!({}));
    let _server = f.serve().await;
    fs::create_dir(f.destination()).unwrap();
    assert_eq!(f.begin().unwrap_err(), AdoptionError::InvalidDirectory);
    assert_eq!(
        f.host.adoption_status().unwrap().native.operation.operation,
        None
    );
    // Without an import there is nothing to adopt.
    let root = tempfile::tempdir().unwrap();
    let host = NativeHost::new(root.path().join("state"));
    assert_eq!(
        host.adoption_choice_allowed().unwrap_err(),
        AdoptionError::PlatformUnavailable
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn production_admission_requires_the_build_pinned_helper() {
    use sha2::{Digest, Sha256};
    let mut f = Fixture::new(serde_json::json!({}));
    // No fixture origin: this is the production path.
    assert_eq!(
        f.host.adoption_status().unwrap().backend,
        Backend::HelperUnavailable
    );
    assert_eq!(f.begin().unwrap_err(), AdoptionError::PlatformUnavailable);
    let helper = f.root.path().join("cimmeria-archive-worker.exe");
    fs::write(&helper, b"pinned helper").unwrap();
    let pinned: String = Sha256::digest(b"pinned helper")
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    f.host.helper = Some(
        cimmeria_launcher_engine::mac_wine::HelperResource::open(helper.clone(), &pinned).unwrap(),
    );
    assert_eq!(
        f.host.adoption_status().unwrap().backend,
        Backend::Available
    );
    // A helper replaced after startup no longer matches its build identity.
    fs::write(&helper, b"replaced helper").unwrap();
    assert_eq!(
        f.host.adoption_status().unwrap().backend,
        Backend::HelperUnavailable
    );
    assert_eq!(f.begin().unwrap_err(), AdoptionError::PlatformUnavailable);
    assert_eq!(
        f.host.adoption_status().unwrap().native.operation.operation,
        None
    );
    assert!(!f.destination().exists());
}

#[test]
fn ipc_accepts_only_closed_choices_and_identities() {
    for value in [
        r#"{"command":"inspect","schema_version":1,"destination":"/other"}"#,
        r#"{"command":"confirm","schema_version":1,"release":"https://other"}"#,
        r#"{"command":"choose","schema_version":1,"path":"/other"}"#,
        r#"{"command":"abandon_preparation","schema_version":1,"preparation_id":"x","operation_revision":0,"confirmed":true}"#,
    ] {
        assert!(serde_json::from_str::<AdoptionCommand>(value).is_err());
    }
    assert_eq!(
        AdoptionCommand::Inspect { schema_version: 2 }
            .validate()
            .unwrap_err(),
        AdoptionError::UnsupportedSchema
    );
}
