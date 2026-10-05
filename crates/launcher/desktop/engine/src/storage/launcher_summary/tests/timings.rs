//! Durations come only from this process's monotonic clock. A row for an attempt
//! this process did not watch from admission carries none, a lost observation
//! carries only the phases that had closed, and a launch never carries the
//! length of the game session.
use super::*;
use install_worker::Outcome;

fn phases(summary: &Summary) -> Vec<(TimedPhase, u32)> {
    summary
        .phases
        .as_ref()
        .expect("phases")
        .iter()
        .map(|entry| (entry.phase, entry.duration_ms.get()))
        .collect()
}

#[test]
fn an_install_carries_its_duration_and_each_timed_phase() {
    let (_root, mut state, clock) = opted_in();
    let id = admit_install(&mut state);
    clock.advance_ms(12);
    observe(&mut state, id, OperationState::Running);
    clock.advance_ms(40);
    state.summary_phase(id, TimedPhase::Download);
    clock.advance_ms(81_000);
    state.summary_phase(id, TimedPhase::Extraction);
    clock.advance_ms(182);
    finish_install(&mut state, id, Outcome::InstallFailed);
    // Time after the terminal commit belongs to nobody.
    clock.advance_ms(5_000);
    state.finalize_summaries();
    let rows = queued(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].duration_ms.map(Millis::get), Some(81_234));
    assert_eq!(
        phases(&rows[0]),
        [
            (TimedPhase::Starting, 12),
            (TimedPhase::Running, 40),
            (TimedPhase::Download, 81_000),
            (TimedPhase::Extraction, 182),
        ]
    );
    assert_eq!(rows[0].phase, SummaryPhase::Extraction);
}

#[test]
fn a_phase_entered_again_accumulates_and_a_repeat_of_the_open_phase_is_ignored() {
    let (_root, mut state, clock) = opted_in();
    let id = admit_install(&mut state);
    observe(&mut state, id, OperationState::Running);
    clock.advance_ms(5);
    state.summary_phase(id, TimedPhase::Download);
    clock.advance_ms(100);
    state.summary_phase(id, TimedPhase::Download);
    clock.advance_ms(100);
    state.summary_phase(id, TimedPhase::Extraction);
    clock.advance_ms(30);
    state.summary_phase(id, TimedPhase::Download);
    clock.advance_ms(7);
    state.operations_mut().unwrap().request_cancel(id).unwrap();
    finish_install(&mut state, id, Outcome::Cancelled);
    state.finalize_summaries();
    let rows = queued(&state);
    assert_eq!(
        phases(&rows[0]),
        [
            (TimedPhase::Starting, 0),
            (TimedPhase::Running, 5),
            (TimedPhase::Download, 207),
            (TimedPhase::Extraction, 30),
        ]
    );
    assert_eq!(rows[0].phase, SummaryPhase::Download);
    assert_eq!(rows[0].duration_ms.map(Millis::get), Some(242));
    assert_eq!(rows[0].outcome, SummaryOutcome::Cancelled);
}

#[test]
fn other_kinds_carry_starting_and_running_only() {
    let (_root, mut state, clock) = opted_in();
    let id = begin(&mut state, OperationKind::Repair);
    clock.advance_ms(900);
    observe(&mut state, id, OperationState::Running);
    clock.advance_ms(3_599_100);
    observe(&mut state, id, OperationState::Succeeded);
    state.finalize_summaries();
    let rows = queued(&state);
    assert_eq!(
        phases(&rows[0]),
        [
            (TimedPhase::Starting, 900),
            (TimedPhase::Running, 3_599_100)
        ]
    );
    assert_eq!(rows[0].duration_ms.map(Millis::get), Some(3_600_000));
    assert_eq!(rows[0].phase, SummaryPhase::Running);
}

