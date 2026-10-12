//! The stub of the daemon's HTTP endpoint for `tests/mcp.rs`: one request per
//! connection, every request recorded, routes matched in order.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// One canned reply: matched on method, path and (optionally) a request body fragment.
#[derive(Clone)]
pub struct Route {
    pub method: &'static str,
    pub path: &'static str,
    /// The request body must contain this text.
    pub body_has: Option<&'static str>,
    pub status: u16,
    pub headers: Vec<(&'static str, &'static str)>,
    pub body: String,
    /// Pause before the status line is sent.
    pub delay_ms: u64,
    /// Pause between the headers and the body.
    pub stall_ms: u64,
    /// Content-Length overstates the body by this much (the connection then
    /// closes early, so reading the body fails).
    pub short_by: usize,
}

/// A route with no delay, no extra header and an honest Content-Length.
pub fn route(
    method: &'static str,
    path: &'static str,
    body_has: Option<&'static str>,
    status: u16,
    body: &str,
) -> Route {
    Route {
        method,
        path,
        body_has,
        status,
        headers: vec![],
        body: body.to_owned(),
        delay_ms: 0,
        stall_ms: 0,
        short_by: 0,
    }
}

/// One request the stub saw.
#[derive(Clone, Debug)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>, // names lower-cased
    pub body: String,
}

impl Recorded {
    /// The value of a header; `name` is lower-case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// The running stub; dropping it stops the accept loop and closes the port.
pub struct Stub {
    addr: SocketAddr,
    seen: Arc<Mutex<Vec<Recorded>>>,
    stop: Arc<AtomicBool>,
}

impl Stub {
    /// Binds a loopback port (port 0) and serves `routes`.
    pub fn start(routes: Vec<Route>) -> Stub {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the stub");
        let addr = listener.local_addr().expect("stub address");
        let routes = Arc::new(routes);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (r, s, st) = (routes.clone(), seen.clone(), stop.clone());
        std::thread::spawn(move || {
            for conn in listener.incoming() {
                if st.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = conn else { continue };
                let (r, s) = (r.clone(), s.clone());
                std::thread::spawn(move || serve(stream, &r, &s));
            }
        });
        Stub { addr, seen, stop }
    }
    /// `http://127.0.0.1:<port>`.
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }
    /// Every request seen so far, in arrival order.
    pub fn requests(&self) -> Vec<Recorded> {
        self.seen.lock().unwrap().clone()
    }
    /// The recorded POST whose body contains `needle`.
    pub fn post_with(&self, needle: &str) -> Recorded {
        self.requests()
            .into_iter()
            .find(|r| r.method == "POST" && r.body.contains(needle))
            .unwrap_or_else(|| panic!("no POST containing {needle}"))
    }
    /// Stops the stub and returns once nothing listens on its port any more.
    pub fn shut_down(self) {
        let addr = self.addr;
        drop(self);
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while TcpStream::connect(addr).is_ok() {
            assert!(
                std::time::Instant::now() < deadline,
                "the stub's port never closed"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Stub {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr); // wakes the accept loop
    }
}

fn serve(stream: TcpStream, routes: &[Route], seen: &Mutex<Vec<Recorded>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut first = String::new();
    if reader.read_line(&mut first).unwrap_or(0) == 0 {
        return;
    }
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("").to_owned();
    let path = parts.next().unwrap_or("").to_owned();
    let mut headers = Vec::new();
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let line = line.trim_end().to_owned();
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim().to_owned());
            if k == "content-length" {
                length = v.parse().unwrap_or(0);
            }
            headers.push((k, v));
        }
    }
    let mut body = vec![0u8; length];
    let _ = reader.read_exact(&mut body);
    let body = String::from_utf8_lossy(&body).into_owned();
    seen.lock().unwrap().push(Recorded {
        method: method.clone(),
        path: path.clone(),
        headers,
        body: body.clone(),
    });
    let hit = routes.iter().find(|r| {
        r.method == method && r.path == path && r.body_has.is_none_or(|n| body.contains(n))
    });
    let reply = hit
        .cloned()
        .unwrap_or_else(|| route("", "", None, 500, "no route"));
    std::thread::sleep(Duration::from_millis(reply.delay_ms));
    let mut head = format!(
        "HTTP/1.1 {} X\r\nContent-Length: {}\r\nConnection: close\r\n",
        reply.status,
        reply.body.len() + reply.short_by
    );
    for (k, v) in &reply.headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    let mut stream = stream;
    let _ = stream.write_all(format!("{head}\r\n").as_bytes());
    let _ = stream.flush();
    std::thread::sleep(Duration::from_millis(reply.stall_ms));
    let _ = stream.write_all(reply.body.as_bytes());
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Both);
}
