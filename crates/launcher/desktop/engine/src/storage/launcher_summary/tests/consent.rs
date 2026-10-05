//! Consent transitions, driven through the preferences write seam and the
//! atomic writer's fault checkpoints.
use super::*;
use crate::storage::{
    atomic::{self, Checkpoint},
    Preferences, StorageError,
};

/// A preferences save whose write fails at `fault`, if any.
fn save(
    state: &mut DesktopState,
    consent: bool,
    fault: Option<Checkpoint>,
) -> Result<Preferences, StorageError> {
    let preferences = state.preferences().clone();
    state.save_preferences_with(
        preferences.install_directory,
        consent,
        preferences.revision,
        |root, next| {
            atomic::write_with(root, "preferences.json", next, |stage| {
                if Some(stage) == fault {
                    Err(std::io::Error::other("injected write fault"))
                } else {
                    Ok(())
                }
            })
        },
    )
}

/// One finalized row and one attempt still in flight.
fn row_and_live_attempt(state: &mut DesktopState) -> Uuid {
    let id = begin(state, OperationKind::Repair);
    observe(state, id, OperationState::Failed);
    let live = begin(state, OperationKind::Launch);
    assert_eq!(queued(state).len(), 1);
    assert!(stored(state).tracking.is_some());
    live
}

fn failed_attempt(state: &mut DesktopState) {
    let id = begin(state, OperationKind::Repair);
    observe(state, id, OperationState::Failed);
    state.finalize_summaries();
}

