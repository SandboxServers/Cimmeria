use super::*;
use commit::Point;
#[tokio::test]
async fn interrupted_marker_writes_can_be_cancelled_without_deleting_content() {
    for point in [Point::OriginalMarked, Point::ReplacementMarked] {
        let (root, mut state, plan) = recovery::tests::interrupted(point, false).await;
        let preferences = state.preferences().clone();
        let revision = state.operations().snapshot().revision;
        assert!(abandon(&mut state, plan.id, revision, false).is_err());
        assert!(abandon(&mut state, plan.id, revision + 1, true).is_err());
        assert!(abandon(&mut state, Uuid::new_v4(), revision, true).is_err());
        abandon(&mut state, plan.id, revision, true).unwrap();
        assert_eq!(
            state
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state,
            OperationState::Cancelled
        );
        assert_eq!(
            std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
            b"damaged"
        );
        assert_eq!(
            std::fs::read(plan.stage().join("later.txt")).unwrap(),
            b"later entry"
        );
        assert_eq!(state.preferences(), &preferences);
        assert!(!plan.backup().exists());
        drop(state);
        let mut state = DesktopState::open(&root.path().join("state")).unwrap();
        assert_eq!(
            state
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state,
            OperationState::Cancelled
        );
        let revision = state.operations().snapshot().revision;
        let admitted = state
            .admit_repair(
                Uuid::new_v4(),
                revision,
                plan.installation.operation_id,
                true,
            )
            .unwrap();
        assert!(admitted.dispatch);
        assert_ne!(admitted.plan.work_directory(), plan.work_directory());
        assert!(plan.stage().join("later.txt").is_file());
    }
}
#[tokio::test]
async fn committed_or_ambiguous_work_cannot_be_abandoned() {
    for point in [
        Point::Planned,
        Point::AfterOriginalRename,
        Point::AfterPromotion,
    ] {
        let (_root, mut state, plan) = recovery::tests::interrupted(point, false).await;
        let revision = state.operations().snapshot().revision;
        assert!(abandon(&mut state, plan.id, revision, true).is_err());
        assert_eq!(
            state
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state,
            OperationState::ReconciliationRequired
        );
    }
    let (_root, mut state, plan) = recovery::tests::interrupted(Point::OriginalMarked, false).await;
    std::fs::create_dir(plan.backup()).unwrap();
    let revision = state.operations().snapshot().revision;
    assert!(abandon(&mut state, plan.id, revision, true).is_err());
    assert!(plan.stage().exists());
}
#[tokio::test]
async fn held_owner_and_failed_terminal_write_keep_recovery_gate() {
    let (root, mut state, plan) = recovery::tests::interrupted(Point::OriginalMarked, false).await;
    let revision = state.operations().snapshot().revision;
    let lock = lock_owner(&plan.installation).unwrap();
    assert!(abandon(&mut state, plan.id, revision, true).is_err());
    drop(lock);
    let journal = root.path().join("state/operation.json");
    std::fs::remove_file(&journal).unwrap();
    std::fs::create_dir(journal).unwrap();
    assert!(abandon(&mut state, plan.id, revision, true).is_err());
    assert_ne!(
        state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::Cancelled
    );
    assert!(plan.stage().join("later.txt").is_file());
    assert_eq!(
        std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
        b"damaged"
    );
}

#[tokio::test]
async fn interrupted_admission_and_partial_staging_can_be_abandoned_without_adoption() {
    for partial in [false, true] {
        let (_root, state, installation, server) = install_worker::tests::prepared_fixture().await;
        {
            let mut state = state.lock().unwrap();
            let revision = state.operations().snapshot().revision;
            let plan = state
                .admit_repair(Uuid::new_v4(), revision, installation, true)
                .unwrap()
                .plan;
            if partial {
                std::fs::create_dir_all(plan.stage()).unwrap();
                std::fs::write(
                    plan.work_directory().join("owner.json"),
                    serde_json::to_vec(&plan).unwrap(),
                )
                .unwrap();
                std::fs::write(plan.stage().join("partial.download"), b"not a game").unwrap();
            }
            state
                .operations_mut()
                .unwrap()
                .mark_uncertain(plan.id)
                .unwrap();
            let revision = state.operations().snapshot().revision;
            abandon(&mut state, plan.id, revision, true).unwrap();
            assert_eq!(
                state
                    .operations()
                    .snapshot()
                    .operation
                    .as_ref()
                    .unwrap()
                    .state,
                OperationState::Cancelled
            );
            assert_eq!(
                std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
                b"later entry"
            );
            if partial {
                assert_eq!(
                    std::fs::read(plan.stage().join("partial.download")).unwrap(),
                    b"not a game"
                );
            }
        }
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn orphan_work_and_conflicting_original_presence_refuse_abandonment() {
    for orphan in [false, true] {
        let (_root, state, installation, _server) = install_worker::tests::prepared_fixture().await;
        let mut state = state.lock().unwrap();
        let revision = state.operations().snapshot().revision;
        let plan = state
            .admit_repair(Uuid::new_v4(), revision, installation, true)
            .unwrap()
            .plan;
        if orphan {
            std::fs::create_dir(plan.work_directory()).unwrap();
        } else {
            std::fs::rename(
                plan.installation.destination.join("game"),
                plan.installation.destination.join("externally-moved"),
            )
            .unwrap();
        }
        state
            .operations_mut()
            .unwrap()
            .mark_uncertain(plan.id)
            .unwrap();
        let revision = state.operations().snapshot().revision;
        assert!(abandon(&mut state, plan.id, revision, true).is_err());
        assert_eq!(
            state
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state,
            OperationState::ReconciliationRequired
        );
    }
}
