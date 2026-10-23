use super::*;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn direct_repair_recovery_promotes_files_without_swapping_selected_root() {
    let (_temp, state, intent, seed) = installed_content::direct_tests::fixture();
    let root = intent.content_root();
    std::fs::write(root.join("later.txt"), b"damaged").unwrap();
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed))
        .mount(&server)
        .await;
    let plan = {
        let mut owner = state.lock().unwrap();
        let revision = owner.operations().snapshot().revision;
        owner
            .admit_repair(Uuid::new_v4(), revision, intent.operation_id, true)
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
    let result = commit::replace(&state, &prepared, |point| {
        if point == commit::Point::Promoted {
            Err(StorageError::Io)
        } else {
            Ok(())
        }
    });
    assert_eq!(result, Err(preparation::Failure::ReconciliationRequired));
    preparation::finish_failure(
        &state,
        plan.id,
        preparation::Failure::ReconciliationRequired,
    );
    drop(prepared);
    let mut owner = state.lock().unwrap();
    let revision = owner.operations().snapshot().revision;
    recovery::reconcile(&mut owner, plan.id, revision).unwrap();
    assert_eq!(
        std::fs::read(root.join("later.txt")).unwrap(),
        b"later entry"
    );
    assert_eq!(
        std::fs::read(plan.backup().join("later.txt")).unwrap(),
        b"damaged"
    );
    assert_eq!(
        std::fs::read(root.join("Working/Binaries/player-notes.txt")).unwrap(),
        b"keep nested notes"
    );
    assert!(!root.join("game").exists());
    assert_eq!(owner.installed_content().unwrap().unwrap().intent, intent);
    assert!(owner.has_valid_install().unwrap());
    assert_eq!(
        cleanup::status(&mut owner).unwrap(),
        cleanup::BackupStatus::Retained
    );
}
