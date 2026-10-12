//! Refused and expired tokens, through a real local HTTP server and a real
//! session file. On 2026-10-12 a lab client whose token had expired POSTed
//! every flush for 4.5 h, every one refused with a 401; these pin the
//! re-read, the backoff and the expiry check the uploader has since.

use super::*;
use crate::queue::channel;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// One request as the server saw it: the `Authorization` header and the
/// decoded rows.
type Seen = Arc<Mutex<Vec<(String, Vec<serde_json::Value>)>>>;

/// A local server that answers 401 to `Bearer <refused>` and 200 to any
/// other token, recording every request until `done` is raised.
fn server(refused: &'static str) -> (String, Seen, Arc<AtomicBool>, thread::JoinHandle<()>) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let addr = server.server_addr().to_ip().unwrap();
    let url = format!(
        "http://{}:{}/api/telemetry/upload-chunk",
        addr.ip(),
        addr.port()
    );
    let seen: Seen = Arc::default();
    let done = Arc::new(AtomicBool::new(false));
    let (seen2, done2) = (seen.clone(), done.clone());
    let handle = thread::spawn(move || {
        while !done2.load(Ordering::Relaxed) {
            let Ok(Some(mut req)) = server.recv_timeout(Duration::from_millis(20)) else {
                continue;
            };
            let auth = req
                .headers()
                .iter()
                .find(|h| h.field.equiv("Authorization"))
                .map(|h| h.value.as_str().to_string())
                .unwrap_or_default();
            let mut body = Vec::new();
            req.as_reader().read_to_end(&mut body).unwrap();
            let mut text = String::new();
            flate2::read::GzDecoder::new(&body[..])
                .read_to_string(&mut text)
                .unwrap();
            let rows = text
                .lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect();
            let refused_now = auth == format!("Bearer {refused}");
            seen2.lock().unwrap().push((auth, rows));
            let resp = if refused_now {
                tiny_http::Response::from_string(r#"{"error":"token_expired"}"#)
                    .with_status_code(401)
                    .boxed()
            } else {
                tiny_http::Response::from_string("ok").boxed()
            };
            req.respond(resp).ok();
        }
    });
    (url, seen, done, handle)
}

/// Write a launcher-shaped `current-session.json` carrying `token`.
fn write_session(path: &Path, token: &str, expires_at_ms: i64) {
    let json = serde_json::json!({
        "install_id": "i", "machine_id": "m", "session_id": "s",
        "telemetry": {
            "enabled": true,
            "token": token,
            "expires_at_ms": expires_at_ms,
            "upload_endpoint": "http://unused/api/telemetry",
        }
    });
    // Write then rename, like a careful launcher, so a re-read never sees a
    // half-written file.
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, json.to_string()).unwrap();
    std::fs::rename(&tmp, path).unwrap();
}

fn cfg(url: String, token: &str, expires_at_ms: i64, file: &Path, flush_ms: u64) -> UploaderConfig {
    UploaderConfig {
        upload_endpoint: url,
        token: token.into(),
        expires_at_ms,
        session_file: Some(file.to_path_buf()),
        flush_interval: Duration::from_millis(flush_ms),
        max_batch: 100,
    }
}

/// Run the uploader for `run` and stop it. The producer outlives the loop,
/// as in the DLL, so the stop path (not `Disconnected`) ends it.
fn run_for(cfg: UploaderConfig, run: Duration, during: impl FnOnce()) {
    let (p, c) = channel();
    p.try_emit(ClientNativeEvent::builder("client.test", "info").field("n", serde_json::json!(1)));
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let upload = thread::spawn(move || run_uploader(c, cfg, move || stop2.load(Ordering::Relaxed)));
    let started = Instant::now();
    during();
    thread::sleep(run.saturating_sub(started.elapsed()));
    stop.store(true, Ordering::Relaxed);
    assert_eq!(upload.join().unwrap(), UploaderExit::Stopped);
    drop(p);
}

/// Seqs of the rows with `target` across every request (a retained batch
/// carries the same row in each retry, so distinct seqs count reports).
fn report_seqs(seen: &Seen, target: &str) -> std::collections::BTreeSet<u64> {
    seen.lock()
        .unwrap()
        .iter()
        .flat_map(|(_, rows)| rows.iter())
        .filter(|r| r["target"] == target)
        .map(|r| r["seq"].as_u64().unwrap())
        .collect()
}

