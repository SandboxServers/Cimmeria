//! Refused and expired tokens, through a real local HTTP server and a real
//! session file. On 2026-10-12 a lab client whose token had expired POSTed
//! every flush for 4.5 h, every one refused with a 401; these pin the
//! re-read, the backoff, the expiry probe and the loop's behaviour during a
//! hold.
//!
//! Timing: each test waits for what it needs by polling with a generous
//! deadline instead of sleeping a fixed time, and asserts only bounds a
//! starved CI runner cannot break (a slow runner makes fewer attempts, never
//! more).

use super::*;
use crate::governor::GovernorConfig;
use crate::queue::{channel, governed_channel, Producer};
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// One request as the server saw it: the `Authorization` header and the
/// decoded rows.
type Seen = Arc<Mutex<Vec<(String, Vec<serde_json::Value>)>>>;

/// How long a test waits for something it expects before failing.
const PATIENCE: Duration = Duration::from_secs(20);

/// A local server that answers 401 to the tokens `refuse` says no to and
/// 200 to the rest, recording every request until `done` is raised.
/// `on_request` runs before each answer (for a test that rotates the file
/// from inside the server).
struct TestServer {
    url: String,
    seen: Seen,
    done: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl TestServer {
    fn start(
        refuse: impl Fn(&str) -> bool + Send + 'static,
        on_request: impl Fn(&str) + Send + 'static,
    ) -> Self {
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
                let token = auth.strip_prefix("Bearer ").unwrap_or("").to_string();
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
                on_request(&token);
                seen2.lock().unwrap().push((token.clone(), rows));
                let resp = if refuse(&token) {
                    tiny_http::Response::from_string(r#"{"error":"token_expired"}"#)
                        .with_status_code(401)
                        .boxed()
                } else {
                    tiny_http::Response::from_string("ok").boxed()
                };
                req.respond(resp).ok();
            }
        });
        Self {
            url,
            seen,
            done,
            handle: Some(handle),
        }
    }

    fn refusing(token: &'static str) -> Self {
        Self::start(move |t| t == token, |_| {})
    }

    fn count(&self) -> usize {
        self.seen.lock().unwrap().len()
    }

    fn tokens(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|(t, _)| t.clone())
            .collect()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
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
    // Write then rename (the launcher's `atomic_write` does the same), so a
    // re-read never sees a half-written file.
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, json.to_string()).unwrap();
    std::fs::rename(&tmp, path).unwrap();
}

fn cfg(url: &str, token: &str, expires_at_ms: i64, file: &Path, flush_ms: u64) -> UploaderConfig {
    UploaderConfig {
        upload_endpoint: url.to_string(),
        token: token.into(),
        expires_at_ms,
        session_file: Some(file.to_path_buf()),
        flush_interval: Duration::from_millis(flush_ms),
        max_batch: 100,
    }
}

/// A running uploader. Stopping it goes through the stop path (the producer
/// outlives the loop, as in the DLL). `wakes` counts loop passes: the loop
/// asks `should_stop` once per pass.
struct Running {
    producer: Producer,
    stop: Arc<AtomicBool>,
    wakes: Arc<AtomicUsize>,
    handle: thread::JoinHandle<UploaderExit>,
}

fn start_with(producer: Producer, consumer: Consumer, cfg: UploaderConfig) -> Running {
    producer.try_emit(
        ClientNativeEvent::builder("client.test", "info").field("n", serde_json::json!(1)),
    );
    let stop = Arc::new(AtomicBool::new(false));
    let wakes = Arc::new(AtomicUsize::new(0));
    let (stop2, wakes2) = (stop.clone(), wakes.clone());
    let handle = thread::spawn(move || {
        run_uploader(consumer, cfg, move || {
            wakes2.fetch_add(1, Ordering::Relaxed);
            stop2.load(Ordering::Relaxed)
        })
    });
    Running {
        producer,
        stop,
        wakes,
        handle,
    }
}

fn start(cfg: UploaderConfig) -> Running {
    let (p, c) = channel();
    start_with(p, c, cfg)
}

