use super::*;
use crate::OperationState;
#[test]
fn repair_binds_original_owner_release_and_separate_work_without_touching_content() {
    let (root, mut state, installed) = crate::runtime_setup::tests::fixture_with_runtime([7; 32]);
    state
        .save_preferences(Some(root.path().join("other")), true, 1)
        .unwrap();
    let preferences = state.preferences().clone();
    let old = installed.destination.join("game/Working/Binaries/SGW.exe");
    std::fs::write(&old, b"user modified content").unwrap();
    let id = Uuid::new_v4();
    let revision = state.operations().snapshot().revision;
    assert!(state
        .admit_repair(id, revision, installed.operation_id, false)
        .is_err());
    assert!(state.repair_plan().unwrap().is_none());
    let admitted = state
        .admit_repair(id, revision, installed.operation_id, true)
        .unwrap();
    assert!(admitted.dispatch);
    assert_eq!(admitted.plan.installation, installed);
    assert!(admitted.plan.original_present);
    assert!(!admitted.plan.stage().exists());
    assert!(!admitted.plan.backup().exists());
    assert_eq!(std::fs::read(&old).unwrap(), b"user modified content");
    assert_eq!(state.preferences(), &preferences);
    assert!(
        !state
            .admit_repair(id, revision, installed.operation_id, true)
            .unwrap()
            .dispatch
    );
    assert!(state.install_intent().unwrap().is_none());
    drop(state);
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    assert_eq!(state.repair_plan().unwrap().unwrap(), admitted.plan);
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
    assert!(
        !state
            .admit_repair(id, revision, installed.operation_id, true)
            .unwrap()
            .dispatch
    );
}
#[test]
fn missing_game_is_recorded_and_conflicting_work_paths_are_not_adopted() {
    let (_root, mut state, installed) = crate::runtime_setup::tests::fixture_with_runtime([7; 32]);
    std::fs::remove_dir_all(installed.destination.join("game")).unwrap();
    let id = Uuid::new_v4();
    let revision = state.operations().snapshot().revision;
    let occupied = installed.destination.join(format!(".cimmeria-repair-{id}"));
    std::fs::create_dir(&occupied).unwrap();
    assert!(state
        .admit_repair(id, revision, installed.operation_id, true)
        .is_err());
    assert_eq!(state.operations().snapshot().revision, revision);
    std::fs::remove_dir(occupied).unwrap();
    let plan = state
        .admit_repair(id, revision, installed.operation_id, true)
        .unwrap()
        .plan;
    assert!(!plan.original_present);
    assert!(!plan.stage().exists());
    let mut changed = plan.clone();
    changed.installation.operation_id = id;
    atomic::write(&state.directory.root, &name(id), &changed).unwrap();
    assert!(state.repair_plan().is_err());
}
#[test]
fn stale_identity_and_foreign_owner_lock_cannot_admit_repair() {
    let (_root, mut state, installed) = crate::runtime_setup::tests::fixture_with_runtime([7; 32]);
    let revision = state.operations().snapshot().revision;
    for (id, rev, owner) in [
        (Uuid::new_v4(), revision - 1, installed.operation_id),
        (Uuid::nil(), revision, installed.operation_id),
        (Uuid::new_v4(), revision, Uuid::new_v4()),
    ] {
        assert!(state.admit_repair(id, rev, owner, true).is_err());
    }
    let lock = lock_owner(&installed).unwrap();
    let duplicate = lock.try_clone().unwrap();
    assert!(state
        .admit_repair(Uuid::new_v4(), revision, installed.operation_id, true)
        .is_err());
    assert_eq!(state.operations().snapshot().revision, revision);
    drop(lock);
    assert!(
        state
            .admit_repair(Uuid::new_v4(), revision, installed.operation_id, true)
            .unwrap()
            .dispatch
    );
    drop(duplicate);
}
