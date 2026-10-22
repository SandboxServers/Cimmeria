//! In-place adoption (D-LX12) through the production host, where it is the
//! adoption backend: no destination is chosen and nothing is copied.
use super::fixture::*;
use super::review::SourceKind;
use super::*;
use cimmeria_launcher_engine::adoption::AdoptionMode;
use std::{path::Path, time::Duration};

/// Index names use `/`; a verbatim folder takes separators literally.
pub(super) fn under(root: &Path, name: &str) -> PathBuf {
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
    assert!(serde_json::to_value(&review)
        .unwrap()
        .get("light")
        .is_none());
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hard_linked_file_is_named_and_dismissed() {
    let mut f = Fixture::new(serde_json::json!({}));
    let _origin = f.serve().await;
    let folder = f.source.game_directory.clone();

    // A second name for the player's edited file makes the folder unsafe to adopt.
    std::fs::hard_link(folder.join("later.txt"), folder.join("later-link.txt")).unwrap();
    f.host.begin_adoption_in_place(f.release()).unwrap();
    let status = until(
        &f.host,
        "the unsafe refusal",
        Duration::from_secs(30),
        |s| s.activity == Activity::Idle && s.last_error.is_some(),
    )
    .await;
    assert_eq!(status.last_error, Some(AdoptionError::UnsafeFile));
    assert_eq!(
        status.unsafe_entries,
        Some(adoption::UnsafeReport {
            entries: vec![
                adoption::UnsafeEntry {
                    reason: adoption::UnsafeReason::HardLink,
                    path: Some("later-link.txt".into()),
                },
                adoption::UnsafeEntry {
                    reason: adoption::UnsafeReason::HardLink,
                    path: Some("later.txt".into()),
                },
            ],
            total: 2,
        })
    );
    // The refusal changes nothing in the folder.
    assert_eq!(
        std::fs::read(folder.join("later.txt")).unwrap(),
        b"edited by the user"
    );
    assert!(!folder.join(".cimmeria-install.json").exists());
    assert!(!folder.join("game").exists());

    // Removing the second name and dismissing clears the refusal and its entries.
    std::fs::remove_file(folder.join("later-link.txt")).unwrap();
    f.host.adoption_command(DISMISS).unwrap();
    let status = f.host.adoption_status().unwrap();
    assert_eq!(status.last_error, None);
    assert_eq!(status.unsafe_entries, None);

    // The folder is reviewed again once it is safe.
    f.host.begin_adoption_in_place(f.release()).unwrap();
    assert!(reviewed(&f.host).await.in_place);
}

#[test]
fn unsafe_entries_serialize_as_the_contract_says() {
    let report = adoption::UnsafeReport {
        entries: vec![
            adoption::UnsafeEntry {
                reason: adoption::UnsafeReason::HardLink,
                path: Some("later.txt".into()),
            },
            adoption::UnsafeEntry {
                reason: adoption::UnsafeReason::Link,
                path: None,
            },
        ],
        total: 2,
    };
    assert_eq!(
        serde_json::to_value(report).unwrap(),
        serde_json::json!({
            "entries": [
                {"reason": "hard_link", "path": "later.txt"},
                {"reason": "link", "path": null}
            ],
            "total": 2
        })
    );
}

/// A host whose state holds no saved import, so a picked folder is the source.
pub(super) fn unimported_host(state_root: PathBuf) -> NativeHost {
    drop(DesktopState::open(&state_root).unwrap());
    NativeHost::new(state_root)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_picked_folder_without_an_import_is_reviewed_and_adopted_light() {
    let mut f = Fixture::new(serde_json::json!({}));
    let folder = f.source.game_directory.clone();
    f.host = unimported_host(f.root.path().join("plain-state"));
    let _origin = f.serve().await;
    assert!(f.host.adoption_choice_allowed().is_ok());
    assert!(f.host.adoption_status().unwrap().imported.is_none());

    // The player picks the `Working` folder inside the game root; the root is
    // what gets adopted. The release file they edited is restored first.
    std::fs::write(folder.join("later.txt"), b"later entry").unwrap();
    // The shell no longer offers in-place adoption, so the refusal is checked here.
    assert!(matches!(
        f.host.begin_folder_adoption(
            folder.join("Working"),
            None,
            AdoptionMode::InPlace,
            f.release(),
        ),
        Err(AdoptionError::UnsupportedMode)
    ));
    assert!(!folder.join("game").exists());
    f.host
        .begin_folder_adoption(
            folder.join("Working"),
            None,
            AdoptionMode::Light,
            f.release(),
        )
        .unwrap();
    let review = reviewed(&f.host).await;
    assert!(review.light.is_some(), "a light review carries its summary");
    let json = serde_json::to_value(&review).unwrap();
    assert!(json["light"]["patches_total"].is_u64());
    assert!(json["light"]["index_state"].is_string());
    assert!(json["light"]["skipped"].is_array());
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
    // Light carries every folder file into `game/`, the release's and the player's
    // own; none is lost.
    assert!(!folder.join("unknown.dll").exists());
    assert_eq!(
        std::fs::read(under(&folder, "game/unknown.dll")).unwrap(),
        b"untrusted plugin"
    );
    assert!(under(&folder, "game/Working/Binaries/SGW.exe").is_file());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adoption_steps_are_logged_for_a_refusal_and_an_adoption() {
    use tracing_subscriber::layer::SubscriberExt;
    let (tx, rx) = std::sync::mpsc::sync_channel(256);
    let _guard = tracing::subscriber::set_default(
        tracing_subscriber::registry().with(crate::telemetry::ForwardLayer::new(tx)),
    );
    let mut f = Fixture::new(serde_json::json!({}));
    let _origin = f.serve().await;
    let folder = f.source.game_directory.clone();

    // The refusal is logged once the preview has failed on the hard link.
    std::fs::hard_link(folder.join("later.txt"), folder.join("later-link.txt")).unwrap();
    f.host.begin_adoption_in_place(f.release()).unwrap();
    until(
        &f.host,
        "the unsafe refusal",
        Duration::from_secs(30),
        |s| s.activity == Activity::Idle && s.last_error.is_some(),
    )
    .await;

    // Once the link is gone, the review is shown and then confirmed and adopted.
    std::fs::remove_file(folder.join("later-link.txt")).unwrap();
    f.host.adoption_command(DISMISS).unwrap();
    f.host.begin_adoption_in_place(f.release()).unwrap();
    let review = reviewed(&f.host).await;
    // The player's edited file is replaced with consent, as in the test above.
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

    // Only the adoption target: the setup-area events share the channel.
    let records: Vec<_> = rx
        .try_iter()
        .filter(|record| record.target == "client.launcher.adoption")
        .collect();
    let steps: Vec<&str> = records
        .iter()
        .map(|record| record.fields["step"].as_str().unwrap())
        .collect();
    assert_eq!(steps, ["refused", "review_ready", "confirmed", "finished"]);
    assert_eq!(records[0].fields["error"], "unsafe_file");
    assert_eq!(records[0].fields["unsafe_total"], 2);
    assert_eq!(records[3].fields["decision_outcome"], "adopted");
    assert_eq!(records[3].fields["mode"], "in_place");
}
