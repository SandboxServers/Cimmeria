//! Failures before admission: closed values in, one coalesced row out.
use super::*;
use crate::storage::launcher_summary::queue::MAX_ENTRIES;

fn manifest_unavailable(state: &mut DesktopState, request: Option<Uuid>) {
    state.summary_pre_admission_failure(
        request,
        SummaryOperation::Install,
        SummaryPhase::CatalogFetch,
        SummaryErrorCode::ManifestUnavailable,
    );
}

#[test]
fn a_pre_admission_failure_is_one_failed_row_without_timing() {
    let (_root, mut state, _clock) = opted_in();
    manifest_unavailable(&mut state, Some(Uuid::new_v4()));
    let on_disk = stored(&state);
    assert_eq!(on_disk.entries.len(), 1);
    assert!(on_disk.entries[0].pre_admission);
    assert_eq!(on_disk.entries[0].created_unix_s, T0);
    assert_eq!(
        on_disk.entries[0].summary,
        Summary {
            event_id: minted(1),
            attempt_id: minted(2),
            operation: SummaryOperation::Install,
            phase: SummaryPhase::CatalogFetch,
            outcome: SummaryOutcome::Failed,
            error_code: Some(SummaryErrorCode::ManifestUnavailable),
            duration_ms: None,
            retry_count: RetryCount::ZERO,
            phases: None,
            launcher_version: LauncherVersion::new((0, 1, 0)),
            os: SummaryOs::current(),
            arch: SummaryArch::current(),
        }
    );
    assert_eq!(on_disk.tracking, None);
}

#[test]
fn two_hundred_identical_failures_leave_one_entry_with_retry_count_100() {
    let (_root, mut state, _clock) = opted_in();
    for _ in 0..200 {
        manifest_unavailable(&mut state, None);
    }
    let rows = queued(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].retry_count.get(), 100);
    assert_eq!(rows[0].event_id, minted(1), "the row keeps its identity");
    assert_eq!(stored(&state).entries[0].summary, rows[0]);
    // A failure differing in any one of the three values is its own row.
    for (operation, phase, code) in [
        (
            SummaryOperation::Repair,
            SummaryPhase::CatalogFetch,
            SummaryErrorCode::ManifestUnavailable,
        ),
        (
            SummaryOperation::Install,
            SummaryPhase::ManifestVerify,
            SummaryErrorCode::ManifestUnavailable,
        ),
        (
            SummaryOperation::Install,
            SummaryPhase::CatalogFetch,
            SummaryErrorCode::LocalIo,
        ),
    ] {
        state.summary_pre_admission_failure(None, operation, phase, code);
    }
    let rows = queued(&state);
    assert_eq!(rows.len(), 4);
    assert!(rows[1..]
        .iter()
        .all(|row| row.retry_count == RetryCount::ZERO));
}

#[test]
fn an_admitted_failure_with_the_same_values_is_never_coalesced_into() {
    let (_root, mut state, _clock) = opted_in();
    let id = begin(&mut state, OperationKind::Install);
    observe(&mut state, id, OperationState::Failed);
    state.finalize_summaries();
    // Same operation and code as the admitted row above, and its end phase.
    state.summary_pre_admission_failure(
        None,
        SummaryOperation::Install,
        SummaryPhase::Starting,
        SummaryErrorCode::Unspecified,
    );
    let on_disk = stored(&state);
    assert_eq!(
        on_disk
            .entries
            .iter()
            .map(|entry| (entry.pre_admission, entry.summary.retry_count.get()))
            .collect::<Vec<_>>(),
        [(false, 0), (true, 0)]
    );
}

#[test]
fn a_failure_naming_the_tracked_or_current_operation_is_ignored() {
    let (_root, mut state, _clock) = opted_in();
    let finished = begin(&mut state, OperationKind::Repair);
    observe(&mut state, finished, OperationState::Failed);
    // The journal's current operation, already terminal.
    manifest_unavailable(&mut state, Some(finished));
    assert_eq!(queued(&state).len(), 1, "only the journal row");
    let tracked = admit_install(&mut state);
    manifest_unavailable(&mut state, Some(tracked));
    assert_eq!(queued(&state).len(), 1);
    // Positive controls: another request id, and none at all, are recorded.
    manifest_unavailable(&mut state, Some(Uuid::new_v4()));
    assert_eq!(queued(&state).len(), 2);
    manifest_unavailable(&mut state, None);
    assert_eq!(queued(&state)[1].retry_count.get(), 1);
}

