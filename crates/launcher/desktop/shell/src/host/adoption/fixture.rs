//! An imported legacy installation in an isolated root, and a signed loopback
//! artifact origin, shared by the host tests and the UAT bridge.
use super::*;
use adoption::test_support::CopyFault;
use cimmeria_launcher_engine::{install_worker::fixtures, migration::LegacySource};
use std::{collections::BTreeMap, fs, path::Path, time::Duration};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

pub(super) const CATALOG: &str =
    "https://github.com/SandboxServers/Cimmeria/releases/download/content-current/manifest.json";

pub(super) struct Fixture {
    pub root: tempfile::TempDir,
    pub host: NativeHost,
    pub source: LegacySource,
    pub seed: Vec<u8>,
}
impl Fixture {
    /// An old launcher folder and game tree, imported through the production
    /// migration path. `config` overrides fields of launcher-config.json.
    pub fn new(config: serde_json::Value) -> Self {
        let root = tempfile::tempdir().unwrap();
        let seed = fixtures::archive(true);
        let source = legacy_source(root.path(), &seed, config);
        fs::create_dir(root.path().join("library")).unwrap();
        let host = imported_host(root.path(), &source);
        Self {
            root,
            host,
            source,
            seed,
        }
    }
    /// The folder a user would pick; the copy is created inside it.
    pub fn location(&self) -> PathBuf {
        self.root.path().join("library")
    }
    pub fn destination(&self) -> PathBuf {
        self.location().canonicalize().unwrap().join(COPY_FOLDER)
    }
    pub fn release(&self) -> VerifiedRelease {
        fixtures::verified(&self.seed)
    }
    pub fn serve_from(&mut self, manifest_url: String) {
        self.host.adoption_fixture = Some(TestDispatch {
            manifest_url,
            helper: None,
            copy_fault: Mutex::new(None),
        });
    }
    pub async fn serve(&mut self) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(path("/seed.zip"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(self.seed.clone()))
            .mount(&server)
            .await;
        self.serve_from(format!("{}/manifest.json", server.uri()));
        server
    }
    pub fn fault(&self, fault: CopyFault) {
        *self
            .host
            .adoption_fixture
            .as_ref()
            .unwrap()
            .copy_fault
            .lock()
            .unwrap() = Some(fault);
    }
    pub fn begin(&self) -> Result<AdoptionStatus, AdoptionError> {
        self.host
            .begin_adoption_preview(self.location(), self.release())
    }
    pub fn revision(&self) -> u64 {
        self.host
            .adoption_status()
            .unwrap()
            .native
            .operation
            .revision
    }
}
pub(super) fn legacy_source(root: &Path, seed: &[u8], config: serde_json::Value) -> LegacySource {
    let launcher = root.join("legacy");
    let game = root.join("old-game");
    fs::create_dir(&launcher).unwrap();
    fs::create_dir(&game).unwrap();
    let archive = root.join("fixture-seed.zip");
    fs::write(&archive, seed).unwrap();
    let (progress, _) = cimmeria_launcher_engine::install_progress::ProgressSink::latest();
    cimmeria_launcher_engine::unpack::unpack(
        &archive,
        &game,
        &cimmeria_launcher_engine::unpack::UnpackSink {
            progress,
            label: "fixture".into(),
            cancel: tokio_util::sync::CancellationToken::new(),
        },
    )
    .unwrap();
    fs::remove_file(archive).unwrap();
    let servers = [("Second", "second"), ("First", "first")].map(|(name, host)| {
        cimmeria_launcher_engine::client_setup::LoginServer {
            name: name.into(),
            url: format!("https://example.invalid/{host}"),
        }
    });
    cimmeria_launcher_engine::client_setup::prepare(&game, &servers).unwrap();
    // One managed file the user changed, and one file the release never shipped.
    fs::write(game.join("later.txt"), b"edited by the user").unwrap();
    fs::write(game.join("unknown.dll"), b"untrusted plugin").unwrap();
    let mut value = serde_json::json!({
        "install_path": "C:\\Old Game",
        "manifest_url": CATALOG,
        "login_servers": [
            {"name": "Second", "url": "https://example.invalid/second"},
            {"name": "First", "url": "https://example.invalid/first"}
        ],
        "client_patches": {"enabled": false},
        "telemetry": {"enabled": true, "opted_in": true, "prompt_answered": true,
            "auth_url": "https://example.invalid/auth"}
    });
    for (key, replacement) in config.as_object().into_iter().flatten() {
        value[key] = replacement.clone();
    }
    fs::write(
        launcher.join("launcher-config.json"),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
    fs::write(
        launcher.join("install.json"),
        br#"{"schema_version":1,"install_id":"72a8a13b-2a4e-4ea0-b5ba-5ba3cf0a619d","machine_id":"fixture","first_seen_ms":1,"created_by_launcher_version":"0.8"}"#,
    )
    .unwrap();
    fs::write(
        game.join("launcher-installed.json"),
        br#"{"seed_adopted":false,"applied_patches":[]}"#,
    )
    .unwrap();
    LegacySource {
        launcher_directory: launcher.canonicalize().unwrap(),
        game_directory: game.canonicalize().unwrap(),
    }
}
pub(super) fn imported_host(root: &Path, source: &LegacySource) -> NativeHost {
    let state_root = root.join("state");
    let mut state = DesktopState::open(&state_root).unwrap();
    let import = state.preview_legacy_import(source).unwrap();
    state
        .import_legacy(source, &import.confirmation, 0)
        .unwrap();
    drop(state);
    NativeHost::new(state_root)
}
/// The bundled prerequisite helper a packaged build carries. Without it the host
/// offers no prerequisite setup for any installation. Inert: it is never run.
pub(super) fn bundle_prerequisite_helper(host: &mut NativeHost, directory: &Path) {
    const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let helper = directory
        .canonicalize()
        .unwrap()
        .join("prerequisite-worker.exe");
    fs::write(&helper, b"").unwrap();
    host.prerequisite_helper = Some(
        cimmeria_launcher_engine::mac_wine::PrerequisiteResource::open(helper, EMPTY).unwrap(),
    );
}
/// Every file below `root` with its exact bytes; the old launcher's lock file is
/// content-free and excluded by name only.
pub(super) fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(at).unwrap() {
            let entry = entry.unwrap();
            let kind = entry.file_type().unwrap();
            assert!(!kind.is_symlink(), "fixtures contain no links");
            if kind.is_dir() {
                walk(root, &entry.path(), out);
            } else {
                out.insert(
                    entry.path().strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
pub(super) fn source_bytes(source: &LegacySource) -> [BTreeMap<PathBuf, Vec<u8>>; 2] {
    [
        tree(&source.game_directory),
        tree(&source.launcher_directory),
    ]
}
pub(super) async fn until(
    host: &NativeHost,
    what: &str,
    limit: Duration,
    ready: impl Fn(&AdoptionStatus) -> bool,
) -> AdoptionStatus {
    tokio::time::timeout(limit, async {
        loop {
            let status = host.adoption_status().unwrap();
            if ready(&status) {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {what}: {:?}", host.adoption_status()))
}
pub(super) async fn reviewed(host: &NativeHost) -> Review {
    let status = until(host, "a review", Duration::from_secs(30), |status| {
        assert_eq!(status.last_error, None, "preparation failed");
        status.review.is_some()
    })
    .await;
    assert_eq!(status.activity, Activity::Review);
    status.review.unwrap()
}
pub(super) fn confirmation(review: &Review) -> AdoptionCommand {
    AdoptionCommand::Confirm {
        schema_version: 1,
        work_id: Uuid::new_v4(),
        preview_handle: review.preview_handle,
        operation_revision: review.operation_revision,
        preferences_revision: review.preferences_revision,
        normalize_managed_files: true,
        accept_unavailable_game_telemetry: true,
        old_game_closed: true,
        confirmed: true,
    }
}
pub(super) fn operation(status: &AdoptionStatus) -> Option<(OperationKind, OperationState)> {
    status
        .native
        .operation
        .operation
        .as_ref()
        .map(|op| (op.kind, op.state))
}
pub(super) fn plans(host: &NativeHost) -> usize {
    fs::read_dir(&host.root)
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("adoption-plan-")
        })
        .count()
}
pub(super) const INSPECT: AdoptionCommand = AdoptionCommand::Inspect { schema_version: 1 };
pub(super) const DISMISS: AdoptionCommand = AdoptionCommand::Dismiss { schema_version: 1 };
pub(super) const CANCEL: AdoptionCommand = AdoptionCommand::Cancel { schema_version: 1 };

/// A loopback origin that declares the signed length, trickles the body and
/// never finishes it, so a preparation is reliably caught mid-download. The
/// accepted connection is made blocking: it inherits the listener's
/// non-blocking mode, and a request that has not arrived yet must be waited for.
pub(super) struct StalledOrigin {
    pub url: String,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl StalledOrigin {
    pub fn new(body: Vec<u8>) -> Self {
        use std::{
            io::{Read, Write},
            sync::atomic::Ordering,
        };
        assert!(body.len() > 1);
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/manifest.json", listener.local_addr().unwrap());
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stopped = stop.clone();
        let thread = std::thread::spawn(move || {
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if stopped.load(Ordering::SeqCst) {
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_millis(100)))
                .unwrap();
            let mut request = [0; 4096];
            let count = stream.read(&mut request).unwrap();
            assert!(request[..count].starts_with(b"GET /seed.zip "));
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            // Cross the production progress throttle without ever sending the
            // last byte or an EOF.
            let mut sent = 0;
            while !stopped.load(Ordering::SeqCst) {
                if sent < body.len() - 1 {
                    if stream.write_all(&body[sent..sent + 1]).is_err() {
                        return;
                    }
                    sent += 1;
                }
                std::thread::sleep(Duration::from_millis(40));
            }
        });
        Self {
            url,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for StalledOrigin {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            if !std::thread::panicking() {
                thread.join().unwrap();
            }
        }
    }
}
