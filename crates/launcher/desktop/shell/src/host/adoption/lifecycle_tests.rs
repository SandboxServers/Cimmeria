//! Retained worker lifecycle: status during a copy, cancellation, dismissal,
//! re-selection, interruption and recovery.
use super::fixture::*;
use super::*;
use adoption::test_support::CopyFault;
use cimmeria_launcher_engine::install_worker::fixtures;
use std::{fs, time::Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_reads_answer_while_the_retained_copy_runs_and_while_the_store_is_held() {
    let mut f = Fixture::new(serde_json::json!({}));
    let _server = f.serve().await;
    f.begin().unwrap();
    let review = reviewed(&f.host).await;
    let (resume, held) = std::sync::mpsc::channel();
    f.fault(CopyFault::Hold(held));
    let accepted = f.host.adoption_command(confirmation(&review)).unwrap();
    assert_eq!(accepted.activity, Activity::Copying);
    // The worker is paused where the copy starts. A status read gets the store.
    let copying = until(&f.host, "the running copy", Duration::from_secs(10), |s| {
        operation(s) == Some((OperationKind::Adopt, OperationState::Running))
            && s.native.operation.revision > review.operation_revision
    })
    .await;
    assert_eq!(copying.activity, Activity::Copying);
    assert!(copying.cancellable && copying.review.is_none());
    // Publication verifies the staged copy under the store for a long time. A
    // reader must still be answered, from the last facts.
    let host = Arc::new(f.host);
    let store = host.store().unwrap();
    let (release, wait) = std::sync::mpsc::channel::<()>();
    let (locked, ready) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let _guard = store.lock().unwrap();
        locked.send(()).unwrap();
        let _ = wait.recv();
    });
    ready.recv().unwrap();
    let reader = host.clone();
    let answered = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::task::spawn_blocking(move || reader.adoption_status()),
    )
    .await;
    release.send(()).unwrap();
    holder.join().unwrap();
    let answered = answered
        .expect("a status read must not wait for the store during a copy")
        .unwrap()
        .unwrap();
    assert_eq!(answered.activity, Activity::Copying);
    assert_eq!(
        answered.native.operation.revision,
        copying.native.operation.revision
    );
    resume.send(()).unwrap();
    let done = until(&host, "publication", Duration::from_secs(30), |status| {
        status.activity == Activity::Idle
    })
    .await;
    assert_eq!(done.last_error, None);
    assert!(done.completed.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_a_copy_before_publication_is_terminal_and_changes_no_ownership() {
    let mut f = Fixture::new(serde_json::json!({}));
    let _server = f.serve().await;
    let before = source_bytes(&f.source);
    let preferences = f.host.adoption_status().unwrap().native.preferences;
    f.begin().unwrap();
    let review = reviewed(&f.host).await;
    let (resume, held) = std::sync::mpsc::channel();
    f.fault(CopyFault::Hold(held));
    f.host.adoption_command(confirmation(&review)).unwrap();
    // Cancel once the copy is admitted, not while it is still being requested.
    until(&f.host, "the running copy", Duration::from_secs(10), |s| {
        operation(s) == Some((OperationKind::Adopt, OperationState::Running))
            && s.native.operation.revision > review.operation_revision
    })
    .await;
    assert!(f.host.adoption_command(CANCEL).unwrap().cancellable);
    resume.send(()).unwrap();
    let stopped = until(
        &f.host,
        "the cancelled copy",
        Duration::from_secs(30),
        |s| s.activity == Activity::Idle,
    )
    .await;
    assert_eq!(stopped.last_error, Some(AdoptionError::Cancelled));
    assert_eq!(
        operation(&stopped),
        Some((OperationKind::Adopt, OperationState::Cancelled))
    );
    assert_eq!(stopped.completed, None);
    assert_eq!(stopped.reconciliation, None);
    assert_eq!(stopped.preparations, Vec::<Uuid>::new());
    assert_eq!(stopped.native.preferences, preferences);
    assert!(!f.destination().join("game").exists());
    assert!(f.host.install_status().unwrap().uninstall.is_none());
    assert_eq!(source_bytes(&f.source), before);
    // The partial folder is left alone, so the same location is refused up front.
    assert_eq!(f.begin().unwrap_err(), AdoptionError::InvalidDirectory);
    assert_eq!(f.revision(), stopped.native.operation.revision);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_running_preparation_refuses_a_second_choice_and_stops_when_cancelled() {
    let mut f = Fixture::new(serde_json::json!({}));
    let before = source_bytes(&f.source);
    let stalled = StalledOrigin::new(f.seed.clone());
    f.serve_from(stalled.url.clone());
    f.begin().unwrap();
    let downloading = until(&f.host, "download progress", Duration::from_secs(10), |s| {
        s.progress.is_some()
    })
    .await;
    assert_eq!(downloading.activity, Activity::Preparing);
    assert_eq!(
        downloading.progress.map(|p| (p.phase, p.total)),
        Some((Phase::Download, f.seed.len() as u64))
    );
    assert_eq!(
        operation(&downloading),
        Some((OperationKind::Adopt, OperationState::Running))
    );
    // Choosing again must not orphan the running download behind a new one.
    assert_eq!(f.begin().unwrap_err(), AdoptionError::Busy);
    assert_eq!(f.revision(), downloading.native.operation.revision);
    f.host.adoption_command(CANCEL).unwrap();
    let stopped = until(
        &f.host,
        "the cancelled preparation",
        Duration::from_secs(10),
        |s| s.activity == Activity::Idle,
    )
    .await;
    assert_eq!(stopped.last_error, Some(AdoptionError::Cancelled));
    assert_eq!(
        operation(&stopped),
        Some((OperationKind::Adopt, OperationState::Cancelled))
    );
    assert_eq!(stopped.preparations, Vec::<Uuid>::new());
    assert_eq!(stopped.reconciliation, None);
    assert!(!f.destination().exists());
    assert_eq!(source_bytes(&f.source), before);
    drop(stalled);

    // The same location can now be prepared, reviewed, dismissed and chosen again.
    let _server = f.serve().await;
    f.begin().unwrap();
    let review = reviewed(&f.host).await;
    let dismissed = f.host.adoption_command(DISMISS).unwrap();
    assert_eq!(dismissed.activity, Activity::Idle);
    assert!(dismissed.review.is_none());
    assert_eq!(
        operation(&dismissed),
        Some((OperationKind::Adopt, OperationState::Cancelled))
    );
    assert_eq!(dismissed.preparations, Vec::<Uuid>::new());
    assert_eq!(
        f.host.adoption_command(confirmation(&review)).unwrap_err(),
        AdoptionError::ReviewUnavailable
    );
    assert!(f.host.adoption_choice_allowed().is_ok());
    assert!(!f.destination().exists());
    assert_eq!(source_bytes(&f.source), before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dismissing_a_running_preparation_stops_its_download() {
    let mut f = Fixture::new(serde_json::json!({}));
    let stalled = StalledOrigin::new(f.seed.clone());
    f.serve_from(stalled.url.clone());
    f.begin().unwrap();
    until(&f.host, "download progress", Duration::from_secs(10), |s| {
        s.progress.is_some()
    })
    .await;
    f.host.adoption_command(DISMISS).unwrap();
    let stopped = until(
        &f.host,
        "the dismissed preparation",
        Duration::from_secs(10),
        |s| s.activity == Activity::Idle,
    )
    .await;
    assert_eq!(
        operation(&stopped),
        Some((OperationKind::Adopt, OperationState::Cancelled))
    );
    assert_eq!(stopped.preparations, Vec::<Uuid>::new());
    assert_eq!(
        fs::read_dir(f.host.root.join("adoption-artifacts"))
            .unwrap()
            .count(),
        0,
        "the partial download must not be kept"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_interrupted_preparation_is_offered_for_removal_and_survives_reopen() {
    let mut f = Fixture::new(serde_json::json!({}));
    // No origin is contacted: the backend only has to be available.
    let origin = "http://127.0.0.1:9/manifest.json";
    f.serve_from(origin.into());
    let before = source_bytes(&f.source);
    let store = f.host.store().unwrap();
    let request = {
        let state = store.lock().unwrap();
        adoption::PreviewRequest {
            import_digest: state.legacy_import().unwrap().unwrap().confirmation,
            destination: f.destination(),
            operation_revision: state.operations().snapshot().revision,
            preferences_revision: state.preferences().revision,
            release: f.release(),
            artifacts: None,
        }
    };
    let id = adoption::test_support::interrupted_preparation(store.clone(), &request).unwrap();
    drop(store);
    let Fixture { root, host, .. } = f;
    let interrupted = host.adoption_status().unwrap();
    assert_eq!(
        interrupted.reconciliation,
        Some(Reconciliation::Preparation { preparation_id: id })
    );
    assert_eq!(interrupted.preparations, Vec::<Uuid>::new());
    assert_eq!(
        host.adoption_choice_allowed().unwrap_err(),
        AdoptionError::RecoveryRequired
    );
    let path = host.root.clone();
    drop(host);
    let mut host = NativeHost::new(path);
    host.adoption_fixture = Some(TestDispatch {
        manifest_url: origin.into(),
        helper: None,
        copy_fault: Mutex::new(None),
    });
    let reopened = host.adoption_status().unwrap();
    assert_eq!(reopened.reconciliation, interrupted.reconciliation);
    let directory = fs::read_dir(&host.root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|entry| {
            entry
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".adoption-reference-")
        })
        .unwrap();
    let remove = |revision, confirmed| AdoptionCommand::AbandonPreparation {
        schema_version: 1,
        preparation_id: id,
        operation_revision: revision,
        confirmed,
    };
    let revision = reopened.native.operation.revision;
    assert_eq!(
        host.adoption_command(remove(revision, false)).unwrap_err(),
        AdoptionError::ConsentRequired
    );
    assert_eq!(
        host.adoption_command(remove(revision - 1, true))
            .unwrap_err(),
        AdoptionError::StaleRevision
    );
    assert!(directory.is_dir());
    let removed = host.adoption_command(remove(revision, true)).unwrap();
    assert_eq!(removed.reconciliation, None);
    assert_eq!(removed.preparations, Vec::<Uuid>::new());
    assert_eq!(
        operation(&removed),
        Some((OperationKind::Adopt, OperationState::Cancelled))
    );
    assert!(!directory.exists());
    assert!(host.adoption_choice_allowed().is_ok());
    assert_eq!(
        [
            tree(&root.path().join("old-game")),
            tree(&root.path().join("legacy"))
        ],
        before
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_interrupted_copy_offers_only_the_recovery_its_checkpoint_allows() {
    // After promotion a verified staged copy exists: recovery finishes it.
    let mut f = Fixture::new(serde_json::json!({}));
    let _server = f.serve().await;
    let before = source_bytes(&f.source);
    f.begin().unwrap();
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
    assert_eq!(failed.last_error, Some(AdoptionError::Io));
    assert_eq!(failed.completed, None);
    let Some(Reconciliation::Copy {
        operation_id,
        directory,
        can_recover: true,
        can_abandon: false,
    }) = failed.reconciliation.clone()
    else {
        panic!("promoted copy must be recoverable only: {failed:?}");
    };
    assert_eq!(directory, f.destination());
    let revision = failed.native.operation.revision;
    let act = |recover: bool, revision| {
        if recover {
            AdoptionCommand::Recover {
                schema_version: 1,
                operation_id,
                operation_revision: revision,
                confirmed: true,
            }
        } else {
            AdoptionCommand::Abandon {
                schema_version: 1,
                operation_id,
                operation_revision: revision,
                confirmed: true,
            }
        }
    };
    assert_eq!(
        f.host
            .adoption_command(act(true, revision - 1))
            .unwrap_err(),
        AdoptionError::StaleRevision
    );
    assert_eq!(
        f.host.adoption_command(act(false, revision)).unwrap_err(),
        AdoptionError::RecoveryRequired
    );
    let recovered = f.host.adoption_command(act(true, revision)).unwrap();
    assert_eq!(recovered.reconciliation, None);
    assert_eq!(recovered.last_error, None);
    assert_eq!(
        recovered.completed,
        Some(Completed {
            directory: f.destination()
        })
    );
    assert_eq!(source_bytes(&f.source), before);

    // Before any checkpoint nothing can be finished: only abandonment remains,
    // and it keeps the copied bytes where they are.
    let mut f = Fixture::new(serde_json::json!({}));
    let _server = f.serve().await;
    f.begin().unwrap();
    let review = reviewed(&f.host).await;
    f.fault(CopyFault::BeforeCheckpoint);
    f.host.adoption_command(confirmation(&review)).unwrap();
    let failed = until(
        &f.host,
        "the interrupted copy",
        Duration::from_secs(30),
        |s| s.activity == Activity::Idle,
    )
    .await;
    let Some(Reconciliation::Copy {
        operation_id,
        can_recover: false,
        can_abandon: true,
        ..
    }) = failed.reconciliation
    else {
        panic!("unstaged copy must be abandonable only: {failed:?}");
    };
    let revision = failed.native.operation.revision;
    assert!(f
        .host
        .adoption_command(AdoptionCommand::Recover {
            schema_version: 1,
            operation_id,
            operation_revision: revision,
            confirmed: true,
        })
        .is_err());
    let abandoned = f
        .host
        .adoption_command(AdoptionCommand::Abandon {
            schema_version: 1,
            operation_id,
            operation_revision: revision,
            confirmed: true,
        })
        .unwrap();
    assert_eq!(abandoned.reconciliation, None);
    assert_eq!(
        operation(&abandoned),
        Some((OperationKind::Adopt, OperationState::Cancelled))
    );
    assert!(f.destination().exists());
    assert!(f.host.install_status().unwrap().uninstall.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_status_reads_cannot_wedge_while_a_preview_completes() {
    let mut f = Fixture::new(serde_json::json!({}));
    let _server = f.serve().await;
    let host = Arc::new(f.host);
    for _ in 0..4 {
        host.begin_adoption_preview(f.root.path().join("library"), fixtures::verified(&f.seed))
            .unwrap();
        // Readers race the worker's completion, the path that takes both the
        // retained job and the store.
        let readers: Vec<_> = (0..6)
            .map(|_| {
                let host = host.clone();
                tokio::task::spawn_blocking(move || {
                    for _ in 0..400 {
                        let status = host.adoption_status().unwrap();
                        if status.review.is_some() {
                            return true;
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    false
                })
            })
            .collect();
        for reader in readers {
            assert!(tokio::time::timeout(Duration::from_secs(20), reader)
                .await
                .expect("status readers deadlocked")
                .unwrap());
        }
        let host = host.clone();
        tokio::time::timeout(
            Duration::from_secs(20),
            tokio::task::spawn_blocking(move || host.adoption_command(DISMISS).unwrap()),
        )
        .await
        .expect("dismissal deadlocked")
        .unwrap();
    }
}
