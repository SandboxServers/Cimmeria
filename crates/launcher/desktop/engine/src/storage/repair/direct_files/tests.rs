use super::*;

fn apply(
    plan: &Plan,
    root: &Path,
    stage: &Path,
    backup: &Path,
    work: &Path,
    previous: &[PathBuf],
) -> Result<(), StorageError> {
    let owner = File::open(root.join(".cimmeria-install.json")).map_err(|_| StorageError::Io)?;
    super::apply(plan, root, stage, backup, work, previous, &owner)
}
fn apply_with(
    plan: &Plan,
    root: &Path,
    stage: &Path,
    backup: &Path,
    work: &Path,
    previous: &[PathBuf],
    hook: impl FnMut(BackupBoundary) -> Result<(), StorageError>,
) -> Result<(), StorageError> {
    let owner = File::open(root.join(".cimmeria-install.json")).map_err(|_| StorageError::Io)?;
    apply_with_owner(plan, root, stage, backup, work, previous, &owner, hook)
}

fn fixture() -> (tempfile::TempDir, Plan) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("selected");
    std::fs::create_dir(&root).unwrap();
    let plan = Plan {
        schema_version: 1,
        id: Uuid::new_v4(),
        installation: InstallIntent {
            schema_version: 2,
            operation_id: Uuid::new_v4(),
            preferences_revision: 1,
            destination: root,
            manifest_digest: [7; 32],
            login_servers: vec![],
            backend: ExtractionBackend::Native,
        },
        original_present: true,
        current_release: None,
    };
    std::fs::create_dir_all(plan.stage().join("Working/Binaries")).unwrap();
    std::fs::create_dir_all(plan.installation.content_root().join("Working/Binaries")).unwrap();
    std::fs::write(
        plan.stage().join("Working/Binaries/SGW.exe"),
        b"new executable",
    )
    .unwrap();
    std::fs::write(
        plan.installation
            .content_root()
            .join("Working/Binaries/SGW.exe"),
        b"old executable",
    )
    .unwrap();
    std::fs::write(
        plan.installation
            .content_root()
            .join("Working/Binaries/player-notes.txt"),
        b"keep me",
    )
    .unwrap();
    std::fs::write(
        plan.installation
            .content_root()
            .join(".cimmeria-install.json"),
        b"owner marker",
    )
    .unwrap();
    (temp, plan)
}
#[test]
fn direct_reconstruction_preserves_root_metadata_and_nested_player_files() {
    let (_temp, plan) = fixture();
    let root = plan.installation.content_root();
    // Incremental stages can contain unchanged, unverified light-adoption inputs.
    std::fs::write(
        plan.stage().join("Working/Binaries/player-notes.txt"),
        b"keep me",
    )
    .unwrap();
    apply(
        &plan,
        &root,
        &plan.stage(),
        &plan.backup(),
        &plan.work_directory(),
        &[],
    )
    .unwrap();
    assert_eq!(
        std::fs::read(root.join("Working/Binaries/SGW.exe")).unwrap(),
        b"new executable"
    );
    assert_eq!(
        std::fs::read(root.join("Working/Binaries/player-notes.txt")).unwrap(),
        b"keep me"
    );
    assert_eq!(
        std::fs::read(root.join(".cimmeria-install.json")).unwrap(),
        b"owner marker"
    );
    assert_eq!(
        std::fs::read(plan.backup().join("Working/Binaries/SGW.exe")).unwrap(),
        b"old executable"
    );
    assert!(!root.join("game").exists());
    assert!(!plan.stage().exists());
    verify(&plan, &root, &plan.work_directory()).unwrap();
    assert!(!paths(&plan, &plan.work_directory())
        .unwrap()
        .contains(&PathBuf::from("Working/Binaries/player-notes.txt")));
    // Reopen/retry is idempotent even after the stage was removed.
    apply(
        &plan,
        &root,
        &plan.stage(),
        &plan.backup(),
        &plan.work_directory(),
        &[],
    )
    .unwrap();
}
#[test]
fn direct_reconstruction_recovers_between_backup_and_file_promotion() {
    let (_temp, plan) = fixture();
    let root = plan.installation.content_root();
    assert_eq!(
        apply_with(
            &plan,
            &root,
            &plan.stage(),
            &plan.backup(),
            &plan.work_directory(),
            &[],
            |boundary| {
                if boundary == BackupBoundary::Published {
                    Err(StorageError::Io)
                } else {
                    Ok(())
                }
            }
        ),
        Err(StorageError::Io)
    );
    assert!(!root.join("Working/Binaries/SGW.exe").exists());
    assert_eq!(
        std::fs::read(root.join("Working/Binaries/player-notes.txt")).unwrap(),
        b"keep me"
    );
    apply(
        &plan,
        &root,
        &plan.stage(),
        &plan.backup(),
        &plan.work_directory(),
        &[],
    )
    .unwrap();
    verify(&plan, &root, &plan.work_directory()).unwrap();
    assert_eq!(
        std::fs::read(root.join("Working/Binaries/SGW.exe")).unwrap(),
        b"new executable"
    );
}
#[test]
fn direct_reconstruction_refuses_changed_original_after_checkpoint() {
    let (_temp, plan) = fixture();
    let root = plan.installation.content_root();
    assert!(apply_with(
        &plan,
        &root,
        &plan.stage(),
        &plan.backup(),
        &plan.work_directory(),
        &[],
        |boundary| {
            if boundary == BackupBoundary::Published {
                Err(StorageError::Io)
            } else {
                Ok(())
            }
        }
    )
    .is_err());
    std::fs::write(
        root.join("Working/Binaries/SGW.exe"),
        b"external replacement",
    )
    .unwrap();
    assert!(apply(
        &plan,
        &root,
        &plan.stage(),
        &plan.backup(),
        &plan.work_directory(),
        &[]
    )
    .is_err());
    assert_eq!(
        std::fs::read(root.join("Working/Binaries/SGW.exe")).unwrap(),
        b"external replacement"
    );
}

