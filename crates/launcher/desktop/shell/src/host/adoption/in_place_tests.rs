//! In-place adoption (D-LX12) through the production host, where it is the
//! adoption backend: no destination is chosen and nothing is copied.
use super::fixture::*;
use super::*;
use std::{path::Path, time::Duration};

/// Index names use `/`; a verbatim folder takes separators literally.
fn under(root: &Path, name: &str) -> PathBuf {
    name.split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_imported_folder_is_adopted_in_place_once_its_release_files_verify() {
    let mut f = Fixture::new(serde_json::json!({}));
    let _origin = f.serve().await;
    let folder = f.source.game_directory.clone();
    assert_eq!(f.host.adoption_status().unwrap().backend, Backend::InPlace);
    assert!(f.host.adoption_choice_allowed().is_ok());

    // The fixture's player edited one release file: the review names it and
    // confirmation is refused with the folder untouched.
    f.host.begin_adoption_in_place(f.release()).unwrap();
    let review = reviewed(&f.host).await;
    assert!(review.in_place);
    assert_eq!((&review.source, &review.destination), (&folder, &folder));
    assert_eq!(review.blocked_files, vec!["later.txt".to_string()]);
    assert_eq!(
        f.host.adoption_command(confirmation(&review)).unwrap_err(),
        AdoptionError::VerificationFailed
    );
    assert!(!folder.join(".cimmeria-install.json").exists());
    assert!(!folder.join("game").exists());

    // Restored to the signed bytes, the same folder verifies and is adopted.
    f.host.adoption_command(DISMISS).unwrap();
    std::fs::write(folder.join("later.txt"), b"later entry").unwrap();
    f.host.begin_adoption_in_place(f.release()).unwrap();
    let review = reviewed(&f.host).await;
    assert!(review.blocked_files.is_empty());
    f.host.adoption_command(confirmation(&review)).unwrap();
    let status = until(&f.host, "publication", Duration::from_secs(30), |status| {
        status.activity == Activity::Idle
    })
    .await;
    assert_eq!(status.last_error, None);
    assert!(status.owned);
    assert_eq!(status.completed.map(|c| c.directory), Some(folder.clone()));
    assert_eq!(
        operation(&f.host.adoption_status().unwrap()),
        Some((OperationKind::Adopt, OperationState::Succeeded))
    );
    assert!(under(&folder, "game/Working/Binaries/SGW.exe").is_file());
    assert!(!under(&folder, "Working/Binaries/SGW.exe").exists());
    // Not part of the release: it stays exactly where the player left it.
    assert_eq!(
        std::fs::read(folder.join("unknown.dll")).unwrap(),
        b"untrusted plugin"
    );
    assert_eq!(
        f.host
            .install_status()
            .unwrap()
            .uninstall
            .map(|target| target.directory),
        Some(folder)
    );
}
