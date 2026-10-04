//! Queue budgets, overflow, expiry and the disposable-file rule.
use super::*;
use crate::storage::{
    launcher_summary::queue::{Entry, Tracking, MAX_ENTRIES, TTL_S},
    StorageError, MAX_STATE_BYTES,
};
use install_worker::Outcome;

/// The largest entry the engine can produce: every optional field present,
/// every integer at its maximum, the longest name of every enum. The struct
/// literals name every field, so a new field has to be decided here too.
fn worst_case_entry() -> Entry {
    fn longest<T: Copy + serde::Serialize>(all: &[T]) -> T {
        *all.iter()
            .max_by_key(|value| serde_json::to_string(value).unwrap().len())
            .unwrap()
    }
    Entry {
        created_unix_s: u64::MAX,
        pre_admission: false,
        summary: Summary {
            event_id: Uuid::from_u128(u128::MAX),
            attempt_id: Uuid::from_u128(u128::MAX),
            operation: longest(SummaryOperation::ALL),
            phase: longest(SummaryPhase::ALL),
            outcome: SummaryOutcome::Failed,
            error_code: Some(longest(SummaryErrorCode::ALL)),
            duration_ms: Some(Millis::MAX),
            retry_count: RetryCount::MAX,
            phases: Some(
                TimedPhase::ALL
                    .iter()
                    .map(|phase| PhaseDuration {
                        phase: *phase,
                        duration_ms: Millis::MAX,
                    })
                    .collect(),
            ),
            launcher_version: LauncherVersion::new((999, 999, 999)),
            os: longest(SummaryOs::ALL),
            arch: longest(SummaryArch::ALL),
        },
    }
}

#[test]
fn worst_case_entry_and_a_full_queue_fit_their_budgets() {
    let entry = worst_case_entry();
    let bytes = serde_json::to_vec(&entry).unwrap().len();
    // Pinned exactly: adding a field or lengthening a name must be a decision.
    assert_eq!(bytes, 592, "worst-case entry size changed");
    assert!(bytes <= 2048);
    let full = Queue {
        schema_version: u32::MAX,
        generation: u64::MAX,
        dropped: DroppedCounts {
            overflow: u16::MAX,
            expired: u16::MAX,
            rejected: u16::MAX,
        },
        tracking: Some(Tracking {
            local_operation_id: Uuid::from_u128(u128::MAX),
            attempt_id: Uuid::from_u128(u128::MAX),
            kind: SummaryOperation::PrepareRuntime,
        }),
        entries: vec![entry; MAX_ENTRIES],
    };
    let total = serde_json::to_vec(&full).unwrap().len();
    assert!(
        total as u64 <= MAX_STATE_BYTES,
        "{total} bytes exceed the atomic helper's cap"
    );
    // The real writer accepts it and the real reader returns it.
    let root = tempfile::tempdir().unwrap();
    assert_eq!(full.store(root.path()), Ok(()));
    let mut reloaded = full.clone();
    reloaded.schema_version = 1;
    reloaded.store(root.path()).unwrap();
    assert_eq!(Queue::load(root.path()), reloaded);
}

fn entry(n: u64, created_unix_s: u64, pre_admission: bool) -> Entry {
    Entry {
        created_unix_s,
        pre_admission,
        summary: Summary {
            event_id: minted(n),
            attempt_id: minted(1000 + n),
            operation: SummaryOperation::Repair,
            phase: SummaryPhase::Running,
            outcome: SummaryOutcome::Succeeded,
            error_code: None,
            duration_ms: None,
            retry_count: RetryCount::ZERO,
            phases: None,
            launcher_version: LauncherVersion::new((0, 1, 0)),
            os: SummaryOs::Linux,
            arch: SummaryArch::X86_64,
        },
    }
}

#[test]
fn the_sixty_fifth_entry_drops_the_oldest_and_counts_it() {
    let mut queue = Queue::empty(0);
    for n in 1..=64 {
        queue.push_admitted(entry(n, T0, false));
    }
    assert_eq!((queue.entries.len(), queue.dropped.overflow), (64, 0));
    queue.push_admitted(entry(65, T0, false));
    assert_eq!((queue.entries.len(), queue.dropped.overflow), (64, 1));
    assert_eq!(queue.entries[0].summary.event_id, minted(2));
    assert_eq!(queue.entries[63].summary.event_id, minted(65));
    queue.dropped.overflow = u16::MAX;
    queue.push_admitted(entry(66, T0, false));
    assert_eq!(queue.dropped.overflow, u16::MAX, "saturates");
}

#[test]
fn sixty_five_admitted_attempts_leave_the_newest_sixty_four_on_disk() {
    let (_root, mut state, _clock) = opted_in();
    let mut attempts = Vec::new();
    for _ in 0..65 {
        let id = begin(&mut state, OperationKind::Repair);
        observe(&mut state, id, OperationState::Failed);
        state.finalize_summaries();
        attempts.push(queued(&state).last().unwrap().attempt_id);
    }
    let on_disk = stored(&state);
    assert_eq!(on_disk.entries.len(), 64);
    assert_eq!(on_disk.dropped.overflow, 1);
    assert_eq!(
        on_disk
            .entries
            .iter()
            .map(|entry| entry.summary.attempt_id)
            .collect::<Vec<_>>(),
        attempts[1..]
    );
}

