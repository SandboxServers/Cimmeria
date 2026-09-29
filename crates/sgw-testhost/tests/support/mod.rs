//! Shared plumbing for the boot tests: finding the staged i686 build,
//! launching the test host through `sgw-start32` in a scratch directory,
//! a mock telemetry endpoint, and a lab-bridge client.

#![allow(dead_code)] // each test file uses a different part of it

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_client_launch::process::RunningProcess;
use cimmeria_client_launch::start32::{self, Request, Target};

/// Set in CI: a missing staged build fails the test instead of skipping it.
pub const REQUIRE_ENV: &str = "CIMMERIA_TESTHOST_REQUIRE";
/// Overrides where the staged build is looked for.
pub const DIR_ENV: &str = "CIMMERIA_TESTHOST_DIR";

pub const PATCHES_DLL: &str = "cimmeria_client_patches.dll";
pub const TELEMETRY_DLL: &str = "cimmeria_client_telemetry.dll";

/// The directory `tools/testhost/stage.sh` filled.
#[derive(Debug, Clone)]
pub struct Stage {
    pub dir: PathBuf,
}

impl Stage {
    /// The staged build, or `None` (with a note on stderr) when it has not
    /// been staged and [`REQUIRE_ENV`] is unset. Looks in [`DIR_ENV`], then
    /// in `testhost/` under each ancestor of this test executable (the
    /// target directory, whatever the lane or CI set it to).
    pub fn find() -> Option<Self> {
        let candidates: Vec<PathBuf> = match std::env::var_os(DIR_ENV) {
            Some(dir) => vec![PathBuf::from(dir)],
            None => std::env::current_exe()
                .ok()
                .map(|exe| exe.ancestors().map(|a| a.join("testhost")).collect())
                .unwrap_or_default(),
        };
        let found = candidates
            .into_iter()
            .find(|d| d.join("sgw-testhost.exe").is_file());
        match found {
            Some(dir) => Some(Self { dir }),
            None if std::env::var_os(REQUIRE_ENV).is_some() => {
                panic!("{REQUIRE_ENV} is set but no staged build was found; run tools/testhost/stage.sh")
            }
            None => {
                eprintln!("skipped: no staged i686 build (run tools/testhost/stage.sh)");
                None
            }
        }
    }

    pub fn patches(&self) -> PathBuf {
        self.dir.join(PATCHES_DLL)
    }

    pub fn telemetry(&self) -> PathBuf {
        self.dir.join(TELEMETRY_DLL)
    }

    pub fn telemetry_lab(&self) -> PathBuf {
        self.dir.join("lab").join(TELEMETRY_DLL)
    }
}

/// One scratch "install": the test host copied into a fresh directory with
/// the DLLs beside it, the way the launcher lays out `Binaries/`. Each DLL
/// writes its log next to the host executable, so the directory is the
/// test's evidence.
pub struct Install {
    pub dir: tempfile::TempDir,
    stage: Stage,
}

impl Install {
    pub fn new(stage: &Stage) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::copy(
            stage.dir.join("sgw-testhost.exe"),
            dir.path().join("sgw-testhost.exe"),
        )
        .expect("copy the test host");
        Self {
            dir,
            stage: stage.clone(),
        }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Copy a staged DLL next to the host; returns its new path.
    pub fn add_dll(&self, staged: &Path) -> PathBuf {
        let to = self.path().join(staged.file_name().expect("dll name"));
        std::fs::copy(staged, &to).expect("copy a DLL");
        to
    }

    /// Write `sessions/current-session.json` for the telemetry DLL.
    pub fn write_session(&self, session: &serde_json::Value) {
        let dir = self.path().join("sessions");
        std::fs::create_dir_all(&dir).expect("sessions dir");
        std::fs::write(dir.join("current-session.json"), session.to_string()).expect("session");
    }

    /// Start the host through `sgw-start32` with `dlls` injected in order,
    /// exactly as the launcher does, and return the running process.
    pub fn launch(&self, dlls: &[PathBuf], run_ms: u64) -> Launched {
        let request = Request {
            target: Target::Spawn {
                exe: self.path().join("sgw-testhost.exe"),
                cwd: Some(self.path().to_path_buf()),
                args: vec!["--run-ms".into(), run_ms.to_string().into()],
            },
            dlls: dlls.to_vec(),
        };
        let helper = self.stage.dir.join(start32::HELPER_EXE_NAME);
        let pid = start32::run(&helper, &request).expect("sgw-start32 started the host");
        let process = RunningProcess::open(pid).expect("open the host by pid");
        Launched { pid, process }
    }

    /// A DLL's log, or `""` if it wrote none.
    pub fn log(&self, name: &str) -> String {
        std::fs::read_to_string(self.path().join(name)).unwrap_or_default()
    }

