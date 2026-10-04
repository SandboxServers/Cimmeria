use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Barrier, Mutex,
};

#[derive(Clone, Default)]
struct Store {
    fail: Arc<AtomicBool>,
    written: Arc<Mutex<Vec<Snapshot>>>,
}
impl Journal for Store {
    fn commit(&mut self, snapshot: &Snapshot) -> Result<(), ContractError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(ContractError::PersistenceFailed);
        }
        self.written.lock().unwrap().push(snapshot.clone());
        Ok(())
    }
}
fn fresh() -> Operations<Store> {
    Operations::restore(Snapshot::default(), Store::default()).unwrap()
}
fn begin(controller: &mut Operations<Store>, id: Uuid) -> Result<(Snapshot, bool), ContractError> {
    controller.begin(
        id,
        OperationKind::Install,
        [7; 32],
        controller.snapshot().revision,
    )
}

#[test]
fn retry_does_not_admit_twice_or_change_configuration() {
    let mut c = fresh();
    let id = Uuid::new_v4();
    assert!(begin(&mut c, id).unwrap().1);
    assert!(!c.begin(id, OperationKind::Install, [7; 32], 0).unwrap().1);
    assert_eq!(
        c.begin(id, OperationKind::Install, [8; 32], 0),
        Err(ContractError::IdentityConflict)
    );
    c.observe(id, OperationState::Succeeded).unwrap();
    assert!(!c.begin(id, OperationKind::Install, [7; 32], 0).unwrap().1);
    assert_eq!(c.snapshot().revision, 2);
}

#[test]
fn old_retry_cannot_replay_after_a_new_operation() {
    let mut c = fresh();
    let old = Uuid::new_v4();
    begin(&mut c, old).unwrap();
    c.observe(old, OperationState::Succeeded).unwrap();
    begin(&mut c, Uuid::new_v4()).unwrap();
    assert_eq!(
        c.begin(old, OperationKind::Install, [7; 32], 0),
        Err(ContractError::StaleRevision)
    );
}

#[test]
fn cancellation_keeps_ownership_until_native_acknowledgement() {
    let mut c = fresh();
    let id = Uuid::new_v4();
    begin(&mut c, id).unwrap();
    assert_eq!(
        c.observe(id, OperationState::Cancelled),
        Err(ContractError::InvalidTransition)
    );
    let pending = c.request_cancel(id).unwrap();
    assert_eq!(
        pending.operation.unwrap().state,
        OperationState::CancelRequested
    );
    assert_eq!(begin(&mut c, Uuid::new_v4()), Err(ContractError::Busy));
    assert_eq!(c.request_cancel(id).unwrap().revision, 2);
    c.observe(id, OperationState::Cancelled).unwrap();
    assert!(begin(&mut c, Uuid::new_v4()).unwrap().1);
}

#[test]
fn completion_wins_cancel_race_and_terminal_state_is_immutable() {
    let mut c = fresh();
    let id = Uuid::new_v4();
    begin(&mut c, id).unwrap();
    c.request_cancel(id).unwrap();
    let result = c.observe(id, OperationState::Succeeded).unwrap();
    assert_eq!(c.request_cancel(id).unwrap(), result);
    assert_eq!(
        c.observe(id, OperationState::Failed),
        Err(ContractError::InvalidTransition)
    );
}

#[test]
fn failed_persistence_does_not_admit_or_publish() {
    let store = Store::default();
    let mut c = Operations::restore(Snapshot::default(), store.clone()).unwrap();
    store.fail.store(true, Ordering::SeqCst);
    assert_eq!(
        begin(&mut c, Uuid::new_v4()),
        Err(ContractError::PersistenceFailed)
    );
    assert_eq!(c.snapshot(), &Snapshot::default());
    store.fail.store(false, Ordering::SeqCst);
    let id = Uuid::new_v4();
    begin(&mut c, id).unwrap();
    store.fail.store(true, Ordering::SeqCst);
    assert_eq!(
        c.observe(id, OperationState::Succeeded),
        Err(ContractError::PersistenceFailed)
    );
    assert_eq!(
        c.snapshot().operation.as_ref().unwrap().state,
        OperationState::Starting
    );
}

