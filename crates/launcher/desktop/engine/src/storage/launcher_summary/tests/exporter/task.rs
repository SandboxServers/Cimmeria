//! `start` and the one exporter task: when it is spawned, what wakes it, and
//! that it never keeps the state directory locked.
use super::*;
use crate::storage::StorageError;

async fn ended(task: JoinHandle<()>) {
    tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .expect("the exporter task ends")
        .unwrap();
}

#[tokio::test]
async fn start_without_an_endpoint_spawns_nothing_and_removes_the_queue_file() {
    let rig = Rig::opted_in().await;
    rig.fail(1);
    assert!(queue_bytes(&rig.owner()).is_some());
    assert!(export::spawn(&rig.state, config(None), ExportEnv::production).is_none());
    assert_eq!(queue_bytes(&rig.owner()), None);
    assert!(rig.owner().summary_export_target().is_none());

    // The public entry point does the same.
    rig.owner().configure_summaries(config(Some(&rig.base)));
    rig.fail(1);
    assert!(queue_bytes(&rig.owner()).is_some());
    start(&rig.state, config(None));
    assert_eq!(queue_bytes(&rig.owner()), None);
    assert!(rig.owner().summary_export_target().is_none());
    assert_eq!(rig.paths().await, [""; 0]);

    // Positive control: with an endpoint the same call starts the task, once.
    let with_endpoint =
        || export::spawn(&rig.state, config(Some(&rig.base)), ExportEnv::production);
    let task = with_endpoint().expect("an exporter task");
    assert!(with_endpoint().is_none(), "there is only one exporter");
    task.abort();
}

#[test]
fn start_outside_a_runtime_configures_and_spawns_nothing() {
    let root = tempfile::tempdir().unwrap();
    let state = Arc::new(Mutex::new(DesktopState::open(root.path()).unwrap()));
    let spawn = || export::spawn(&state, config(Some(ENDPOINT)), ExportEnv::production);
    assert!(spawn().is_none());
    assert!(state.lock().unwrap().summary_export_target().is_some());
    start(&state, config(Some(ENDPOINT)));
    // Nothing was claimed: inside a runtime the same call starts the task.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    assert!(runtime.block_on(async { spawn() }).is_some());
}

#[tokio::test]
async fn the_task_delivers_at_start_and_on_each_new_row_and_ends_with_the_state() {
    let mut rig = Rig::opted_in().await;
    mount_ok(&rig.server).await;
    // A row left by an earlier run is sent without any trigger.
    rig.fail(1);
    let task = rig.start_exporter().expect("an exporter task");
    assert_eq!(rig.paths_after(2).await, [MINT, INGEST]);
    // A new terminal row wakes the task.
    rig.fail(1);
    assert_eq!(rig.paths_after(4).await, [MINT, INGEST, MINT, INGEST]);
    queue_empties(&rig).await;

    // Positive control: while the state lives, its directory is locked.
    let state_root = rig.owner().state_root().to_path_buf();
    assert!(matches!(
        DesktopState::open(&state_root),
        Err(StorageError::InUse)
    ));
    // Dropping the last strong handle ends the idle task and frees the lock.
    let Rig { state, root, .. } = rig;
    drop(state);
    ended(task).await;
    assert!(DesktopState::open(&state_root).is_ok());
    drop(root);
}

#[tokio::test]
async fn the_task_ends_when_the_server_has_no_summary_routes() {
    let mut rig = Rig::opted_in().await;
    mount(&rig.server, MINT, ResponseTemplate::new(404)).await;
    let rows = rig.fail(1);
    let task = rig.start_exporter().expect("an exporter task");
    ended(task).await;
    assert_eq!(rig.paths().await, [MINT]);
    assert_eq!(queued(&rig.owner()), rows);
    // No exporter is left to make a request, and none can be started again.
    assert!(rig.start_exporter().is_none());
    assert_eq!(rig.fail(1).len(), 2);
    assert_eq!(rig.paths().await, [MINT]);
}

async fn queue_empties(rig: &Rig) {
    for _ in 0..500 {
        if queued(&rig.owner()).is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(queued(&rig.owner()), []);
}

// Consent is off for every new installation, so this is how the task starts for
// most users. It must still be there when they opt in.
#[tokio::test]
async fn a_task_started_without_consent_delivers_after_the_opt_in() {
    let mut rig = Rig::start().await;
    mount_ok(&rig.server).await;
    let task = rig.start_exporter().expect("an exporter task");
    // The task has not run yet. This copy of the reopen flag is refreshed only
    // by a look at the gate, and nothing but the task's first cycle takes one
    // here: once it is fresh, that cycle has seen consent off and ended.
    let stale = |rig: &Rig| lock(&rig.owner().summaries).reopen;
    lock(&rig.owner().summaries).reopen = true;
    for _ in 0..500 {
        if !stale(&rig) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!stale(&rig), "the first cycle ran");
    assert_eq!(
        rig.paths().await,
        [""; 0],
        "nothing is sent without consent"
    );

    set_consent(&mut rig.owner(), true);
    assert_eq!(rig.fail(1).len(), 1);
    assert_eq!(rig.paths_after(2).await, [MINT, INGEST]);
    queue_empties(&rig).await;
    task.abort();
}

#[tokio::test]
async fn the_task_survives_giving_up_and_delivers_once_the_server_answers() {
    let mut rig = Rig::opted_in().await;
    mount(&rig.server, MINT, minted()).await;
    mount(&rig.server, INGEST, ResponseTemplate::new(500)).await;
    let waiting = rig.fail(1);
    let task = rig.start_exporter().expect("an exporter task");
    // The row was queued before the task started, so its wake-up is still
    // stored: the task runs the cycle at start and one more. Each makes three
    // attempts and gives up.
    assert_eq!(rig.paths_after(12).await, [MINT, INGEST].repeat(6));
    assert_eq!(rig.probe.sleeps(), [BACKOFFS, BACKOFFS].concat());
    assert_eq!(queued(&rig.owner()), waiting, "the row waits");

    // The server recovers. The next row wakes the same task, which sends both.
    rig.server.reset().await;
    mount_ok(&rig.server).await;
    assert_eq!(rig.fail(1).len(), 2);
    assert_eq!(rig.paths_after(2).await, [MINT, INGEST]);
    queue_empties(&rig).await;
    let posts = rig.bodies(INGEST).await;
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["summaries"].as_array().map(Vec::len), Some(2));
    task.abort();
}

#[test]
fn production_tuning_is_the_recorded_contract() {
    let tuning = ExportTuning::PRODUCTION;
    assert_eq!(tuning.mint_timeout, Duration::from_secs(2));
    assert_eq!(tuning.post_timeout, Duration::from_secs(2));
    assert_eq!(tuning.max_retries, 2);
    assert_eq!(tuning.retry_after_cap, Duration::from_secs(60));
    let (low, high) = (Duration::from_millis(250), Duration::from_secs(2));
    let mut seen = std::collections::BTreeSet::new();
    for retry in 1..=5 {
        for _ in 0..200 {
            let wait = (tuning.backoff)(retry);
            assert!((low..=high).contains(&wait), "retry {retry}: {wait:?}");
            seen.insert(wait);
        }
    }
    assert!(seen.len() > 10, "the wait is jittered");
}