#[test]
fn direct_removal_recovers_after_backup_without_touching_player_extras() {
    let (_temp, plan) = fixture();
    let root = plan.installation.content_root();
    let removed = PathBuf::from("version-two.txt");
    std::fs::write(root.join(&removed), b"version two").unwrap();
    // Isolate the removal so the fault lands after its backup, before deletion.
    let staged = plan.stage().join("Working/Binaries/SGW.exe");
    std::fs::remove_file(staged).unwrap();
    assert_eq!(
        apply_with(
            &plan,
            &root,
            &plan.stage(),
            &plan.backup(),
            &plan.work_directory(),
            std::slice::from_ref(&removed),
            |boundary| {
                if boundary == BackupBoundary::Published {
                    Err(StorageError::Io)
                } else {
                    Ok(())
                }
            }
        ),
        Err(StorageError::Io)
    );
    assert_eq!(std::fs::read(root.join(&removed)).unwrap(), b"version two");
    assert_eq!(
        std::fs::read(plan.backup().join(&removed)).unwrap(),
        b"version two"
    );
    apply(
        &plan,
        &root,
        &plan.stage(),
        &plan.backup(),
        &plan.work_directory(),
        &[],
    )
    .unwrap();
    apply(
        &plan,
        &root,
        &plan.stage(),
        &plan.backup(),
        &plan.work_directory(),
        &[],
    )
    .unwrap();
    verify(&plan, &root, &plan.work_directory()).unwrap();
    assert!(!root.join(&removed).exists());
    assert!(paths(&plan, &plan.work_directory()).unwrap().is_empty());
    assert_eq!(
        std::fs::read(root.join("Working/Binaries/player-notes.txt")).unwrap(),
        b"keep me"
    );
    // An absent-output receipt also rejects a foreign file recreated afterwards.
    std::fs::write(root.join(&removed), b"player edit").unwrap();
    assert_eq!(
        verify(&plan, &root, &plan.work_directory()),
        Err(StorageError::Corrupt)
    );
    assert!(apply(
        &plan,
        &root,
        &plan.stage(),
        &plan.backup(),
        &plan.work_directory(),
        &[]
    )
    .is_err());
    assert_eq!(std::fs::read(root.join(&removed)).unwrap(), b"player edit");
}

