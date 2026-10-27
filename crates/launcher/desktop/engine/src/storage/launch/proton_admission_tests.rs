//! The Linux Proton admission gate, end to end through `admit_launch` (PP-06).
//! The fixture is a Native install: a Wine backend would be refused by the
//! Wine arm before the Proton gate ever ran.
use super::resource::ProtonSelection;
use super::tests::artifact;
use super::*;

fn native() -> (tempfile::TempDir, DesktopState, InstallIntent) {
    crate::storage::runtime_setup::tests::fixture_with_extraction(
        ExtractionBackend::Native,
        &"a".repeat(64),
        1,
    )
}

fn selection(root: &Path, version: &str) -> ProtonSelection {
    ProtonSelection {
        id: "proton-test".into(),
        root: root.into(),
        version: version.into(),
    }
}

/// Admits a Launch with `proton`; returns the result, whether a plan was
/// written, and the events.
fn admit(
    proton: impl FnOnce(&Path) -> Option<ProtonSelection>,
) -> (
    Result<Admission, IntentError>,
    bool,
    Vec<crate::test_capture::Captured>,
) {
    let (root, mut state, installed) = native();
    let directory = root.path().canonicalize().unwrap();
    let resources = Resources {
        proton: proton(&directory),
        helper: artifact(&directory, "helper.exe"),
        client_patches: None,
        client_telemetry: None,
        graphics: None,
    };
    let revision = state.operations.snapshot().revision;
    let (result, events) = crate::test_capture::capture(|| {
        state.admit_launch(Uuid::new_v4(), revision, installed.operation_id, resources)
    });
    let has_plan = state.launch_plan().unwrap().is_some();
    (result, has_plan, events)
}

fn refused(events: &[crate::test_capture::Captured]) -> bool {
    events
        .iter()
        .any(|e| e.field("reason") == Some("native_not_admitted"))
}

#[test]
fn a_linux_play_without_a_proton_selection_is_refused_at_admission() {
    let (result, has_plan, events) = admit(|_| None);
    assert!(matches!(
        result,
        Err(IntentError::Storage(StorageError::Corrupt))
    ));
    assert!(!has_plan);
    assert!(refused(&events), "{events:?}");
}

#[test]
fn a_changed_proton_version_is_refused_at_admission() {
    let (result, has_plan, events) = admit(|dir| {
        std::fs::write(dir.join("version"), "GE-Proton10-1\n").unwrap();
        Some(selection(dir, "GE-Proton11-11"))
    });
    assert!(matches!(
        result,
        Err(IntentError::Storage(StorageError::Corrupt))
    ));
    assert!(!has_plan);
    assert!(refused(&events), "{events:?}");
}

#[test]
fn a_matching_proton_selection_is_admitted_and_carried_in_the_plan() {
    let mut picked = None;
    let (result, _, events) = admit(|dir| {
        std::fs::write(dir.join("version"), "1792432585 GE-Proton11-11\n").unwrap();
        picked = Some(selection(dir, "1792432585 GE-Proton11-11"));
        picked.clone()
    });
    let admission = result.unwrap_or_else(|e| panic!("{e:?} {events:?}"));
    assert!(admission.dispatch);
    assert_eq!(admission.plan.resources.proton, picked);
    assert!(!refused(&events), "{events:?}");
}
