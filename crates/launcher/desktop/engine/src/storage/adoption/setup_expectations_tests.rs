//! Real Light adoption interruptions must never overwrite later setup edits.
use super::in_place_tests::commit_with;
use super::light_tests::{write, Light};
use super::*;
const EXE: &str = "Working/Binaries/SGW.exe";
const LOGIN: &str = "Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua";
fn reopen(mut f: Light) -> Light {
    drop(f.state);
    f.state = Arc::new(Mutex::new(
        DesktopState::open(&f.root.path().join("state")).unwrap(),
    ));
    f
}
fn plan(f: &Light) -> Plan {
    let state = f.state.lock().unwrap();
    let id = state.operations().snapshot().operation.as_ref().unwrap().id;
    inspect(&state, id).unwrap().plan
}
fn action(f: &Light, rollback: bool) -> Result<(), Error> {
    let mut state = f.state.lock().unwrap();
    let snapshot = state.operations().snapshot();
    let id = snapshot.operation.as_ref().unwrap().id;
    let revision = snapshot.revision;
    if rollback {
        abandon(&mut state, id, revision)
    } else {
        recover(&mut state, id, revision)
    }
}
fn stop(f: &Light) {
    assert!(commit_with(f.preview().unwrap(), |p| {
        if p == publication::Point::SetUp {
            Err(StorageError::Io.into())
        } else {
            Ok(())
        }
    })
    .is_err());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn setup_recovery_and_abandon_preserve_edited_login_and_executable() {
    for key in [LOGIN, EXE] {
        for rollback in [false, true] {
            let f = Light::new(vec![], &[(LOGIN, b"original login")]).await;
            stop(&f);
            let plan = plan(&f);
            let saved = inventory::native(
                &plan.stage().join(super::in_place_check::SETUP_ORIGINALS),
                key,
            );
            let original = std::fs::read(&saved).unwrap();
            let mut edit = std::fs::read(inventory::native(&f.folder, key)).unwrap();
            // Keep the PE parseable: accepting any executable which parses loses this edit.
            edit[0] ^= if key == EXE { 0 } else { 1 };
            if key == EXE {
                edit[100] ^= 1;
            }
            write(&f.folder, key, &edit);
            let f = reopen(f);
            let (result, events) = crate::test_capture::capture(|| action(&f, rollback));
            assert_eq!(
                result,
                Err(Error::SourceChanged),
                "{key} rollback={rollback}"
            );
            assert!(events
                .iter()
                .any(|event| event.target == "adoption"
                    && event.field("code") == Some("source_changed")));
            assert_eq!(
                std::fs::read(inventory::native(&f.folder, key)).unwrap(),
                edit
            );
            assert_eq!(std::fs::read(saved).unwrap(), original);
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn setup_abandon_preserves_a_new_file_at_an_originally_absent_target() {
    let f = Light::new(vec![], &[]).await;
    stop(&f);
    let plan = plan(&f);
    let expectations = std::fs::read(plan.stage().join(super::setup_expectations::NAME)).unwrap();
    write(&f.folder, LOGIN, b"new player login");
    let f = reopen(f);
    assert_eq!(action(&f, true), Err(Error::SourceChanged));
    assert_eq!(
        std::fs::read(inventory::native(&f.folder, LOGIN)).unwrap(),
        b"new player login"
    );
    assert_eq!(
        std::fs::read(plan.stage().join(super::setup_expectations::NAME)).unwrap(),
        expectations
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn setup_missing_approved_targets_are_refused_before_other_setup_writes() {
    for key in [LOGIN, EXE] {
        for rollback in [false, true] {
            let f = Light::new(vec![], &[(LOGIN, b"original login")]).await;
            stop(&f);
            let plan = plan(&f);
            let saved = inventory::native(
                &plan.stage().join(super::in_place_check::SETUP_ORIGINALS),
                key,
            );
            let original = std::fs::read(&saved).unwrap();
            std::fs::remove_file(inventory::native(&f.folder, key)).unwrap();
            let ledger = std::fs::read(f.folder.join("launcher-installed.json")).unwrap();
            let f = reopen(f);
            assert_eq!(action(&f, rollback), Err(Error::SourceChanged));
            assert!(!inventory::native(&f.folder, key).exists());
            assert_eq!(std::fs::read(saved).unwrap(), original);
            assert_eq!(
                std::fs::read(f.folder.join("launcher-installed.json")).unwrap(),
                ledger
            );
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn setup_expected_states_allow_repeated_recovery_and_abandonment() {
    for rollback in [false, true] {
        let f = Light::new(vec![], &[(LOGIN, b"original login")]).await;
        let before = std::fs::read(inventory::native(&f.folder, EXE)).unwrap();
        stop(&f);
        let f = reopen(f);
        let plan = plan(&f);
        if rollback {
            // A crash while restoring setup accepts exact already-restored originals.
            super::setup_expectations::restore(&plan).unwrap();
            super::setup_expectations::restore(&plan).unwrap();
            action(&f, true).unwrap();
            assert_eq!(
                std::fs::read(inventory::native(&f.folder, EXE)).unwrap(),
                before
            );
            assert_eq!(
                std::fs::read(inventory::native(&f.folder, LOGIN)).unwrap(),
                b"original login"
            );
        } else {
            super::setup_expectations::check(&plan).unwrap();
            super::direct_setup::prepare(&plan).unwrap();
            super::direct_setup::prepare(&plan).unwrap();
            action(&f, false).unwrap();
            assert!(f.state.lock().unwrap().has_valid_install().unwrap());
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn setup_case_normalization_has_explicit_original_and_approved_states() {
    const LOWER: &str = "Working/SGWGame/Content/UI/Startup/EULA/eula.lua";
    const UPPER: &str = "Working/SGWGame/Content/UI/Startup/EULA/EULA.lua";
    for changed in [false, true] {
        let f = Light::new(vec![], &[(LOWER, b"original eula")]).await;
        stop(&f);
        let f = reopen(f);
        if changed {
            write(&f.folder, UPPER, b"edited eula");
            assert_eq!(action(&f, true), Err(Error::SourceChanged));
            assert_eq!(
                std::fs::read(inventory::native(&f.folder, UPPER)).unwrap(),
                b"edited eula"
            );
        } else {
            action(&f, true).unwrap();
            let parent = inventory::native(&f.folder, LOWER)
                .parent()
                .unwrap()
                .to_owned();
            let names: Vec<_> = std::fs::read_dir(parent)
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect();
            assert!(names.iter().any(|name| name == "eula.lua"));
            assert!(!names.iter().any(|name| name == "EULA.lua"));
            assert_eq!(
                std::fs::read(inventory::native(&f.folder, LOWER)).unwrap(),
                b"original eula"
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn setup_originals_without_expectations_never_repin_a_later_player_edit() {
    for complete_manifest in [false, true] {
        let f = Light::new(vec![], &[(LOGIN, b"original login")]).await;
        assert!(commit_with(f.preview().unwrap(), |point| {
            if point == publication::Point::AfterPromotion {
                Err(StorageError::Io.into())
            } else {
                Ok(())
            }
        })
        .is_err());
        let plan = plan(&f);
        // Model the durable boundary inside record(): backup copies are on disk,
        // but publication of the plan-bound expectations has not happened yet.
        super::in_place_check::save_setup_originals(&plan, &f.folder).unwrap();
        if !complete_manifest {
            std::fs::remove_file(plan.stage().join(super::in_place_check::SETUP_MANIFEST)).unwrap();
        }
        let saved = inventory::native(
            &plan.stage().join(super::in_place_check::SETUP_ORIGINALS),
            EXE,
        );
        let original = std::fs::read(&saved).unwrap();
        let mut edited = original.clone();
        edited[100] ^= 1;
        write(&f.folder, EXE, &edited);
        let f = reopen(f);
        assert_eq!(action(&f, false), Err(Error::SourceChanged));
        assert_eq!(
            std::fs::read(inventory::native(&f.folder, EXE)).unwrap(),
            edited
        );
        assert_eq!(std::fs::read(saved).unwrap(), original);
        assert!(!plan.stage().join(super::setup_expectations::NAME).exists());
    }
}
