use super::*;
use std::time::Duration;

pub(crate) async fn fixture(
    missing: bool,
) -> (tempfile::TempDir, Arc<Mutex<DesktopState>>, Prepared) {
    let (root, state, installation, server) = install_worker::tests::prepared_fixture().await;
    let plan = {
        let mut owner = state.lock().unwrap();
        let installed = owner.installed_content().unwrap().unwrap();
        if missing {
            std::fs::remove_dir_all(installed.intent.destination.join("game")).unwrap();
        } else {
            std::fs::write(
                installed.intent.destination.join("game/later.txt"),
                b"damaged",
            )
            .unwrap();
        }
        let revision = owner.operations().snapshot().revision;
        owner
            .admit_repair(Uuid::new_v4(), revision, installation, true)
            .unwrap()
            .plan
    };
    let prepared = preparation::start(
        state.clone(),
        plan.id,
        reqwest::Client::new(),
        format!("{}/manifest.json", server.uri()),
    )
    .unwrap()
    .result
    .await
    .unwrap()
    .unwrap();
    (root, state, prepared)
}
fn operation(state: &Mutex<DesktopState>) -> OperationState {
    state
        .lock()
        .unwrap()
        .operations()
        .snapshot()
        .operation
        .as_ref()
        .unwrap()
        .state
}
#[tokio::test]
async fn retained_commit_replaces_damaged_content_and_preserves_backup_and_identity() {
    for missing in [false, true] {
        let (root, state, prepared) = fixture(missing).await;
        let plan = prepared.plan.clone();
        let preferences = state.lock().unwrap().preferences().clone();
        let result = start(state.clone(), prepared).unwrap().await.unwrap();
        assert_eq!(result, Ok(()));
        assert_eq!(operation(&state), OperationState::Succeeded);
        assert_eq!(
            std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
            b"later entry"
        );
        assert_eq!(plan.backup().exists(), !missing);
        if !missing {
            assert_eq!(
                std::fs::read(plan.backup().join("later.txt")).unwrap(),
                b"damaged"
            );
        }
        assert_eq!(state.lock().unwrap().preferences(), &preferences);
        assert_eq!(
            state
                .lock()
                .unwrap()
                .installed_content()
                .unwrap()
                .unwrap()
                .intent,
            plan.installation
        );
        let saved: Record = read(
            &root
                .path()
                .join(format!("state/repair-commit-{}.json", plan.id)),
        )
        .unwrap()
        .unwrap();
        assert_eq!(saved.phase, Phase::Published);
        // The abandonment observer may still own state briefly; successful work
        // must never regress to uncertainty after the delivered result is dropped.
        tokio::task::yield_now().await;
        assert_eq!(operation(&state), OperationState::Succeeded);
    }
}
#[tokio::test]
async fn lost_commit_observer_does_not_abort_and_precommit_cancel_keeps_old_tree() {
    let (_root, state, prepared) = fixture(false).await;
    let plan = prepared.plan.clone();
    drop(start(state.clone(), prepared).unwrap());
    tokio::time::timeout(Duration::from_secs(5), async {
        while operation(&state) != OperationState::Succeeded {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        std::fs::read(plan.backup().join("later.txt")).unwrap(),
        b"damaged"
    );

    let (root, state, prepared) = fixture(false).await;
    let plan = prepared.plan.clone();
    state
        .lock()
        .unwrap()
        .operations_mut()
        .unwrap()
        .request_cancel(plan.id)
        .unwrap();
    assert_eq!(
        start(state.clone(), prepared).unwrap().await.unwrap(),
        Err(Failure::Cancelled)
    );
    assert_eq!(operation(&state), OperationState::Cancelled);
    assert!(!plan.backup().exists());
    assert!(!root
        .path()
        .join(format!("state/repair-commit-{}.json", plan.id))
        .exists());
    assert_eq!(
        std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
        b"damaged"
    );
}
#[tokio::test]
async fn every_rename_checkpoint_failure_retains_old_bytes_and_refuses_replay() {
    for stop in [
        Point::Planned,
        Point::BeforeOriginalRename,
        Point::AfterOriginalRename,
        Point::OriginalMoved,
        Point::BeforePromotion,
        Point::AfterPromotion,
        Point::Promoted,
        Point::Published,
    ] {
        let (_root, state, prepared) = fixture(false).await;
        let plan = prepared.plan.clone();
        let mut reached = false;
        assert_eq!(
            replace(&state, &prepared, |point| {
                if point == stop {
                    reached = true;
                    Err(StorageError::Io)
                } else {
                    Ok(())
                }
            }),
            Err(Failure::ReconciliationRequired)
        );
        assert!(reached, "fault point was not exercised: {stop:?}");
        let old = if plan.backup().exists() {
            plan.backup()
        } else {
            plan.installation.destination.join("game")
        };
        assert_eq!(
            std::fs::read(old.join("later.txt")).unwrap(),
            b"damaged",
            "{stop:?}"
        );
        // No implicit retry may reinterpret partially promoted content.
        assert_eq!(
            replace(&state, &prepared, |_| Ok(())),
            Err(Failure::ReconciliationRequired)
        );
        assert_ne!(operation(&state), OperationState::Succeeded);
        preparation::finish_failure(&state, plan.id, Failure::ReconciliationRequired);
        assert_eq!(operation(&state), OperationState::ReconciliationRequired);
    }
}
#[tokio::test]
async fn terminal_persistence_failure_preserves_backup_and_reports_uncertainty() {
    let (root, state, prepared) = fixture(false).await;
    let plan = prepared.plan.clone();
    assert_eq!(
        replace(&state, &prepared, |point| {
            if point == Point::Published {
                let journal = root.path().join("state/operation.json");
                std::fs::remove_file(&journal).unwrap();
                std::fs::create_dir(journal).unwrap();
            }
            Ok(())
        }),
        Err(Failure::ReconciliationRequired)
    );
    assert_eq!(
        std::fs::read(plan.backup().join("later.txt")).unwrap(),
        b"damaged"
    );
    assert_eq!(
        std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
        b"later entry"
    );
    assert_ne!(operation(&state), OperationState::Succeeded);
}

#[tokio::test]
async fn conflicting_backup_or_checkpoint_refuses_commit_without_overwrite() {
    for backup_conflict in [true, false] {
        let (root, state, prepared) = fixture(false).await;
        let plan = prepared.plan.clone();
        let conflict = if backup_conflict {
            plan.backup()
        } else {
            root.path()
                .join(format!("state/repair-commit-{}.json", plan.id))
        };
        std::fs::create_dir(&conflict).unwrap();
        std::fs::write(conflict.join("foreign.txt"), b"preserve").unwrap();
        assert_eq!(
            start(state.clone(), prepared).unwrap().await.unwrap(),
            Err(Failure::ReconciliationRequired)
        );
        assert_eq!(
            std::fs::read(conflict.join("foreign.txt")).unwrap(),
            b"preserve"
        );
        assert_eq!(
            std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
            b"damaged"
        );
        assert!(plan.stage().exists());
    }
}

#[cfg(unix)]
#[tokio::test]
async fn nested_link_in_either_tree_refuses_replacement() {
    for staged in [false, true] {
        let (root, state, prepared) = fixture(false).await;
        let plan = prepared.plan.clone();
        let tree = if staged {
            plan.stage()
        } else {
            plan.installation.destination.join("game")
        };
        let outside = root.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(outside.join("foreign.txt"), b"preserve").unwrap();
        std::os::unix::fs::symlink(&outside, tree.join("linked")).unwrap();
        assert_eq!(
            start(state.clone(), prepared).unwrap().await.unwrap(),
            Err(Failure::ReconciliationRequired)
        );
        assert!(!plan.backup().exists());
        assert_eq!(
            std::fs::read(outside.join("foreign.txt")).unwrap(),
            b"preserve"
        );
        assert_eq!(
            std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
            b"damaged"
        );
    }
}

#[tokio::test]
async fn reconstruction_and_commit_keep_current_release_separate_from_original_owner() {
    use std::io::{Cursor, Write};
    use wiremock::{matchers::path, Mock, ResponseTemplate};
    let (_root, state, installation, server) = install_worker::tests::prepared_fixture().await;
    let mut archive =
        zip::ZipWriter::new_append(Cursor::new(install_worker::tests::archive(true))).unwrap();
    archive
        .start_file("version-two.txt", zip::write::SimpleFileOptions::default())
        .unwrap();
    archive.write_all(b"new release content").unwrap();
    let seed = archive.finish().unwrap().into_inner();
    let release = install_worker::tests::verified(&seed);
    server.reset().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed))
        .mount(&server)
        .await;
    let (plan, owner_bytes, current) = {
        let mut state = state.lock().unwrap();
        let installed = state.installed_content().unwrap().unwrap();
        let owner = installed.intent;
        let owner_bytes = std::fs::read(owner.destination.join(".cimmeria-install.json")).unwrap();
        let current = ReleaseIdentity {
            evidence_id: Uuid::new_v4(),
            manifest_digest: release.digest(),
        };
        state
            .save_release_evidence(current.evidence_id, &release)
            .unwrap();
        installed_content::write_ready(&owner.destination, &owner, current).unwrap();
        atomic::write(
            &state.directory.root,
            "installed-content.json",
            &serde_json::json!({
                "schema_version": 3, "intent": owner, "current_release": current
            }),
        )
        .unwrap();
        std::fs::write(owner.destination.join("game/later.txt"), b"damaged").unwrap();
        let revision = state.operations().snapshot().revision;
        let plan = state
            .admit_repair(Uuid::new_v4(), revision, installation, true)
            .unwrap()
            .plan;
        (plan, owner_bytes, current)
    };
    let prepared = preparation::start(
        state.clone(),
        plan.id,
        reqwest::Client::new(),
        format!("{}/manifest.json", server.uri()),
    )
    .unwrap()
    .result
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        start(state.clone(), prepared).unwrap().await.unwrap(),
        Ok(())
    );
    let mut state = state.lock().unwrap();
    let installed = state.installed_content().unwrap().unwrap();
    assert_eq!(installed.current_release, current);
    assert_eq!(installed.intent.operation_id, installation);
    assert_eq!(installed.release.digest(), current.manifest_digest);
    assert_ne!(installed.intent.manifest_digest, current.manifest_digest);
    assert_eq!(
        std::fs::read(plan.installation.destination.join(".cimmeria-install.json")).unwrap(),
        owner_bytes
    );
    assert_eq!(
        std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
        b"later entry"
    );
    assert_eq!(
        std::fs::read(plan.installation.destination.join("game/version-two.txt")).unwrap(),
        b"new release content"
    );
    assert!(!plan.backup().join("version-two.txt").exists());
    assert_eq!(
        std::fs::read(plan.backup().join("later.txt")).unwrap(),
        b"damaged"
    );
}
