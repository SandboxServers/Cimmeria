//! Record tool tests over the fakes in [`super::fakes`]: no lab, no
//! client. Each guard fails when its rule is reverted (the worknote lists
//! the mutations tried).

use std::sync::atomic::Ordering;

use serde_json::json;

use super::fakes::{id_of, interact, log_at, FakeServer, Lab, T0};
use super::run::{max_minutes_of, Failure, BODY_CAP, RECORD_FILE};
use super::stop::{draft_outcome, TRANSFORM_ERROR_FILE};
use super::*;
use crate::lease::AcquireRequest;
use crate::uat::record::emit::draft_file_name;
use crate::uat::record::log::read_log;

#[tokio::test]
async fn start_refuses_a_second_recording_on_the_instance() {
    let lab = Lab::new(FakeServer::default());
    let a = lab.start(None).await.unwrap();
    assert_eq!(a["ok"], true);
    assert_eq!(a["instance"], "p2");
    assert!(id_of(&a).ends_with("-p2"), "{a}");

    let err = lab.start(None).await.unwrap_err();
    assert_eq!(err.exit, 2);
    assert!(err.error.contains("already running"), "{}", err.error);
    assert_eq!(
        lab.server.count("server_packet_tap_start"),
        1,
        "a refused start never reaches the server"
    );
    assert_eq!(lab.status()["recording_id"], a["recording_id"]);
    lab.stop(&id_of(&a), true).await.unwrap();
}

/// The lab is someone else's: refused before any server call.
#[tokio::test]
async fn start_is_refused_while_the_lab_is_leased() {
    let lab = Lab::new(FakeServer::default());
    lab.book
        .acquire(AcquireRequest {
            owner: "agent".into(),
            purpose: "driving".into(),
            ..Default::default()
        })
        .unwrap();
    let err = lab.start(None).await.unwrap_err();
    assert_eq!(err.exit, 2, "{}", err.error);
    assert!(lab.server.calls.lock().unwrap().is_empty());
    assert_eq!(lab.status()["recording"], false);
    // The refused start freed the slot.
    assert!(lab.slot.reserve().is_ok());
}

#[tokio::test]
async fn stop_with_a_wrong_id_is_refused_and_keeps_recording() {
    let lab = Lab::new(FakeServer::default());
    let id = id_of(&lab.start(None).await.unwrap());
    let err = lab.stop("20261012-000000-p2", false).await.unwrap_err();
    assert_eq!(err.exit, 2);
    assert!(err.error.contains(&id), "{}", err.error);

    let before = lab.status()["ticks"].as_u64().unwrap();
    lab.until("more ticks", |s| {
        s["recording"] == true && s["ticks"].as_u64().unwrap() > before + 2
    })
    .await;
    assert!(lab.book.is_held(), "the recorder still holds its lease");
    lab.stop(&id, true).await.unwrap();
}

#[tokio::test]
async fn stop_writes_the_draft_and_releases_the_lease() {
    let lab = Lab::new(FakeServer::with_tap(vec![json!([]), json!([interact()])]));
    let started = lab.start(None).await.unwrap();
    let id = id_of(&started);
    assert_eq!(lab.book.status()["lease"]["owner"], "lab_record");
    assert_eq!(
        lab.book.status()["lease"]["purpose"],
        "recording a human: first-session"
    );
    lab.until("ticks", |s| s["ticks"].as_u64().unwrap() >= 3)
        .await;

    let out = lab.stop(&id, false).await.unwrap();
    let draft = PathBuf::from(out["draft"].as_str().unwrap());
    assert!(draft.is_file(), "{out}");
    assert!(draft.starts_with(lab.specs().join("drafts")), "{out}");
    assert!(
        std::fs::read_to_string(&draft)
            .unwrap()
            .contains("UNCONFIRMED"),
        "a draft, not something else"
    );
    assert!(
        !lab.book.is_held(),
        "the stop released the recorder's lease"
    );
    assert_eq!(lab.book.status()["recent"][0]["owner"], "lab_record");
    assert_eq!(lab.status()["recording"], false);
    let log = log_at(&out["raw"]);
    assert_eq!(log.stop.as_ref().unwrap().reason, "stopped");
    assert_eq!(lab.server.count("server_packet_tap_stop"), 1);

    // D-LR16: one object under 400 characters, no nulls.
    let body = out.to_string();
    assert!(body.len() < BODY_CAP, "{} chars: {body}", body.len());
    assert!(!body.contains("null"), "{body}");
    for k in ["draft", "rows", "steps", "todo", "dropped", "raw"] {
        assert!(out.get(k).is_some(), "{k} missing: {body}");
    }
}

