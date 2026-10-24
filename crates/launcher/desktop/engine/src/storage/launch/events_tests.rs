//! Launch lifecycle events on a real failing path (launcher-telemetry LT-17a).
//! The helper is replaced, so the first verify fails before the game starts.
use super::*;
use crate::test_capture::{capture, Captured};
use tracing::Level;

fn named<'a>(events: &[&'a Captured], name: &str) -> Vec<&'a Captured> {
    events
        .iter()
        .copied()
        .filter(|e| e.field("event") == Some(name))
        .collect()
}

#[test]
fn a_replaced_helper_logs_launch_not_started_verify_failed() {
    // A second live Dispatch keeps tracing's callsite interest per dispatcher; with one,
    // a callsite first hit by another test with no subscriber stays disabled.
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let ((), events) = capture(|| {
        tracing::callsite::rebuild_interest_cache();
        rt.block_on(async {
            let (_root, state, plan) = super::tests::fixture();
            std::fs::write(plan.resources.helper.path(), b"replacement").unwrap();
            let state = Arc::new(std::sync::Mutex::new(state));
            let _worker = dispatch(state.clone(), plan.id).unwrap();
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    if state
                        .lock()
                        .unwrap()
                        .operations()
                        .snapshot()
                        .operation
                        .as_ref()
                        .unwrap()
                        .state
                        .terminal()
                    {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            // The worker noted the code before its terminal record kept it.
            assert_eq!(
                state.lock().unwrap().launch_failure(),
                Ok(Some(crate::failure::FailureCode::GameFilesInvalid))
            );
        });
    });
    let launch: Vec<&Captured> = events
        .iter()
        .filter(|e| e.target == "launcher.launch")
        .collect();
    let started = named(&launch, "launch_started");
    let not_started = named(&launch, "launch_not_started");
    let finished = named(&launch, "launch_finished");
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].field("telemetry_dll"), Some("false"));
    // A plain install has no adoption provenance and no incomplete patches.
    assert_eq!(started[0].field("adoption_check"), Some("none"));
    assert_eq!(started[0].field("patches_incomplete"), Some("0"));
    assert_eq!(not_started.len(), 1);
    assert_eq!(not_started[0].level, Level::WARN);
    assert_eq!(not_started[0].field("reason"), Some("verify_failed"));
    assert_eq!(
        not_started[0].field("failure_code"),
        Some("game_files_invalid")
    );
    assert_eq!(finished.len(), 1);
    assert_eq!(finished[0].field("outcome"), Some("not_started"));
    let operation = started[0].field("operation");
    assert!(operation.is_some());
    assert_eq!(not_started[0].field("operation"), operation);
    assert_eq!(finished[0].field("operation"), operation);
    let resources: Vec<_> = launch
        .iter()
        .filter(|e| e.field("phase") == Some("play_resources"))
        .collect();
    assert_eq!(resources.len(), 2);
    assert_eq!(resources[1].field("outcome"), Some("failed"));
    assert!(resources[1].field("duration_ms").is_some());
    assert!(!launch
        .iter()
        .any(|e| e.field("phase") == Some("play_client_setup")));
}

#[test]
fn refused_admission_and_process_started_have_pre_game_timing() {
    let (_, events) = capture(|| {
        let (_root, mut state, plan) = super::tests::fixture();
        let result = state.admit_launch(
            Uuid::new_v4(),
            u64::MAX,
            plan.installation.operation_id,
            plan.resources.clone(),
        );
        assert!(matches!(
            result,
            Err(IntentError::Operation(ContractError::StaleRevision))
        ));
        events::game_started(plan.id);
    });
    assert!(events.iter().any(
        |e| e.field("phase") == Some("play_admission") && e.field("outcome") == Some("failed")
    ));
    let started = events
        .iter()
        .find(|e| e.field("event") == Some("game_started"))
        .unwrap();
    assert!(started.field("startup_ms").unwrap().parse::<u64>().is_ok());
}

#[test]
fn an_unreadable_choice_logs_game_telemetry_skipped() {
    // A second live Dispatch keeps tracing's callsite interest per dispatcher; with one,
    // a callsite first hit by another test with no subscriber stays disabled.
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let ((), events) = capture(|| {
        tracing::callsite::rebuild_interest_cache();
        rt.block_on(async {
            let (root, state, mut plan) = super::tests::fixture();
            plan.resources.client_telemetry = Some(super::tests::artifact(
                &root.path().canonicalize().unwrap(),
                "telemetry.dll",
            ));
            // A directory where the choice file belongs cannot be read as one.
            let choice = state.state_root().join("game-telemetry.json");
            std::fs::create_dir(&choice).unwrap();
            let session =
                super::telemetry::session(&Arc::new(std::sync::Mutex::new(state)), &plan, &[])
                    .await;
            assert!(session.is_none());
        });
    });
    let skipped: Vec<&Captured> = events
        .iter()
        .filter(|e| e.target == "launcher.launch")
        .filter(|e| e.field("event") == Some("game_telemetry_skipped"))
        .collect();
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].level, Level::WARN);
    assert_eq!(skipped[0].field("reason"), Some("choice_unreadable"));
    assert!(skipped[0].field("operation").is_some());
}

