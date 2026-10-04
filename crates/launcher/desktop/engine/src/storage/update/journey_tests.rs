use super::*;
use crate::OperationState;
use std::{
    io::{Cursor, Write},
    sync::Mutex,
    time::Duration,
};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

async fn fixture() -> (
    tempfile::TempDir,
    Arc<Mutex<DesktopState>>,
    preparation::Prepared,
    MockServer,
) {
    let (root, state, installation, server) = install_worker::tests::prepared_fixture().await;
    let mut archive =
        zip::ZipWriter::new_append(Cursor::new(install_worker::tests::archive(true))).unwrap();
    archive
        .start_file("version-two.txt", zip::write::SimpleFileOptions::default())
        .unwrap();
    archive.write_all(b"new signed content").unwrap();
    let seed = archive.finish().unwrap().into_inner();
    let target = install_worker::tests::verified(&seed);
    server.reset().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed))
        .mount(&server)
        .await;
    let plan = {
        let mut state = state.lock().unwrap();
        let installed = state.installed_content().unwrap().unwrap();
        std::fs::write(
            installed.intent.destination.join("game/custom.txt"),
            b"preserve old modifications",
        )
        .unwrap();
        let revision = state.operations().snapshot().revision;
        state
            .admit_update(Request {
                id: Uuid::new_v4(),
                operation_revision: revision,
                installation_id: installation,
                expected_current: installed.current_release,
                target: &target,
                confirmed: true,
            })
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
    (root, state, prepared, server)
}
async fn close(state: Arc<Mutex<DesktopState>>) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while Arc::strong_count(&state) != 1 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    drop(Arc::try_unwrap(state).ok().unwrap().into_inner().unwrap());
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
async fn signed_update_reopen_repair_current_and_uninstall_keep_permanent_owner() {
    let (root, state, prepared, server) = fixture().await;
    let plan = prepared.plan.clone();
    let owner_bytes = std::fs::read(plan.owner.destination.join(".cimmeria-install.json")).unwrap();
    assert_eq!(
        commit::start(state.clone(), prepared)
            .unwrap()
            .await
            .unwrap(),
        Ok(())
    );
    assert_eq!(
        std::fs::read(plan.backup().join("custom.txt")).unwrap(),
        b"preserve old modifications"
    );
    assert!(!plan.owner.destination.join("game/custom.txt").exists());
    close(state).await;
    let state = Arc::new(Mutex::new(
        DesktopState::open(&root.path().join("state")).unwrap(),
    ));
    let repair = {
        let mut state = state.lock().unwrap();
        let installed = state.installed_content().unwrap().unwrap();
        assert_eq!(installed.intent, plan.owner);
        assert_eq!(installed.current_release, plan.target);
        assert_eq!(
            std::fs::read(plan.owner.destination.join(".cimmeria-install.json")).unwrap(),
            owner_bytes
        );
        std::fs::remove_file(plan.owner.destination.join("game/version-two.txt")).unwrap();
        let revision = state.operations().snapshot().revision;
        state
            .admit_repair(Uuid::new_v4(), revision, plan.owner.operation_id, true)
            .unwrap()
            .plan
    };
    assert_eq!(repair.release_identity(), plan.target);
    let prepared = repair::preparation::start(
        state.clone(),
        repair.id,
        reqwest::Client::new(),
        format!("{}/manifest.json", server.uri()),
    )
    .unwrap()
    .result
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        repair::commit::start(state.clone(), prepared)
            .unwrap()
            .await
            .unwrap(),
        Ok(())
    );
    assert_eq!(
        std::fs::read(plan.owner.destination.join("game/version-two.txt")).unwrap(),
        b"new signed content"
    );
    close(state).await;
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    let revision = state.operations().snapshot().revision;
    state
        .uninstall(Uuid::new_v4(), revision, plan.owner.operation_id, true)
        .unwrap();
    assert!(!plan.owner.destination.exists());
    assert!(state.installed_content().unwrap().is_none());
    for identity in [plan.previous, plan.target] {
        assert_eq!(
            state.verify_release_identity(identity).unwrap().digest(),
            identity.manifest_digest
        );
    }
}
#[tokio::test]
async fn every_update_rename_and_publication_boundary_recovers_without_replaying_downloads() {
    use commit::Point::*;
    for point in [
        Planned,
        BeforeOriginalRename,
        AfterOriginalRename,
        OriginalMoved,
        BeforePromotion,
        AfterPromotion,
        Promoted,
        BeforeReceipt,
        AfterReceipt,
        BeforeIndex,
        AfterIndex,
        Published,
    ] {
        let (root, state, prepared, server) = fixture().await;
        let plan = prepared.plan.clone();
        let requests = server.received_requests().await.unwrap().len();
        let mut reached = false;
        assert_eq!(
            commit::replace(&state, &prepared, |current| {
                if current == point {
                    reached = true;
                    Err(StorageError::Io)
                } else {
                    Ok(())
                }
            }),
            Err(preparation::Failure::ReconciliationRequired)
        );
        assert!(reached, "{point:?}");
        drop(prepared);
        close(state).await;
        let mut state = DesktopState::open(&root.path().join("state")).unwrap();
        let revision = state.operations().snapshot().revision;
        assert_eq!(
            state
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state,
            OperationState::ReconciliationRequired
        );
        recovery::reconcile(&mut state, plan.id, revision).unwrap();
        assert_eq!(
            state.installed_content().unwrap().unwrap().current_release,
            plan.target
        );
        assert_eq!(
            std::fs::read(plan.backup().join("custom.txt")).unwrap(),
            b"preserve old modifications"
        );
        assert_eq!(server.received_requests().await.unwrap().len(), requests);
        assert!(recovery::reconcile(&mut state, plan.id, revision).is_err());
    }
}
#[tokio::test]
async fn cancellation_and_lost_commit_observer_preserve_old_tree_and_never_replay() {
    let (_root, state, prepared, _server) = fixture().await;
    let plan = prepared.plan.clone();
    state
        .lock()
        .unwrap()
        .operations_mut()
        .unwrap()
        .request_cancel(plan.id)
        .unwrap();
    assert_eq!(
        commit::start(state.clone(), prepared)
            .unwrap()
            .await
            .unwrap(),
        Err(preparation::Failure::Cancelled)
    );
    assert_eq!(operation(&state), OperationState::Cancelled);
    assert!(!plan.backup().exists());
    assert_eq!(
        std::fs::read(plan.owner.destination.join("game/custom.txt")).unwrap(),
        b"preserve old modifications"
    );
    let (_root, state, prepared, server) = fixture().await;
    let plan = prepared.plan.clone();
    let requests = server.received_requests().await.unwrap().len();
    drop(commit::start(state.clone(), prepared).unwrap());
    tokio::time::timeout(Duration::from_secs(5), async {
        while operation(&state) != OperationState::Succeeded {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        preparation::start(state.clone(), plan.id, reqwest::Client::new(), server.uri()).is_err()
    );
    assert_eq!(server.received_requests().await.unwrap().len(), requests);
}
#[tokio::test]
async fn foreign_stage_owner_and_backup_are_refused_without_touching_old_content() {
    for case in 0..4 {
        let (_root, state, prepared, _server) = fixture().await;
        let plan = prepared.plan.clone();
        match case {
            0 => {
                use std::io::{Seek, SeekFrom};
                let mut file = &*prepared._work_owner;
                file.seek(SeekFrom::Start(0)).unwrap();
                file.set_len(0).unwrap();
                file.write_all(b"foreign owner").unwrap();
            }
            1 => {
                std::fs::create_dir(plan.backup()).unwrap();
                std::fs::write(plan.backup().join("foreign.txt"), b"foreign").unwrap();
            }
            2 => {
                std::fs::remove_file(plan.stage().join("Working/Binaries/SGW.exe")).unwrap();
            }
            _ => {
                std::fs::write(
                    plan.stage().join("Working/Binaries/SGW.exe"),
                    b"foreign nonempty executable",
                )
                .unwrap();
            }
        }
        assert_eq!(
            commit::start(state.clone(), prepared)
                .unwrap()
                .await
                .unwrap(),
            Err(preparation::Failure::ReconciliationRequired)
        );
        assert_eq!(
            std::fs::read(plan.owner.destination.join("game/custom.txt")).unwrap(),
            b"preserve old modifications"
        );
    }
}
#[tokio::test]
async fn rollback_is_new_confirmed_signed_reconstruction_and_keeps_both_backups() {
    let (_root, state, prepared, server) = fixture().await;
    let forward = prepared.plan.clone();
    assert_eq!(
        commit::start(state.clone(), prepared)
            .unwrap()
            .await
            .unwrap(),
        Ok(())
    );
    let rollback = {
        let mut state = state.lock().unwrap();
        let revision = state.operations().snapshot().revision;
        assert!(state
            .admit_update_rollback(forward.id, Uuid::new_v4(), revision, forward.target, false)
            .is_err());
        state
            .admit_update_rollback(forward.id, Uuid::new_v4(), revision, forward.target, true)
            .unwrap()
            .plan
    };
    assert_eq!(rollback.previous, forward.target);
    assert_eq!(
        rollback.target.manifest_digest,
        forward.previous.manifest_digest
    );
    server.reset().await;
    Mock::given(path("/seed.zip"))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(install_worker::tests::archive(true)),
        )
        .mount(&server)
        .await;
    let prepared = preparation::start(
        state.clone(),
        rollback.id,
        reqwest::Client::new(),
        format!("{}/manifest.json", server.uri()),
    )
    .unwrap()
    .result
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        commit::start(state.clone(), prepared)
            .unwrap()
            .await
            .unwrap(),
        Ok(())
    );
    assert!(!rollback
        .owner
        .destination
        .join("game/version-two.txt")
        .exists());
    assert!(rollback.backup().join("version-two.txt").exists());
    assert!(forward.backup().join("custom.txt").exists());
    assert_eq!(
        state
            .lock()
            .unwrap()
            .installed_content()
            .unwrap()
            .unwrap()
            .intent,
        forward.owner
    );
}