fn fill_with_admitted(state: &mut DesktopState, count: usize) {
    for _ in 0..count {
        let id = begin(state, OperationKind::Repair);
        observe(state, id, OperationState::Failed);
    }
    state.finalize_summaries();
}

#[test]
fn pre_admission_failures_never_evict_an_admitted_attempts_entry() {
    let (_root, mut state, _clock) = opted_in();
    fill_with_admitted(&mut state, MAX_ENTRIES);
    let admitted = queued(&state);
    assert_eq!(admitted.len(), MAX_ENTRIES);
    for _ in 0..200 {
        manifest_unavailable(&mut state, None);
    }
    // Every one was dropped and counted; the admitted rows are untouched.
    assert_eq!(queued(&state), admitted);
    let on_disk = stored(&state);
    assert_eq!(on_disk.dropped.overflow, 200);
    assert!(on_disk.entries.iter().all(|entry| !entry.pre_admission));
}

#[test]
fn a_full_queue_replaces_its_oldest_pre_admission_entry_instead() {
    let (_root, mut state, _clock) = opted_in();
    manifest_unavailable(&mut state, None);
    state.summary_pre_admission_failure(
        None,
        SummaryOperation::Launch,
        SummaryPhase::Admission,
        SummaryErrorCode::StateInvalid,
    );
    fill_with_admitted(&mut state, MAX_ENTRIES - 2);
    let before = queued(&state);
    assert_eq!(before.len(), MAX_ENTRIES);
    state.summary_pre_admission_failure(
        None,
        SummaryOperation::Repair,
        SummaryPhase::DestinationCheck,
        SummaryErrorCode::InvalidDirectory,
    );
    let after = stored(&state);
    assert_eq!(after.entries.len(), MAX_ENTRIES);
    assert_eq!(after.dropped.overflow, 1);
    // The oldest pre-admission row made room; every admitted row is still there.
    assert_eq!(
        after
            .entries
            .iter()
            .filter(|entry| entry.pre_admission)
            .map(|entry| entry.summary.error_code.unwrap())
            .collect::<Vec<_>>(),
        [
            SummaryErrorCode::StateInvalid,
            SummaryErrorCode::InvalidDirectory
        ]
    );
    for row in before
        .iter()
        .filter(|row| row.phase == SummaryPhase::Starting)
    {
        assert!(after.entries.iter().any(|entry| entry.summary == *row));
    }
    // An admitted attempt arriving at a full queue drops the oldest of any kind.
    fill_with_admitted(&mut state, 1);
    let last = stored(&state);
    assert_eq!(last.dropped.overflow, 2);
    assert_eq!(last.entries.len(), MAX_ENTRIES);
}

#[test]
fn nothing_is_recorded_while_the_gate_is_closed() {
    // Positive control: the same call with consent given records a row.
    for consent in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let (mut state, _clock) = configured(root.path());
        if consent {
            set_consent(&mut state, true);
        }
        manifest_unavailable(&mut state, None);
        assert_eq!(queued(&state).len(), usize::from(consent));
        assert_eq!(queue_bytes(&state).is_some(), consent);
    }
}

// The producer arguments are closed, `Copy` values: no owned string, path or
// error text can be passed in, so none can reach a row.
#[test]
fn producer_arguments_are_copy_values() {
    fn copy<T: Copy + Send + 'static>() {}
    copy::<Uuid>();
    copy::<Option<Uuid>>();
    copy::<TimedPhase>();
    copy::<SummaryOperation>();
    copy::<SummaryPhase>();
    copy::<SummaryErrorCode>();
    // Pin the signatures themselves to those types.
    let _: fn(&mut DesktopState, Uuid, TimedPhase) = DesktopState::summary_phase;
    let _: fn(&mut DesktopState, Option<Uuid>, SummaryOperation, SummaryPhase, SummaryErrorCode) =
        DesktopState::summary_pre_admission_failure;
}
