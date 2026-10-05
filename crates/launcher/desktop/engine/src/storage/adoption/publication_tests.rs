use super::tests::*;
use super::*;

fn sink() -> crate::install_progress::ProgressSink {
    crate::install_progress::ProgressSink::latest().0
}
fn operation(f: &Fixture) -> OperationState {
    f.state
        .lock()
        .unwrap()
        .operations()
        .snapshot()
        .operation
        .as_ref()
        .unwrap()
        .state
}

#[test]
fn copy_runs_without_the_state_guard_so_status_reads_are_not_blocked() {
    let f = Fixture::new();
    let preview = f.preview().unwrap();
    let handle = preview.report.preview_handle;
    let id = Uuid::new_v4();
    let state = f.state.clone();
    let mut observed = None;
    let (progress, copied) = crate::install_progress::ProgressSink::latest();
    publication::confirm(
        preview,
        id,
        handle,
        choices(),
        CancellationToken::new(),
        &progress,
        |point| {
            if point == publication::Point::Plan {
                // The copy starts here. A status reader must get the guard and
                // see the admitted Running operation, not wait for publication.
                observed = state.try_lock().ok().map(|state| {
                    let op = state.operations().snapshot().operation.clone().unwrap();
                    (op.id, op.kind, op.state)
                });
            }
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(
        observed,
        Some((id, OperationKind::Adopt, OperationState::Running))
    );
    let total = f
        .state
        .lock()
        .unwrap()
        .installed_content()
        .unwrap()
        .map(|_| inventory::scan(&f.destination().join("game"), &CancellationToken::new()).unwrap())
        .unwrap()
        .len();
    assert!(matches!(
        &*copied.borrow(),
        Some(crate::install::Progress::Extracting { current, total: reported, .. })
            if *current == total && *reported == total
    ));
}

#[test]
fn cancellation_during_the_copy_is_terminal_publishes_nothing_and_releases_the_reference() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    let preview = f.preview().unwrap();
    let handle = preview.report.preview_handle;
    let id = Uuid::new_v4();
    let cancel = CancellationToken::new();
    let during = cancel.clone();
    assert_eq!(
        publication::confirm(preview, id, handle, choices(), cancel, &sink(), |point| {
            if point == publication::Point::Plan {
                during.cancel();
            }
            Ok(())
        }),
        Err(Error::Cancelled)
    );
    assert_eq!(operation(&f), OperationState::Cancelled);
    assert!(!f.destination().join("game").exists());
    let mut state = f.state.lock().unwrap();
    assert!(state.installed_content().unwrap().is_none());
    assert_eq!(state.preferences().install_directory, {
        let imported = state.legacy_import().unwrap().unwrap();
        Some(imported.source.game_directory)
    });
    assert!(list_preparations(&state).unwrap().is_empty());
    drop(state);
    assert_eq!(f.source_snapshot(), before);
}