    /// The host's own report: `(frames, exit_code)`.
    pub fn host_report(&self) -> Option<(u64, i32)> {
        let text = std::fs::read_to_string(self.path().join("sgw-testhost.out")).ok()?;
        let mut frames = None;
        let mut code = None;
        for field in text.split_whitespace() {
            if let Some(v) = field.strip_prefix("frames=") {
                frames = v.parse().ok();
            } else if let Some(v) = field.strip_prefix("exit_code=") {
                code = v.parse().ok();
            }
        }
        Some((frames?, code?))
    }
}

pub struct Launched {
    pub pid: u32,
    process: RunningProcess,
}

impl Launched {
    /// Wait for the host to exit; returns its exit code. A host that is
    /// still running after a minute (a DLL that hung it) fails the test.
    pub fn wait(self) -> i32 {
        let (tx, rx) = std::sync::mpsc::channel();
        let process = self.process;
        std::thread::spawn(move || {
            let _ = tx.send(process.wait());
        });
        rx.recv_timeout(Duration::from_secs(60))
            .expect("the host exited within 60 s")
            .expect("wait for the host")
    }
}

/// The messages of a DLL log (`[<prefix> +<ms>ms] <message>`), prefix
/// stripped.
pub fn messages(log: &str) -> Vec<String> {
    log.lines()
        .filter_map(|l| l.split_once("ms] ").map(|(_, m)| m.to_string()))
        .collect()
}

/// A mock `/api/telemetry/upload-chunk`: records every request's bearer
/// token and gunzipped NDJSON body.
pub struct MockUpload {
    pub url: String,
    received: Arc<Mutex<Vec<Upload>>>,
}

#[derive(Debug, Clone)]
pub struct Upload {
    pub path: String,
    pub authorization: Option<String>,
    pub events: Vec<serde_json::Value>,
}

impl MockUpload {
    pub fn start() -> Self {
        let server = tiny_http::Server::http("127.0.0.1:0").expect("bind the mock endpoint");
        let port = server.server_addr().to_ip().expect("ip listener").port();
        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = received.clone();
        std::thread::spawn(move || {
            for mut request in server.incoming_requests() {
                let authorization = request
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("Authorization"))
                    .map(|h| h.value.to_string());
                let mut raw = Vec::new();
                let _ = request.as_reader().read_to_end(&mut raw);
                let mut text = String::new();
                let _ = flate2::read::GzDecoder::new(raw.as_slice()).read_to_string(&mut text);
                let events = text
                    .lines()
                    .filter_map(|l| serde_json::from_str(l).ok())
                    .collect();
                sink.lock().unwrap().push(Upload {
                    path: request.url().to_string(),
                    authorization,
                    events,
                });
                let _ = request.respond(tiny_http::Response::empty(200));
            }
        });
        Self {
            url: format!("http://127.0.0.1:{port}/api/telemetry/upload-chunk"),
            received,
        }
    }

    pub fn uploads(&self) -> Vec<Upload> {
        self.received.lock().unwrap().clone()
    }

    /// Every event received so far, from every upload.
    pub fn events(&self) -> Vec<serde_json::Value> {
        self.uploads().into_iter().flat_map(|u| u.events).collect()
    }
}

/// A telemetry session that uploads to `endpoint` every 200 ms.
pub fn session(endpoint: &str, token: &str, lab: Option<serde_json::Value>) -> serde_json::Value {
    let mut s = serde_json::json!({
        "install_id": "i-testhost",
        "machine_id": "m-testhost",
        "session_id": "s-testhost",
        "telemetry": {
            "enabled": true,
            "token": token,
            "upload_endpoint": endpoint,
            "flush_interval_ms": 200
        }
    });
    if let Some(lab) = lab {
        s["lab"] = lab;
    }
    s
}

/// A loopback port nobody is listening on right now.
pub fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .expect("free port")
}

/// The lab bridge's framing: a 4-byte little-endian length, then JSON.
/// Written from the ADR rather than borrowed from the DLL crate, so the
/// test checks the DLL against the documented wire shape.
pub fn write_frame(stream: &mut TcpStream, body: &[u8]) {
    stream
        .write_all(&(body.len() as u32).to_le_bytes())
        .and_then(|_| stream.write_all(body))
        .unwrap_or_else(|e| panic!("write a frame: {e}"));
}

pub fn read_frame(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len)?;
    let mut body = vec![0u8; u32::from_le_bytes(len) as usize];
    stream.read_exact(&mut body)?;
    Ok(body)
}

/// Connect to `addr`, retrying while the DLL boots.
pub fn connect(addr: SocketAddr, within: Duration) -> TcpStream {
    let deadline = Instant::now() + within;
    loop {
        match TcpStream::connect_timeout(&addr, Duration::from_millis(250)) {
            Ok(s) => {
                s.set_read_timeout(Some(Duration::from_secs(15))).unwrap();
                return s;
            }
            Err(e) if Instant::now() >= deadline => {
                panic!("nothing listening on {addr} after {within:?}: {e}")
            }
            Err(_) => std::thread::sleep(Duration::from_millis(100)),
        }
    }
}