impl Running {
    fn stop(self) {
        self.stop.store(true, Ordering::Relaxed);
        assert_eq!(self.handle.join().unwrap(), UploaderExit::Stopped);
        drop(self.producer);
    }
}

/// Poll `cond` until it holds or [`PATIENCE`] runs out.
fn wait_until(what: &str, cond: impl Fn() -> bool) {
    let deadline = Instant::now() + PATIENCE;
    while !cond() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(10));
    }
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
    // The file holds a newer token than the DLL loaded.
    write_session(&file, "new", 0);
    let srv = TestServer::refusing("old");
    let up = start(cfg(&srv.url, "old", 0, &file, 50));
    wait_until("the refusal and its retry", || srv.count() >= 2);
    up.stop();

    let got = srv.seen.lock().unwrap().clone();
    assert_eq!(got[0].0, "old");
    assert_eq!(got[1].0, "new", "the retry uses the re-read token");
    assert_eq!(got[1].1, got[0].1, "the retry carries the refused batch");
    assert!(
        got[1..].iter().all(|(t, _)| t == "new"),
        "the refused token is never sent again"
    );
    assert!(
        report_seqs(&srv.seen, UPLOAD_PAUSED_TARGET).is_empty(),
        "a swap that works is not a pause"
    );
}

/// The server rotates the file again when it refuses the first rotated
/// token: the uploader adopts the second one without a backoff hold and the
/// batch goes through with it.
#[test]
fn a_second_rotation_is_adopted_without_a_hold() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("current-session.json");
    write_session(&file, "new1", 0);
    let file2 = file.clone();
    let srv = TestServer::start(
        |t| t == "old" || t == "new1",
        move |t| {
            if t == "new1" {
                write_session(&file2, "new2", 0);
            }
        },
    );
    let up = start(cfg(&srv.url, "old", 0, &file, 50));
    wait_until("a POST with the second rotated token", || {
        srv.tokens().iter().any(|t| t == "new2")
    });
    up.stop();
    let tokens = srv.tokens();
    assert_eq!(&tokens[..3], ["old", "new1", "new2"], "{tokens:?}");
    assert!(
        report_seqs(&srv.seen, UPLOAD_PAUSED_TARGET).is_empty(),
        "each refusal found a new token: no pause"
    );
}

/// With the same token in the file, refusals back off instead of retrying
/// every flush, and the pause is reported once. At a 20 ms flush the old
/// loop sent about 35 requests in 700 ms; with the backoff (20, 40, 80, 160,
/// 320 ms) at most about seven fit, and a slow runner fits fewer.
#[test]
fn refused_uploads_back_off_instead_of_retrying_every_flush() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("current-session.json");
    write_session(&file, "old", 0);
    let srv = TestServer::refusing("old");
    let up = start(cfg(&srv.url, "old", 0, &file, 20));
    wait_until("the first refusal", || srv.count() >= 1);
    let first = Instant::now();
    thread::sleep(Duration::from_millis(700));
    let in_window = srv.count();
    up.stop();
    // Elapsed since the first refusal, however long the runner took.
    let window = first.elapsed();
    let backoff_bound = 4 + (window.as_millis() / 20).max(2).ilog2() as usize;
    assert!(
        in_window <= backoff_bound,
        "{in_window} POSTs in {window:?}; a doubling backoff allows {backoff_bound}"
    );
    assert_eq!(
        report_seqs(&srv.seen, UPLOAD_PAUSED_TARGET).len(),
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
    let srv = TestServer::refusing("old");
    let up = start(cfg(&srv.url, "old", 0, &file, 20));
    // Paused once the same token has been refused twice.
    wait_until("two refusals", || srv.count() >= 2);
    write_session(&file, "new", 0);
    wait_until("the resume report", || {
        !report_seqs(&srv.seen, UPLOAD_RESUMED_TARGET).is_empty()
    });
    up.stop();

    let accepted: Vec<_> = srv
        .seen
        .lock()
        .unwrap()
        .iter()
        .filter(|(t, _)| t == "new")
        .cloned()
        .collect();
    assert!(
        accepted[0]
            .1
            .iter()
            .any(|r| r["target"] == UPLOAD_PAUSED_TARGET),
        "the first accepted POST carries the pause report"
    );
    assert_eq!(report_seqs(&srv.seen, UPLOAD_PAUSED_TARGET).len(), 1);
    assert_eq!(report_seqs(&srv.seen, UPLOAD_RESUMED_TARGET).len(), 1);
}

