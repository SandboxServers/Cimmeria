use super::*;
use crate::OperationState;

fn target() -> VerifiedRelease {
    install_worker::tests::verified(&install_worker::tests::archive(true))
}
fn request<'a>(
    state: &DesktopState,
    owner: &InstallIntent,
    target: &'a VerifiedRelease,
    id: Uuid,
) -> Request<'a> {
    Request {
        id,
        operation_revision: state.operations().snapshot().revision,
        installation_id: owner.operation_id,
        expected_current: owner.release_identity(),
        target,
        confirmed: true,
    }
}

#[test]
fn update_admission_retains_both_releases_without_mutating_owner_or_game_and_reopens() {
    let (root, mut state, owner) = runtime_setup::tests::fixture_with_runtime([7; 32]);
    let target = target();
    let id = Uuid::new_v4();
    let game = owner.destination.join("game/Working/Binaries/SGW.exe");
    std::fs::write(&game, b"user modifications awaiting confirmed replacement").unwrap();
    let marker = std::fs::read(owner.destination.join(".cimmeria-install.json")).unwrap();
    let original = std::fs::read(owner.destination.join("content-ready.json")).unwrap();
    let preferences = state.preferences().clone();
    let input = request(&state, &owner, &target, id);
    let revision = input.operation_revision;
    let admitted = state.admit_update(input).unwrap();
    assert!(admitted.dispatch);
    assert_eq!(admitted.plan.owner, owner);
    assert_eq!(admitted.plan.previous, owner.release_identity());
    assert_eq!(admitted.plan.target.manifest_digest, target.digest());
    assert!(!admitted.plan.work_directory().exists());
    assert!(!admitted.plan.backup().exists());
    assert_eq!(state.preferences(), &preferences);
    assert_eq!(
        std::fs::read(&game).unwrap(),
        b"user modifications awaiting confirmed replacement"
    );
    assert_eq!(
        std::fs::read(owner.destination.join(".cimmeria-install.json")).unwrap(),
        marker
    );
    assert_eq!(
        std::fs::read(owner.destination.join("content-ready.json")).unwrap(),
        original
    );
    assert_eq!(
        state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .kind,
        OperationKind::Update
    );
    assert!(state.repair_plan().unwrap().is_none());
    let mut retry = request(&state, &owner, &target, id);
    retry.operation_revision = revision;
    assert!(!state.admit_update(retry).unwrap().dispatch);
    drop(state);
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    assert_eq!(state.update_plan().unwrap().unwrap(), admitted.plan);
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
    let retry = request(&state, &owner, &target, id);
    assert!(!state.admit_update(retry).unwrap().dispatch);
    assert_eq!(
        state.installed_content().unwrap().unwrap().current_release,
        owner.release_identity()
    );
}

#[test]
fn stale_confirmation_wrong_release_and_occupied_paths_never_admit() {
    let (root, mut state, owner) = runtime_setup::tests::fixture_with_runtime([7; 32]);
    let target = target();
    let revision = state.operations().snapshot().revision;
    for variant in 0..5 {
        let id = Uuid::new_v4();
        let mut input = request(&state, &owner, &target, id);
        match variant {
            0 => input.confirmed = false,
            1 => input.operation_revision -= 1,
            2 => input.installation_id = Uuid::new_v4(),
            3 => input.expected_current.evidence_id = Uuid::new_v4(),
            _ => input.id = Uuid::nil(),
        }
        assert!(state.admit_update(input).is_err());
        assert_eq!(state.operations().snapshot().revision, revision);
        assert!(!root.path().join("state").join(name(id)).exists());
    }
    let id = Uuid::new_v4();
    let collision = root
        .path()
        .join("state")
        .join(format!("release-evidence-{id}.bin"));
    std::fs::write(&collision, b"unrelated retained bytes").unwrap();
    let input = request(&state, &owner, &target, id);
    assert!(state.admit_update(input).is_err());
    assert_eq!(
        std::fs::read(collision).unwrap(),
        b"unrelated retained bytes"
    );
    assert_eq!(state.operations().snapshot().revision, revision);
    let old = state
        .verify_release_identity(owner.release_identity())
        .unwrap();
    let input = request(&state, &owner, &old, Uuid::new_v4());
    assert!(state.admit_update(input).is_err());
}

#[test]
fn update_reopen_reverifies_both_signed_releases_and_refuses_tampered_plan() {
    let (root, mut state, owner) = runtime_setup::tests::fixture_with_runtime([7; 32]);
    let target = target();
    let id = Uuid::new_v4();
    let input = request(&state, &owner, &target, id);
    let plan = state.admit_update(input).unwrap().plan;
    for evidence in [owner.operation_id, id] {
        let file = root
            .path()
            .join("state")
            .join(format!("release-evidence-{evidence}.bin"));
        let original = std::fs::read(&file).unwrap();
        std::fs::write(&file, b"invalid signed evidence").unwrap();
        assert!(state.update_plan().is_err());
        std::fs::write(file, original).unwrap();
    }
    let mut changed = plan;
    changed.previous.manifest_digest = changed.target.manifest_digest;
    atomic::write(&state.directory.root, &name(id), &changed).unwrap();
    assert!(state.update_plan().is_err());
}
