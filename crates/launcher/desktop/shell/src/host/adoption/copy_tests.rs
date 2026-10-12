//! Verified-copy adoption (AI-12) through the production host where in-place
//! adoption also runs (Windows and Linux): the picked folder is copied into a new
//! folder and is left exactly as it was.
use super::fixture::*;
use super::in_place_tests::{under, unimported_host};
use super::review::SourceKind;
use super::*;
use cimmeria_launcher_engine::adoption::AdoptionMode;
use std::time::Duration;

#[cfg(windows)]
#[test]
fn windows_offers_both_in_place_and_copy() {
    let f = Fixture::new(serde_json::json!({}));
    assert_eq!(
        f.host.adoption_status().unwrap().modes,
        vec![AdoptionMode::InPlace, AdoptionMode::Copy]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_picked_folder_is_copied_into_a_new_folder_and_left_as_it_was() {
    let mut f = Fixture::new(serde_json::json!({}));
    let folder = f.source.game_directory.clone();
    f.host = unimported_host(f.root.path().join("plain-state"));
    let _origin = f.serve().await;
    assert!(f.host.adoption_mode_allowed(AdoptionMode::Copy).is_ok());

    // The release file the player edited is restored first, as in the in-place test.
    std::fs::write(folder.join("later.txt"), b"later entry").unwrap();
    let before = tree(&folder);

    // The player picks the `Working` folder, then the folder the copy goes into.
    f.host
        .begin_folder_adoption(
            folder.join("Working"),
            Some(f.location()),
            AdoptionMode::Copy,
            f.release(),
        )
        .unwrap();
    let review = reviewed(&f.host).await;
    assert!(matches!(review.source_kind, SourceKind::Folder));
    assert!(!review.in_place);
    assert_eq!(review.source, folder);
    assert_eq!(review.destination, f.destination());
    f.host.adoption_command(confirmation(&review)).unwrap();
    let status = until(&f.host, "publication", Duration::from_secs(30), |status| {
        status.activity == Activity::Idle
    })
    .await;
    assert_eq!(status.last_error, None);
    assert_eq!(status.completed.map(|c| c.directory), Some(f.destination()));
    assert_eq!(
        operation(&f.host.adoption_status().unwrap()),
        Some((OperationKind::Adopt, OperationState::Succeeded))
    );
    assert!(under(&f.destination(), "game/Working/Binaries/SGW.exe").is_file());

    // The picked folder is untouched: the same files with the same bytes.
    assert_eq!(tree(&folder), before);
    assert_eq!(
        std::fs::read(folder.join("unknown.dll")).unwrap(),
        b"untrusted plugin"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_copy_of_a_folder_with_a_modified_file_needs_no_replacement_consent() {
    let mut f = Fixture::new(serde_json::json!({}));
    let folder = f.source.game_directory.clone();
    f.host = unimported_host(f.root.path().join("plain-state"));
    let _origin = f.serve().await;

    // The copy leaves the folder alone, so the modified file is kept as it is.
    f.host
        .begin_folder_adoption(
            folder.join("Working"),
            Some(f.location()),
            AdoptionMode::Copy,
            f.release(),
        )
        .unwrap();
    let review = reviewed(&f.host).await;
    assert!(!review.in_place);
    assert_eq!(review.counts.modified, 1);
    assert!(!review.requires_replacement);
}