/// A token past its `expires_at_ms`, with nothing newer in the file, goes
/// out as one probe and is then held: no POST per flush, and none at exit.
#[test]
fn an_expired_token_is_probed_once_and_then_held() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("current-session.json");
    write_session(&file, "dead", 1);
    let srv = TestServer::refusing("dead");
    let up = start(cfg(&srv.url, "dead", 1, &file, 20));
    wait_until("the probe", || srv.count() >= 1);
    thread::sleep(Duration::from_millis(300));
    up.stop();
    assert_eq!(srv.count(), 1, "one probe; the next is a cap away");
}

/// While a hold runs the loop sleeps until the hold ends (capped at the stop
/// poll), instead of waking every millisecond. The old loop, with a held
/// batch past its cadence, passed about 300 times in 300 ms; the sleep
/// leaves a handful (one per 100 ms stop poll).
#[test]
fn the_loop_sleeps_through_a_hold() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("current-session.json");
    write_session(&file, "dead", 1);
    let srv = TestServer::refusing("dead");
    let up = start(cfg(&srv.url, "dead", 1, &file, 20));
    wait_until("the probe", || srv.count() >= 1);
    let before = up.wakes.load(Ordering::Relaxed);
    let t = Instant::now();
    thread::sleep(Duration::from_millis(300));
    let passes = up.wakes.load(Ordering::Relaxed) - before;
    let window = t.elapsed();
    up.stop();
    // One pass per 100 ms stop poll, plus slack for a wake on the edges.
    let bound = (window.as_millis() / 100) as usize + 10;
    assert!(
        passes <= bound,
        "{passes} loop passes in {window:?} during a hold (bound {bound})"
    );
}

/// The governor keeps adding health rows while nothing ships, and the batch
/// stays bounded anyway: the oldest half goes once it reaches 2× `max_batch`,
/// and the drops are counted. Without the per-pass trim the held batch grew
/// by one health row per pass, for as long as the hold lasted.
#[test]
fn a_held_batch_stays_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("current-session.json");
    write_session(&file, "dead", 1);
    let srv = TestServer::refusing("dead");
    // A health row on every governor tick, which runs once per loop pass.
    let (p, c) = governed_channel(GovernorConfig {
        health_every_ms: 1,
        ..GovernorConfig::default()
    });
    let mut config = cfg(&srv.url, "dead", 1, &file, 20);
    config.max_batch = 2;
    let up = start_with(p, c, config);
    wait_until("the probe", || srv.count() >= 1);
    // Enough passes (one per 100 ms) for the old loop to pile up rows.
    let base = up.wakes.load(Ordering::Relaxed);
    wait_until("twenty loop passes", || {
        up.wakes.load(Ordering::Relaxed) >= base + 20
    });
    // A usable token lets the exit flush ship what the batch holds.
    write_session(&file, "fresh", 0);
    up.stop();
    let got = srv.seen.lock().unwrap().clone();
    let (token, last) = got.last().unwrap();
    assert_eq!(token, "fresh", "the exit flush used the new token");
    // At most 2 × max_batch held, plus the closing rollup and health rows.
    assert!(
        last.len() <= 2 * 2 + 4,
        "the held batch grew to {} rows",
        last.len()
    );
    let health = last
        .iter()
        .rfind(|r| r["target"] == "client.telemetry.health")
        .expect("a final health row");
    assert!(
        health["fields"]["upload_dropped_total"].as_u64().unwrap() > 0,
        "the trimmed rows are counted: {health}"
    );
}
