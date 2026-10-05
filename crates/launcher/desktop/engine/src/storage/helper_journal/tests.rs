use super::*;
use crate::{
    catalog::{verify_release, VerifiedRelease},
    helper_supervisor::{Fault, Outcome},
};
use ed25519_dalek::{Signer, SigningKey};
fn release() -> VerifiedRelease {
    let body=serde_json::to_vec(&serde_json::json!({"schema":1,"seed":{"blob":"seed.zip","size":1,"sha256":"a".repeat(64)},"patches":[]})).unwrap();
    let sig = SigningKey::from_bytes(&crate::manifest::DEV_MANIFEST_PRIVKEY).sign(&body);
    let sig: String = sig.to_bytes().iter().map(|b| format!("{b:02x}")).collect();
    verify_release(&body, sig.as_bytes()).unwrap()
}
fn setup() -> (tempfile::TempDir, DesktopState, Uuid, VerifiedRelease) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    state
        .save_preferences(Some(root.path().join("game")), false, 0)
        .unwrap();
    let id = Uuid::new_v4();
    let release = release();
    state
        .admit_install_backend(AdmissionRequest {
            id,
            operation_revision: 0,
            preferences_revision: 1,
            release: &release,
            login_servers: vec![],
            backend: ExtractionBackend::Wine {
                runtime_sha256: [1; 32],
                helper_sha256: [2; 32],
            },
        })
        .unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Running)
        .unwrap();
    (root, state, id, release)
}
#[test]
fn helper_checkpoints_persist_identity_and_never_admit_duplicate_attempt() {
    let (root, mut state, id, _) = setup();
    let record = state.begin_helper(id).unwrap();
    assert_eq!(record.phase, HelperPhase::LaunchIntent);
    assert!(matches!(
        state.begin_helper(id),
        Err(IntentError::Operation(ContractError::Busy))
    ));
    assert!(state
        .finish_helper(id, record.attempt_id, Outcome::Completed)
        .is_err());
    assert!(state.record_helper_host(id, Uuid::new_v4(), 42).is_err());
    state.record_helper_host(id, record.attempt_id, 42).unwrap();
    assert_eq!(
        state.helper_record(id).unwrap().unwrap().phase,
        HelperPhase::HostStarted
    );
    state
        .finish_helper(id, record.attempt_id, Outcome::Completed)
        .unwrap();
    drop(state);
    let state = DesktopState::open(&root.path().join("state")).unwrap();
    let restored = state.helper_record(id).unwrap().unwrap();
    assert_eq!(restored.attempt_id, record.attempt_id);
    assert_eq!(
        restored.phase,
        HelperPhase::Finished {
            result: HelperResult::Completed
        }
    );
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
}
#[test]
fn reopening_any_dispatch_checkpoint_retains_recovery_gate_even_without_output() {
    for phase in 0..3 {
        let (root, mut state, id, release) = setup();
        let record = state.begin_helper(id).unwrap();
        if phase > 0 {
            state.record_helper_host(id, record.attempt_id, 42).unwrap();
        }
        if phase > 1 {
            state
                .finish_helper(
                    id,
                    record.attempt_id,
                    Outcome::ReconciliationRequired(Fault::Transport),
                )
                .unwrap();
        }
        drop(state);
        let mut state = DesktopState::open(&root.path().join("state")).unwrap();
        assert_eq!(
            state.helper_record(id).unwrap().unwrap().host_pid,
            if phase > 0 { Some(42) } else { None }
        );
        assert!(!root.path().join("game").exists());
        assert!(super::super::install_recovery::reconcile(&mut state, &release).is_err());
        assert!(state.begin_helper(id).is_err());
        assert!(state.record_helper_host(id, record.attempt_id, 43).is_err());
        assert!(state
            .finish_helper(id, record.attempt_id, Outcome::Completed)
            .is_err());
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
    }
}
#[tokio::test]
async fn native_dispatch_and_resume_refuse_wine_intent_without_mutations() {
    let (root, state, id, release) = setup();
    let before = state.operations().snapshot().clone();
    let state = Arc::new(std::sync::Mutex::new(state));
    assert!(super::super::install_worker::dispatch(state.clone(), id, release).is_err());
    assert_eq!(state.lock().unwrap().operations().snapshot(), &before);
    state
        .lock()
        .unwrap()
        .operations_mut()
        .unwrap()
        .mark_uncertain(id)
        .unwrap();
    let revision = state.lock().unwrap().operations().snapshot().revision;
    assert!(super::super::install_worker::resume(state.clone(), id, revision).is_err());
    assert!(!root.path().join("game").exists());
    assert_eq!(
        state.lock().unwrap().operations().snapshot().revision,
        revision
    );
}
#[test]
fn not_started_is_recorded_but_completed_requires_a_started_host() {
    let (_root, mut state, id, _) = setup();
    let record = state.begin_helper(id).unwrap();
    state
        .finish_helper(id, record.attempt_id, Outcome::NotStarted(Fault::Spawn))
        .unwrap();
    assert_eq!(
        state.helper_record(id).unwrap().unwrap().phase,
        HelperPhase::Finished {
            result: HelperResult::NotStarted
        }
    );
    assert!(state.record_helper_host(id, record.attempt_id, 42).is_err());
}
#[test]
fn changed_helper_evidence_fails_closed() {
    let (root, mut state, id, _) = setup();
    let mut record = state.begin_helper(id).unwrap();
    record.intent_digest[0] ^= 1;
    std::fs::write(
        root.path().join("state").join(name(id)),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        state.helper_record(id),
        Err(IntentError::Storage(StorageError::Corrupt))
    ));
    assert!(state.record_helper_host(id, record.attempt_id, 42).is_err());
}