#[test]
fn opt_out_purges_the_queue_bumps_the_generation_and_cancels_the_upload() {
    let (_root, mut state, _clock) = opted_in();
    let live = row_and_live_attempt(&mut state);
    let before = stored(&state).generation;
    let batch::Take::Ready(in_flight) = state.summary_take_batch() else {
        panic!("a row is waiting");
    };
    assert!(!in_flight.cancel.is_cancelled());

    set_consent(&mut state, false);
    assert!(in_flight.cancel.is_cancelled());
    let on_disk = stored(&state);
    assert_eq!(on_disk, Queue::empty(before + 1));
    assert_eq!(queued(&state), []);
    assert_eq!(gate(&mut state), Gate::NoConsent);
    // The attempt that was in flight is forgotten, not reported later.
    observe(&mut state, live, OperationState::Failed);
    state.finalize_summaries();
    assert_eq!(queued(&state), []);
    assert_eq!(stored(&state), Queue::empty(before + 1));
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Case {
    NoOptOut,
    WriteFails,
    WriteUncertain,
}

#[test]
fn an_opt_out_whose_write_fails_leaves_the_gate_closed_for_the_rest_of_the_run() {
    after_opt_out(&[Case::NoOptOut, Case::WriteFails]);
}

#[test]
fn an_opt_out_whose_write_is_uncertain_leaves_the_gate_closed_and_the_queue_empty() {
    after_opt_out(&[Case::NoOptOut, Case::WriteUncertain]);
}

// The first case is the positive control: the same run without the opt-out.
fn after_opt_out(cases: &[Case]) {
    for case in cases.iter().copied() {
        let (_root, mut state, _clock) = opted_in();
        failed_attempt(&mut state);
        let before = stored(&state);
        assert_eq!(before.entries.len(), 1);
        match case {
            Case::NoOptOut => (),
            Case::WriteFails => assert_eq!(
                save(&mut state, false, Some(Checkpoint::BeforeReplace)),
                Err(StorageError::Io)
            ),
            Case::WriteUncertain => assert_eq!(
                save(&mut state, false, Some(Checkpoint::AfterReplace)),
                Err(StorageError::PersistenceUncertain)
            ),
        }
        // The launcher still believes the last confirmed preferences.
        assert!(state.preferences().launcher_summary_consent, "{case:?}");
        assert_eq!(
            state.requires_reopen(),
            case == Case::WriteUncertain,
            "{case:?}"
        );
        if case == Case::NoOptOut {
            // Positive control: without the failed opt-out everything flows.
            assert_eq!(gate(&mut state), Gate::Open);
            failed_attempt(&mut state);
            assert_eq!(queued(&state).len(), 2);
            assert!(matches!(state.summary_take_batch(), batch::Take::Ready(_)));
            continue;
        }
        // Closed by the withdrawal itself, whatever else is also true.
        assert_eq!(gate(&mut state), Gate::Blocked, "{case:?}");
        assert!(
            matches!(
                state.summary_take_batch(),
                batch::Take::Closed(Gate::Blocked)
            ),
            "{case:?}"
        );
        state.summary_pre_admission_failure(
            None,
            SummaryOperation::Install,
            SummaryPhase::CatalogFetch,
            SummaryErrorCode::ManifestUnavailable,
        );
        if case == Case::WriteFails {
            // Nothing reached the disk, so the rows wait for a later run.
            assert_eq!(stored(&state), before);
            failed_attempt(&mut state);
            assert_eq!(queued(&state).len(), 1, "no new row while blocked");
            // Saving again without changing consent does not reopen the gate.
            assert!(save(&mut state, true, None).is_ok());
            failed_attempt(&mut state);
            assert_eq!(gate(&mut state), Gate::Blocked);
            assert_eq!(stored(&state), before);
        } else {
            // The replacement may be on disk, so the queue was emptied as well.
            assert_eq!(stored(&state), Queue::empty(before.generation + 1));
            assert_eq!(queued(&state), []);
        }
        assert_eq!(state.summary_faults(), SummaryFaults::default());
    }
}

#[test]
fn opt_in_empties_the_queue_before_preferences_are_written() {
    let root = tempfile::tempdir().unwrap();
    let (mut state, _clock) = configured(root.path());
    // Rows a previous consent period left behind, unknown to this process.
    let mut stale = Queue::empty(41);
    stale.dropped.overflow = 9;
    stale.store(root.path()).unwrap();
    let generation = lock(&state.summaries).queue.generation;
    let mut seen = None;
    let saved = state.save_preferences_with(None, true, 0, |directory, next| {
        // At the moment preferences are written the queue is already empty and
        // the consent on disk is still off.
        seen = Some((
            std::fs::read(directory.join(FILE)).unwrap(),
            directory.join("preferences.json").exists(),
        ));
        atomic::write(directory, "preferences.json", next)
    });
    assert!(saved.unwrap().launcher_summary_consent);
    let (queue_at_write, preferences_existed) = seen.expect("the write seam ran");
    assert_eq!(
        serde_json::from_slice::<Queue>(&queue_at_write).unwrap(),
        Queue::empty(generation + 1)
    );
    assert!(!preferences_existed);
    assert_eq!(gate(&mut state), Gate::Open);
    assert_eq!(stored(&state), Queue::empty(generation + 1));
}

#[test]
fn a_failing_queue_write_leaves_consent_off() {
    // Positive control: the same opt-in with a writable queue path succeeds.
    for blocked in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let (mut state, _clock) = configured(root.path());
        if blocked {
            std::fs::create_dir(root.path().join(FILE)).unwrap();
        }
        let mut written = false;
        let saved = state.save_preferences_with(None, true, 0, |directory, next| {
            written = true;
            atomic::write(directory, "preferences.json", next)
        });
        if !blocked {
            assert_eq!(saved.map(|saved| saved.launcher_summary_consent), Ok(true));
            assert!(written);
            assert_eq!(gate(&mut state), Gate::Open);
            continue;
        }
        assert_eq!(saved, Err(StorageError::Io));
        assert!(!written, "preferences must not be attempted");
        assert_eq!(state.preferences(), &Preferences::default());
        assert!(!root.path().join("preferences.json").exists());
        assert!(!state.requires_reopen());
        assert_eq!(gate(&mut state), Gate::NoConsent);
        // The launcher itself is unaffected.
        failed_attempt(&mut state);
        assert_eq!(queued(&state), []);
        // Every other preference still saves.
        assert!(state
            .save_preferences(Some(root.path().join("game")), false, 0)
            .is_ok());
    }
}