// `running` of a launch lasts until the game process exits, so it and the total
// would be the length of the play session. Neither is exported.
#[test]
fn a_launch_exports_its_starting_phase_and_no_game_session_length() {
    // Positive control: an install with the same clock carries both.
    for kind in [OperationKind::Install, OperationKind::Launch] {
        for (end, outcome) in [
            (OperationState::Succeeded, SummaryOutcome::Succeeded),
            (OperationState::Failed, SummaryOutcome::Failed),
        ] {
            let (_root, mut state, clock) = opted_in();
            let id = begin(&mut state, kind);
            clock.advance_ms(900);
            observe(&mut state, id, OperationState::Running);
            // The game is played for an hour, then its process exits.
            clock.advance_ms(3_599_100);
            observe(&mut state, id, end);
            state.finalize_summaries();
            let rows = queued(&state);
            assert_eq!(rows.len(), 1, "{kind:?} {end:?}");
            assert_eq!(rows[0].outcome, outcome);
            // Where the attempt ended is said either way.
            assert_eq!(rows[0].phase, SummaryPhase::Running, "{kind:?} {end:?}");
            let json = serde_json::to_value(&rows[0]).unwrap();
            if kind == OperationKind::Launch {
                assert_eq!(phases(&rows[0]), [(TimedPhase::Starting, 900)]);
                assert_eq!(rows[0].duration_ms, None);
                assert!(json.get("duration_ms").is_none());
                assert!(!json.to_string().contains("3599100"), "{json}");
            } else {
                assert_eq!(
                    phases(&rows[0]),
                    [
                        (TimedPhase::Starting, 900),
                        (TimedPhase::Running, 3_599_100)
                    ]
                );
                assert_eq!(rows[0].duration_ms.map(Millis::get), Some(3_600_000));
                assert_eq!(json["duration_ms"], 3_600_000);
            }
            // Still a valid wire row.
            assert_eq!(serde_json::from_value::<Summary>(json).unwrap(), rows[0]);
        }
    }
}

#[test]
fn a_launch_that_never_ran_carries_its_starting_phase_only() {
    let (_root, mut state, clock) = opted_in();
    let id = begin(&mut state, OperationKind::Launch);
    clock.advance_ms(250);
    observe(&mut state, id, OperationState::Failed);
    state.finalize_summaries();
    let rows = queued(&state);
    assert_eq!(phases(&rows[0]), [(TimedPhase::Starting, 250)]);
    assert_eq!(rows[0].duration_ms, None);
    assert_eq!(rows[0].phase, SummaryPhase::Starting);
}

#[test]
fn a_phase_for_any_other_operation_is_ignored() {
    // Positive control: the same call with the tracked id adds the phase.
    for tracked in [true, false] {
        let (_root, mut state, clock) = opted_in();
        let id = admit_install(&mut state);
        clock.advance_ms(10);
        state.summary_phase(
            if tracked { id } else { Uuid::new_v4() },
            TimedPhase::Download,
        );
        clock.advance_ms(10);
        observe(&mut state, id, OperationState::Failed);
        state.finalize_summaries();
        let expected: &[(TimedPhase, u32)] = if tracked {
            &[(TimedPhase::Starting, 10), (TimedPhase::Download, 10)]
        } else {
            &[(TimedPhase::Starting, 20)]
        };
        assert_eq!(phases(&queued(&state)[0]), expected);
    }
}