#[test]
fn direct_removal_recovers_empty_and_prefix_partial_copies() {
    for boundary in [BackupBoundary::Created, BackupBoundary::PrefixCopied] {
        let (_temp, plan) = fixture();
        let root = plan.installation.content_root();
        let removed = PathBuf::from("version-two.txt");
        let bytes = vec![0x5a; 131072];
        std::fs::write(root.join(&removed), &bytes).unwrap();
        std::fs::remove_file(plan.stage().join("Working/Binaries/SGW.exe")).unwrap();
        assert_eq!(
            apply_with(
                &plan,
                &root,
                &plan.stage(),
                &plan.backup(),
                &plan.work_directory(),
                std::slice::from_ref(&removed),
                |at| {
                    if at == boundary {
                        Err(StorageError::Io)
                    } else {
                        Ok(())
                    }
                },
            ),
            Err(StorageError::Io)
        );
        let held = plan.backup().join(&removed);
        let partial = removal_partial(&held);
        let final_before_retry = held.exists();
        let partial_length = std::fs::metadata(&partial)
            .ok()
            .map(|metadata| metadata.len());
        assert_eq!(std::fs::read(root.join(&removed)).unwrap(), bytes);
        // Reopen the immutable disk journal, with no in-memory copy progress.
        let reopened: Journal<Plan> = read(&plan.work_directory().join("direct-files.json"))
            .unwrap()
            .unwrap();
        apply(
            &reopened.plan,
            &root,
            &plan.stage(),
            &plan.backup(),
            &plan.work_directory(),
            &[],
        )
        .unwrap();
        verify(&reopened.plan, &root, &plan.work_directory()).unwrap();
        assert!(!final_before_retry);
        assert_eq!(
            partial_length,
            Some(if boundary == BackupBoundary::Created {
                0
            } else {
                65536
            })
        );
        assert_eq!(std::fs::read(&held).unwrap(), bytes);
        assert!(!root.join(&removed).exists());
        assert!(!partial.exists());
        assert_eq!(
            std::fs::read(root.join("Working/Binaries/player-notes.txt")).unwrap(),
            b"keep me"
        );
    }
}

#[test]
fn direct_removal_partial_retry_refuses_changed_live_or_invalid_final_backup() {
    for invalid_final in [false, true] {
        let (_temp, plan) = fixture();
        let root = plan.installation.content_root();
        let removed = PathBuf::from("version-two.txt");
        std::fs::write(root.join(&removed), b"original bytes").unwrap();
        std::fs::remove_file(plan.stage().join("Working/Binaries/SGW.exe")).unwrap();
        assert_eq!(
            apply_with(
                &plan,
                &root,
                &plan.stage(),
                &plan.backup(),
                &plan.work_directory(),
                std::slice::from_ref(&removed),
                |at| {
                    if at == BackupBoundary::PrefixCopied {
                        Err(StorageError::Io)
                    } else {
                        Ok(())
                    }
                },
            ),
            Err(StorageError::Io)
        );
        let held = plan.backup().join(&removed);
        let partial = removal_partial(&held);
        let partial_bytes = std::fs::read(&partial).unwrap();
        if invalid_final {
            std::fs::write(&held, b"invalid backup").unwrap();
        } else {
            std::fs::write(root.join(&removed), b"player edit").unwrap();
        }
        let live_bytes = std::fs::read(root.join(&removed)).unwrap();
        assert_eq!(
            apply(
                &plan,
                &root,
                &plan.stage(),
                &plan.backup(),
                &plan.work_directory(),
                &[]
            ),
            Err(StorageError::Corrupt)
        );
        assert_eq!(std::fs::read(root.join(&removed)).unwrap(), live_bytes);
        assert_eq!(std::fs::read(&partial).unwrap(), partial_bytes);
        if invalid_final {
            assert_eq!(std::fs::read(&held).unwrap(), b"invalid backup");
        }
    }
}

#[test]
fn direct_reconstruction_retains_live_parents_through_backup_and_promotion() {
    let (_temp, plan) = fixture();
    let outside = tempfile::tempdir().unwrap();
    let sentinel = outside.path().join("SGW.exe");
    std::fs::write(&sentinel, b"outside executable").unwrap();
    let before = file_id(&sentinel);
    let root = plan.installation.content_root();
    let parent = root.join("Working/Binaries");
    let displaced = root.join("retained-binaries");
    apply_with(
        &plan,
        &root,
        &plan.stage(),
        &plan.backup(),
        &plan.work_directory(),
        &[],
        |boundary| {
            if boundary == BackupBoundary::Published {
                #[cfg(windows)]
                assert!(std::fs::rename(&parent, &displaced).is_err());
                #[cfg(unix)]
                {
                    std::fs::rename(&parent, &displaced).unwrap();
                    std::os::unix::fs::symlink(outside.path(), &parent).unwrap();
                }
            }
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"outside executable");
    assert_eq!(file_id(&sentinel), before);
    #[cfg(windows)]
    let retained = parent;
    #[cfg(unix)]
    let retained = displaced;
    assert_eq!(
        std::fs::read(retained.join("SGW.exe")).unwrap(),
        b"new executable"
    );
    assert_eq!(
        std::fs::read(retained.join("player-notes.txt")).unwrap(),
        b"keep me"
    );
}

fn file_id(path: &Path) -> (u64, u64) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = std::fs::metadata(path).unwrap();
        (metadata.dev(), metadata.ino())
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
        };
        let file = File::open(path).unwrap();
        // SAFETY: initialized out parameter and an owned live file handle.
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        assert_ne!(
            unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) },
            0
        );
        (
            u64::from(info.dwVolumeSerialNumber),
            (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        )
    }
}

mod alias;
