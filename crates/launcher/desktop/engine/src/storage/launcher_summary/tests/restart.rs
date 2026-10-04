//! What a later process makes of the queue file an earlier one left behind.
use super::*;
use install_worker::Outcome;

fn state_dir(root: &tempfile::TempDir) -> std::path::PathBuf {
    root.path().join("state")
}

#[test]
fn tracking_is_on_disk_from_the_admission_commit() {
    let (_root, mut state, _clock) = opted_in();
    let id = admit_install(&mut state);
    let on_disk = stored(&state);
    let tracking = on_disk.tracking.expect("tracking persisted at admission");
    assert_eq!(tracking.local_operation_id, id);
    assert_eq!(tracking.attempt_id, minted(1));
    assert_eq!(tracking.kind, SummaryOperation::Install);
    assert_eq!(on_disk.entries, []);
}

#[test]
fn a_terminal_reached_before_the_restart_becomes_one_row_without_timings() {
    let (root, mut state, clock) = opted_in();
    let id = admit_install(&mut state);
    clock.advance_ms(50);
    observe(&mut state, id, OperationState::Running);
    finish_install(&mut state, id, Outcome::ContentInvalid);
    // The process ends before anything finalized the committed terminal.
    assert_eq!(stored(&state).entries, []);
    drop(state);

    let (mut state, _clock) = reopened(&state_dir(&root));
    let rows = queued(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].attempt_id, minted(1), "the admission's attempt id");
    assert_eq!(rows[0].event_id, minted(101));
    assert_eq!(rows[0].operation, SummaryOperation::Install);
    assert_eq!(rows[0].outcome, SummaryOutcome::Failed);
    assert_eq!(rows[0].error_code, Some(SummaryErrorCode::ContentInvalid));
    assert_eq!(rows[0].phase, SummaryPhase::None);
    assert_eq!((rows[0].duration_ms, rows[0].phases.clone()), (None, None));
    assert_eq!(stored(&state).tracking, None);
    // Configuring again, finalizing and a further restart add nothing.
    state.configure_summaries(config(Some(ENDPOINT)));
    state.finalize_summaries();
    drop(state);
    let (state, _clock) = reopened(&state_dir(&root));
    assert_eq!(queued(&state), rows);
}

#[test]
fn an_attempt_interrupted_by_the_restart_becomes_one_unknown_row() {
    let (root, mut state, _clock) = opted_in();
    let id = admit_install(&mut state);
    observe(&mut state, id, OperationState::Running);
    drop(state);

    let (mut state, _clock) = reopened(&state_dir(&root));
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
    let rows = queued(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].attempt_id, minted(1));
    assert_eq!(rows[0].outcome, SummaryOutcome::Unknown);
    assert_eq!(rows[0].error_code, None);
    assert_eq!((rows[0].duration_ms, rows[0].phases.clone()), (None, None));
    // The later reconciled terminal belongs to the attempt already reported.
    state
        .operations_mut()
        .unwrap()
        .reconcile(id, OperationState::Failed)
        .unwrap();
    state.finalize_summaries();
    assert_eq!(queued(&state), rows);
    assert_eq!(stored(&state).tracking, None);
}

