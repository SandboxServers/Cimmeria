//! The exporter against a loopback mock of the server's one route. Every test
//! awaits `run_cycle` itself, asserts its typed outcome and the exact requests
//! the mock received. No backoff sleeps for real: the injected sleeper records.
use super::*;
use export::{run_cycle, ExportEnv, ExportTuning};
use std::sync::{atomic::AtomicBool, Mutex, MutexGuard, Weak};
use tokio::task::JoinHandle;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, Request, Respond, ResponseTemplate,
};

pub(in crate::storage) use export::CycleOutcome;

mod delivery;
mod held;
mod hostile;
mod outage;
mod races;
mod task;

/// The mock is configured under a base path, as production's server is, so
/// every asserted path pins that the route is joined to the configured base.
const BASE_PATH: &str = "/api";
/// The only path a summary request may have.
pub(in crate::storage) const INGEST: &str = "/api/telemetry/launcher-summary";
/// Headers that would identify or authenticate the sender. No request has one.
const CREDENTIALS: [&str; 3] = ["authorization", "proxy-authorization", "cookie"];
/// Every header a summary request carries, sorted. It is an allowlist: a
/// request with any other header fails, so there is no `user-agent` and no
/// header that could name the sender or the installation.
const HEADERS: [&str; 4] = ["accept", "content-length", "content-type", "host"];
/// Loopback with nothing listening. Not `127.0.0.1`: under WSL2 mirrored
/// networking a connection to an unbound port there hangs instead of failing.
const DEAD: &str = "http://[::1]:9";
/// The test tuning's ceiling for a server-requested wait.
const CAP: Duration = Duration::from_secs(7);
/// The waits before the two retries of the test tuning.
pub(in crate::storage) const BACKOFFS: [Duration; 2] =
    [Duration::from_millis(100), Duration::from_millis(200)];
const DELIVERED_ONE: CycleOutcome = CycleOutcome::Delivered {
    accepted: 1,
    duplicate: 0,
    rejected: 0,
};

fn backoff(retry: u32) -> Duration {
    Duration::from_millis(100 * u64::from(retry))
}

/// What the injected sleeper saw, and what it does while "asleep".
#[derive(Clone, Default)]
pub(in crate::storage) struct Probe {
    sleeps: Arc<Mutex<Vec<Duration>>>,
    during_sleep: Arc<Mutex<Option<Box<dyn FnMut() + Send>>>>,
}
impl Probe {
    pub(in crate::storage) fn sleeps(&self) -> Vec<Duration> {
        self.sleeps.lock().unwrap().clone()
    }
    fn during_sleep(&self, action: impl FnMut() + Send + 'static) {
        *self.during_sleep.lock().unwrap() = Some(Box::new(action));
    }
}

/// Production's environment for the configured endpoint, with deadlines no
/// loaded machine reaches, small recorded waits and no real sleep.
fn test_env(state: &DesktopState) -> (ExportEnv, Probe) {
    let target = state.summary_export_target().expect("an endpoint");
    let mut env = ExportEnv::production(&target).expect("http client");
    env.tuning = ExportTuning {
        post_timeout: Duration::from_secs(30),
        max_retries: 2,
        retry_after_cap: CAP,
        backoff,
    };
    let probe = Probe::default();
    let seen = probe.clone();
    env.sleep = Box::new(move |wait| {
        seen.sleeps.lock().unwrap().push(wait);
        if let Some(action) = seen.during_sleep.lock().unwrap().as_mut() {
            action();
        }
        Box::pin(std::future::ready(()))
    });
    (env, probe)
}

