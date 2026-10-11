//! LR-07 review tests: a bounded, retryable stop, the input checks, the
//! worst-case answer length, recording-folder collisions, and a recorder
//! that fails to begin or panics. Over the fakes in [`super::fakes`].

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use super::fakes::{id_of, interact, FakeServer, Lab, PANIC, T0};
use super::run::{StartReq, BODY_CAP, RECORD_FILE};
use super::stop::{StopReq, TRANSFORM_ERROR_FILE};
use crate::supervisor::record_slot::RecordHandle;
use crate::uat::record::emit::{draft_file_name, transform, Draft, TransformOptions};
use crate::uat::record::log::RecordLog;
use crate::uat::record::sampler::recording_id;

/// A handle armed by hand on `lab`'s slot, its task `task`, its folder
/// holding the committed fixture log.
fn arm(lab: &Lab, id: &str, task: tokio::task::JoinHandle<Result<PathBuf, String>>) -> PathBuf {
    let dir = lab.raw(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(RECORD_FILE),
        include_str!("../../uat/record/fixtures/fs_p3_record.jsonl"),
    )
    .unwrap();
    lab.slot
        .reserve()
        .unwrap()
        .arm(RecordHandle {
            recording_id: id.into(),
            dir: dir.clone(),
            section: "first-session".into(),
            specs_dir: Some(lab.specs()),
            started_ms: T0,
            ticks: Arc::default(),
            anchors: Arc::default(),
            stop: tokio::sync::watch::channel(None).0,
            done: task,
            ended: None,
        })
        .unwrap();
    dir
}

/// A stop never waits forever: a recorder that does not finish leaves the
/// recording in the slot, and the answer names the raw folder in full.
#[tokio::test]
async fn a_wedged_finish_times_out_and_the_recording_stays() {
    let lab = Lab::new(FakeServer::default());
    let wedged = tokio::spawn(async {
        std::future::pending::<()>().await;
        Ok(PathBuf::new())
    });
    let dir = arm(&lab, "20261012-143000-p2", wedged);
    let req = StopReq {
        wait: Duration::from_millis(100),
        ..lab.stop_req("20261012-143000-p2")
    };
    let f = tokio::time::timeout(Duration::from_secs(5), lab.stop_with(req))
        .await
        .expect("the stop is bounded")
        .unwrap_err();
    assert_eq!(f.exit, 1);
    let v: Value = serde_json::from_str(&f.body()).unwrap();
    let error = v["error"].as_str().unwrap();
    assert!(error.starts_with(&dir.display().to_string()), "{error}");
    assert!(error.contains("call lab_record_stop again"), "{error}");
    assert_eq!(lab.status()["recording_id"], "20261012-143000-p2");
    assert_eq!(lab.status()["recording"], true);
    lab.slot.take().unwrap().done.abort();
}

/// Fails the transform when `character_expr` is `Bad`; else the real one.
fn picky(log: &RecordLog, o: &TransformOptions) -> Result<Draft, String> {
    if o.character_expr.as_deref() == Some("Bad") {
        return Err("refused by the test".into());
    }
    transform(log, o)
}

/// A failed transform leaves the ended recording in the slot; a second
/// stop with fixed options writes the draft.
#[tokio::test]
async fn a_failed_transform_is_retried_by_a_second_stop() {
    let lab = Lab::new(FakeServer::with_tap(vec![json!([]), json!([interact()])]));
    let id = id_of(&lab.start(None).await.unwrap());
    lab.until("ticks", |s| s["ticks"].as_u64().unwrap() >= 3)
        .await;
    let bad = StopReq {
        character_expr: Some("Bad".into()),
        transform: picky,
        ..lab.stop_req(&id)
    };
    let f = lab.stop_with(bad).await.unwrap_err();
    assert_eq!(f.exit, 1, "{}", f.error);
    assert!(lab.raw(&id).join(TRANSFORM_ERROR_FILE).is_file());
    assert!(!lab.book.is_held(), "the task ended and released its lease");
    assert_eq!(
        lab.status()["recording_id"],
        id.as_str(),
        "kept for a retry"
    );
    assert_eq!(lab.status()["recording"], false);

    let good = StopReq {
        character_expr: Some("Px${run_id}".into()),
        transform: picky,
        ..lab.stop_req(&id)
    };
    let out = lab.stop_with(good).await.unwrap();
    let draft = lab
        .specs()
        .join("drafts")
        .join(out["draft"].as_str().unwrap());
    assert!(draft.is_file(), "{out}");
    assert!(lab.slot.status().is_none(), "collected");
}

/// Every answer stays under 400 characters with the longest legal inputs:
/// a 48-character section, a 16-character instance and deep folders.
#[tokio::test]
async fn answers_stay_under_400_with_worst_case_inputs() {
    let mut lab = Lab::new(FakeServer::with_tap(vec![json!([]), json!([interact()])]));
    let deep: PathBuf = (0..9).map(|i| format!("deep-folder-name-{i:02}")).collect();
    lab.instance = "abcdefghijklmnop".into();
    lab.records = lab.root.path().join(&deep).join("records");
    let specs = lab.root.path().join(&deep).join("uat-specs");
    std::fs::create_dir_all(&specs).unwrap();
    lab.specs = specs;
    let section = format!("s{}", "-x".repeat(23));
    assert_eq!(section.len(), 47);
    let section = format!("{section}9");

    let req = StartReq {
        section: section.clone(),
        ..lab.start_req()
    };
    let started = lab.start_with(req).await.unwrap();
    let id = id_of(&started);
    lab.until("ticks", |s| s["ticks"].as_u64().unwrap() >= 3)
        .await;
    let status = lab.status();
    let stopped = lab.stop(&id, false).await.unwrap();
    for (what, v) in [("start", started), ("status", status), ("stop", stopped)] {
        let body = v.to_string();
        assert!(
            body.len() < BODY_CAP,
            "{what}: {} chars: {body}",
            body.len()
        );
        assert!(
            !body.contains("deep-folder"),
            "{what} carries a path: {body}"
        );
    }
}

