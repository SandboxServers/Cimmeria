//! The exporter against a loopback mock of the server's two routes. Every test
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

pub(in crate::storage) const MINT: &str = "/auth/dev-session";
pub(in crate::storage) const INGEST: &str = "/telemetry/launcher-summary";
const TOKEN: &str = "mock-token";
/// The `install_id` of `fixtures/mint-request.json`.
const GOLDEN_INSTALL_ID: Uuid = Uuid::from_u128(0x0000_0000_0000_4000_8000_0000_0000_00aa);
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
    mints: Arc<AtomicU64>,
    sleeps: Arc<Mutex<Vec<Duration>>>,
    during_sleep: Arc<Mutex<Option<Box<dyn FnMut() + Send>>>>,
}
impl Probe {
    /// How many mint ids were drawn: one for every attempt, answered or not.
    fn mints(&self) -> u64 {
        self.mints.load(Ordering::SeqCst)
    }
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
        mint_timeout: Duration::from_secs(30),
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
    // Random for every mint, as in production; only counted here.
    let seen = probe.clone();
    env.mint_id = Box::new(move || {
        seen.mints.fetch_add(1, Ordering::SeqCst);
        Uuid::new_v4()
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
        let base = server.uri();
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
        rig.env.tuning.mint_timeout = Duration::from_secs(3);
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
    /// own and this rig's deadlines; `probe` then reports that task's waits.
    pub(in crate::storage) fn start_exporter(&mut self) -> Option<JoinHandle<()>> {
        let (mut env, probe) = test_env(&self.owner());
        env.tuning.mint_timeout = self.env.tuning.mint_timeout;
        env.tuning.post_timeout = self.env.tuning.post_timeout;
        self.probe = probe;
        export::spawn(&self.state, config(Some(&self.base)), move |_| Some(env))
    }

    /// Mints the golden fixture's `install_id` instead of a random one.
    pub(in crate::storage) fn fix_mint_id(&mut self) {
        self.env.mint_id = Box::new(|| GOLDEN_INSTALL_ID);
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

    /// The JSON bodies received on `route`, in order.
    pub(in crate::storage) async fn bodies(&self, route: &str) -> Vec<serde_json::Value> {
        let requests = self.server.received_requests().await.unwrap();
        requests
            .iter()
            .filter(|request| request.url.path() == route)
            .map(|request| serde_json::from_slice(&request.body).unwrap())
            .collect()
    }
}

fn fail(state: &mut DesktopState, count: usize) {
    for _ in 0..count {
        let id = begin(state, OperationKind::Repair);
        observe(state, id, OperationState::Failed);
    }
    state.finalize_summaries();
}

async fn mount(server: &MockServer, route: &str, responder: impl Respond + 'static) {
    Mock::given(method("POST"))
        .and(path(route))
        .respond_with(responder)
        .mount(server)
        .await;
}

/// The server's mint answer. Only `token` is meant to be used.
fn minted() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "session_id": "mock-session",
        "token": TOKEN,
        "expires_at_ms": 1_800_000_900_000_i64,
        "upload_endpoint": "/telemetry/client-chunk",
        "chunk_max_bytes": 65_536,
        "flush_interval_ms": 5_000,
    }))
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

/// Both routes answer as a server with the summary kind does.
pub(in crate::storage) async fn mount_ok(server: &MockServer) {
    mount(server, MINT, minted()).await;
    mount(server, INGEST, accept_all).await;
}