#[test]
fn an_inert_component_touches_no_queue_file_on_consent_changes() {
    let root = tempfile::tempdir().unwrap();
    // Never configured, and the queue path is not even a file.
    std::fs::create_dir(root.path().join(FILE)).unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    set_consent(&mut state, true);
    set_consent(&mut state, false);
    set_consent(&mut state, true);
    assert!(root.path().join(FILE).is_dir());
    assert_eq!(state.summary_faults(), SummaryFaults::default());
    assert_eq!(gate(&mut state), Gate::NoEndpoint);
}

#[test]
fn an_opt_in_while_inert_removes_a_queue_file_an_earlier_run_left() {
    // Positive control: a save that does not change consent leaves the file.
    for opt_in in [false, true] {
        let (root, mut state, _clock) = opted_in();
        failed_attempt(&mut state);
        let mut preferences = state.preferences().clone();
        drop(state);
        let directory = root.path().join("state");
        // Consent went off without the queue file being removed: a preferences
        // file restored by hand, or a removal that failed.
        preferences.launcher_summary_consent = false;
        atomic::write(&directory, "preferences.json", &preferences).unwrap();
        assert!(directory.join(FILE).is_file());

        // This run never configures summaries.
        let mut state = DesktopState::open(&directory).unwrap();
        let saved = state.save_preferences(None, opt_in, preferences.revision);
        assert_eq!(
            saved.map(|saved| saved.launcher_summary_consent),
            Ok(opt_in)
        );
        assert_eq!(directory.join(FILE).exists(), !opt_in);
        assert_eq!(state.summary_faults(), SummaryFaults::default());
        drop(state);
        if opt_in {
            // A later configured run finds consent on and nothing to send.
            let (state, _clock) = reopened(&directory);
            assert_eq!(queued(&state), []);
        }
    }
}

#[test]
fn a_withdrawal_while_inert_still_removes_rows_an_earlier_run_queued() {
    let (root, mut state, _clock) = opted_in();
    failed_attempt(&mut state);
    drop(state);
    let directory = root.path().join("state");
    assert!(directory.join(FILE).is_file());
    // This run never configures summaries.
    let mut state = DesktopState::open(&directory).unwrap();
    set_consent(&mut state, false);
    assert!(!directory.join(FILE).exists());
    set_consent(&mut state, true);
    drop(state);
    let (state, _clock) = reopened(&directory);
    assert_eq!(queued(&state), []);
}

#[test]
fn opt_out_then_opt_in_never_resurrects_old_entries() {
    let (_root, mut state, _clock) = opted_in();
    let live = row_and_live_attempt(&mut state);
    let before = stored(&state).generation;
    set_consent(&mut state, false);
    set_consent(&mut state, true);
    assert_eq!(gate(&mut state), Gate::Open);
    assert_eq!(queued(&state), []);
    assert_eq!(stored(&state), Queue::empty(before + 2));
    // The attempt admitted under the earlier consent stays unreported.
    observe(&mut state, live, OperationState::Failed);
    state.finalize_summaries();
    assert_eq!(queued(&state), []);
    // Positive control: an attempt admitted under the new consent is reported.
    failed_attempt(&mut state);
    let rows = queued(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].operation, SummaryOperation::Repair);
}

#[test]
fn saving_without_a_consent_change_has_no_queue_effect() {
    let (root, mut state, _clock) = opted_in();
    let live = row_and_live_attempt(&mut state);
    let before = stored(&state);
    // A refused save and an accepted one that keeps consent as it is.
    assert_eq!(
        state.save_preferences(None, false, 99),
        Err(StorageError::StaleRevision)
    );
    assert_eq!(
        state.save_preferences(Some(root.path().join("elsewhere")), false, 1),
        Err(StorageError::Busy)
    );
    assert!(save(&mut state, true, None).is_ok());
    assert_eq!(stored(&state), before);
    assert_eq!(gate(&mut state), Gate::Open);
    // The live attempt is still tracked and reports normally.
    observe(&mut state, live, OperationState::Succeeded);
    state.finalize_summaries();
    assert_eq!(queued(&state).len(), 2);
}