#[tokio::test]
async fn stop_never_overwrites_a_draft() {
    let lab = Lab::new(FakeServer::default());
    let started = lab.start(None).await.unwrap();
    let dir = PathBuf::from(started["dir"].as_str().unwrap());
    // The start line is on disk once start returns: the draft's name is known.
    let text = std::fs::read_to_string(dir.join(RECORD_FILE)).unwrap();
    let name = draft_file_name(&read_log(&text).unwrap().start);
    let drafts = lab.specs().join("drafts");
    std::fs::create_dir_all(&drafts).unwrap();
    std::fs::write(drafts.join(&name), "keep me").unwrap();

    let err = lab.stop(&id_of(&started), false).await.unwrap_err();
    assert_eq!(err.exit, 1);
    assert!(err.error.contains("not overwritten"), "{}", err.error);
    assert_eq!(
        std::fs::read_to_string(drafts.join(&name)).unwrap(),
        "keep me"
    );
    assert!(dir.join(RECORD_FILE).is_file(), "the raw log is kept");
    assert!(!lab.book.is_held());
}

/// LR-06 carried item: a forced takeover makes the recorder's client reads
/// fail (they run under its lease's permit) and stops it with `lease_lost`;
/// the taker's lease stays.
#[tokio::test]
async fn a_takeover_stops_the_recording_with_lease_lost() {
    let lab = Lab::new(FakeServer::default());
    let id = id_of(&lab.start(None).await.unwrap());
    lab.until("ticks", |s| s["ticks"].as_u64().unwrap() >= 2)
        .await;
    assert_eq!(lab.client.refused.load(Ordering::SeqCst), 0);

    lab.book
        .acquire(AcquireRequest {
            owner: "other".into(),
            purpose: "takeover".into(),
            force: true,
            reason: Some("test".into()),
            ..Default::default()
        })
        .unwrap();
    lab.until("the recorder to stop", |s| s["recording"] == false)
        .await;

    let out = lab.stop(&id, true).await.unwrap();
    let log = log_at(&out["raw"]);
    assert_eq!(log.stop.as_ref().unwrap().reason, "lease_lost");
    assert!(
        lab.client.refused.load(Ordering::SeqCst) > 0,
        "the final tick's client reads ran under the revoked lease"
    );
    let last = log.ticks.last().unwrap();
    assert!(
        last.errors.iter().any(|e| e.contains("lease revoked")),
        "{:?}",
        last.errors
    );
    assert_eq!(lab.book.status()["lease"]["owner"], "other");
}

/// LR-06 carried item: the wall-clock limit ends a recording even when
/// `max_ticks` is far away.
#[tokio::test]
async fn max_minutes_stops_on_host_time() {
    let lab = Lab::new(FakeServer::default());
    let id = id_of(&lab.start(Some(1)).await.unwrap());
    lab.until("ticks", |s| s["ticks"].as_u64().unwrap() >= 2)
        .await;
    assert_eq!(lab.status()["recording"], true);
    lab.clock.fetch_add(61_000, Ordering::SeqCst);
    lab.until("the recorder to stop", |s| s["recording"] == false)
        .await;

    let out = lab.stop(&id, true).await.unwrap();
    let log = log_at(&out["raw"]);
    assert_eq!(log.stop.as_ref().unwrap().reason, "max_minutes");
    assert!(
        log.ticks.len() < 1000,
        "stopped by the clock, not the tick cap"
    );
    assert!(!lab.book.is_held());
}

#[test]
fn max_minutes_defaults_to_and_is_capped_at_30() {
    assert_eq!(max_minutes_of(None).unwrap(), 30);
    assert_eq!(max_minutes_of(Some(45)).unwrap(), 30);
    assert_eq!(max_minutes_of(Some(5)).unwrap(), 5);
    assert_eq!(max_minutes_of(Some(0)).unwrap_err().exit, 2);
}

/// Anchors are counted through LR-02's `decode_call`: an outbound row, a
/// base call that only shares a name, a `setTargetID` and noise are not
/// anchors; the interact and the useAbility are.
#[tokio::test]
async fn status_counts_anchors_with_decode_call() {
    let rows = json!([
        interact(),
        { "dir": "out", "msg_id": 189, "method_index": 74, "msg_name": "interact",
          "args_hex": "000000000d8f890100" },
        // A base message (0xC0 and up) is never read through the cell table.
        { "dir": "in", "msg_id": 0xC3, "method_index": 3, "msg_name": "useItem",
          "args_hex": "00000000050000008f890100" },
        { "dir": "in", "msg_id": 186, "method_index": 0, "msg_name": "setTargetID",
          "args_hex": "000000008f890100" },
        { "dir": "in", "msg_id": 186, "method_index": 68, "msg_name": "useAbility",
          "args_hex": "00000000050000008f890100" },
        { "dir": "in", "msg_name": "perfStats", "args_hex": "00" },
    ]);
    let lab = Lab::new(FakeServer::with_tap(vec![json!([]), rows]));
    let id = id_of(&lab.start(None).await.unwrap());
    lab.until("ticks", |s| s["ticks"].as_u64().unwrap() >= 4)
        .await;
    assert_eq!(lab.status()["anchors"], 2, "{}", lab.status());
    lab.stop(&id, true).await.unwrap();
}

