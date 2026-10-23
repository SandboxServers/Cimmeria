use super::*;
use commit::Point;
use std::time::Duration;
pub(crate) async fn interrupted(
    point: Point,
    missing: bool,
) -> (tempfile::TempDir, DesktopState, Plan) {
    let (root, state, prepared) = commit::tests::fixture(missing).await;
    let plan = prepared.plan.clone();
    let mut reached = false;
    assert_eq!(
        commit::replace(&state, &prepared, |current| {
            if current == point {
                reached = true;
                Err(StorageError::Io)
            } else {
                Ok(())
            }
        }),
        Err(preparation::Failure::ReconciliationRequired)
    );
    assert!(reached);
    drop(prepared);
    tokio::time::timeout(Duration::from_secs(5), async {
        while Arc::strong_count(&state) != 1 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    drop(Arc::try_unwrap(state).ok().unwrap().into_inner().unwrap());
    let state = DesktopState::open(&root.path().join("state")).unwrap();
    (root, state, plan)
}
fn revision(state: &DesktopState) -> u64 {
    state.operations().snapshot().revision
}
#[tokio::test]
async fn reopened_checkpoint_matrix_recovers_exact_trees_without_downloads() {
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    for missing in [false, true] {
        for point in [
            Point::Planned,
            Point::OriginalMoved,
            Point::BeforePromotion,
            Point::AfterPromotion,
            Point::Promoted,
            Point::Published,
        ] {
            let (root, mut state, plan) = interrupted(point, missing).await;
            let preferences = state.preferences().clone();
            let rev = revision(&state);
            let (result, events) =
                crate::test_capture::capture(|| reconcile(&mut state, plan.id, rev));
            result.unwrap();
            let identities: Vec<_> = events
                .iter()
                .filter(|e| e.field("event") == Some("applied_content_identity"))
                .collect();
            assert_eq!(identities.len(), 1, "{point:?}, missing={missing}");
            let digest: String = plan
                .release_identity()
                .manifest_digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            assert_eq!(
                identities[0].field("manifest_sha256"),
                Some(digest.as_str())
            );
            assert_eq!(identities[0].field("phase"), Some("applied"));
            assert_eq!(identities[0].field("overlay_ids"), Some("[]"));
            assert_eq!(identities[0].field("incomplete_overlay_ids"), Some("[]"));
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
            assert_eq!(
                std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
                b"later entry"
            );
            assert_eq!(plan.backup().exists(), !missing);
            if !missing {
                assert_eq!(
                    std::fs::read(plan.backup().join("later.txt")).unwrap(),
                    b"damaged"
                );
            }
            assert_eq!(state.preferences(), &preferences);
            assert_eq!(
                state.installed_content().unwrap().unwrap().intent,
                plan.installation
            );
            assert!(reconcile(&mut state, plan.id, rev).is_err());
            drop(state);
            let state = DesktopState::open(&root.path().join("state")).unwrap();
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
}
#[tokio::test]
async fn first_rename_without_its_checkpoint_is_reconciled() {
    let (_root, mut state, plan) = interrupted(Point::AfterOriginalRename, false).await;
    assert!(!plan.installation.destination.join("game").exists());
    let rev = revision(&state);
    reconcile(&mut state, plan.id, rev).unwrap();
    assert_eq!(
        std::fs::read(plan.backup().join("later.txt")).unwrap(),
        b"damaged"
    );
    assert_eq!(
        std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
        b"later entry"
    );
}
#[tokio::test]
async fn foreign_game_missing_backup_and_stale_requests_never_authorize_mutation() {
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    for foreign in [false, true] {
        let (_root, mut state, plan) = interrupted(Point::AfterOriginalRename, false).await;
        let rev = revision(&state);
        assert!(reconcile(&mut state, plan.id, rev + 1).is_err());
        assert!(reconcile(&mut state, Uuid::new_v4(), rev).is_err());
        if foreign {
            let game = plan.installation.destination.join("game");
            std::fs::create_dir(&game).unwrap();
            std::fs::write(game.join("foreign.txt"), b"keep").unwrap();
        } else {
            // Simulates external loss; recovery must not fabricate the old backup.
            std::fs::remove_dir_all(plan.backup()).unwrap();
        }
        let (result, events) = crate::test_capture::capture(|| reconcile(&mut state, plan.id, rev));
        assert!(result.is_err());
        assert!(!events
            .iter()
            .any(|e| e.field("event") == Some("applied_content_identity")));
        assert!(plan.stage().join("later.txt").is_file());
        if foreign {
            assert_eq!(
                std::fs::read(plan.installation.destination.join("game/foreign.txt")).unwrap(),
                b"keep"
            );
        }
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
#[tokio::test]
async fn mismatched_role_and_legacy_record_remain_gated() {
    for legacy in [false, true] {
        let (root, mut state, plan) = interrupted(Point::AfterOriginalRename, false).await;
        if legacy {
            let path = root
                .path()
                .join(format!("state/repair-commit-{}.json", plan.id));
            let mut record: Record = read(&path).unwrap().unwrap();
            record.schema_version = 1;
            std::fs::write(path, serde_json::to_vec(&record).unwrap()).unwrap();
        } else {
            let name = format!(".cimmeria-repair-tree-{}.json", plan.id);
            std::fs::copy(plan.stage().join(&name), plan.backup().join(name)).unwrap();
        }
        let rev = revision(&state);
        assert!(reconcile(&mut state, plan.id, rev).is_err());
        assert!(!plan.installation.destination.join("game").exists());
        assert_eq!(
            std::fs::read(plan.backup().join("later.txt")).unwrap(),
            b"damaged"
        );
    }
}

#[tokio::test]
async fn interruption_during_recovery_can_be_reopened_and_reconciled_again() {
    for stop in [
        Point::BeforeOriginalRename,
        Point::AfterOriginalRename,
        Point::OriginalMoved,
        Point::BeforePromotion,
        Point::AfterPromotion,
        Point::Promoted,
        Point::Published,
    ] {
        let (root, mut state, plan) = interrupted(Point::Planned, false).await;
        let rev = revision(&state);
        let mut reached = false;
        assert!(reconcile_with(&mut state, plan.id, rev, |point| {
            if point == stop {
                reached = true;
                Err(StorageError::Io)
            } else {
                Ok(())
            }
        })
        .is_err());
        assert!(reached, "unreached recovery fault {stop:?}");
        drop(state);
        let mut state = DesktopState::open(&root.path().join("state")).unwrap();
        let rev = revision(&state);
        reconcile(&mut state, plan.id, rev).unwrap();
        assert_eq!(
            std::fs::read(plan.backup().join("later.txt")).unwrap(),
            b"damaged"
        );
        assert_eq!(
            std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
            b"later entry"
        );
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
async fn interruption_while_marking_without_commit_record_remains_gated() {
    for point in [Point::OriginalMarked, Point::ReplacementMarked] {
        let (_root, mut state, plan) = interrupted(point, false).await;
        let rev = revision(&state);
        assert!(reconcile(&mut state, plan.id, rev).is_err());
        assert!(!plan.backup().exists());
        assert!(plan.stage().join("later.txt").is_file());
        assert_eq!(
            std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
            b"damaged"
        );
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
