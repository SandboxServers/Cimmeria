use super::*;
use cimmeria_launcher_engine::preflight::{CheckAction, CheckStatus};
use cimmeria_launcher_engine::DesktopState;
use uuid::Uuid;

fn host() -> (tempfile::TempDir, Arc<NativeHost>) {
    let root = tempfile::tempdir().unwrap();
    let host = NativeHost::new(root.path().join("state"));
    (root, Arc::new(host))
}

/// Admits an install, leaving an operation that is not terminal.
fn admit_install(state: &mut DesktopState, game: std::path::PathBuf) {
    use ed25519_dalek::{Signer, SigningKey};
    state.save_preferences(Some(game), false, 0).unwrap();
    let body = serde_json::to_vec(&serde_json::json!({"schema":1,"seed":{"blob":"http://127.0.0.1:9/seed","size":1,"sha256":"a".repeat(64)},"patches":[],"min_launcher":"launcher-20261004-fixture"})).unwrap();
    let signature: String = SigningKey::from_bytes(&[0x2a; 32])
        .sign(&body)
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let release =
        cimmeria_launcher_engine::catalog::verify_release(&body, signature.as_bytes()).unwrap();
    state
        .admit_install_backend(cimmeria_launcher_engine::AdmissionRequest {
            id: Uuid::new_v4(),
            operation_revision: 0,
            preferences_revision: 1,
            release: &release,
            login_servers: vec![],
            backend: if cfg!(windows) {
                cimmeria_launcher_engine::ExtractionBackend::Native
            } else {
                cimmeria_launcher_engine::ExtractionBackend::Wine {
                    runtime_sha256: [7; 32],
                    helper_sha256: [8; 32],
                }
            },
        })
        .unwrap();
}

#[test]
fn requests_decode_and_unknown_fields_are_rejected() {
    let inspect: PreflightRequest = serde_json::from_value(
        serde_json::json!({"command":"inspect","schema_version":1,"operation":"play"}),
    )
    .unwrap();
    assert_eq!(inspect.operation(), PreflightOperation::Play);
    let install: PreflightRequest = serde_json::from_value(serde_json::json!({
        "command":"install","schema_version":1,"operation":"repair","check":"wine_runtime"
    }))
    .unwrap();
    assert!(matches!(
        install,
        PreflightRequest::Install {
            check: CheckId::WineRuntime,
            operation: PreflightOperation::Repair,
            ..
        }
    ));
    assert!(
        serde_json::from_value::<PreflightRequest>(serde_json::json!({
            "command":"inspect","schema_version":1,"operation":"play","path":"x"
        }))
        .is_err()
    );
}

/// Drives `run` to completion on a current-thread runtime inside the capture,
/// so the events it emits land in the thread-local subscriber.
fn run_captured(
    host: Arc<NativeHost>,
    request: PreflightRequest,
) -> (Result<Report, JobError>, Vec<crate::test_capture::Captured>) {
    crate::test_capture::capture(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run(host, request))
    })
}

#[test]
fn a_schema_version_other_than_one_is_refused() {
    let (_root, host) = host();
    let request: PreflightRequest = serde_json::from_value(
        serde_json::json!({"command":"inspect","schema_version":2,"operation":"play"}),
    )
    .unwrap();
    let (result, events) = run_captured(host, request);
    assert_eq!(result.unwrap_err(), JobError::UnsupportedSchema);
    let refused: Vec<_> = events
        .iter()
        .filter(|event| event.field("event") == Some("preflight_refused"))
        .collect();
    assert_eq!(refused.len(), 1);
    let event = refused[0];
    assert_eq!(event.target, "launcher.launch");
    assert_eq!(event.level, tracing::Level::WARN);
    assert_eq!(event.field("operation"), Some("play"));
    assert_eq!(event.field("command"), Some("inspect"));
    assert_eq!(event.field("reason"), Some("unsupported_schema"));
    assert_eq!(event.field("failure_code"), Some("launcher_too_old"));
    assert_eq!(event.field("check"), None);
}

#[test]
fn install_without_a_chosen_folder_is_not_ready_and_warns_on_the_install_target() {
    let (_root, host) = host();
    let (report, events) = crate::test_capture::capture(|| {
        host.preflight_inspect(PreflightOperation::Install, AsyncFacts::default())
    });
    let report = report.unwrap();
    assert!(!report.ready);
    let folder = report
        .checks
        .iter()
        .find(|check| check.id == CheckId::GameFolder)
        .unwrap();
    assert_eq!(folder.status, CheckStatus::Missing);
    assert_eq!(folder.action, CheckAction::AskUser);
    assert_eq!(
        folder.failure,
        Some(cimmeria_launcher_engine::failure::FailureCode::GameFolderNotChosen)
    );
    assert!(folder.blocking);
    // No folder means no disk to measure, and Install with no catalog fact has no manifest row.
    assert!(report
        .checks
        .iter()
        .all(|check| check.id != CheckId::DiskSpace && check.id != CheckId::ReleaseManifest));
    let checked: Vec<_> = events
        .iter()
        .filter(|event| event.field("event") == Some("preflight_checked"))
        .collect();
    assert_eq!(checked.len(), 1);
    let event = checked[0];
    assert_eq!(event.target, "launcher.install");
    assert_eq!(event.level, tracing::Level::WARN);
    assert_eq!(event.field("operation"), Some("install"));
    assert_eq!(event.field("ready"), Some("false"));
    assert!(event
        .field("missing")
        .unwrap()
        .split(',')
        .any(|code| code == "game_folder"));
    let blocking = report.checks.iter().filter(|check| check.blocking).count();
    assert_eq!(
        event.field("blocking_checks"),
        Some(blocking.to_string().as_str())
    );
}

