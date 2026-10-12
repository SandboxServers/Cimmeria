//! TG-PIPE-03: the final flush at exit, through a real local HTTP server.

use super::*;
use crate::governor::GovernorConfig;
use crate::queue::governed_channel;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

fn decode(body: &[u8]) -> Vec<serde_json::Value> {
    let mut text = String::new();
    flate2::read::GzDecoder::new(body)
        .read_to_string(&mut text)
        .unwrap();
    text.lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// The stop path ships a last POST whose health row says `final: true`,
/// and an ordinary periodic health row does not. Reverting `governor_finish`
/// to the plain health row (or dropping the final POST) fails this.
#[test]
fn the_last_post_holds_a_health_row_marked_final() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let addr = server.server_addr().to_ip().unwrap();
    let url = format!("http://{}:{}/x", addr.ip(), addr.port());

    let posts: Arc<Mutex<Vec<Vec<serde_json::Value>>>> = Arc::default();
    let posts2 = posts.clone();
    let server_done = Arc::new(AtomicBool::new(false));
    let server_done2 = server_done.clone();
    let server_handle = thread::spawn(move || {
        while !server_done2.load(Ordering::Relaxed) {
            if let Ok(Some(mut req)) = server.recv_timeout(Duration::from_millis(50)) {
                let mut body = Vec::new();
                req.as_reader().read_to_end(&mut body).unwrap();
                posts2.lock().unwrap().push(decode(&body));
                req.respond(tiny_http::Response::from_string("ok")).ok();
            }
        }
    });

    let (p, c) = governed_channel(GovernorConfig::default());
    p.try_emit(ClientNativeEvent::builder("client.test", "info").field("n", serde_json::json!(1)));

    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let upload = thread::spawn(move || {
        let cfg = UploaderConfig {
            upload_endpoint: url,
            token: "t".into(),
            expires_at_ms: 0,
            session_file: None,
            // Long enough that only the stop path can ship the health row.
            flush_interval: Duration::from_secs(30),
            max_batch: 100,
        };
        run_uploader(c, cfg, move || stop2.load(Ordering::Relaxed))
    });

    // The stop must be noticed inside a poll tick, not a flush interval.
    thread::sleep(Duration::from_millis(50));
    let raised = Instant::now();
    stop.store(true, Ordering::Relaxed);
    let exit = upload.join().unwrap();
    let took = raised.elapsed();
    server_done.store(true, Ordering::Relaxed);
    server_handle.join().unwrap();
    drop(p);

    assert_eq!(exit, UploaderExit::Stopped);
    assert!(
        took < Duration::from_secs(2),
        "stop took {took:?}; the idle loop must wake on a short tick"
    );
    let posts = posts.lock().unwrap();
    let last = posts.last().expect("the stop path posted a last batch");
    let health: Vec<_> = last
        .iter()
        .filter(|e| e["target"] == "client.telemetry.health")
        .collect();
    assert_eq!(health.len(), 1, "one health row in the last POST: {last:?}");
    assert_eq!(health[0]["fields"]["final"], serde_json::json!(true));
    assert!(
        last.iter().any(|e| e["target"] == "client.test"),
        "the queued event rides the same POST"
    );
}

/// TG-CLI-04, end to end: an exit hook's flush puts exactly one
/// `client.session.end {reason}` in the final POST, ahead of the closing
/// health row, and a second exit hook adds none. Dropping the emit from
/// `Shutdown::flush_reporting` fails the count.
#[test]
fn the_final_post_carries_one_session_end_before_the_final_health_row() {
    use crate::shutdown::{ExitReason, FlushOutcome, Shutdown, SESSION_END_TARGET};

    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let addr = server.server_addr().to_ip().unwrap();
    let url = format!("http://{}:{}/x", addr.ip(), addr.port());
    let posts: Arc<Mutex<Vec<Vec<serde_json::Value>>>> = Arc::default();
    let posts2 = posts.clone();
    let server_done = Arc::new(AtomicBool::new(false));
    let server_done2 = server_done.clone();
    let server_handle = thread::spawn(move || {
        while !server_done2.load(Ordering::Relaxed) {
            if let Ok(Some(mut req)) = server.recv_timeout(Duration::from_millis(50)) {
                let mut body = Vec::new();
                req.as_reader().read_to_end(&mut body).unwrap();
                posts2.lock().unwrap().push(decode(&body));
                req.respond(tiny_http::Response::from_string("ok")).ok();
            }
        }
    });

    let (p, c) = governed_channel(GovernorConfig::default());
    let shutdown = Arc::new(Shutdown::new());
    shutdown.arm();
    let upload = {
        let shutdown = shutdown.clone();
        thread::spawn(move || {
            let cfg = UploaderConfig {
                upload_endpoint: url,
                token: "t".into(),
                expires_at_ms: 0,
                session_file: None,
                flush_interval: Duration::from_secs(30),
                max_batch: 100,
            };
            let exit = run_uploader(c, cfg, || shutdown.stop_requested());
            shutdown.mark_flushed();
            exit
        })
    };

    let budget = Duration::from_secs(5);
    let first = shutdown.flush_reporting(ExitReason::CrtExit, budget, |ev| {
        p.try_emit(ev);
    });
    let second = shutdown.flush_reporting(ExitReason::ExitProcess, budget, |ev| {
        p.try_emit(ev);
    });
    assert_eq!(upload.join().unwrap(), UploaderExit::Stopped);
    server_done.store(true, Ordering::Relaxed);
    server_handle.join().unwrap();
    assert_eq!(
        (first, second),
        (FlushOutcome::Flushed, FlushOutcome::Flushed)
    );

    let posts = posts.lock().unwrap();
    let all: Vec<_> = posts.iter().flatten().collect();
    let ends: Vec<_> = all
        .iter()
        .filter(|e| e["target"] == SESSION_END_TARGET)
        .collect();
    assert_eq!(ends.len(), 1, "one session end across every POST: {all:?}");
    assert_eq!(ends[0]["fields"]["reason"], "crt_exit");
    let last = posts.last().expect("the stop path posted");
    let at = |t: &str| last.iter().position(|e| e["target"] == t);
    let end_at = at(SESSION_END_TARGET).expect("the session end is in the final POST");
    let health_at = at("client.telemetry.health").expect("the final health row");
    assert!(
        end_at < health_at,
        "session end precedes the closing health row"
    );
}

/// A server that never answers cannot hold the exit: the final POST gives
/// up inside `FINAL_POST_TIMEOUT` instead of the agent's 30 s read limit.
#[test]
fn the_final_post_is_bounded_when_the_server_stalls() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/x", listener.local_addr().unwrap());
    // Accept and hold the connection open without ever replying.
    let held = thread::spawn(move || {
        let (conn, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_secs(5));
        drop(conn);
    });

    let (p, c) = governed_channel(GovernorConfig::default());
    p.try_emit(ClientNativeEvent::builder("client.test", "info"));
    let cfg = UploaderConfig {
        upload_endpoint: url,
        token: "t".into(),
        expires_at_ms: 0,
        session_file: None,
        flush_interval: Duration::from_secs(30),
        max_batch: 100,
    };
    let started = Instant::now();
    let exit = run_uploader(c, cfg, || true);
    let took = started.elapsed();
    drop(p);

    assert_eq!(exit, UploaderExit::Stopped);
    assert!(
        took < Duration::from_secs(3),
        "final POST took {took:?} against a stalled server"
    );
    drop(held);
}