#[test]
fn a_duration_beyond_seven_days_saturates_at_the_wire_maximum() {
    let (_root, mut state, clock) = opted_in();
    let id = begin(&mut state, OperationKind::Repair);
    clock.advance_ms(8 * 24 * 60 * 60 * 1000);
    observe(&mut state, id, OperationState::Failed);
    state.finalize_summaries();
    let rows = queued(&state);
    assert_eq!(rows[0].duration_ms, Some(Millis::MAX));
    assert_eq!(
        phases(&rows[0]),
        [(TimedPhase::Starting, Millis::MAX.get())]
    );
    // Still a valid wire row.
    let json = serde_json::to_value(&rows[0]).unwrap();
    assert_eq!(serde_json::from_value::<Summary>(json).unwrap(), rows[0]);
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum End {
    Observed,
    LostInProcess,
    LostByRestart,
}

#[test]
fn a_lost_observation_carries_no_duration_and_no_phase_that_was_still_open() {
    for end in [End::Observed, End::LostInProcess, End::LostByRestart] {
        let (root, mut state, clock) = opted_in();
        let id = admit_install(&mut state);
        clock.advance_ms(12);
        observe(&mut state, id, OperationState::Running);
        clock.advance_ms(3);
        state.summary_phase(id, TimedPhase::Download);
        clock.advance_ms(40);
        match end {
            End::Observed => finish_install(&mut state, id, Outcome::InstallFailed),
            End::LostInProcess => {
                state.operations_mut().unwrap().mark_uncertain(id).unwrap();
            }
            End::LostByRestart => {
                drop(state);
                (state, _) = reopened(&root.path().join("state"));
            }
        }
        state.finalize_summaries();
        let rows = queued(&state);
        assert_eq!(rows.len(), 1, "{end:?}");
        let json = serde_json::to_value(&rows[0]).unwrap();
        match end {
            // Positive control: an observed end carries the total and the phase
            // it ended in.
            End::Observed => {
                assert_eq!(rows[0].outcome, SummaryOutcome::Failed);
                assert_eq!(rows[0].duration_ms.map(Millis::get), Some(55));
                assert_eq!(
                    phases(&rows[0]),
                    [
                        (TimedPhase::Starting, 12),
                        (TimedPhase::Running, 3),
                        (TimedPhase::Download, 40),
                    ]
                );
                assert_eq!(rows[0].phase, SummaryPhase::Download);
            }
            // Nobody saw the attempt or `download` end, so neither has a
            // duration. The phases that had closed keep theirs.
            End::LostInProcess => {
                assert_eq!(rows[0].outcome, SummaryOutcome::Unknown);
                assert_eq!(rows[0].duration_ms, None);
                assert!(json.get("duration_ms").is_none());
                assert_eq!(
                    phases(&rows[0]),
                    [(TimedPhase::Starting, 12), (TimedPhase::Running, 3)]
                );
                assert_eq!(rows[0].phase, SummaryPhase::Download);
            }
            End::LostByRestart => {
                assert_eq!(rows[0].outcome, SummaryOutcome::Unknown);
                assert_eq!((rows[0].duration_ms, rows[0].phases.clone()), (None, None));
                assert!(json.get("duration_ms").is_none());
                assert!(json.get("phases").is_none());
                assert_eq!(rows[0].phase, SummaryPhase::None);
            }
        }
        assert_eq!(serde_json::from_value::<Summary>(json).unwrap(), rows[0]);
    }
}

#[test]
fn a_lost_observation_drops_the_open_phase_even_when_it_was_entered_before() {
    let (_root, mut state, clock) = opted_in();
    let id = admit_install(&mut state);
    observe(&mut state, id, OperationState::Running);
    clock.advance_ms(5);
    state.summary_phase(id, TimedPhase::Download);
    clock.advance_ms(100);
    state.summary_phase(id, TimedPhase::Extraction);
    clock.advance_ms(30);
    // A patch download is open again when observation is lost.
    state.summary_phase(id, TimedPhase::Download);
    clock.advance_ms(7);
    state.operations_mut().unwrap().mark_uncertain(id).unwrap();
    state.finalize_summaries();
    let rows = queued(&state);
    assert_eq!(
        phases(&rows[0]),
        [
            (TimedPhase::Starting, 0),
            (TimedPhase::Running, 5),
            (TimedPhase::Extraction, 30),
        ]
    );
    assert_eq!(rows[0].duration_ms, None);
    assert_eq!(rows[0].phase, SummaryPhase::Download);
}

#[test]
fn an_attempt_lost_in_its_first_phase_has_no_phases_key() {
    // Positive control: the same attempt, failing instead, has the key.
    for lost in [false, true] {
        let (_root, mut state, clock) = opted_in();
        let id = begin(&mut state, OperationKind::Repair);
        clock.advance_ms(12);
        if lost {
            state.operations_mut().unwrap().mark_uncertain(id).unwrap();
        } else {
            observe(&mut state, id, OperationState::Failed);
        }
        state.finalize_summaries();
        let rows = queued(&state);
        assert_eq!(rows[0].phase, SummaryPhase::Starting);
        let json = serde_json::to_value(&rows[0]).unwrap();
        assert_eq!(json.get("phases").is_some(), !lost);
        assert_eq!(json.get("duration_ms").is_some(), !lost);
        assert_eq!(rows[0].phases.is_some(), !lost);
        assert_eq!(serde_json::from_value::<Summary>(json).unwrap(), rows[0]);
    }
}
