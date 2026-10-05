use super::tests::*;
use super::*;

fn sink() -> crate::install_progress::ProgressSink {
    crate::install_progress::ProgressSink::latest().0
}

#[test]
fn every_durable_publication_fault_reopens_without_redispatch_and_preserves_source() {
    for point in [
        publication::Point::Staged,
        publication::Point::BeforePromotion,
        publication::Point::AfterPromotion,
        publication::Point::Receipt,
        publication::Point::Preferences,
        publication::Point::Published,
    ] {
        let f = Fixture::new();
        let before = f.source_snapshot();
        let preview = f.preview().unwrap();
        let handle = preview.report.preview_handle;
        let id = Uuid::new_v4();
        assert_eq!(
            publication::confirm(
                preview,
                id,
                handle,
                choices(),
                CancellationToken::new(),
                &sink(),
                |p| if p == point {
                    Err(StorageError::PersistenceUncertain.into())
                } else {
                    Ok(())
                }
            ),
            Err(Error::Storage(StorageError::PersistenceUncertain))
        );
        assert_eq!(f.source_snapshot(), before);
        let root = f.root.path().join("state");
        drop(f.state);
        let mut reopened = DesktopState::open(&root).unwrap();
        assert!(
            reopened.installed_content().is_err()
                || reopened.installed_content().unwrap().is_none()
        );
        let revision = reopened.operations().snapshot().revision;
        recover(&mut reopened, id, revision).unwrap();
        assert_eq!(
            reopened
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state,
            OperationState::Succeeded
        );
        assert!(reopened.installed_content().unwrap().is_some());
        let revision = reopened.operations().snapshot().revision;
        recover(&mut reopened, id, revision).unwrap();
        assert_eq!(reopened.operations().snapshot().revision, revision);
    }
}
#[test]
fn missing_checkpoint_never_authorizes_recovery_or_automatic_copy_retry() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    let preview = f.preview().unwrap();
    let handle = preview.report.preview_handle;
    let id = Uuid::new_v4();
    assert!(publication::confirm(
        preview,
        id,
        handle,
        choices(),
        CancellationToken::new(),
        &sink(),
        |point| if point == publication::Point::Plan {
            Err(StorageError::Io.into())
        } else {
            Ok(())
        }
    )
    .is_err());
    let mut state = f.state.lock().unwrap();
    let revision = state.operations().snapshot().revision;
    assert!(recover(&mut state, id, revision).is_err());
    assert!(!f.destination().join("game").exists());
    abandon(&mut state, id, revision).unwrap();
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
    assert_eq!(f.source_snapshot(), before);
    assert!(f.destination().exists());
}
#[test]
fn recovery_refuses_foreign_game_tree_and_stale_preferences() {
    for change_preferences in [false, true] {
        let f = Fixture::new();
        let preview = f.preview().unwrap();
        let handle = preview.report.preview_handle;
        let id = Uuid::new_v4();
        assert!(publication::confirm(
            preview,
            id,
            handle,
            choices(),
            CancellationToken::new(),
            &sink(),
            |point| if point == publication::Point::Staged {
                Err(StorageError::Io.into())
            } else {
                Ok(())
            }
        )
        .is_err());
        let mut state = f.state.lock().unwrap();
        if change_preferences {
            state.preferences.revision += 1;
            state.preferences.launcher_summary_consent = true;
        } else {
            std::fs::create_dir(f.destination().join("game")).unwrap();
            std::fs::write(f.destination().join("game/foreign"), b"preserve").unwrap();
        }
        let revision = state.operations().snapshot().revision;
        assert!(recover(&mut state, id, revision).is_err());
        if !change_preferences {
            assert_eq!(
                std::fs::read(f.destination().join("game/foreign")).unwrap(),
                b"preserve"
            );
        }
    }
}