/// Acts on the state from a mock responder, on the mock server's own thread,
/// while the exporter waits for that answer; or from the injected sleeper.
#[derive(Clone)]
struct Meddler {
    state: Weak<Mutex<DesktopState>>,
    held: Arc<AtomicBool>,
}
impl Meddler {
    fn run(&self, action: impl FnOnce(&mut DesktopState)) {
        let Some(state) = self.state.upgrade() else {
            return;
        };
        // The exporter must never hold the state across a request or a sleep.
        match state.try_lock() {
            Ok(mut owner) => action(&mut owner),
            Err(_) => self.held.store(true, Ordering::SeqCst),
        };
    }
}

/// A state root, the mock server it is configured for, and one exporter
/// environment. Layout: `<temp>/state`.
pub(in crate::storage) struct Rig {
    pub(in crate::storage) server: MockServer,
    root: tempfile::TempDir,
    base: String,
    pub(in crate::storage) state: Arc<Mutex<DesktopState>>,
    pub(in crate::storage) clock: Clock,
    env: ExportEnv,
    pub(in crate::storage) probe: Probe,
    held: Arc<AtomicBool>,
}
impl Rig {
    /// Configured for its own mock server. The user has not opted in.
    pub(in crate::storage) async fn start() -> Self {
        let server = MockServer::start().await;
        let base = format!("{}{BASE_PATH}", server.uri());
        Self::open(server, tempfile::tempdir().unwrap(), base, 0)
    }

    pub(in crate::storage) async fn opted_in() -> Self {
        let rig = Self::start().await;
        set_consent(&mut rig.owner(), true);
        rig
    }

    /// Opted in, with an endpoint on which nothing listens.
    pub(in crate::storage) async fn dead() -> Self {
        let server = MockServer::start().await;
        let mut rig = Self::open(server, tempfile::tempdir().unwrap(), DEAD.into(), 0);
        // Should a host leave the connection hanging instead, the wait is short.
        rig.env.tuning.post_timeout = Duration::from_secs(3);
        set_consent(&mut rig.owner(), true);
        rig
    }

    fn open(server: MockServer, root: tempfile::TempDir, base: String, issued: u64) -> Self {
        let mut state = DesktopState::open(&root.path().join("state")).unwrap();
        let clock = Clock::install_after(&mut state, issued);
        state.configure_summaries(config(Some(&base)));
        let (env, probe) = test_env(&state);
        Self {
            server,
            root,
            base,
            state: Arc::new(Mutex::new(state)),
            clock,
            env,
            probe,
            held: Arc::default(),
        }
    }

    /// A later process on the same state root and server. Ids start at `minted(101)`.
    fn restart(self) -> Self {
        let Self {
            server,
            root,
            base,
            state,
            ..
        } = self;
        assert_eq!(Arc::strong_count(&state), 1, "the old process is gone");
        drop(state);
        Self::open(server, root, base, 100)
    }

    pub(in crate::storage) fn root(&self) -> &Path {
        self.root.path()
    }