#[test]
fn queued_entries_survive_a_reopen_unchanged() {
    let (root, mut state, _clock) = opted_in();
    for kind in [OperationKind::Repair, OperationKind::Launch] {
        let id = begin(&mut state, kind);
        observe(&mut state, id, OperationState::Failed);
    }
    state.summary_pre_admission_failure(
        None,
        SummaryOperation::Install,
        SummaryPhase::CatalogFetch,
        SummaryErrorCode::ManifestUnavailable,
    );
    let before = stored(&state);
    assert_eq!(before.entries.len(), 3);
    drop(state);
    let (state, _clock) = reopened(&state_dir(&root));
    assert_eq!(stored(&state), before);
    assert_eq!(
        queued(&state),
        before
            .entries
            .iter()
            .map(|entry| entry.summary.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn tracking_for_an_operation_the_journal_no_longer_shows_is_dropped_silently() {
    // Positive control: with the journal untouched in between, the same
    // leftover tracking yields its `unknown` row.
    for replaced in [false, true] {
        let (root, mut state, _clock) = opted_in();
        let first = admit_install(&mut state);
        drop(state);
        if replaced {
            // A process that never configured summaries moved the journal on.
            let mut state = DesktopState::open(&state_dir(&root)).unwrap();
            state
                .operations_mut()
                .unwrap()
                .reconcile(first, OperationState::Failed)
                .unwrap();
            let next = begin(&mut state, OperationKind::Repair);
            observe(&mut state, next, OperationState::Succeeded);
        }
        let (state, _clock) = reopened(&state_dir(&root));
        assert_eq!(queued(&state).len(), usize::from(!replaced));
        assert_eq!(stored(&state).tracking, None);
    }
}

#[test]
fn consent_found_off_at_startup_scrubs_the_queue_file() {
    // Positive control: with consent still on, the rows are kept.
    for consent_on_disk in [true, false] {
        let (root, mut state, _clock) = opted_in();
        let id = begin(&mut state, OperationKind::Repair);
        observe(&mut state, id, OperationState::Failed);
        let _tracked = admit_install(&mut state);
        let mut preferences = state.preferences().clone();
        assert_eq!(stored(&state).entries.len(), 1);
        drop(state);
        if !consent_on_disk {
            // A preferences file from before the opt-in, restored by hand.
            preferences.launcher_summary_consent = false;
            crate::storage::atomic::write(&state_dir(&root), "preferences.json", &preferences)
                .unwrap();
        }
        let (mut state, _clock) = reopened(&state_dir(&root));
        if consent_on_disk {
            assert_eq!(queued(&state).len(), 2, "the row and the lost attempt");
            continue;
        }
        assert_eq!(queue_bytes(&state), None, "scrubbed");
        assert_eq!(queued(&state), []);
        // Opting in afterwards starts empty: nothing is resurrected.
        set_consent(&mut state, true);
        assert_eq!(queued(&state), []);
        let on_disk = stored(&state);
        assert_eq!((on_disk.entries, on_disk.tracking), (vec![], None));
    }
}

#[test]
fn a_state_that_must_be_reopened_leaves_the_attempt_for_the_next_process() {
    // Positive control: without the uncertainty the row is finalized in place.
    for uncertain in [false, true] {
        let (root, mut state, clock) = opted_in();
        let id = admit_install(&mut state);
        clock.advance_ms(9);
        finish_install(&mut state, id, Outcome::InstallFailed);
        // A later state write whose durability is unknown, as any module sets it.
        state.preferences_uncertain = uncertain;
        state.finalize_summaries();
        assert_eq!(queued(&state).len(), usize::from(!uncertain));
        assert_eq!(stored(&state).tracking.is_some(), uncertain);
        assert_eq!(stored(&state).entries.len(), usize::from(!uncertain));
        drop(state);
        let (state, _clock) = reopened(&state_dir(&root));
        let rows = queued(&state);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].error_code, Some(SummaryErrorCode::InstallFailed));
        assert_eq!(rows[0].attempt_id, minted(1));
        // Timed by the process that finalized it, or not at all.
        assert_eq!(rows[0].duration_ms.is_some(), !uncertain);
        assert_eq!(stored(&state).tracking, None);
    }
}

// A row whose queue write failed exists only in memory, while the file still
// holds the admission's tracking. The write is tried again at the next entry
// point; until it succeeds, a restart reports the attempt a second time.
#[test]
fn a_failed_queue_write_at_finalize_is_retried_at_the_next_entry_point() {
    for retried in [true, false] {
        let (root, mut state, _clock) = opted_in();
        let id = admit_install(&mut state);
        observe(&mut state, id, OperationState::Running);
        finish_install(&mut state, id, Outcome::InstallFailed);
        let at_admission = stored(&state);
        assert!(at_admission.tracking.is_some());

        lock(&state.summaries).store_fault = true;
        state.finalize_summaries();
        let rows = queued(&state);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].event_id, minted(2));
        assert_eq!(stored(&state), at_admission, "the write failed");
        assert_eq!(state.summary_faults().queue_writes, 1);
        if retried {
            // The disk works again, and any entry point comes along.
            lock(&state.summaries).store_fault = false;
            state.finalize_summaries();
            let on_disk = stored(&state);
            assert_eq!(on_disk.tracking, None);
            assert_eq!(on_disk.entries.len(), 1);
            assert_eq!(on_disk.entries[0].summary, rows[0]);
            // Nothing is left to retry: the next entry point writes nothing.
            std::fs::remove_file(state_dir(&root).join(FILE)).unwrap();
            state.finalize_summaries();
            assert_eq!(queue_bytes(&state), None);
            on_disk.store(&state_dir(&root)).unwrap();
        } else {
            // Every later write fails too, until the process ends.
            state.finalize_summaries();
            assert_eq!(stored(&state), at_admission);
            assert_eq!(state.summary_faults().queue_writes, 2, "it was retried");
        }
        drop(state);

        let (state, _clock) = reopened(&state_dir(&root));
        let after = queued(&state);
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].attempt_id, rows[0].attempt_id);
        // With the retry the row keeps its identity. Without a successful write
        // the next process can only build the row again, under a new event id.
        assert_eq!(after[0] == rows[0], retried);
        assert_eq!(after[0].event_id == rows[0].event_id, retried);
    }
}
