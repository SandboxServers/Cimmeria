//! The journal observer through the engine's own admission and terminal paths.
//! Real install-worker runs are in `install_worker/summary_tests.rs`.
use super::*;
use install_worker::Outcome;

fn row(
    summary: &Summary,
) -> (
    SummaryOperation,
    SummaryPhase,
    SummaryOutcome,
    Option<SummaryErrorCode>,
) {
    (
        summary.operation,
        summary.phase,
        summary.outcome,
        summary.error_code,
    )
}

#[test]
fn each_install_result_yields_exactly_one_row_with_its_closed_code() {
    for (outcome, expected, code) in [
        (Outcome::ContentPrepared, SummaryOutcome::Succeeded, None),
        (Outcome::Cancelled, SummaryOutcome::Cancelled, None),
        (
            Outcome::DestinationUnavailable,
            SummaryOutcome::Failed,
            Some(SummaryErrorCode::DestinationUnavailable),
        ),
        (
            Outcome::InstallFailed,
            SummaryOutcome::Failed,
            Some(SummaryErrorCode::InstallFailed),
        ),
        (
            Outcome::ContentInvalid,
            SummaryOutcome::Failed,
            Some(SummaryErrorCode::ContentInvalid),
        ),
        (
            Outcome::RosettaRequired,
            SummaryOutcome::Failed,
            Some(SummaryErrorCode::RosettaRequired),
        ),
        (
            Outcome::RuntimeUnavailable,
            SummaryOutcome::Failed,
            Some(SummaryErrorCode::RuntimeUnavailable),
        ),
    ] {
        let (_root, mut state, _clock) = opted_in();
        let id = admit_install(&mut state);
        observe(&mut state, id, OperationState::Running);
        if outcome == Outcome::Cancelled {
            state.operations_mut().unwrap().request_cancel(id).unwrap();
        }
        assert_eq!(queued(&state), [], "nothing before the terminal commit");
        finish_install(&mut state, id, outcome);
        state.finalize_summaries();
        let rows = queued(&state);
        assert_eq!(rows.len(), 1, "{outcome:?}");
        assert_eq!(
            row(&rows[0]),
            (
                SummaryOperation::Install,
                SummaryPhase::Running,
                expected,
                code
            ),
            "{outcome:?}"
        );
        assert_eq!(rows[0].retry_count, RetryCount::ZERO);
        // The same row is on disk and the attempt is no longer tracked.
        let on_disk = stored(&state);
        assert_eq!(on_disk.tracking, None);
        assert_eq!(on_disk.entries.len(), 1);
        assert_eq!(on_disk.entries[0].summary, rows[0]);
        assert!(!on_disk.entries[0].pre_admission);
        // A second finalize, a repeated terminal and an identical retry of the
        // admission add nothing.
        state.finalize_summaries();
        state.finalize_summaries();
        let terminal = state
            .operations()
            .snapshot()
            .operation
            .clone()
            .unwrap()
            .state;
        assert!(state
            .operations_mut()
            .unwrap()
            .observe(id, terminal)
            .is_err());
        let (operation, preferences) = (
            state.operations().snapshot().revision,
            state.preferences().revision,
        );
        let retried =
            state.admit_install(id, operation, preferences, &release(), default_servers());
        assert!(!retried.unwrap().dispatch);
        state.finalize_summaries();
        assert_eq!(queued(&state), rows, "{outcome:?}");
    }
}

#[test]
fn a_failure_without_a_result_record_is_unspecified() {
    let (_root, mut state, _clock) = opted_in();
    let id = admit_install(&mut state);
    observe(&mut state, id, OperationState::Failed);
    state.finalize_summaries();
    let rows = queued(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        row(&rows[0]),
        (
            SummaryOperation::Install,
            SummaryPhase::Starting,
            SummaryOutcome::Failed,
            Some(SummaryErrorCode::Unspecified)
        )
    );
}

#[test]
fn every_operation_kind_maps_to_its_wire_operation_and_failure_code() {
    for (kind, operation, code) in [
        (
            OperationKind::PrepareRuntime,
            SummaryOperation::PrepareRuntime,
            SummaryErrorCode::PrerequisiteFailed,
        ),
        (
            OperationKind::Repair,
            SummaryOperation::Repair,
            SummaryErrorCode::Unspecified,
        ),
        (
            OperationKind::Uninstall,
            SummaryOperation::Uninstall,
            SummaryErrorCode::Unspecified,
        ),
        (
            OperationKind::Launch,
            SummaryOperation::Launch,
            SummaryErrorCode::Unspecified,
        ),
    ] {
        let (_root, mut state, _clock) = opted_in();
        let id = begin(&mut state, kind);
        observe(&mut state, id, OperationState::Running);
        observe(&mut state, id, OperationState::Failed);
        state.finalize_summaries();
        let rows = queued(&state);
        assert_eq!(rows.len(), 1, "{kind:?}");
        assert_eq!(
            row(&rows[0]),
            (
                operation,
                SummaryPhase::Running,
                SummaryOutcome::Failed,
                Some(code)
            ),
            "{kind:?}"
        );
    }
}