/// A 401 makes the uploader re-read the session file, and the rotated token
/// there is used for an immediate retry of the same batch. Without the
/// re-read every POST carries the refused token.
#[test]
fn a_401_rereads_the_session_file_and_retries_with_the_rotated_token() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("current-session.json");
    // The launcher already rotated: the file holds the new token, the DLL
    // still the old one.
    write_session(&file, "new", 0);
    let (url, seen, done, handle) = server("old");
    run_for(
        cfg(url, "old", 0, &file, 50),
        Duration::from_millis(300),
        || {},
    );
    done.store(true, Ordering::Relaxed);
    handle.join().unwrap();

    {
        let got = seen.lock().unwrap();
        assert!(got.len() >= 2, "a refusal and its retry: {got:?}");
        assert_eq!(got[0].0, "Bearer old");
        assert_eq!(got[1].0, "Bearer new", "the retry uses the re-read token");
        assert_eq!(got[1].1, got[0].1, "the retry carries the refused batch");
        assert!(
            got[1..].iter().all(|(auth, _)| auth == "Bearer new"),
            "the refused token is never sent again"
        );
    }
    assert!(
        report_seqs(&seen, UPLOAD_PAUSED_TARGET).is_empty(),
        "a swap that works is not a pause"
    );
}

/// With the same token in the file, refusals back off instead of retrying
/// every flush, and the pause is reported once. At a 20 ms flush the old
/// loop sent about 35 requests in 700 ms; the backoff (20, 40, 80, 160,
/// 320 ms) leaves well under ten.
#[test]
fn refused_uploads_back_off_instead_of_retrying_every_flush() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("current-session.json");
    write_session(&file, "old", 0);
    let (url, seen, done, handle) = server("old");
    run_for(
        cfg(url, "old", 0, &file, 20),
        Duration::from_millis(700),
        || {},
    );
    done.store(true, Ordering::Relaxed);
    handle.join().unwrap();

    let n = seen.lock().unwrap().len();
    assert!(
        (2..=9).contains(&n),
        "expected a handful of backed-off POSTs, got {n}"
    );
    assert_eq!(
        report_seqs(&seen, UPLOAD_PAUSED_TARGET).len(),
        1,
        "one pause report, however many attempts"
    );
}

/// A rotation that lands during the pause is found at a backoff deadline;
/// the next POST uses it, and the resume is reported once.
#[test]
fn a_rotation_during_the_pause_resumes_uploads() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("current-session.json");
    write_session(&file, "old", 0);
    let (url, seen, done, handle) = server("old");
    let file2 = file.clone();
    run_for(
        cfg(url, "old", 0, &file, 20),
        Duration::from_millis(1200),
        move || {
            thread::sleep(Duration::from_millis(150));
            write_session(&file2, "new", 0);
        },
    );
    done.store(true, Ordering::Relaxed);
    handle.join().unwrap();

    let accepted: Vec<_> = seen
        .lock()
        .unwrap()
        .iter()
        .filter(|(auth, _)| auth == "Bearer new")
        .cloned()
        .collect();
    assert!(!accepted.is_empty(), "the rotated token was never used");
    assert!(
        accepted[0]
            .1
            .iter()
            .any(|r| r["target"] == UPLOAD_PAUSED_TARGET),
        "the first accepted POST carries the pause report"
    );
    assert_eq!(report_seqs(&seen, UPLOAD_PAUSED_TARGET).len(), 1);
    assert_eq!(
        report_seqs(&seen, UPLOAD_RESUMED_TARGET).len(),
        1,
        "one resume report, shipped with a later POST"
    );
}

/// A token past its `expires_at_ms`, with nothing newer in the file, is
/// never sent: not on the cadence and not in the final flush at exit.
#[test]
fn an_expired_token_is_never_sent() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("current-session.json");
    write_session(&file, "dead", 1);
    let (url, seen, done, handle) = server("dead");
    run_for(
        cfg(url, "dead", 1, &file, 20),
        Duration::from_millis(300),
        || {},
    );
    done.store(true, Ordering::Relaxed);
    handle.join().unwrap();
    assert_eq!(
        seen.lock().unwrap().len(),
        0,
        "no request with an expired token"
    );
}
