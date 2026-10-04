//! The seam the exporter drives: take a batch, send it unlocked, apply verdicts.
use super::*;
use batch::Take;

fn ready(state: &mut DesktopState) -> batch::Batch {
    match state.summary_take_batch() {
        Take::Ready(batch) => batch,
        Take::Empty => panic!("queue is empty"),
        Take::Closed(gate) => panic!("gate is {gate:?}"),
    }
}

fn failed_attempts(state: &mut DesktopState, count: usize) {
    for _ in 0..count {
        let id = begin(state, OperationKind::Repair);
        observe(state, id, OperationState::Failed);
    }
}

#[test]
fn taking_a_batch_finalizes_pending_rows_and_leaves_them_queued() {
    let (_root, mut state, _clock) = opted_in();
    assert!(matches!(state.summary_take_batch(), Take::Empty));
    failed_attempts(&mut state, 1);
    assert_eq!(queued(&state), [], "the terminal is not finalized yet");
    let batch = ready(&mut state);
    assert_eq!(batch.summaries, queued(&state));
    assert_eq!(batch.summaries.len(), 1);
    assert_eq!(batch.generation, stored(&state).generation);
    assert!(!batch.cancel.is_cancelled());
    // The body is the wire request, valid by construction.
    let body = serde_json::to_value(batch.request()).unwrap();
    let typed: SummaryRequest = serde_json::from_value(body).unwrap();
    assert_eq!(typed.summaries, batch.summaries);
    assert_eq!(typed.client_dropped, DroppedCounts::default());
    // Taking again without a verdict hands out the same rows: nothing is lost.
    assert_eq!(ready(&mut state).summaries, batch.summaries);
}

#[test]
fn a_batch_holds_at_most_thirty_two_of_the_oldest_rows() {
    let (_root, mut state, _clock) = opted_in();
    failed_attempts(&mut state, 40);
    let batch = ready(&mut state);
    assert_eq!(batch.summaries.len(), MAX_BATCH);
    assert_eq!(batch.summaries, queued(&state)[..MAX_BATCH]);
    assert!(serde_json::to_vec(&batch.request()).unwrap().len() <= 48 * 1024);
    assert!(state.summary_apply_results(&batch, &[SummaryResult::Accepted; MAX_BATCH]));
    assert_eq!(ready(&mut state).summaries.len(), 8);
}

#[test]
fn every_verdict_removes_its_row_and_rejections_are_counted() {
    let (_root, mut state, _clock) = opted_in();
    failed_attempts(&mut state, 4);
    state.finalize_summaries();
    lock(&state.summaries).queue.dropped = DroppedCounts {
        overflow: 5,
        expired: 2,
        rejected: 1,
    };
    let batch = ready(&mut state);
    assert_eq!(batch.dropped.overflow, 5);
    // A drop that happens while the upload is in flight stays counted.
    lock(&state.summaries).queue.dropped.overflow = 6;
    failed_attempts(&mut state, 1);
    state.finalize_summaries();
    assert!(state.summary_apply_results(
        &batch,
        &[
            SummaryResult::Accepted,
            SummaryResult::Duplicate,
            SummaryResult::Rejected,
            SummaryResult::Rejected,
        ],
    ));
    let on_disk = stored(&state);
    assert_eq!(on_disk.entries.len(), 1, "only the row queued in between");
    assert!(!batch
        .summaries
        .iter()
        .any(|sent| sent.event_id == on_disk.entries[0].summary.event_id));
    assert_eq!(
        on_disk.dropped,
        DroppedCounts {
            overflow: 1,
            expired: 0,
            rejected: 2,
        }
    );
}

#[test]
fn a_verdict_list_of_the_wrong_length_applies_nothing() {
    let (_root, mut state, _clock) = opted_in();
    failed_attempts(&mut state, 2);
    let batch = ready(&mut state);
    let before = stored(&state);
    assert!(!state.summary_apply_results(&batch, &[SummaryResult::Accepted]));
    assert!(!state.summary_apply_results(&batch, &[SummaryResult::Accepted; 3]));
    assert_eq!(stored(&state), before);
    // Positive control.
    assert!(state.summary_apply_results(&batch, &[SummaryResult::Accepted; 2]));
    assert_eq!(stored(&state).entries, []);
}

