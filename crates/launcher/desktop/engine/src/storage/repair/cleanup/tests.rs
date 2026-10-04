use super::*;
async fn fixture(missing: bool) -> (tempfile::TempDir, DesktopState, Plan) {
    let (root, mut state, plan) =
        recovery::tests::interrupted(commit::Point::Published, missing).await;
    let revision = state.operations().snapshot().revision;
    recovery::reconcile(&mut state, plan.id, revision).unwrap();
    (root, state, plan)
}
#[tokio::test]
async fn removes_only_successful_repairs_backup_and_repeated_cleanup_is_idempotent() {
    for missing in [false, true] {
        let (root, mut state, plan) = fixture(missing).await;
        let preferences = state.preferences().clone();
        let revision = state.operations().snapshot().revision;
        cleanup(&mut state, plan.id, revision, |_| Ok(())).unwrap();
        assert!(!plan.backup().exists());
        assert_eq!(
            std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
            b"later entry"
        );
        assert!(plan.work_directory().join("owner.json").is_file());
        assert_eq!(state.preferences(), &preferences);
        assert_eq!(state.operations().snapshot().revision, revision);
        drop(state);
        let mut state = DesktopState::open(&root.path().join("state")).unwrap();
        cleanup(&mut state, plan.id, revision, |_| Ok(())).unwrap();
        assert_eq!(
            state.installed_content().unwrap().unwrap().intent,
            plan.installation
        );
    }
}
#[tokio::test]
async fn every_cleanup_boundary_survives_reopen_without_touching_active_game() {
    for stop in [
        Point::Planned,
        Point::EntryRemoved,
        Point::EmptyRecorded,
        Point::MarkerRemoved,
        Point::DirectoryRemoved,
        Point::RemovedRecorded,
    ] {
        let (root, mut state, plan) = fixture(false).await;
        let revision = state.operations().snapshot().revision;
        let mut reached = false;
        assert!(cleanup(&mut state, plan.id, revision, |point| {
            if stop == point {
                reached = true;
                Err(StorageError::Io)
            } else {
                Ok(())
            }
        })
        .is_err());
        assert!(reached, "unreached {stop:?}");
        assert_eq!(
            std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
            b"later entry"
        );
        drop(state);
        let mut state = DesktopState::open(&root.path().join("state")).unwrap();
        cleanup(&mut state, plan.id, revision, |_| Ok(())).unwrap();
        assert!(!plan.backup().exists());
        assert_eq!(
            state
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state,
            OperationState::Succeeded
        );
    }
}
#[tokio::test]
async fn uncertain_repair_stale_requests_and_missing_backup_refuse_cleanup() {
    let (_root, mut state, plan) =
        recovery::tests::interrupted(commit::Point::Published, false).await;
    let revision = state.operations().snapshot().revision;
    assert!(cleanup(&mut state, plan.id, revision, |_| Ok(())).is_err());
    assert_eq!(
        std::fs::read(plan.backup().join("later.txt")).unwrap(),
        b"damaged"
    );
    recovery::reconcile(&mut state, plan.id, revision).unwrap();
    let revision = state.operations().snapshot().revision;
    assert!(cleanup(&mut state, plan.id, revision + 1, |_| Ok(())).is_err());
    assert!(cleanup(&mut state, Uuid::new_v4(), revision, |_| Ok(())).is_err());
    std::fs::rename(
        plan.backup(),
        plan.installation.destination.join("externally-moved"),
    )
    .unwrap();
    assert!(cleanup(&mut state, plan.id, revision, |_| Ok(())).is_err());
    assert_eq!(
        std::fs::read(
            plan.installation
                .destination
                .join("externally-moved/later.txt")
        )
        .unwrap(),
        b"damaged"
    );
}
#[tokio::test]
async fn checkpoint_failure_and_foreign_content_after_empty_preserve_backup() {
    let (root, mut state, plan) = fixture(false).await;
    let revision = state.operations().snapshot().revision;
    std::fs::create_dir(root.path().join("state").join(name(plan.id))).unwrap();
    assert!(cleanup(&mut state, plan.id, revision, |_| Ok(())).is_err());
    assert_eq!(
        std::fs::read(plan.backup().join("later.txt")).unwrap(),
        b"damaged"
    );

    let (_root, mut state, plan) = fixture(false).await;
    let revision = state.operations().snapshot().revision;
    assert!(cleanup(&mut state, plan.id, revision, |point| {
        if point == Point::EmptyRecorded {
            Err(StorageError::Io)
        } else {
            Ok(())
        }
    })
    .is_err());
    std::fs::write(plan.backup().join("foreign.txt"), b"preserve").unwrap();
    assert!(cleanup(&mut state, plan.id, revision, |_| Ok(())).is_err());
    assert_eq!(
        std::fs::read(plan.backup().join("foreign.txt")).unwrap(),
        b"preserve"
    );
}
#[cfg(unix)]
#[tokio::test]
async fn nested_backup_link_is_refused_before_any_deletion() {
    let (root, mut state, plan) = fixture(false).await;
    let outside = root.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("keep.txt"), b"outside").unwrap();
    std::os::unix::fs::symlink(&outside, plan.backup().join("linked")).unwrap();
    let revision = state.operations().snapshot().revision;
    assert!(cleanup(&mut state, plan.id, revision, |_| Ok(())).is_err());
    assert_eq!(
        std::fs::read(plan.backup().join("later.txt")).unwrap(),
        b"damaged"
    );
    assert_eq!(std::fs::read(outside.join("keep.txt")).unwrap(), b"outside");
}
