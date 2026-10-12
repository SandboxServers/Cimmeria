//! Shared test support: the environment lock and guard, and a stub HTTP server.
#![allow(dead_code)]

use std::ffi::OsString;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::Duration;

use cimmeria_lab_cli::daemon::status::Endpoint;

/// Tests that change the process environment hold this lock (nextest gives
/// each test its own process, but `cargo test` runs them as threads of one).
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Takes [`ENV_LOCK`], ignoring poison from an earlier failed test.
pub fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Sets or removes environment variables and restores them when dropped.
pub struct EnvGuard {
    saved: Vec<(String, Option<OsString>)>,
}

impl EnvGuard {
    /// `Some(value)` sets the variable, `None` removes it.
    pub fn set(vars: &[(&str, Option<&str>)]) -> Self {
        let mut saved = Vec::new();
        for (key, value) in vars {
            saved.push((key.to_string(), std::env::var_os(key)));
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        EnvGuard { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, old) in self.saved.iter().rev() {
            match old {
                Some(old) => std::env::set_var(key, old),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// The token every stub [`Endpoint`] carries.
pub const STUB_TOKEN: &str = "test-token-not-real";

/// One request the stub received. Header names are lower-cased.
#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub method: String,
    /// The path without its query string.
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl RecordedRequest {
    /// A header value by name (any case).
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.as_str())
    }
}

struct Route {
    method: String,
    path: String,
    status: u16,
    body: String,
}

/// A stub HTTP server on `127.0.0.1:0`: one request per connection, answered
/// from `routes`, every request recorded. Stops when dropped.
pub struct StubServer {
    addr: SocketAddr,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl StubServer {
    /// `routes` are `(method, path, status, body)`; a request that matches
    /// none gets `404 not found`. The returned [`Endpoint`] points at the
    /// stub and carries [`STUB_TOKEN`].
    pub fn start(routes: Vec<(&str, &str, u16, &str)>) -> (StubServer, Endpoint) {
        let routes: Vec<Route> = routes
            .into_iter()
            .map(|(method, path, status, body)| Route {
                method: method.to_string(),
                path: path.to_string(),
                status,
                body: body.to_string(),
            })
            .collect();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the stub server");
        let addr = listener.local_addr().expect("stub address");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let handle = {
            let requests = Arc::clone(&requests);
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    handle_connection(stream, &routes, &requests);
                }
            })
        };
        let endpoint = Endpoint {
            base_url: format!("http://{addr}"),
            token: Some(STUB_TOKEN.to_string()),
        };
        (
            StubServer {
                addr,
                requests,
                stop,
                handle: Some(handle),
            },
            endpoint,
        )
    }

    /// Every request received so far, in order.
    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().expect("requests lock").clone()
    }
}

impl Drop for StubServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the accept loop so it sees the stop flag.
        let _ = TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        401 => "Unauthorized",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Status",
    }
}

fn handle_connection(mut stream: TcpStream, routes: &[Route], log: &Mutex<Vec<RecordedRequest>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 1024];
    let head_end = loop {
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos;
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut head_lines = head.split("\r\n");
    let request_line = head_lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or_default().to_string();
    let headers: Vec<(String, String)> = head_lines
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect();
    let length: usize = headers
        .iter()
        .find(|(key, _)| key == "content-length")
        .and_then(|(_, value)| value.parse().ok())
        .unwrap_or(0);
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < length {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => body.extend_from_slice(&chunk[..n]),
        }
    }
    let path = target.split('?').next().unwrap_or_default().to_string();
    log.lock().expect("requests lock").push(RecordedRequest {
        method: method.clone(),
        path: path.clone(),
        headers,
        body: String::from_utf8_lossy(&body).into_owned(),
    });
    let (status, text) = routes
        .iter()
        .find(|r| r.method.eq_ignore_ascii_case(&method) && r.path == path)
        .map(|r| (r.status, r.body.clone()))
        .unwrap_or((404, "not found".to_string()));
    let response = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
        reason(status),
        text.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Both);
}

/// The bind an isolated lab home names: port 9 (discard) on loopback, which
/// nothing listens on.
pub const CLOSED_PORT_BIND: &str = "127.0.0.1:9";

/// Writes `<home>\labd.pid` naming [`CLOSED_PORT_BIND`], so a `lab` child or a
/// `status::fetch` run with `CIMMERIA_LAB_HOME=<home>` finds no daemon. An
/// empty home falls back to `127.0.0.1:8779`, where the machine's real lab
/// daemon may be listening.
pub fn write_closed_port_pid(home: &std::path::Path) {
    std::fs::write(
        home.join("labd.pid"),
        format!(r#"{{"pid":1,"bind":"{CLOSED_PORT_BIND}"}}"#),
    )
    .expect("write labd.pid");
}

/// A fresh temp lab home whose `labd.pid` names a closed port. Every test that
/// runs the `lab` binary, or calls `Endpoint::discover`, uses this unless it
/// points at a [`StubServer`] on purpose.
pub fn isolated_lab_home() -> tempfile::TempDir {
    let home = tempfile::TempDir::new().expect("temp lab home");
    write_closed_port_pid(home.path());
    home
}

/// An [`Endpoint`] on a loopback port that nothing listens on: a listener is
/// bound to get a free port, then dropped.
pub fn closed_port_endpoint() -> Endpoint {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    Endpoint {
        base_url: format!("http://127.0.0.1:{port}"),
        token: Some(STUB_TOKEN.to_string()),
    }
}

/// A server that accepts a request and then stalls, to test timeouts: with
/// `partial_body` it first sends the headers of a 100 byte body and 7 bytes of
/// it, else nothing at all. Stops when dropped.
pub struct HangServer {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl HangServer {
    /// Binds `127.0.0.1:0` and starts stalling every connection.
    pub fn start(partial_body: bool) -> (HangServer, Endpoint) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the hang server");
        let addr = listener.local_addr().expect("hang address");
        let stop = Arc::new(AtomicBool::new(false));
        let handle = {
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(mut stream) = stream else { continue };
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                    let mut buf = [0u8; 1024];
                    let _ = stream.read(&mut buf);
                    if partial_body {
                        let head = "HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n{\"daemon\"";
                        let _ = stream.write_all(head.as_bytes());
                        let _ = stream.flush();
                    }
                    // Hold the connection open until the test is over.
                    while !stop.load(Ordering::SeqCst) {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                }
            })
        };
        let endpoint = Endpoint {
            base_url: format!("http://{addr}"),
            token: Some(STUB_TOKEN.to_string()),
        };
        (
            HangServer {
                addr,
                stop,
                handle: Some(handle),
            },
            endpoint,
        )
    }
}

impl Drop for HangServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
