//! Launch rows. A launch that succeeds means the observed game process exited
//! with code 0; nothing here spawns a game.
use super::*;
use crate::storage::{atomic, runtime_setup};
use launch::{Artifact, Plan, Resources};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Mutex;

// The launch module's own digest: the hash of the serialized plan.
fn digest(plan: &Plan) -> [u8; 32] {
    Sha256::digest(serde_json::to_vec(plan).unwrap()).into()
}

/// A prepared installation with consent given, then a launch admitted through
/// the journal with the plan file the launch module reads back.
fn admitted() -> (tempfile::TempDir, DesktopState, Clock, Plan) {
    let (root, mut state, installation) = runtime_setup::tests::fixture_with_runtime([7; 32]);
    let clock = Clock::install(&mut state);
    state.configure_summaries(config(Some(ENDPOINT)));
    set_consent(&mut state, true);
    let helper = root.path().canonicalize().unwrap().join("helper.exe");
    std::fs::write(&helper, b"fixture").unwrap();
    let hash: String = Sha256::digest(b"fixture")
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let plan = Plan {
        id: Uuid::new_v4(),
        installation,
        runtime: None,
        resources: Resources {
            helper: Artifact::open(helper, &hash).unwrap(),
            client_patches: None,
            graphics: None,
        },
    };
    atomic::write(
        state.state_root(),
        &format!("launch-plan-{}.json", plan.id),
        &plan,
    )
    .unwrap();
    let revision = state.operations().snapshot().revision;
    state
        .operations_mut()
        .unwrap()
        .begin(plan.id, OperationKind::Launch, digest(&plan), revision)
        .unwrap();
    assert_eq!(state.launch_plan().unwrap(), Some(plan.clone()));
    (root, state, clock, plan)
}

async fn terminal(state: &Arc<Mutex<DesktopState>>) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let ended = state
                .lock()
                .unwrap()
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state
                .terminal();
            if ended {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("launch worker commits its terminal");
}

#[tokio::test]
async fn a_launch_that_never_starts_is_one_failed_row_through_the_real_worker() {
    let (_root, state, _clock, plan) = admitted();
    // A replaced helper fails verification before anything is spawned.
    std::fs::write(plan.resources.helper.path(), b"replacement").unwrap();
    let state = Arc::new(Mutex::new(state));
    drop(launch::dispatch(state.clone(), plan.id).unwrap());
    terminal(&state).await;
    let mut owner = state.lock().unwrap();
    owner.finalize_summaries();
    let rows = queued(&owner);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].operation, SummaryOperation::Launch);
    assert_eq!(rows[0].outcome, SummaryOutcome::Failed);
    assert_eq!(rows[0].error_code, Some(SummaryErrorCode::LaunchNotStarted));
    assert_eq!(rows[0].phase, SummaryPhase::Starting);
    // The exported ids are engine-minted, never the plan's operation id.
    assert_ne!(rows[0].attempt_id, plan.id);
    assert_ne!(rows[0].event_id, plan.id);
}

#[tokio::test]
async fn a_launch_cancelled_before_dispatch_is_one_cancelled_row() {
    let (_root, state, _clock, plan) = admitted();
    let state = Arc::new(Mutex::new(state));
    let worker = launch::dispatch(state.clone(), plan.id).unwrap();
    worker.request_cancel().unwrap();
    terminal(&state).await;
    let mut owner = state.lock().unwrap();
    owner.finalize_summaries();
    let rows = queued(&owner);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].operation, rows[0].outcome, rows[0].error_code),
        (SummaryOperation::Launch, SummaryOutcome::Cancelled, None)
    );
}

#[test]
fn launch_exit_observations_map_to_their_closed_codes() {
    use SummaryErrorCode as Code;
    // The supervisor's exit record as (code, early), if it wrote one.
    for (record, expected, error) in [
        (Some((0, false)), OperationState::Succeeded, None),
        (
            Some((3, true)),
            OperationState::Failed,
            Some(Code::LaunchEarlyExit),
        ),
        (
            Some((3, false)),
            OperationState::Failed,
            Some(Code::LaunchExitNonzero),
        ),
        (
            Some((-1, false)),
            OperationState::Failed,
            Some(Code::LaunchExitNonzero),
        ),
        // A failure the record does not explain is never given a cause: the
        // process exited with 0, or there is no record at all.
        (
            Some((0, false)),
            OperationState::Failed,
            Some(Code::Unspecified),
        ),
        (
            Some((0, true)),
            OperationState::Failed,
            Some(Code::Unspecified),
        ),
        (None, OperationState::Failed, Some(Code::Unspecified)),
    ] {
        let case = format!("{record:?} {expected:?}");
        let (_root, mut state, _clock, plan) = admitted();
        observe(&mut state, plan.id, OperationState::Running);
        if let Some((code, early)) = record {
            // The supervisor's result record, in the shape `launch_observation` reads.
            atomic::write(
                state.state_root(),
                &format!("launch-result-{}.json", plan.id),
                &json!({
                    "id": plan.id,
                    "digest": digest(&plan),
                    "observation": {
                        "phase": "process_exited",
                        "host_pid": 41,
                        "guest_pid": 80,
                        "code": code,
                        "early": early,
                    },
                }),
            )
            .unwrap();
        }
        observe(&mut state, plan.id, expected);
        match (record, state.launch_observation().unwrap()) {
            (Some(_), Some(launch::Observation::ProcessExited { .. })) | (None, None) => (),
            (_, observed) => panic!("{case}: {observed:?}"),
        }
        state.finalize_summaries();
        let rows = queued(&state);
        assert_eq!(rows.len(), 1, "{case}");
        assert_eq!(rows[0].operation, SummaryOperation::Launch);
        assert_eq!(rows[0].phase, SummaryPhase::Running);
        assert_eq!(rows[0].error_code, error, "{case}");
        assert_eq!(
            rows[0].outcome,
            if error.is_some() {
                SummaryOutcome::Failed
            } else {
                SummaryOutcome::Succeeded
            }
        );
    }
}

#[test]
fn a_launch_whose_observation_was_lost_is_unknown_never_succeeded() {
    let (_root, mut state, _clock, plan) = admitted();
    observe(&mut state, plan.id, OperationState::Running);
    state
        .operations_mut()
        .unwrap()
        .mark_uncertain(plan.id)
        .unwrap();
    state.finalize_summaries();
    let rows = queued(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].operation, rows[0].outcome, rows[0].error_code),
        (SummaryOperation::Launch, SummaryOutcome::Unknown, None)
    );
}
