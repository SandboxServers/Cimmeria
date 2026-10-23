use super::*;
use std::io::{Cursor, Write};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn direct_update_rollback_then_uninstall_preserves_selected_root_and_nested_extras() {
    let (_temp, state, intent, original_seed) = installed_content::direct_tests::fixture();
    let mut archive = zip::ZipWriter::new_append(Cursor::new(original_seed.clone())).unwrap();
    archive
        .start_file("version-two.txt", zip::write::SimpleFileOptions::default())
        .unwrap();
    archive.write_all(b"version two").unwrap();
    let seed = archive.finish().unwrap().into_inner();
    let target = install_worker::fixtures::verified(&seed);
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed))
        .mount(&server)
        .await;
    let plan = {
        let mut owner = state.lock().unwrap();
        let installed = owner.installed_content().unwrap().unwrap();
        let revision = owner.operations().snapshot().revision;
        owner
            .admit_update(Request {
                id: Uuid::new_v4(),
                operation_revision: revision,
                installation_id: intent.operation_id,
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
    assert_eq!(
        commit::start(state.clone(), prepared)
            .unwrap()
            .await
            .unwrap(),
        Ok(())
    );
    let root = intent.content_root();
    assert_eq!(
        std::fs::read(root.join("version-two.txt")).unwrap(),
        b"version two"
    );
    assert_eq!(
        std::fs::read(root.join("Working/Binaries/player-notes.txt")).unwrap(),
        b"keep nested notes"
    );
    assert!(!root.join("game").exists());
    let rollback = {
        let mut owner = state.lock().unwrap();
        let revision = owner.operations().snapshot().revision;
        owner
            .admit_update_rollback(plan.id, Uuid::new_v4(), revision, plan.target, true)
            .unwrap()
            .plan
    };
    server.reset().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(original_seed))
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
    assert!(!root.join("version-two.txt").exists());
    assert_eq!(
        std::fs::read(rollback.backup().join("version-two.txt")).unwrap(),
        b"version two"
    );
    assert_eq!(
        std::fs::read(root.join("Working/Binaries/player-notes.txt")).unwrap(),
        b"keep nested notes"
    );
    assert_eq!(root.canonicalize().unwrap(), intent.destination);
    let mut owner = state.lock().unwrap();
    assert_eq!(
        owner.installed_content().unwrap().unwrap().current_release,
        rollback.target
    );
    assert!(owner.has_valid_install().unwrap());
    let revision = owner.operations().snapshot().revision;
    owner
        .uninstall(Uuid::new_v4(), revision, intent.operation_id, true)
        .unwrap();
    assert!(root.is_dir());
    assert!(!root.join("version-two.txt").exists());
    assert_eq!(
        std::fs::read(root.join("Working/Binaries/player-notes.txt")).unwrap(),
        b"keep nested notes"
    );
}