#[tokio::test]
async fn start_refuses_a_bad_section_or_specs_dir_before_any_server_call() {
    let lab = Lab::new(FakeServer::default());
    let bad_section = StartReq {
        section: "First Session".into(),
        ..lab.start_req()
    };
    assert_eq!(lab.start_with(bad_section).await.unwrap_err().exit, 2);
    for dir in [
        lab.root.path().join("missing"),
        PathBuf::from(r"\\server\share\specs"),
    ] {
        let req = StartReq {
            specs_dir: Some(dir.clone()),
            ..lab.start_req()
        };
        let f = lab.start_with(req).await.unwrap_err();
        assert_eq!(f.exit, 2, "{dir:?}: {}", f.error);
        assert!(f.error.starts_with("specs_dir"), "{}", f.error);
    }
    assert!(lab.server.calls.lock().unwrap().is_empty());
    assert!(!lab.book.is_held());
}

#[tokio::test]
async fn stop_refuses_a_bad_character_expr_and_keeps_recording() {
    let lab = Lab::new(FakeServer::default());
    let id = id_of(&lab.start(None).await.unwrap());
    let req = StopReq {
        character_expr: Some("x'; DROP TABLE sgw_player; --".into()),
        ..lab.stop_req(&id)
    };
    let f = lab.stop_with(req).await.unwrap_err();
    assert_eq!(f.exit, 2, "{}", f.error);
    assert_eq!(lab.status()["recording"], true, "not signalled");
    lab.stop(&id, true).await.unwrap();
}

/// Two recordings in one second: the second gets `-2`, keeps its own
/// folder, and its draft name still carries the time stamp.
#[tokio::test]
async fn a_taken_recording_folder_gets_a_suffix() {
    let lab = Lab::new(FakeServer::default());
    let base = recording_id(T0, "p2").unwrap();
    std::fs::create_dir_all(lab.raw(&base)).unwrap();
    std::fs::write(lab.raw(&base).join(RECORD_FILE), "earlier").unwrap();

    let id = id_of(&lab.start(None).await.unwrap());
    assert_eq!(id, format!("{base}-2"));
    let log = lab.log(&id);
    assert_eq!(log.start.recording_id, id);
    assert_eq!(
        draft_file_name(&log.start),
        format!("first-session-{}.toml", &base[..13])
    );
    assert_eq!(
        std::fs::read_to_string(lab.raw(&base).join(RECORD_FILE)).unwrap(),
        "earlier"
    );
    lab.stop(&id, true).await.unwrap();
}

/// An unknown character fails begin: the lease is released, the slot is
/// free, no tap is left running and no empty folder is left behind.
#[tokio::test]
async fn a_begin_failure_releases_the_lease_and_frees_the_slot() {
    let lab = Lab::new(FakeServer::default());
    let req = StartReq {
        character: "Nobody".into(),
        ..lab.start_req()
    };
    let f = lab.start_with(req).await.unwrap_err();
    assert_eq!(f.exit, 1);
    assert!(f.error.contains("no sgw_player named"), "{}", f.error);
    assert!(!lab.book.is_held());
    assert_eq!(lab.status()["recording"], false);
    assert!(lab.slot.reserve().is_ok(), "the slot is free");
    assert_eq!(
        lab.server.count("server_packet_tap_start"),
        lab.server.count("server_packet_tap_stop"),
        "no tap left running"
    );
    let left = std::fs::read_dir(lab.records()).unwrap().count();
    assert_eq!(left, 0, "the claimed folder is removed");
}

/// A panic in the recorder's task still releases its lease, the slot takes
/// a new recording, and a stop of the panicked one says why it ended.
#[tokio::test]
async fn a_panicking_recorder_releases_the_lease_and_frees_the_slot() {
    // Begin reads the first batch; a later tick reads the panic.
    let lab = Lab::new(FakeServer::with_tap(vec![
        json!([]),
        json!([]),
        json!(PANIC),
    ]));
    let id = id_of(&lab.start(None).await.unwrap());
    lab.until("the recorder to end", |s| s["recording"] == false)
        .await;
    assert!(!lab.book.is_held(), "the panic dropped the lease");

    let out = lab.stop(&id, true).await.unwrap();
    let ended = out["ended"].as_str().unwrap();
    assert!(ended.contains("panic"), "{out}");
    assert!(out.to_string().len() < BODY_CAP);

    // Fresh second, so the folder is new; the slot takes it.
    lab.clock
        .fetch_add(1_000, std::sync::atomic::Ordering::SeqCst);
    let next = id_of(&lab.start(None).await.unwrap());
    lab.stop(&next, true).await.unwrap();
}

/// A task that ended with an error (a failed log write) is collected, and
/// the answer carries a short `ended` reason.
#[tokio::test]
async fn a_task_error_is_reported_as_ended() {
    let lab = Lab::new(FakeServer::default());
    let long = format!("record.jsonl: disk full {}", "x".repeat(300));
    let failed = tokio::spawn(async move { Err::<PathBuf, _>(long) });
    arm(&lab, "20261012-143000-p2", failed);
    let out = lab.stop("20261012-143000-p2", true).await.unwrap();
    let ended = out["ended"].as_str().unwrap();
    assert!(ended.starts_with("record.jsonl: disk full"), "{out}");
    assert!(out.to_string().len() < BODY_CAP, "{out}");
}