#[test]
fn restore_requires_inspection_and_never_replays_mutation() {
    let mut c = fresh();
    let id = Uuid::new_v4();
    begin(&mut c, id).unwrap();
    let mut restored = Operations::restore(c.snapshot().clone(), Store::default()).unwrap();
    assert_eq!(restored.snapshot().revision, 2);
    assert_eq!(
        restored.snapshot().operation.as_ref().unwrap().state,
        OperationState::ReconciliationRequired
    );
    assert_eq!(
        begin(&mut restored, Uuid::new_v4()),
        Err(ContractError::Busy)
    );
    assert_eq!(
        restored.observe(id, OperationState::Succeeded),
        Err(ContractError::InvalidTransition)
    );
    restored.reconcile(id, OperationState::Failed).unwrap();
    assert!(begin(&mut restored, Uuid::new_v4()).unwrap().1);
}

#[test]
fn concurrent_admission_under_adapter_lock_has_one_owner() {
    let controller = Arc::new(Mutex::new(fresh()));
    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let controller = controller.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                begin(&mut controller.lock().unwrap(), Uuid::new_v4())
            })
        })
        .collect();
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == Err(ContractError::Busy))
            .count(),
        1
    );
}

#[test]
fn schema_and_numeric_boundaries_fail_closed() {
    let mut snapshot = Snapshot {
        schema_version: 99,
        ..Snapshot::default()
    };
    assert!(matches!(
        Operations::restore(snapshot.clone(), Store::default()),
        Err(ContractError::UnsupportedSchema)
    ));
    snapshot.schema_version = SCHEMA_VERSION;
    snapshot.revision = MAX_REVISION;
    let mut c = Operations::restore(snapshot.clone(), Store::default()).unwrap();
    assert_eq!(
        begin(&mut c, Uuid::new_v4()),
        Err(ContractError::InvalidRevision)
    );
    assert_eq!(c.snapshot(), &snapshot);
}

#[test]
fn json_contract_has_explicit_version_and_stable_state_tags() {
    assert_eq!(
        serde_json::to_string(&Snapshot::default()).unwrap(),
        r#"{"schema_version":1,"revision":0,"operation":null}"#
    );
    assert_eq!(
        serde_json::to_string(&OperationState::CancelRequested).unwrap(),
        "\"cancel_requested\""
    );
    assert!(serde_json::from_str::<Snapshot>(
        r#"{"schema_version":1,"revision":0,"operation":null,"surprise":true}"#
    )
    .is_err());
}

#[test]
fn uncertain_commit_blocks_all_commands_until_reopened() {
    struct Uncertain;
    impl Journal for Uncertain {
        fn commit(&mut self, _: &Snapshot) -> Result<(), ContractError> {
            Err(ContractError::PersistenceUncertain)
        }
    }
    let mut c = Operations::restore(Snapshot::default(), Uncertain).unwrap();
    let id = Uuid::new_v4();
    assert_eq!(
        c.begin(id, OperationKind::Install, [1; 32], 0),
        Err(ContractError::PersistenceUncertain)
    );
    assert!(c.requires_reopen());
    assert_eq!(
        c.begin(Uuid::new_v4(), OperationKind::Launch, [1; 32], 0),
        Err(ContractError::PersistenceUncertain)
    );
    assert_eq!(
        c.request_cancel(id),
        Err(ContractError::PersistenceUncertain)
    );
    assert_eq!(
        c.observe(id, OperationState::Succeeded),
        Err(ContractError::PersistenceUncertain)
    );
    assert_eq!(
        c.reconcile(id, OperationState::Failed),
        Err(ContractError::PersistenceUncertain)
    );
}
