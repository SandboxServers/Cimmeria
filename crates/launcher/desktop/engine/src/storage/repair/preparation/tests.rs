use super::*;
fn admit(state: &Mutex<DesktopState>, installation: Uuid) -> Plan {
    let mut owner = state.lock().unwrap();
    let revision = owner.operations().snapshot().revision;
    owner
        .admit_repair(Uuid::new_v4(), revision, installation, true)
        .unwrap()
        .plan
}
#[tokio::test]
async fn reconstructs_real_signed_seed_fresh_and_retains_both_locks_for_commit() {
    let (_root, state, installation, server) = install_worker::tests::prepared_fixture().await;
    let plan = admit(&state, installation);
    let old = plan.installation.destination.join("game/later.txt");
    std::fs::write(&old, b"damaged old tree").unwrap();
    let preparation = start(
        state.clone(),
        plan.id,
        reqwest::Client::new(),
        format!("{}/manifest.json", server.uri()),
    )
    .unwrap();
    assert!(start(state.clone(), plan.id, reqwest::Client::new(), server.uri()).is_err());
    let prepared = preparation.result.await.unwrap().unwrap();
    assert_eq!(std::fs::read(&old).unwrap(), b"damaged old tree");
    assert_eq!(
        std::fs::read(prepared.plan.stage().join("later.txt")).unwrap(),
        b"later entry"
    );
    assert!(lock_owner(&plan.installation).is_err());
    assert!(!plan.backup().exists());
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
    assert!(!state.lock().unwrap().preferences().launcher_summary_consent);
    drop(prepared);
    assert!(lock_owner(&plan.installation).is_ok());
}
#[tokio::test]
async fn pre_cancel_does_not_create_work_or_download_and_observer_loss_preserves_old_tree() {
    let (_root, state, installation, server) = install_worker::tests::prepared_fixture().await;
    let plan = admit(&state, installation);
    let preparation = start(state.clone(), plan.id, reqwest::Client::new(), server.uri()).unwrap();
    preparation.request_cancel().unwrap();
    assert!(matches!(
        preparation.result.await.unwrap(),
        Err(Failure::Cancelled)
    ));
    assert!(!plan.work_directory().exists());
    assert!(plan.installation.destination.join("game").exists());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    let plan = admit(&state, installation);
    drop(
        start(
            state.clone(),
            plan.id,
            reqwest::Client::new(),
            format!("{}/manifest.json", server.uri()),
        )
        .unwrap(),
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if state
                .lock()
                .unwrap()
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state
                == OperationState::ReconciliationRequired
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(plan.stage().join("later.txt").is_file());
    assert!(plan
        .installation
        .destination
        .join("game/later.txt")
        .is_file());
    assert!(!plan.backup().exists());
}

#[tokio::test]
async fn failed_terminal_persistence_is_reported_as_uncertain() {
    let (root, state, installation, _server) = install_worker::tests::prepared_fixture().await;
    let plan = admit(&state, installation);
    let journal = root.path().join("state/operation.json");
    std::fs::remove_file(&journal).unwrap();
    std::fs::create_dir(journal).unwrap();
    assert_eq!(
        finish_failure(&state, plan.id, Failure::Failed),
        Failure::ReconciliationRequired
    );
    assert!(plan
        .installation
        .destination
        .join("game/later.txt")
        .is_file());
}
