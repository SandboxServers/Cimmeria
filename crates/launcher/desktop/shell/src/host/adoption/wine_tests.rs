//! The production Wine helper backend through the host, in an isolated state
//! root and prefix. Ignored by default: these need the build-pinned Windows
//! helper and a prepared runtime tree, named by the environment.
use super::fixture::*;
use super::*;
use cimmeria_launcher_engine::{
    mac_wine::HelperResource, ExtractionBackend, HelperPhase, HelperResult,
};
use std::{fs, path::Path, time::Duration};

/// `CIMMERIA_WINE_HELPER` and its independently supplied build identity.
pub(super) fn helper() -> Option<HelperResource> {
    let path = std::env::var_os("CIMMERIA_WINE_HELPER")?;
    Some(
        HelperResource::open(
            PathBuf::from(path),
            &std::env::var("CIMMERIA_WINE_HELPER_SHA256").expect("independent build identity"),
        )
        .expect("the helper must match its pinned identity"),
    )
}
/// Copies a prepared runtime so the test never downloads one. The engine still
/// verifies the copied tree against its pinned manifest.
pub(super) fn seed_runtime(state_root: &Path) {
    let cached = std::env::var_os("CIMMERIA_WINE_RUNTIME_TREE").expect("prepared runtime tree");
    let runtimes = state_root.join("runtimes");
    let destination = runtimes.join("wine-r17-dc67cf0c2dd1e4c1");
    if destination.exists() {
        return;
    }
    fs::create_dir_all(&runtimes).unwrap();
    assert!(std::process::Command::new("/bin/cp")
        .arg("-R")
        .arg(cached)
        .arg(destination)
        .status()
        .unwrap()
        .success());
}
async fn wine_fixture() -> (Fixture, wiremock::MockServer, HelperResource) {
    let mut f = Fixture::new(serde_json::json!({}));
    let helper = helper().expect("CIMMERIA_WINE_HELPER names the pinned helper");
    seed_runtime(&f.host.root);
    let server = f.serve().await;
    f.host.adoption_fixture.as_mut().unwrap().helper = Some(helper.clone());
    (f, server, helper)
}
const SLOW: Duration = Duration::from_secs(600);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "runs the pinned Windows helper headlessly in an isolated Wine prefix"]
async fn wine_backed_journey_downloads_extracts_reviews_and_publishes_a_wine_owned_copy() {
    let (f, server, helper) = wine_fixture().await;
    let before = source_bytes(&f.source);
    f.begin().unwrap();
    let status = until(&f.host, "the Wine-extracted review", SLOW, |status| {
        assert_eq!(status.last_error, None, "preparation failed");
        status.review.is_some()
    })
    .await;
    let review = status.review.unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    let store = f.host.store().unwrap();
    assert_eq!(
        store
            .lock()
            .unwrap()
            .helper_record(review.preview_handle)
            .unwrap()
            .expect("the helper ran under the supervisor journal")
            .phase,
        HelperPhase::Finished {
            result: HelperResult::Completed
        }
    );
    assert_eq!(review.counts.modified, 1);
    assert_eq!(review.counts.matched, 2);
    f.host.adoption_command(confirmation(&review)).unwrap();
    let done = until(&f.host, "publication", SLOW, |status| {
        status.activity == Activity::Idle
    })
    .await;
    assert_eq!(done.last_error, None);
    assert_eq!(
        done.completed,
        Some(Completed {
            directory: f.destination()
        })
    );
    assert_eq!(done.preparations, Vec::<Uuid>::new());
    // The adopted installation records the Wine backend and the helper that
    // extracted it: the identity prerequisite setup and Play admission require.
    let installed = store.lock().unwrap().installed_content().unwrap().unwrap();
    assert_eq!(installed.intent.backend, helper.backend());
    assert!(matches!(
        installed.intent.backend,
        ExtractionBackend::Wine { .. }
    ));
    assert_eq!(installed.intent.destination, f.destination());
    let install = f.host.install_status().unwrap();
    assert_eq!(install.uninstall.unwrap().directory, f.destination());
    println!(
        "ADOPTED_WINE_COPY prerequisite_target={:?}",
        install.runtime_setup
    );
    assert!(f
        .destination()
        .join("game/Working/Binaries/SGW.exe")
        .is_file());
    assert_eq!(
        fs::read(f.destination().join("game/later.txt")).unwrap(),
        b"later entry"
    );
    assert_eq!(source_bytes(&f.source), before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "runs the pinned Windows helper headlessly in an isolated Wine prefix"]
async fn a_wine_preparation_stopped_after_download_never_leaves_an_unowned_reference() {
    let (f, _server, _helper) = wine_fixture().await;
    let before = source_bytes(&f.source);
    f.begin().unwrap();
    // The signed blob is stored; prefix preparation or extraction follows.
    until(&f.host, "the stored artifact", SLOW, |_| {
        fs::read_dir(f.host.root.join("adoption-artifacts")).is_ok_and(|entries| {
            entries
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().ends_with(".artifact"))
        })
    })
    .await;
    f.host
        .adoption_command(AdoptionCommand::Cancel { schema_version: 1 })
        .unwrap();
    let stopped = until(&f.host, "the stopped preparation", SLOW, |status| {
        status.activity == Activity::Idle
    })
    .await;
    // Stopping inside the helper window is uncertain by design and must be
    // resolved explicitly; outside it the preparation ends cleanly. A finished
    // review is also possible when extraction won the race.
    let host = Arc::new(f.host);
    let settled = match (stopped.reconciliation.clone(), stopped.review.is_some()) {
        (Some(Reconciliation::Preparation { preparation_id }), _) => {
            println!("WINE_CANCEL outcome=reconciliation");
            let revision = stopped.native.operation.revision;
            let worker = host.clone();
            tokio::task::spawn_blocking(move || {
                worker.adoption_command(AdoptionCommand::AbandonPreparation {
                    schema_version: 1,
                    preparation_id,
                    operation_revision: revision,
                    confirmed: true,
                })
            })
            .await
            .unwrap()
            .unwrap()
        }
        (None, true) => {
            println!("WINE_CANCEL outcome=review_completed_first");
            let worker = host.clone();
            tokio::task::spawn_blocking(move || {
                worker.adoption_command(AdoptionCommand::Dismiss { schema_version: 1 })
            })
            .await
            .unwrap()
            .unwrap()
        }
        (None, false) => {
            println!("WINE_CANCEL outcome=clean_cancel");
            assert_eq!(stopped.last_error, Some(AdoptionError::Cancelled));
            stopped
        }
        other => panic!("unexpected reconciliation: {other:?}"),
    };
    assert_eq!(settled.reconciliation, None);
    assert_eq!(settled.preparations, Vec::<Uuid>::new());
    assert_eq!(
        settled
            .native
            .operation
            .operation
            .map(|op| (op.kind, op.state)),
        Some((OperationKind::Adopt, OperationState::Cancelled))
    );
    assert!(host.adoption_choice_allowed().is_ok());
    assert!(!fs::read_dir(&host.root)
        .unwrap()
        .flatten()
        .any(|entry| entry
            .file_name()
            .to_string_lossy()
            .starts_with(".adoption-reference-")));
    assert_eq!(source_bytes(&f.source), before);
}
