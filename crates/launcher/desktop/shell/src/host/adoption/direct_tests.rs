//! macOS light adoption records Wine identity while keeping the selected root.
//! The pinned helper is an inert fixture: this journey must never execute it.
use super::fixture::*;
use super::*;
use cimmeria_launcher_engine::{
    adoption::AdoptionMode, mac_wine::HelperResource, ExtractionBackend,
};
use sha2::{Digest, Sha256};
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn macos_light_adoption_uses_the_original_folder_and_keeps_wine_identity() {
    let mut f = Fixture::new(serde_json::json!({}));
    let _server = f.serve().await;
    let helper = f.root.path().join("inert-helper.exe");
    std::fs::write(&helper, b"not executable").unwrap();
    let digest: String = Sha256::digest(b"not executable")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let helper = HelperResource::open(helper, &digest).unwrap();
    f.host.adoption_fixture.as_mut().unwrap().helper = Some(helper.clone());
    f.host.helper = Some(helper);
    let root = f.source.game_directory.clone();
    let before = std::fs::metadata(root.join("later.txt")).unwrap();
    f.host
        .begin_folder_adoption(root.clone(), None, AdoptionMode::Light, f.release())
        .unwrap();
    let review = reviewed(&f.host).await;
    assert!(review.in_place && review.light.is_some());
    assert_eq!(review.destination, root);
    f.host.adoption_command(confirmation(&review)).unwrap();
    let status = until(
        &f.host,
        "direct-root publication",
        Duration::from_secs(30),
        |s| s.activity == Activity::Idle,
    )
    .await;
    assert_eq!(status.last_error, None);
    assert!(status.has_valid_install);
    assert!(!root.join("game").exists());
    use std::os::unix::fs::MetadataExt;
    assert_eq!(
        before.ino(),
        std::fs::metadata(root.join("later.txt")).unwrap().ino()
    );
    let store = f.host.store().unwrap();
    let mut state = store.lock().unwrap();
    let installed = state.installed_content().unwrap().unwrap();
    assert_eq!(installed.intent.content_root(), root);
    assert!(matches!(
        installed.intent.backend,
        ExtractionBackend::Wine { .. }
    ));
}
