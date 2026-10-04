//! The component can neither fail the launcher nor widen who reads consent.
use super::*;
use crate::storage::StorageError;
use install_worker::Outcome;
use std::sync::Mutex;

/// A whole install attempt with every summary entry point exercised on the way.
/// Returns each launcher-visible result.
fn install_attempt(
    state: &Mutex<DesktopState>,
) -> (
    Result<bool, crate::IntentError>,
    Result<u64, StorageError>,
    Option<Outcome>,
    OperationState,
) {
    let mut owner = state.lock().unwrap();
    let install = owner.state_root().parent().unwrap().join("install");
    let saved = owner
        .save_preferences(Some(install), true, 1)
        .map(|preferences| preferences.revision);
    owner.configure_summaries(config(Some(ENDPOINT)));
    owner.summary_pre_admission_failure(
        None,
        SummaryOperation::Install,
        SummaryPhase::CatalogFetch,
        SummaryErrorCode::ManifestUnavailable,
    );
    let id = Uuid::new_v4();
    let admitted = owner
        .admit_install(id, 0, 2, &release(), default_servers())
        .map(|admission| admission.dispatch);
    owner.summary_phase(id, TimedPhase::Download);
    observe(&mut owner, id, OperationState::Running);
    finish_install(&mut owner, id, Outcome::ContentInvalid);
    owner.finalize_summaries();
    let _ = owner.summary_take_batch();
    let terminal = owner
        .operations()
        .snapshot()
        .operation
        .as_ref()
        .unwrap()
        .state;
    (admitted, saved, owner.install_outcome().unwrap(), terminal)
}

#[test]
fn a_panic_inside_any_summary_entry_point_stays_inside() {
    let mut results = Vec::new();
    for inject in [false, true] {
        let (_root, state, _clock) = opted_in();
        lock(&state.summaries).panic_hook = inject;
        let state = Mutex::new(state);
        results.push(install_attempt(&state));
        assert!(!state.is_poisoned(), "the command mutex survives");
        let owner = state.lock().unwrap();
        let faults = owner.summary_faults();
        if inject {
            // Every entry point and every journal commit hit the fault: eight
            // entry points (the preferences save and both `operations_mut`
            // calls finalize first, so they count) and three journal commits.
            assert_eq!(faults.panics, 11, "{faults:?}");
            assert_eq!(queued(&owner), []);
        } else {
            assert_eq!(faults, SummaryFaults::default());
            assert_eq!(queued(&owner).len(), 2);
        }
    }
    // With the fault, admission and every other result match the clean run.
    assert_eq!(results[1], results[0]);
    assert_eq!(
        results[0],
        (
            Ok(true),
            Ok(2),
            Some(Outcome::ContentInvalid),
            OperationState::Failed
        )
    );
}

#[test]
fn a_fault_while_changing_consent_fails_closed() {
    // Withdrawal: the preferences save still succeeds and the gate is shut.
    let (_root, mut state, _clock) = opted_in();
    lock(&state.summaries).panic_hook = true;
    assert_eq!(
        state
            .save_preferences(None, false, 1)
            .map(|saved| saved.launcher_summary_consent),
        Ok(false)
    );
    assert!(lock(&state.summaries).export_blocked);
    assert!(!state.requires_reopen());

    // Granting: consent stays off, exactly as when the queue cannot be emptied.
    let root = tempfile::tempdir().unwrap();
    let (mut state, _clock) = configured(root.path());
    lock(&state.summaries).panic_hook = true;
    assert_eq!(state.save_preferences(None, true, 0), Err(StorageError::Io));
    assert!(!state.preferences().launcher_summary_consent);
    assert!(!state.requires_reopen());
    lock(&state.summaries).panic_hook = false;
    // Positive control: without the fault the same request succeeds.
    assert!(state.save_preferences(None, true, 0).is_ok());
}

// Consent for launcher summaries is read by the preferences code and by this
// component only. Launch, install, repair and game-telemetry code must not grow
// a dependency on it; a new reader has to be added here on purpose. The shell
// crate holds the launch, install and repair host glue, so it is scanned too.
#[test]
fn only_the_allowed_files_mention_the_summary_consent_flag() {
    const ENGINE: &[&str] = &[
        "commands.rs",
        "storage/launcher_summary/consent.rs",
        "storage/launcher_summary/mod.rs",
        "storage/migration/mod.rs",
        "storage/mod.rs",
    ];
    // `host.rs` carries the saved value over when it seeds the default install
    // directory; `host/summary.rs` sets it in its own tests.
    const SHELL: &[&str] = &["host.rs", "host/summary.rs"];
    fn is_test(path: &Path) -> bool {
        let name = path.file_name().unwrap().to_string_lossy();
        name == "tests.rs"
            || name.ends_with("_tests.rs")
            || path.components().any(|part| part.as_os_str() == "tests")
    }
    fn scan(directory: &Path, root: &Path, found: &mut Vec<String>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                scan(&path, root, found);
            } else if path.extension().is_some_and(|extension| extension == "rs")
                && !is_test(&path)
                && std::fs::read_to_string(&path)
                    .unwrap()
                    .contains(concat!("launcher_summary", "_consent"))
            {
                let relative = path.strip_prefix(root).unwrap();
                found.push(
                    relative
                        .components()
                        .map(|part| part.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/"),
                );
            }
        }
    }
    fn mentions(root: &Path) -> Vec<String> {
        let mut found = Vec::new();
        scan(root, root, &mut found);
        found.sort();
        found
    }
    let engine = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let shell = Path::new(env!("CARGO_MANIFEST_DIR")).join("../shell/src");
    assert_eq!(mentions(&engine), ENGINE);
    assert_eq!(mentions(&shell), SHELL);
    // The scan sees what it should: the flag's own definition is on the engine
    // list, and the shell's one production reader is on the shell list.
    assert!(std::fs::read_to_string(engine.join("storage/mod.rs"))
        .unwrap()
        .contains(concat!("pub launcher_summary", "_consent: bool")));
    assert!(std::fs::read_to_string(shell.join("host.rs"))
        .unwrap()
        .contains(concat!("preferences().launcher_summary", "_consent")));
}