/// D-LR16: a failed transform keeps the raw log, writes its whole error
/// beside it, writes no draft, and answers `{ok:false, exit, error}` under
/// 400 characters.
#[test]
fn a_failed_transform_keeps_the_raw_log_and_answers_the_failure_shape() {
    let root = tempfile::tempdir().unwrap();
    let raw = root.path().join("records").join("20261012-143000-p2");
    std::fs::create_dir_all(&raw).unwrap();
    std::fs::write(raw.join(RECORD_FILE), "{\"start\":1}\n").unwrap();
    let specs = root.path().join("specs");
    let long = format!(
        "the draft does not parse: {}",
        "row REC-01 \\ bad; ".repeat(100)
    );

    let f = draft_outcome(&raw, Some(&specs), Err(long.clone())).unwrap_err();
    assert_eq!(f.exit, 1);
    let body = f.body();
    assert!(body.len() < BODY_CAP, "{} chars: {body}", body.len());
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["exit"], 1);
    assert!(
        v["error"].as_str().unwrap().starts_with("transform: "),
        "{v}"
    );
    assert_eq!(v.as_object().unwrap().len(), 3, "{v}");
    assert_eq!(
        std::fs::read_to_string(raw.join(TRANSFORM_ERROR_FILE)).unwrap(),
        long
    );
    assert_eq!(
        std::fs::read_to_string(raw.join(RECORD_FILE)).unwrap(),
        "{\"start\":1}\n"
    );
    assert!(
        !specs.join("drafts").exists(),
        "no draft for a failed transform"
    );
}

fn supervisor(instance: Option<&str>) -> Arc<crate::supervisor::Supervisor> {
    use crate::client::BridgeClient;
    use crate::supervisor::{Supervisor, SupervisorConfig};
    let config = SupervisorConfig {
        install_dir: None,
        dll_path: None,
        patches_dll: None,
        helper_path: None,
        bind: "127.0.0.1".into(),
        port: 8770,
        instance: instance.map(String::from),
        telemetry: Default::default(),
    };
    let bridge = Arc::new(BridgeClient::new("127.0.0.1:1", ""));
    Arc::new(Supervisor::new(bridge, config).with_leases(Arc::default()))
}

/// F12: status and stop act on the instance the call is routed to. A
/// recording armed on p2 is invisible to (and not stopped by) a call that
/// lands on the default instance.
#[tokio::test]
async fn status_and_stop_route_by_instance() {
    use crate::daemon::http_tests::{spawn_daemon, McpSession};
    use crate::server::{Hosted, Instances};
    let p2 = supervisor(Some("p2"));
    let server = LabServer::new_multi(Instances::new(vec![
        Hosted {
            label: "default".into(),
            supervisor: supervisor(None),
        },
        Hosted {
            label: "p2".into(),
            supervisor: p2.clone(),
        },
    ]));
    let running = tokio::spawn(async {
        std::future::pending::<()>().await;
        Ok(PathBuf::new())
    });
    p2.record_slot()
        .reserve()
        .unwrap()
        .arm(crate::supervisor::record_slot::RecordHandle {
            recording_id: "20261012-143000-p2".into(),
            dir: PathBuf::from("records"),
            section: "first-session".into(),
            specs_dir: None,
            started_ms: T0,
            ticks: Arc::default(),
            anchors: Arc::default(),
            stop: tokio::sync::watch::channel(None).0,
            done: running,
        })
        .unwrap();
    let url = spawn_daemon(server).await;
    let mut s = McpSession::open(&url).await;
    let text = |r: &Value| -> Value {
        serde_json::from_str(r["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    };

    let st = text(
        &s.call("lab_record_status", json!({ "instance": "p2" }))
            .await,
    );
    assert_eq!(st["recording_id"], "20261012-143000-p2", "{st}");
    assert_eq!(st["instance"], "p2");
    let st = text(&s.call("lab_record_status", json!({})).await);
    assert_eq!(st["recording"], false, "{st}");
    assert_eq!(st["instance"], "default");

    let r = s
        .call(
            "lab_record_stop",
            json!({ "recording_id": "20261012-143000-p2" }),
        )
        .await;
    assert_eq!(r["result"]["isError"], true, "{r}");
    assert_eq!(text(&r)["exit"], 2, "{r}");
    assert!(p2.record_slot().status().unwrap().running);
    p2.record_slot().take().unwrap().done.abort();
}

#[test]
fn a_short_failure_is_not_cut() {
    let f = Failure {
        exit: 2,
        error: "no recording on this instance".into(),
    };
    let v: Value = serde_json::from_str(&f.body()).unwrap();
    assert_eq!(
        v,
        json!({ "ok": false, "exit": 2, "error": "no recording on this instance" })
    );
}