#[test]
fn a_play_report_logs_on_the_launch_target_at_the_level_ready_implies() {
    let root = tempfile::tempdir().unwrap();
    let game = root.path().canonicalize().unwrap().join("game");
    std::fs::create_dir(&game).unwrap();
    let host = NativeHost::new(root.path().join("state"));
    host.with_state(|state| state.save_preferences(Some(game), false, 0))
        .unwrap();
    let (report, events) = crate::test_capture::capture(|| {
        host.preflight_inspect(PreflightOperation::Play, AsyncFacts::default())
    });
    let report = report.unwrap();
    let event = events
        .iter()
        .find(|event| event.field("event") == Some("preflight_checked"))
        .unwrap();
    assert_eq!(event.target, "launcher.launch");
    assert_eq!(event.field("operation"), Some("play"));
    let folder = report
        .checks
        .iter()
        .find(|check| check.id == CheckId::GameFolder)
        .unwrap();
    assert_eq!(folder.status, CheckStatus::Ok);
    // The platform helper check may block (no bundled helper in a test host);
    // the level follows `ready` and `missing` follows the checks that are not Ok.
    let expected = if report.ready {
        tracing::Level::INFO
    } else {
        tracing::Level::WARN
    };
    assert_eq!(event.level, expected);
    let not_ok = report
        .checks
        .iter()
        .filter(|check| check.status != CheckStatus::Ok)
        .count();
    assert_eq!(event.field("missing").is_some(), not_ok > 0);
}

#[test]
fn install_is_refused_while_an_operation_is_open() {
    let (root, host) = host();
    let game = root.path().canonicalize().unwrap().join("game");
    host.with_state(|state| {
        admit_install(state, game);
        assert!(state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|operation| !operation.state.terminal()));
        Ok(())
    })
    .unwrap();
    // GameFolder installs nothing, so without the guard this would be
    // PlatformUnavailable; with it, the open operation refuses first.
    let request = PreflightRequest::Install {
        schema_version: 1,
        operation: PreflightOperation::Play,
        check: CheckId::GameFolder,
    };
    let (result, events) = run_captured(host, request);
    assert_eq!(result.unwrap_err(), JobError::Busy);
    let refused: Vec<_> = events
        .iter()
        .filter(|event| event.field("event") == Some("preflight_refused"))
        .collect();
    assert_eq!(refused.len(), 1);
    let event = refused[0];
    assert_eq!(event.target, "launcher.launch");
    assert_eq!(event.level, tracing::Level::WARN);
    assert_eq!(event.field("command"), Some("install"));
    assert_eq!(event.field("check"), Some("game_folder"));
    assert_eq!(event.field("reason"), Some("busy"));
    assert_eq!(event.field("failure_code"), Some("operation_busy"));
}

#[test]
fn install_needs_the_wine_backend() {
    assert_eq!(
        installable(CheckId::Rosetta, true),
        Some(Installable::Rosetta)
    );
    assert_eq!(installable(CheckId::Rosetta, false), None);
    assert_eq!(
        installable(CheckId::WineRuntime, true),
        Some(Installable::WineRuntime)
    );
    assert_eq!(installable(CheckId::WineRuntime, false), None);
    for check in [
        CheckId::CompatibilityHelper,
        CheckId::DiskSpace,
        CheckId::GameFolder,
        CheckId::ReleaseManifest,
        CheckId::AppLocation,
        CheckId::Physx,
    ] {
        assert_eq!(installable(check, true), None, "{check:?}");
    }
}

#[cfg(target_os = "macos")]
#[test]
fn a_mac_report_checks_the_app_location() {
    let (_root, host) = host();
    let report = host
        .preflight_inspect(PreflightOperation::Play, AsyncFacts::default())
        .unwrap();
    let location = report
        .checks
        .iter()
        .find(|check| check.id == CheckId::AppLocation)
        .expect("a macOS report has an AppLocation check");
    // A test binary is not translocated.
    assert_eq!(location.status, CheckStatus::Ok);
    // The test host has no helper, so the Wine backend does not apply.
    assert!(report
        .checks
        .iter()
        .all(|check| check.id != CheckId::Rosetta && check.id != CheckId::WineRuntime));
}

#[tokio::test]
async fn install_of_a_check_with_nothing_to_install_is_unavailable_and_frees_the_lease() {
    let (_root, host) = host();
    let request = PreflightRequest::Install {
        schema_version: 1,
        operation: PreflightOperation::Play,
        check: CheckId::DiskSpace,
    };
    assert_eq!(
        run(host.clone(), request).await.unwrap_err(),
        JobError::PlatformUnavailable
    );
    let store = host.store().unwrap();
    assert!(!store.lock().unwrap().prerequisite_setup_active());
}

#[test]
fn catalog_errors_map_to_manifest_facts() {
    assert_eq!(manifest_probe(Ok(())), ManifestProbe::Ok);
    assert_eq!(
        manifest_probe(Err(CatalogError::Network)),
        ManifestProbe::Unreachable
    );
    for error in [
        CatalogError::SigningKeyUnavailable,
        CatalogError::DevelopmentSigningKey,
    ] {
        assert_eq!(
            manifest_probe(Err(error)),
            ManifestProbe::SigningKeyUnavailable
        );
    }
    for error in [
        CatalogError::TooLarge,
        CatalogError::Signature,
        CatalogError::InvalidManifest,
    ] {
        assert_eq!(manifest_probe(Err(error)), ManifestProbe::Invalid);
    }
}
