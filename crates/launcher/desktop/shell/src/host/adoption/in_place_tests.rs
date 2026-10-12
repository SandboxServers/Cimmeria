//! In-place adoption (D-LX12) through the production host, where it is the
//! adoption backend: no destination is chosen and nothing is copied.
use super::fixture::*;
use super::review::SourceKind;
use super::*;
use cimmeria_launcher_engine::adoption::AdoptionMode;
use std::{path::Path, time::Duration};

/// Index names use `/`; a verbatim folder takes separators literally.
fn under(root: &Path, name: &str) -> PathBuf {
    name.split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

/// The backup folders an adoption left in the folder's root.
fn backups(folder: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name().is_some_and(|name| {
                name.to_string_lossy()
                    .starts_with("cimmeria-adoption-backup-")
            })
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_imported_folder_is_adopted_in_place_once_its_release_files_verify() {
    let mut f = Fixture::new(serde_json::json!({}));
    let _origin = f.serve().await;
    let folder = f.source.game_directory.clone();
    assert_eq!(f.host.adoption_status().unwrap().backend, Backend::InPlace);
    assert!(f.host.adoption_choice_allowed().is_ok());

    // The fixture's player edited one release file. The launcher can replace it,
    // so it is shown as modified and needs consent rather than blocking.
    f.host.begin_adoption_in_place(f.release()).unwrap();
    let review = reviewed(&f.host).await;
    assert!(review.in_place);
    assert_eq!((&review.source, &review.destination), (&folder, &folder));
    assert!(review.blocked_files.is_empty());
    assert!(review.requires_replacement);
    assert_eq!(review.counts.modified, 1);
    // Confirming without consent to replace it is refused with nothing moved.
    assert_eq!(
        f.host.adoption_command(confirmation(&review)).unwrap_err(),
        AdoptionError::ConsentRequired
    );
    assert_eq!(
        std::fs::read(folder.join("later.txt")).unwrap(),
        b"edited by the user"
    );
    assert!(!folder.join(".cimmeria-install.json").exists());
    assert!(!folder.join("game").exists());
    assert!(backups(&folder).is_empty());

    // Consent to replacing the modified file: the same folder is adopted and the
    // player's copy is kept in the backup folder.
    f.host.adoption_command(DISMISS).unwrap();
    f.host.begin_adoption_in_place(f.release()).unwrap();
    let review = reviewed(&f.host).await;
    assert!(review.blocked_files.is_empty());
    assert!(review.requires_replacement);
    let mut command = confirmation(&review);
    if let AdoptionCommand::Confirm {
        replace_modified_files,
        ..
    } = &mut command
    {
        *replace_modified_files = true;
    }
    f.host.adoption_command(command).unwrap();
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
    // The original, modified later.txt is preserved in the backup folder.
    let backup = backups(&folder);
    assert_eq!(backup.len(), 1);
    assert_eq!(
        std::fs::read(backup[0].join("later.txt")).unwrap(),
        b"edited by the user"
    );
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

/// A host whose state holds no saved import, so a picked folder is the source.
fn unimported_host(state_root: PathBuf) -> NativeHost {
    drop(DesktopState::open(&state_root).unwrap());
    NativeHost::new(state_root)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_picked_folder_without_an_import_is_reviewed_and_adopted_in_place() {
    let mut f = Fixture::new(serde_json::json!({}));
    let folder = f.source.game_directory.clone();
    f.host = unimported_host(f.root.path().join("plain-state"));
    let _origin = f.serve().await;
    assert!(f.host.adoption_choice_allowed().is_ok());
    assert!(f.host.adoption_status().unwrap().imported.is_none());

    // The player picks the `Working` folder inside the game root; the root is
    // what gets adopted. The release file they edited is restored first.
    std::fs::write(folder.join("later.txt"), b"later entry").unwrap();
    f.host
        .begin_folder_adoption(folder.join("Working"), AdoptionMode::InPlace, f.release())
        .unwrap();
    let review = reviewed(&f.host).await;
    assert!(matches!(review.source_kind, SourceKind::Folder));
    assert!(review.in_place);
    assert_eq!((&review.source, &review.destination), (&folder, &folder));
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
    // Not part of the release: it stays exactly where the player left it.
    assert_eq!(
        std::fs::read(folder.join("unknown.dll")).unwrap(),
        b"untrusted plugin"
    );
}