/// Adoption has no value in wire schema v1. It is neither tracked nor
/// reported; the install beside it is the control.
#[test]
fn a_kind_without_a_wire_value_is_never_tracked_or_reported() {
    for (kind, reported) in [(OperationKind::Adopt, 0), (OperationKind::Install, 1)] {
        let (_root, mut state, _clock) = opted_in();
        let id = begin(&mut state, kind);
        assert_eq!(stored(&state).tracking.is_some(), reported == 1, "{kind:?}");
        observe(&mut state, id, OperationState::Running);
        observe(&mut state, id, OperationState::Failed);
        state.finalize_summaries();
        assert_eq!(queued(&state).len(), reported, "{kind:?}");
    }
}

#[test]
fn lost_observation_is_one_unknown_row_and_a_later_reconciled_terminal_adds_nothing() {
    let (_root, mut state, _clock) = opted_in();
    let id = admit_install(&mut state);
    observe(&mut state, id, OperationState::Running);
    state.operations_mut().unwrap().mark_uncertain(id).unwrap();
    state.finalize_summaries();
    let rows = queued(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        row(&rows[0]),
        (
            SummaryOperation::Install,
            SummaryPhase::Running,
            SummaryOutcome::Unknown,
            None
        )
    );
    // Reconciliation, including a resumed run, belongs to the same attempt.
    state
        .operations_mut()
        .unwrap()
        .reconcile(id, OperationState::Running)
        .unwrap();
    observe(&mut state, id, OperationState::Succeeded);
    state.finalize_summaries();
    assert_eq!(queued(&state), rows);
    // Positive control: the next admitted attempt is reported again.
    let next = admit_install(&mut state);
    observe(&mut state, next, OperationState::Failed);
    state.finalize_summaries();
    assert_eq!(queued(&state).len(), 2);
}

#[test]
fn two_ends_without_a_finalize_between_them_are_both_kept() {
    let (_root, mut state, _clock) = opted_in();
    let first = begin(&mut state, OperationKind::Repair);
    // Bypass the lazy finalize the way an admission closure does.
    state
        .operations
        .observe(first, OperationState::Failed)
        .unwrap();
    let revision = state.operations.snapshot().revision;
    let second = Uuid::new_v4();
    state
        .operations
        .begin(second, OperationKind::Uninstall, [4; 32], revision)
        .unwrap();
    state
        .operations
        .observe(second, OperationState::Succeeded)
        .unwrap();
    state.finalize_summaries();
    let rows = queued(&state);
    assert_eq!(
        rows.iter().map(row).collect::<Vec<_>>(),
        [
            (
                SummaryOperation::Repair,
                SummaryPhase::Starting,
                SummaryOutcome::Failed,
                Some(SummaryErrorCode::Unspecified)
            ),
            (
                SummaryOperation::Uninstall,
                SummaryPhase::Starting,
                SummaryOutcome::Succeeded,
                None
            )
        ]
    );
    assert_eq!(stored(&state).tracking, None);
}

#[test]
fn a_failed_commit_observes_nothing() {
    // Positive control: the same sequence with a working journal yields a row.
    for break_journal in [false, true] {
        let (_root, mut state, _clock) = opted_in();
        let id = begin(&mut state, OperationKind::Repair);
        let path = state.state_root().join("operation.json");
        if break_journal {
            std::fs::remove_file(&path).unwrap();
            std::fs::create_dir(&path).unwrap();
        }
        let committed = state.operations.observe(id, OperationState::Failed);
        assert_eq!(committed.is_err(), break_journal);
        state.finalize_summaries();
        assert_eq!(queued(&state).len(), usize::from(!break_journal));
    }
}

/// When the summary component learns about the attempt, relative to admission.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Admitted {
    GateOpen,
    ConsentOff,
    BeforeConfigure,
    WithoutEndpoint,
}

