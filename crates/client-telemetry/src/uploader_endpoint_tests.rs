//! The upload URL the DLL derives from `current-session.json`.
//!
//! The launcher writes the dev-session response's `upload_endpoint`, which
//! is the upload *base* (`…/api/telemetry`), and appends the route itself.
//! The DLL posted to the value verbatim, so none of its batches reached
//! `/api/telemetry/upload-chunk`. These pin the normalisation and the path
//! a real POST takes.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::{chunk_url, run_uploader, UploaderConfig, UploaderExit};
use crate::events::ClientNativeEvent;
use crate::queue::channel;
use crate::session::TelemetryBlock;

fn block(endpoint: &str) -> TelemetryBlock {
    TelemetryBlock {
        enabled: true,
        token: "t".into(),
        upload_endpoint: endpoint.into(),
        expires_at_ms: 0,
        chunk_max_bytes: 0,
        flush_interval_ms: 0,
    }
}

#[test]
fn a_base_endpoint_gets_the_chunk_route_appended() {
    assert_eq!(
        chunk_url("https://x.example/api/telemetry"),
        "https://x.example/api/telemetry/upload-chunk"
    );
    assert_eq!(
        chunk_url("https://x.example/api/telemetry/"),
        "https://x.example/api/telemetry/upload-chunk"
    );
}

#[test]
fn a_full_chunk_url_is_kept_as_is() {
    assert_eq!(
        chunk_url("http://127.0.0.1:8443/api/telemetry/upload-chunk"),
        "http://127.0.0.1:8443/api/telemetry/upload-chunk"
    );
}

/// The shape the launcher and the lab supervisor actually write, and the
/// server's default: the config the uploader runs with names the chunk
/// route. Reverting `from_session` to copy the value verbatim fails this.
#[test]
fn from_session_normalises_the_launcher_written_base() {
    let cfg = UploaderConfig::from_session(&block("http://localhost:8443/api/telemetry"));
    assert_eq!(
        cfg.upload_endpoint,
        "http://localhost:8443/api/telemetry/upload-chunk"
    );
}

/// End to end: a session file carrying the base URL makes the uploader
/// POST to `/api/telemetry/upload-chunk` on a real local server.
#[test]
fn a_base_endpoint_session_posts_to_the_chunk_route() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let addr = server.server_addr().to_ip().unwrap();
    let base = format!("http://{}:{}/api/telemetry", addr.ip(), addr.port());

    let paths: Arc<Mutex<Vec<String>>> = Arc::default();
    let paths_clone = paths.clone();
    let server_handle = thread::spawn(move || {
        if let Ok(mut req) = server.recv() {
            paths_clone.lock().unwrap().push(req.url().to_string());
            let mut body = Vec::new();
            req.as_reader().read_to_end(&mut body).unwrap();
            req.respond(tiny_http::Response::from_string("ok")).ok();
        }
    });

    let (p, c) = channel();
    p.try_emit(ClientNativeEvent::builder("client.test", "info"));
    drop(p);
    let mut cfg = UploaderConfig::from_session(&block(&base));
    cfg.flush_interval = Duration::from_millis(50);
    let exit = run_uploader(c, cfg, || false);
    server_handle.join().unwrap();

    assert_eq!(exit, UploaderExit::Disconnected);
    assert_eq!(
        paths.lock().unwrap().as_slice(),
        ["/api/telemetry/upload-chunk"]
    );
}