#[test]
fn expiry_keeps_exactly_twenty_four_hours_and_drops_one_second_more() {
    let mut queue = Queue::empty(0);
    queue.push_admitted(entry(1, T0 - 1, false));
    queue.push_admitted(entry(2, T0, false));
    queue.push_admitted(entry(3, T0 + 5_000_000, false));
    assert!(!queue.expire(T0 + TTL_S - 1));
    assert_eq!(queue.entries.len(), 3, "exactly 24 h old is kept");
    assert!(queue.expire(T0 + TTL_S));
    assert_eq!(
        queue
            .entries
            .iter()
            .map(|entry| entry.summary.event_id)
            .collect::<Vec<_>>(),
        [minted(2), minted(3)],
        "24 h + 1 s is dropped; exactly 24 h and a future-dated entry are kept"
    );
    assert_eq!(queue.dropped.expired, 1);
    assert!(queue.expire(T0 + TTL_S + 1));
    assert_eq!(queue.entries.len(), 1);
    // A clock that went backwards makes an entry age zero, not ancient.
    assert!(!queue.expire(0));
    assert_eq!(queue.entries[0].summary.event_id, minted(3));
    assert_eq!(queue.dropped.expired, 2);
}

#[test]
fn expiry_runs_when_a_batch_is_taken_and_is_counted_on_disk() {
    let (_root, mut state, clock) = opted_in();
    let id = begin(&mut state, OperationKind::Repair);
    observe(&mut state, id, OperationState::Failed);
    state.finalize_summaries();
    clock.advance_s(TTL_S);
    assert!(matches!(state.summary_take_batch(), batch::Take::Ready(_)));
    clock.advance_s(1);
    assert!(matches!(state.summary_take_batch(), batch::Take::Empty));
    let on_disk = stored(&state);
    assert_eq!((on_disk.entries.len(), on_disk.dropped.expired), (0, 1));
}

/// One run on a state root whose queue file was prepared by `plant`. Returns
/// every launcher result the summary component must not influence.
type Results = (
    Result<u64, StorageError>,
    bool,
    Result<(), crate::ContractError>,
    Result<(), crate::ContractError>,
    Option<Outcome>,
    crate::Snapshot,
);
fn run_with_queue_file(plant: impl FnOnce(&Path)) -> (Results, Vec<Summary>, SummaryFaults) {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("state");
    // An earlier run opted in and selected the install directory.
    let (mut state, _clock) = configured(&directory);
    set_consent(&mut state, true);
    let install = root.path().join("install");
    state
        .save_preferences(Some(install.clone()), true, 1)
        .unwrap();
    drop(state);
    plant(&directory.join(FILE));

    let (mut state, _clock) = reopened(&directory);
    let saved = state
        .save_preferences(Some(install), true, 2)
        .map(|preferences| preferences.revision);
    let id = Uuid::new_v4();
    let admitted = state
        .admit_install(id, 0, 3, &release(), default_servers())
        .unwrap()
        .dispatch;
    let running = state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Running)
        .map(drop);
    state
        .prepare_install_result(id, Outcome::InstallFailed)
        .unwrap();
    let terminal = state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Failed)
        .map(drop);
    state.finalize_summaries();
    let mut snapshot = state.operations().snapshot().clone();
    // The operation id is random per run; everything else must match.
    snapshot.operation.as_mut().unwrap().id = Uuid::nil();
    snapshot.operation.as_mut().unwrap().intent_digest = [0; 32];
    (
        (
            saved,
            admitted,
            running,
            terminal,
            state.install_outcome().unwrap(),
            snapshot,
        ),
        queued(&state),
        state.summary_faults(),
    )
}

#[test]
fn a_broken_queue_file_never_changes_any_launcher_result() {
    let (control, control_rows, control_faults) =
        run_with_queue_file(|path| std::fs::remove_file(path).unwrap());
    assert_eq!(control.0, Ok(3));
    assert!(control.1);
    assert_eq!(control.4, Some(Outcome::InstallFailed));
    assert_eq!(control_rows.len(), 1);
    assert_eq!(control_faults, SummaryFaults::default());

    let oversized = vec![b' '; 65 * 1024];
    let plants: [(&str, Box<dyn FnOnce(&Path)>); 4] = [
        (
            "truncated JSON",
            Box::new(|path| std::fs::write(path, br#"{"schema_version":1,"gener"#).unwrap()),
        ),
        (
            "schema_version 99",
            Box::new(|path| {
                let mut queue = Queue::empty(7);
                queue.schema_version = 99;
                queue.entries.push(entry(900, T0, false));
                std::fs::write(path, serde_json::to_vec(&queue).unwrap()).unwrap();
            }),
        ),
        (
            "65 KiB",
            Box::new(move |path| std::fs::write(path, oversized).unwrap()),
        ),
        (
            "a directory",
            Box::new(|path| {
                std::fs::remove_file(path).unwrap();
                std::fs::create_dir(path).unwrap();
            }),
        ),
    ];
    for (form, plant) in plants {
        let (results, rows, faults) = run_with_queue_file(plant);
        assert_eq!(results, control, "{form}");
        // The broken content is treated as an empty queue, never as rows.
        assert_eq!(rows.len(), 1, "{form}");
        assert_eq!(rows[0].error_code, control_rows[0].error_code, "{form}");
        // Only a non-file at the path keeps the queue from being rewritten,
        // and that is counted instead of surfacing anywhere.
        assert_eq!(faults.panics, 0, "{form}");
        assert_eq!(faults.queue_writes != 0, form == "a directory", "{form}");
    }
}

#[test]
fn open_ignores_the_queue_file_entirely() {
    for bytes in [None, Some(b"{broken".as_slice())] {
        let root = tempfile::tempdir().unwrap();
        if let Some(bytes) = bytes {
            std::fs::write(root.path().join(FILE), bytes).unwrap();
        }
        let state = DesktopState::open(root.path()).unwrap();
        assert_eq!(state.inspect().preferences, crate::Preferences::default());
        assert!(!state.requires_reopen());
        // Not read, not repaired, not removed: loading waits for configuration.
        assert_eq!(std::fs::read(root.path().join(FILE)).ok().as_deref(), bytes);
    }
}