#[test]
fn a_permanently_refused_batch_is_dropped_and_counted_with_its_counters_kept() {
    let (_root, mut state, _clock) = opted_in();
    failed_attempts(&mut state, 3);
    state.finalize_summaries();
    lock(&state.summaries).queue.dropped.expired = 4;
    let batch = ready(&mut state);
    assert!(state.summary_reject_batch(&batch));
    let on_disk = stored(&state);
    assert_eq!(on_disk.entries, []);
    assert_eq!(
        on_disk.dropped,
        DroppedCounts {
            overflow: 0,
            expired: 4,
            rejected: 3,
        }
    );
}

#[test]
fn a_batch_taken_before_a_withdrawal_can_no_longer_be_applied() {
    // Positive control: without the withdrawal the same batch applies.
    for withdraw in [false, true] {
        let (_root, mut state, _clock) = opted_in();
        failed_attempts(&mut state, 1);
        let batch = ready(&mut state);
        assert!(state.summary_batch_current(batch.generation));
        if withdraw {
            set_consent(&mut state, false);
            // Consent given again is a new generation with an empty queue.
            set_consent(&mut state, true);
            failed_attempts(&mut state, 1);
            state.finalize_summaries();
        }
        let before = stored(&state);
        assert_eq!(batch.cancel.is_cancelled(), withdraw);
        assert_eq!(state.summary_batch_current(batch.generation), !withdraw);
        assert_eq!(
            state.summary_apply_results(&batch, &[SummaryResult::Accepted]),
            !withdraw
        );
        if withdraw {
            assert!(!state.summary_reject_batch(&batch));
            assert_eq!(stored(&state), before, "the new consent's row is untouched");
            // A batch of the new generation carries a fresh, live token.
            let next = ready(&mut state);
            assert_eq!(next.generation, batch.generation + 2);
            assert!(!next.cancel.is_cancelled());
        }
    }
}

#[test]
fn the_export_target_exists_only_with_an_endpoint_and_a_row_wakes_the_exporter() {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    assert!(state.summary_export_target().is_none());
    state.configure_summaries(config(None));
    assert!(state.summary_export_target().is_none());
    state.configure_summaries(config(Some(ENDPOINT)));
    let target = state.summary_export_target().expect("configured");
    assert_eq!(target.endpoint, SummaryEndpoint::parse(ENDPOINT).unwrap());
    assert_eq!(target.launcher_version, LauncherVersion::new((0, 1, 0)));
    set_consent(&mut state, true);

    // No permit is stored until a row is waiting, then exactly one wake-up.
    let woken = |target: &batch::ExportTarget| {
        use std::{
            future::Future,
            pin::pin,
            task::{Context, Poll, Waker},
        };
        let notified = pin!(target.wake.notified());
        matches!(
            notified.poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(())
        )
    };
    assert!(!woken(&target));
    let id = begin(&mut state, OperationKind::Repair);
    assert!(!woken(&target), "an admission is not a row");
    observe(&mut state, id, OperationState::Failed);
    assert!(woken(&target));
    assert!(!woken(&target));

    // A failure before admission is a new row as well. An identical repeat
    // only raises that row's count: it is already waiting, so nobody is woken.
    let manifest_unavailable = |state: &mut DesktopState| {
        state.summary_pre_admission_failure(
            None,
            SummaryOperation::Install,
            SummaryPhase::CatalogFetch,
            SummaryErrorCode::ManifestUnavailable,
        );
    };
    manifest_unavailable(&mut state);
    assert!(woken(&target));
    manifest_unavailable(&mut state);
    assert!(!woken(&target));
    let rows = queued(&state);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].retry_count.get(), 1, "the repeat was recorded");
}

#[test]
fn reconfiguring_never_lets_an_older_batch_look_current_again() {
    let (_root, mut state, _clock) = opted_in();
    failed_attempts(&mut state, 1);
    let batch = ready(&mut state);
    assert!(state.summary_batch_current(batch.generation));
    state.configure_summaries(config(None));
    assert!(!state.summary_batch_current(batch.generation));
    state.configure_summaries(config(Some(ENDPOINT)));
    assert_eq!(gate(&mut state), Gate::Open);
    assert!(!state.summary_batch_current(batch.generation));
    assert!(!state.summary_apply_results(&batch, &[SummaryResult::Accepted]));
}