// Everything is in place by the terminal commit in every case; only the moment
// of admission differs.
fn rows_after_failing_install(case: Admitted) -> (Vec<Summary>, Gate) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    let _clock = Clock::install(&mut state);
    match case {
        Admitted::GateOpen | Admitted::ConsentOff => {
            state.configure_summaries(config(Some(ENDPOINT)));
        }
        Admitted::WithoutEndpoint => state.configure_summaries(config(None)),
        Admitted::BeforeConfigure => (),
    }
    if case != Admitted::ConsentOff {
        set_consent(&mut state, true);
    }
    let id = admit_install(&mut state);
    state.configure_summaries(config(Some(ENDPOINT)));
    if case == Admitted::ConsentOff {
        set_consent(&mut state, true);
    }
    observe(&mut state, id, OperationState::Running);
    finish_install(&mut state, id, Outcome::InstallFailed);
    let gate = gate(&mut state);
    assert_eq!(lock(&state.summaries).queue.tracking, None);
    (queued(&state), gate)
}

// The positive control and one negative case that differs only in when the
// attempt was admitted.
fn reported_only_when_admitted_with_the_gate_open(case: Admitted) {
    let (rows, gate) = rows_after_failing_install(Admitted::GateOpen);
    assert_eq!(gate, Gate::Open);
    assert_eq!(rows.len(), 1, "positive control");
    assert_eq!(rows[0].error_code, Some(SummaryErrorCode::InstallFailed));
    let (rows, gate) = rows_after_failing_install(case);
    // The gate is open at the terminal, so only the admission rule is at work.
    assert_eq!(gate, Gate::Open, "{case:?}");
    assert_eq!(rows, [], "{case:?}");
}

#[test]
fn an_attempt_admitted_while_consent_was_off_is_never_reported() {
    reported_only_when_admitted_with_the_gate_open(Admitted::ConsentOff);
}

#[test]
fn an_attempt_admitted_before_configuration_is_never_reported() {
    reported_only_when_admitted_with_the_gate_open(Admitted::BeforeConfigure);
}

#[test]
fn an_attempt_admitted_without_an_endpoint_is_never_reported() {
    reported_only_when_admitted_with_the_gate_open(Admitted::WithoutEndpoint);
}

#[test]
fn without_an_endpoint_nothing_is_tracked_recorded_or_left_on_disk() {
    for endpoint in [Some(ENDPOINT), None] {
        let root = tempfile::tempdir().unwrap();
        let mut state = DesktopState::open(&root.path().join("state")).unwrap();
        state.configure_summaries(config(endpoint));
        set_consent(&mut state, true);
        let id = admit_install(&mut state);
        observe(&mut state, id, OperationState::Failed);
        state.summary_pre_admission_failure(
            None,
            SummaryOperation::Install,
            SummaryPhase::CatalogFetch,
            SummaryErrorCode::ManifestUnavailable,
        );
        assert_eq!(queued(&state).len(), if endpoint.is_some() { 2 } else { 0 });
        assert_eq!(queue_bytes(&state).is_some(), endpoint.is_some());
        assert_eq!(
            gate(&mut state),
            if endpoint.is_some() {
                Gate::Open
            } else {
                Gate::NoEndpoint
            }
        );
    }
}

#[test]
fn configuring_without_an_endpoint_removes_an_existing_queue_file() {
    let (_root, mut state, _clock) = opted_in();
    let id = begin(&mut state, OperationKind::Repair);
    observe(&mut state, id, OperationState::Failed);
    state.finalize_summaries();
    assert!(queue_bytes(&state).is_some());
    state.configure_summaries(config(None));
    assert_eq!(queue_bytes(&state), None);
    assert_eq!(queued(&state), []);
    // Configuring again with an endpoint does not bring the rows back.
    state.configure_summaries(config(Some(ENDPOINT)));
    assert_eq!(queued(&state), []);
}

#[test]
fn uninstall_through_its_real_path_yields_one_succeeded_row() {
    let (root, mut state, _clock) = opted_in();
    // A promoted native installation, as the install worker leaves it.
    let install = admit_install(&mut state);
    observe(&mut state, install, OperationState::Running);
    let intent = state.install_intent().unwrap().unwrap();
    std::fs::create_dir_all(intent.destination.join("game")).unwrap();
    for name in [".cimmeria-install.json", "content-ready.json"] {
        crate::storage::atomic::write(&intent.destination, name, &intent).unwrap();
    }
    state.remember_prepared_content().unwrap();
    finish_install(&mut state, install, Outcome::ContentPrepared);
    let revision = state.operations().snapshot().revision;
    state
        .uninstall(Uuid::new_v4(), revision, install, true)
        .unwrap();
    assert!(!root.path().join("install").exists());
    state.finalize_summaries();
    assert_eq!(
        queued(&state).iter().map(row).collect::<Vec<_>>(),
        [
            (
                SummaryOperation::Install,
                SummaryPhase::Running,
                SummaryOutcome::Succeeded,
                None
            ),
            (
                SummaryOperation::Uninstall,
                SummaryPhase::Running,
                SummaryOutcome::Succeeded,
                None
            )
        ]
    );
}
