//! The launch failure field and spawn classification (PD-05, C-PD3).
use super::*;
use std::io::ErrorKind;

#[test]
fn spawn_errors_classify_to_their_failure_codes() {
    for (kind, raw, macos, expected) in [
        (
            ErrorKind::Other,
            Some(86),
            true,
            FailureCode::RosettaMissing,
        ),
        // EBADARCH means Rosetta only on macOS; elsewhere 86 is another error.
        (ErrorKind::Other, Some(86), false, FailureCode::SpawnFailed),
        (
            ErrorKind::NotFound,
            Some(2),
            true,
            FailureCode::SpawnNotFound,
        ),
        (ErrorKind::NotFound, None, false, FailureCode::SpawnNotFound),
        (
            ErrorKind::PermissionDenied,
            Some(13),
            false,
            FailureCode::SpawnPermissionDenied,
        ),
        (ErrorKind::Other, None, true, FailureCode::SpawnFailed),
    ] {
        assert_eq!(
            classify_spawn(kind, raw, macos),
            expected,
            "{kind:?} {raw:?} {macos}"
        );
    }
}

#[test]
fn a_noted_failure_survives_the_terminal_record() {
    let (_root, mut state, plan) = super::super::tests::fixture();
    state.record_launch(&plan, Observation::Preparing).unwrap();
    state
        .note_launch_failure(&plan, FailureCode::GameFilesInvalid)
        .unwrap();
    // Noting changes only the failure, never the observation.
    assert_eq!(state.launch_observation(), Ok(Some(Observation::Preparing)));
    state.record_launch(&plan, Observation::NotStarted).unwrap();
    assert_eq!(
        state.launch_observation(),
        Ok(Some(Observation::NotStarted))
    );
    assert_eq!(
        state.launch_failure(),
        Ok(Some(FailureCode::GameFilesInvalid))
    );
}

#[test]
fn an_old_record_without_failure_reads_as_none() {
    let (_root, mut state, plan) = super::super::tests::fixture();
    let digest = plan.digest().unwrap();
    atomic::write(
        state.state_root(),
        &format!("launch-result-{}.json", plan.id),
        &serde_json::json!({
            "id": plan.id,
            "digest": digest,
            "observation": {"phase": "not_started"},
        }),
    )
    .unwrap();
    assert_eq!(
        state.launch_observation(),
        Ok(Some(Observation::NotStarted))
    );
    assert_eq!(state.launch_failure(), Ok(None));
    // The record is still a launch record: noting a failure on it works.
    state
        .note_launch_failure(&plan, FailureCode::SpawnNotFound)
        .unwrap();
    assert_eq!(state.launch_failure(), Ok(Some(FailureCode::SpawnNotFound)));
}

#[test]
fn a_not_started_record_is_one_write_with_its_failure() {
    let (_root, mut state, plan) = super::super::tests::fixture();
    state
        .record_not_started(&plan, FailureCode::LocalIo)
        .unwrap();
    assert_eq!(
        state.launch_observation(),
        Ok(Some(Observation::NotStarted))
    );
    assert_eq!(state.launch_failure(), Ok(Some(FailureCode::LocalIo)));
    let mut other = plan.clone();
    other.id = Uuid::new_v4();
    assert!(state
        .record_not_started(&other, FailureCode::LocalIo)
        .is_err());
}
