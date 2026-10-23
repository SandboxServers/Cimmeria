use super::*;

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
            || Err(StorageError::Io)
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
        || Err(StorageError::Io)
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
            || Err(StorageError::Io)
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