    pub(in crate::storage) fn owner(&self) -> MutexGuard<'_, DesktopState> {
        self.state.lock().unwrap()
    }

    pub(in crate::storage) async fn cycle(&self) -> CycleOutcome {
        run_cycle(&Arc::downgrade(&self.state), &self.env).await
    }

    /// Starts the exporter task as `start` does, with a test environment of its
    /// own and this rig's deadline; `probe` then reports that task's waits.
    pub(in crate::storage) fn start_exporter(&mut self) -> Option<JoinHandle<()>> {
        let (mut env, probe) = test_env(&self.owner());
        env.tuning.post_timeout = self.env.tuning.post_timeout;
        self.probe = probe;
        export::spawn(&self.state, config(Some(&self.base)), move |_| Some(env))
    }

    fn meddler(&self) -> Meddler {
        Meddler {
            state: Arc::downgrade(&self.state),
            held: self.held.clone(),
        }
    }

    /// False once a responder or the sleeper found the state mutex taken.
    fn lock_was_free(&self) -> bool {
        !self.held.load(Ordering::SeqCst)
    }

    /// Queues `count` failed repair attempts; returns everything queued.
    fn fail(&self, count: usize) -> Vec<Summary> {
        let mut owner = self.owner();
        fail(&mut owner, count);
        queued(&owner)
    }

    /// The path of every request the mock received, in order.
    pub(in crate::storage) async fn paths(&self) -> Vec<String> {
        let requests = self.server.received_requests().await.unwrap();
        requests
            .iter()
            .map(|request| request.url.path().to_owned())
            .collect()
    }

    /// As `paths`, once at least `count` requests arrived. A request the client
    /// gave up on is recorded when the mock reads it, which may be later.
    async fn paths_after(&self, count: usize) -> Vec<String> {
        for _ in 0..500 {
            if self.paths().await.len() >= count {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        self.paths().await
    }

    /// The JSON bodies received on the ingest route, in order.
    pub(in crate::storage) async fn posts(&self) -> Vec<serde_json::Value> {
        let requests = self.server.received_requests().await.unwrap();
        requests
            .iter()
            .filter(|request| request.url.path() == INGEST)
            .map(|request| serde_json::from_slice(&request.body).unwrap())
            .collect()
    }

    /// A cycle of an exporter started for the endpoint configured now.
    async fn cycle_where_configured(&self) -> CycleOutcome {
        let (env, _) = test_env(&self.owner());
        run_cycle(&Arc::downgrade(&self.state), &env).await
    }

    /// Every request the mock received is a bare `POST` to the ingest route:
    /// no query, no header that identifies or authenticates the sender, and no
    /// header at all besides `HEADERS`, each with a value that says nothing.
    /// `delivery.rs` has the control: the mock does record any other header.
    async fn assert_anonymous(&self, context: &str) {
        let host = self.server.address().to_string();
        for request in self.server.received_requests().await.unwrap() {
            assert_eq!(request.method.as_str(), "POST", "{context}");
            assert_eq!(request.url.path(), INGEST, "{context}");
            assert_eq!(request.url.query(), None, "{context}");
            for name in CREDENTIALS {
                assert!(!request.headers.contains_key(name), "{context}: {name}");
            }
            assert_eq!(header_names(&request), HEADERS, "{context}");
            let length = request.body.len().to_string();
            for (name, value) in [
                ("accept", "*/*"),
                ("content-length", length.as_str()),
                ("content-type", "application/json"),
                ("host", host.as_str()),
            ] {
                assert_eq!(request.headers[name], value, "{context}: {name}");
            }
        }
    }
}

/// The name of every header of a recorded request, sorted. A header sent more
/// than once is listed once for each value.
fn header_names(request: &Request) -> Vec<&str> {
    let mut names: Vec<&str> = request
        .headers
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    names.sort_unstable();
    names
}

fn fail(state: &mut DesktopState, count: usize) {
    for _ in 0..count {
        let id = begin(state, OperationKind::Repair);
        observe(state, id, OperationState::Failed);
    }
    state.finalize_summaries();
}

/// The ingest route answers every `POST` with `responder`.
async fn mount(server: &MockServer, responder: impl Respond + 'static) {
    Mock::given(method("POST"))
        .and(path(INGEST))
        .respond_with(responder)
        .mount(server)
        .await;
}

fn results(verdicts: &[&str]) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({ "results": verdicts }))
}

/// `accepted` for every summary in the request.
fn accept_all(request: &Request) -> ResponseTemplate {
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap_or_default();
    let count = body["summaries"].as_array().map_or(0, Vec::len);
    results(&vec!["accepted"; count])
}

/// The route answers as the server does for a batch of valid rows.
pub(in crate::storage) async fn mount_ok(server: &MockServer) {
    mount(server, accept_all).await;
}

/// A second server with the summary route, and its base: a different valid
/// endpoint.
async fn elsewhere() -> (MockServer, String) {
    let server = MockServer::start().await;
    mount_ok(&server).await;
    let base = format!("{}{BASE_PATH}", server.uri());
    (server, base)
}
