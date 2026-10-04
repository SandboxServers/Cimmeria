//! Source to ingest: a real install worker's result is queued by the journal
//! observer and delivered by the exporter to a loopback mock of the server's two
//! routes. No launch code takes part, and a dead endpoint changes no result.
use super::tests::outcome;
use super::*;
use crate::{
    client_setup::login_servers::default_servers,
    launcher_summary::{SummaryArch, SummaryOs},
    storage::launcher_summary::tests::{
        exporter::{mount_ok, CycleOutcome, Rig, BACKOFFS, INGEST, MINT},
        queue_bytes, queued,
    },
    OperationKind,
};
use fixtures::{archive, verified};
use serde_json::{json, Value};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

fn fixture(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

#[tokio::test]
async fn a_failed_install_reaches_the_ingest_route_before_any_launch() {
    let release = verified(&archive(true));
    let mut rig = Rig::start().await;
    rig.fix_mint_id();
    mount_ok(&rig.server).await;
    let id = Uuid::new_v4();
    {
        let mut owner = rig.owner();
        owner
            .save_preferences(Some(rig.root().join("install")), true, 0)
            .unwrap();
        owner
            .admit_install(id, 0, 1, &release, default_servers())
            .unwrap();
    }
    // The injected clock moves only here, so the timings in the body are exact.
    rig.clock.advance_ms(12);
    // The download host has no such blob: the install fails before any byte.
    let downloads = MockServer::start().await;
    let worker = dispatch_with(
        rig.state.clone(),
        id,
        release,
        format!("{}/manifest.json", downloads.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    assert_eq!(outcome(worker.result.clone()).await, Outcome::InstallFailed);

    assert_eq!(
        rig.cycle().await,
        CycleOutcome::Delivered {
            accepted: 1,
            duplicate: 0,
            rejected: 0,
        }
    );
    assert_eq!(rig.paths().await, [MINT, INGEST]);
    assert_eq!(
        rig.bodies(MINT).await,
        [fixture(include_str!(
            "../launcher_summary/fixtures/mint-request.json"
        ))]
    );
    // The fixture was recorded on one platform; `os` and `arch` are this build's.
    let mut golden = fixture(include_str!(
        "../launcher_summary/fixtures/request-install-failure.json"
    ));
    golden["summaries"][0]["os"] = json!(SummaryOs::current());
    golden["summaries"][0]["arch"] = json!(SummaryArch::current());
    assert_eq!(rig.bodies(INGEST).await, [golden]);

    let owner = rig.owner();
    assert_eq!(queued(&owner), [], "delivered rows leave the queue");
    // Nothing of a launch exists: the only operation ever admitted is the install.
    let operation = owner.operations().snapshot().operation.clone().unwrap();
    assert_eq!((operation.id, operation.kind), (id, OperationKind::Install));
    assert_eq!(owner.launch_observation().unwrap(), None);
}

/// Runs one real install against a download host that has the seed. Returns
/// the worker's result with the journal's final state and revision.
async fn install(
    state: &Arc<Mutex<DesktopState>>,
    directory: PathBuf,
) -> (Outcome, OperationState, u64) {
    let seed = archive(true);
    let release = verified(&seed);
    let downloads = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed))
        .mount(&downloads)
        .await;
    let id = Uuid::new_v4();
    {
        let mut owner = state.lock().unwrap();
        let revision = owner.preferences().revision;
        owner
            .save_preferences(Some(directory), true, revision)
            .unwrap();
        owner
            .admit_install(id, 0, revision + 1, &release, default_servers())
            .unwrap();
    }
    let worker = dispatch_with(
        state.clone(),
        id,
        release,
        format!("{}/manifest.json", downloads.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    let result = outcome(worker.result.clone()).await;
    let owner = state.lock().unwrap();
    let snapshot = owner.operations().snapshot();
    let operation = snapshot.operation.as_ref().unwrap();
    assert_eq!(operation.id, id);
    (result, operation.state, snapshot.revision)
}

#[tokio::test]
async fn an_install_ends_the_same_with_a_dead_endpoint_as_with_no_exporter() {
    // Baseline: summaries were never configured, so there is no exporter at all.
    let root = tempfile::tempdir().unwrap();
    let plain = DesktopState::open(&root.path().join("state")).unwrap();
    let plain = Arc::new(Mutex::new(plain));
    let baseline = install(&plain, root.path().join("install")).await;
    assert_eq!(baseline.0, Outcome::ContentPrepared);
    assert_eq!(queue_bytes(&plain.lock().unwrap()), None);

    // The same install with consent on, an exporter running and nothing listening.
    let mut rig = Rig::dead().await;
    let task = rig.start_exporter().expect("an exporter task");
    let dead = install(&rig.state, rig.root().join("install")).await;
    assert_eq!(dead, baseline);

    // The exporter did try: three attempts, then the row waits for a later run.
    tokio::time::timeout(Duration::from_secs(30), async {
        while rig.probe.sleeps().len() < BACKOFFS.len() || queued(&rig.owner()).is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the exporter gives up");
    assert_eq!(rig.probe.sleeps(), BACKOFFS);
    assert_eq!(queued(&rig.owner()).len(), 1);
    assert!(!task.is_finished(), "the exporter stays alive");
    assert_eq!(
        rig.owner().install_outcome().unwrap(),
        Some(Outcome::ContentPrepared)
    );
}
