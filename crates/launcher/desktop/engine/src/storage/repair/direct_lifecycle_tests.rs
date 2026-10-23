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

#[tokio::test]
async fn direct_repair_metadata_refuses_a_substituted_root_after_promotion() {
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

    let displaced = _temp.path().join("retained");
    let mut attempted = false;
    #[cfg(unix)]
    let mut outside = None;
    let result = commit::replace(&state, &prepared, |point| {
        if point == commit::Point::Promoted {
            attempted = true;
            #[cfg(windows)]
            assert!(std::fs::rename(&root, &displaced).is_err());
            #[cfg(unix)]
            {
                std::fs::rename(&root, &displaced).unwrap();
                clone_tree(&displaced, &root);
                use std::os::unix::fs::MetadataExt;
                let receipt = root.join("content-ready.json");
                outside = Some((
                    std::fs::metadata(&receipt).unwrap().ino(),
                    std::fs::read(&receipt).unwrap(),
                ));
            }
        }
        Ok(())
    });
    assert!(attempted);
    #[cfg(windows)]
    assert_eq!(result, Ok(()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(result, Err(preparation::Failure::ReconciliationRequired));
        let receipt = root.join("content-ready.json");
        assert_eq!(
            outside,
            Some((
                std::fs::metadata(&receipt).unwrap().ino(),
                std::fs::read(&receipt).unwrap()
            ))
        );
    }
}

#[cfg(unix)]
fn clone_tree(from: &Path, to: &Path) {
    std::fs::create_dir(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            clone_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}
