//! Incremental stage aliases must not weaken direct-root link safety.
use super::*;

#[test]
fn direct_reconstruction_discards_unchanged_stage_alias_before_live_validation() {
    let (_temp, plan) = fixture();
    let root = plan.installation.content_root();
    let relative = Path::new("Working/Binaries/SGW.exe");
    let live = root.join(relative);
    let before = file_id(&live);
    std::fs::remove_file(plan.stage().join(relative)).unwrap();
    std::fs::hard_link(&live, plan.stage().join(relative)).unwrap();
    apply(
        &plan,
        &root,
        &plan.stage(),
        &plan.backup(),
        &plan.work_directory(),
        &[],
    )
    .unwrap();
    assert_eq!(std::fs::read(&live).unwrap(), b"old executable");
    assert_eq!(file_id(&live), before);
    assert!(!plan.backup().join(relative).exists());
    assert!(!plan.stage().exists());
    assert!(!paths(&plan, &plan.work_directory())
        .unwrap()
        .contains(&relative.to_path_buf()));
}

#[test]
fn direct_reconstruction_does_not_relax_live_hardlink_refusal() {
    let (_temp, plan) = fixture();
    let root = plan.installation.content_root();
    let relative = Path::new("Working/Binaries/SGW.exe");
    let live = root.join(relative);
    let outside = tempfile::tempdir().unwrap();
    let alias = outside.path().join("SGW.exe");
    std::fs::hard_link(&live, &alias).unwrap();
    let before = file_id(&alias);
    std::fs::write(plan.stage().join(relative), b"old executable").unwrap();
    assert_eq!(
        apply(
            &plan,
            &root,
            &plan.stage(),
            &plan.backup(),
            &plan.work_directory(),
            &[]
        ),
        Err(StorageError::UnsafeFile)
    );
    assert_eq!(std::fs::read(&alias).unwrap(), b"old executable");
    assert_eq!(file_id(&alias), before);
    assert_eq!(file_id(&live), before);
}

#[test]
fn direct_reconstruction_refuses_a_substituted_root_before_creating_work() {
    let (_temp, plan) = fixture();
    let root = plan.installation.content_root();
    let owner = File::open(root.join(".cimmeria-install.json")).unwrap();
    let displaced = root.with_file_name("original-root");
    #[cfg(windows)]
    {
        let _retained_owner = &owner;
        assert!(std::fs::rename(&root, &displaced).is_err());
    }
    #[cfg(unix)]
    {
        std::fs::rename(&root, &displaced).unwrap();
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join(".cimmeria-install.json"), b"owner marker").unwrap();
        let before = file_id(&root.join(".cimmeria-install.json"));
        assert_eq!(
            super::super::apply(
                &plan,
                &root,
                &plan.stage(),
                &plan.backup(),
                &plan.work_directory(),
                &[],
                &owner
            ),
            Err(StorageError::UnsafeFile)
        );
        assert_eq!(file_id(&root.join(".cimmeria-install.json")), before);
        assert!(!plan.work_directory().exists());
        assert!(!plan.backup().exists());
        assert_eq!(
            std::fs::read(displaced.join("Working/Binaries/SGW.exe")).unwrap(),
            b"old executable"
        );
    }
}