static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

#[test]
fn launch_started_carries_the_adoption_tags() {
    let _serial = serial(); // first line: one capture test of this module at a time
                            // A second live Dispatch keeps tracing's callsite interest per dispatcher; with one,
                            // a callsite first hit by another test with no subscriber stays disabled.
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let id = Uuid::new_v4();
    let ((), events) = capture(|| {
        tracing::callsite::rebuild_interest_cache();
        super::events::launch_started(id, false, "light", 1)
    });
    let launch: Vec<&Captured> = events
        .iter()
        .filter(|e| e.target == "launcher.launch")
        .collect();
    let started = named(&launch, "launch_started");
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].level, Level::INFO);
    assert_eq!(started[0].field("adoption_check"), Some("light"));
    assert_eq!(started[0].field("patches_incomplete"), Some("1"));
}

#[test]
fn launch_tags_read_the_copys_provenance_and_ledger() {
    use crate::storage::adoption::{FileCheck, Provenance};
    use cimmeria_launcher_core::state::{IncompletePatch, InstalledState};
    let provenance = |check: FileCheck| Provenance {
        work_id: Uuid::new_v4(),
        legacy_install_id: None,
        import_digest: None,
        reference_digest: [0; 32],
        setup_policy_version: 1,
        folder_scan_digest: None,
        file_index_sha256: None,
        check,
    };
    let game = tempfile::tempdir().unwrap();
    InstalledState {
        incomplete_patches: vec![IncompletePatch {
            state_key: "patch:one".to_owned(),
            skipped: Vec::new(),
        }],
        ..Default::default()
    }
    .save(game.path())
    .unwrap();
    let light = Some(provenance(FileCheck::Light));
    assert_eq!(
        super::adoption_tags::launch_tags(light.as_ref(), game.path()),
        ("light", 1)
    );
    let full = Some(provenance(FileCheck::Full));
    assert_eq!(
        super::adoption_tags::launch_tags(full.as_ref(), game.path()),
        ("full", 1)
    );
    // No provenance is a downloaded install; no ledger is no incomplete patch.
    let plain = tempfile::tempdir().unwrap();
    assert_eq!(
        super::adoption_tags::launch_tags(None, plain.path()),
        ("none", 0)
    );
}

#[test]
fn adoption_tags_read_the_adopted_copy_and_its_ledger() {
    use crate::storage::effective_settings::fixtures::{adopt_native, Legacy};
    use cimmeria_launcher_core::state::{IncompletePatch, InstalledState};
    // A real copy adoption, then one incomplete patch in the copy's own ledger.
    let legacy = Legacy::new(false, false);
    let state = Arc::new(std::sync::Mutex::new(legacy.open()));
    let _provenance = adopt_native(&legacy, &state);
    let game = legacy.destination().join("game");
    let mut ledger = InstalledState::load(&game);
    ledger.incomplete_patches.push(IncompletePatch {
        state_key: "patch:one".to_owned(),
        skipped: Vec::new(),
    });
    ledger.save(&game).unwrap();
    // A copy adoption checks `full`. Reverted to a constant `("none", 0)`, this fails.
    let tags = super::adoption_tags::adoption_tags(&state.lock().unwrap());
    assert_eq!(tags, ("full", 1));
}

#[test]
fn an_unreadable_record_launches_untagged_and_logs_one_closed_warning() {
    use crate::storage::effective_settings::fixtures::{adopt_native, Legacy};
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let legacy = Legacy::new(false, false);
    let state = Arc::new(std::sync::Mutex::new(legacy.open()));
    let _provenance = adopt_native(&legacy, &state);
    // A record that is not JSON cannot be read as installed.
    let record = state
        .lock()
        .unwrap()
        .directory
        .root
        .join("installed-content.json");
    std::fs::write(record, b"not json").unwrap();
    let (tags, events) = capture(|| {
        tracing::callsite::rebuild_interest_cache();
        super::adoption_tags::adoption_tags(&state.lock().unwrap())
    });
    assert_eq!(tags, ("none", 0));
    let rows = named(
        &events.iter().collect::<Vec<_>>(),
        "launch_tags_unavailable",
    );
    assert_eq!(rows.len(), 1, "{events:?}");
    assert_eq!(rows[0].target, "launcher.launch");
    assert_eq!(rows[0].level, Level::WARN);
}