#[tokio::test]
async fn lost_prepared_handoff_requires_explicit_abandon_then_discard() {
    let (root, state, prepared, server) = fixture().await;
    let plan = prepared.plan.clone();
    let requests = server.received_requests().await.unwrap().len();
    drop(prepared);
    close(state).await;
    let state = Arc::new(Mutex::new(
        DesktopState::open(&root.path().join("state")).unwrap(),
    ));
    let revision = state.lock().unwrap().operations().snapshot().revision;
    assert!(
        preparation::start(state.clone(), plan.id, reqwest::Client::new(), server.uri()).is_err()
    );
    assert!(discard::dispatch(state.clone(), plan.id, revision, true)
        .unwrap()
        .await
        .unwrap()
        .is_err());
    abandon::dispatch(state.clone(), plan.id, revision, true)
        .unwrap()
        .await
        .unwrap()
        .unwrap();
    let revision = state.lock().unwrap().operations().snapshot().revision;
    discard::dispatch(state.clone(), plan.id, revision, true)
        .unwrap()
        .await
        .unwrap()
        .unwrap();
    assert!(!plan.work_directory().exists());
    assert!(!plan.backup().exists());
    assert_eq!(
        std::fs::read(plan.owner.destination.join("game/custom.txt")).unwrap(),
        b"preserve old modifications"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), requests);
    assert_eq!(
        state
            .lock()
            .unwrap()
            .installed_content()
            .unwrap()
            .unwrap()
            .current_release,
        plan.previous
    );
    discard::dispatch(state.clone(), plan.id, revision, true)
        .unwrap()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn backup_cleanup_keeps_current_release_and_foreign_auxiliary_is_not_uninstalled() {
    let (_root, state, prepared, _server) = fixture().await;
    let plan = prepared.plan.clone();
    commit::start(state.clone(), prepared)
        .unwrap()
        .await
        .unwrap()
        .unwrap();
    let revision = state.lock().unwrap().operations().snapshot().revision;
    assert_eq!(
        cleanup::status(&mut state.lock().unwrap()).unwrap(),
        cleanup::BackupStatus::Retained
    );
    cleanup::dispatch(state.clone(), plan.id, revision)
        .unwrap()
        .await
        .unwrap()
        .unwrap();
    assert!(!plan.backup().exists());
    assert_eq!(
        cleanup::status(&mut state.lock().unwrap()).unwrap(),
        cleanup::BackupStatus::Removed
    );
    std::fs::write(plan.work_directory().join("owner.json"), b"foreign owner").unwrap();
    let mut state = state.lock().unwrap();
    assert!(state
        .uninstall(Uuid::new_v4(), revision, plan.owner.operation_id, true)
        .is_err());
    assert!(plan
        .owner
        .destination
        .join("game/version-two.txt")
        .is_file());
}
