//! The Live Research Lab **supervisor**: owns the SGW.exe process for
//! the session, drives autologin, watches the heartbeat, and recovers
//! from crashes (ADR §3.4, §6; issue #685).
//!
//! Layout (directory from day one — 4+ siblings per the file-org rule):
//! - [`session_file`] — write `current-session.json` + `lab` block +
//!   fresh token; read `lab-account.json`.
//! - [`process`] — native launch/inject/status/terminate + window
//!   resolution by PID.
//! - [`heartbeat`] — the staleness watchdog decision (pure, tested).
//! - [`recovery`] — command journal (+ quarantine) and the 3-in-10-min
//!   relaunch cap (pure, tested).
//! - [`autologin`] — the Lua-driven login state machine (pure, tested);
//!   [`autologin_bridge`] adapts it onto the async bridge.
//! - [`screenshot`] — GDI window capture → PNG.
//! - [`crash_report`] — assemble `lab_crash_report`.

pub mod autologin;
pub mod autologin_bridge;
pub mod crash_report;
pub mod heartbeat;
pub mod process;
pub mod recovery;
pub mod screenshot;
pub mod session_file;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::client::BridgeClient;
use crate::timeline::client_events::HeartbeatSample;
use crate::timeline::clock::ClockOffset;

use autologin::LoginOutcome;
use heartbeat::{HeartbeatState, HeartbeatWatchdog};
use recovery::{CommandJournal, RecoveryTracker};
use session_file::{DEFAULT_BRIDGE_BIND, DEFAULT_BRIDGE_PORT};

/// Heartbeat poll cadence for the background watchdog.
const WATCHDOG_POLL: Duration = Duration::from_secs(1);
/// No Tick advance for this long ⇒ hung/crash-dialog ⇒ terminate.
const HEARTBEAT_STALE_AFTER: Duration = Duration::from_secs(8);
/// Consecutive failed heartbeat polls tolerated before we treat the
/// client as dead (belt-and-braces alongside the direct liveness check).
const MAX_HEARTBEAT_FAILS: u32 = 5;
/// Command-journal ring depth surfaced by `lab_crash_report`.
const JOURNAL_CAP: usize = 64;
/// Autologin poll budget.
const AUTOLOGIN_MAX_POLLS: u32 = 120;
/// Heartbeat-observation ring depth fed to `lab_timeline` as the client
/// event source that exists today (ADR §5; the full #686 event ring
/// plugs in later — see `timeline::client_events`).
const HEARTBEAT_RING_CAP: usize = 256;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Where the client lives and how the bridge is reached.
#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    /// Game install directory (contains `Binaries/SGW.exe`).
    pub install_dir: Option<PathBuf>,
    /// Telemetry DLL (built with `--features lab-bridge`) to inject.
    pub dll_path: Option<PathBuf>,
    /// Bridge bind address written into the session file.
    pub bind: String,
    /// Bridge port.
    pub port: u16,
    /// Telemetry upload endpoint written into the session file (uploads
    /// are best-effort; a bad endpoint just fails silently).
    pub upload_endpoint: String,
}

impl SupervisorConfig {
    /// Read config from the environment.
    pub fn from_env() -> Self {
        let install_dir = std::env::var("CIMMERIA_LAB_INSTALL_DIR")
            .ok()
            .map(PathBuf::from);
        let dll_path = std::env::var("CIMMERIA_LAB_DLL")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                install_dir
                    .as_ref()
                    .map(|d| d.join("Binaries").join("cimmeria-client-telemetry.dll"))
            });
        Self {
            install_dir,
            dll_path,
            bind: std::env::var("CIMMERIA_LAB_BRIDGE_BIND")
                .unwrap_or_else(|_| DEFAULT_BRIDGE_BIND.to_string()),
            port: std::env::var("CIMMERIA_LAB_BRIDGE_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_BRIDGE_PORT),
            upload_endpoint: std::env::var("CIMMERIA_LAB_UPLOAD_ENDPOINT")
                .unwrap_or_else(|_| "http://127.0.0.1/api/telemetry/upload-chunk".to_string()),
        }
    }

    /// Loopback-safe host to connect the bridge client to.
    fn connect_host(&self) -> &str {
        if self.bind == "0.0.0.0" {
            "127.0.0.1"
        } else {
            &self.bind
        }
    }
}

/// Login progress, reported by `lab_client_status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginState {
    NotStarted,
    LoggingIn,
    InWorld,
    Failed,
    Crashed,
}

impl LoginState {
    fn as_str(self) -> &'static str {
        match self {
            LoginState::NotStarted => "not_started",
            LoginState::LoggingIn => "logging_in",
            LoginState::InWorld => "in_world",
            LoginState::Failed => "failed",
            LoginState::Crashed => "crashed",
        }
    }
}

/// Mutable supervisor state behind one async lock.
struct SupervisorState {
    pid: Option<u32>,
    started_at: Option<Instant>,
    token: Option<String>,
    login: LoginState,
    watchdog: HeartbeatWatchdog,
    recovery: RecoveryTracker,
    journal: CommandJournal,
    /// Recent heartbeat observations, the client-event source for
    /// `lab_timeline`. Bounded ring; oldest dropped past the cap.
    heartbeats: VecDeque<HeartbeatSample>,
    /// Cached client↔server clock offset (ADR §5). Re-pinned whenever a
    /// packet-tap ping is available; used by `lab_timeline` between pins.
    clock_offset: Option<ClockOffset>,
}

impl SupervisorState {
    fn new() -> Self {
        Self {
            pid: None,
            started_at: None,
            token: None,
            login: LoginState::NotStarted,
            watchdog: HeartbeatWatchdog::new(HEARTBEAT_STALE_AFTER),
            recovery: RecoveryTracker::new_default(),
            journal: CommandJournal::new(JOURNAL_CAP),
            heartbeats: VecDeque::with_capacity(HEARTBEAT_RING_CAP),
            clock_offset: None,
        }
    }

    /// Append a heartbeat observation, dropping the oldest past the cap.
    fn record_heartbeat(&mut self, tick_count: u64, observed_ms: i64) {
        if self.heartbeats.len() == HEARTBEAT_RING_CAP {
            self.heartbeats.pop_front();
        }
        self.heartbeats.push_back(HeartbeatSample {
            tick_count,
            observed_ms,
        });
    }
}

/// The supervisor. Cloneable-cheap (everything is behind `Arc`), shared
/// by the MCP server across requests and by the background watchdog.
#[derive(Clone)]
pub struct Supervisor {
    bridge: Arc<BridgeClient>,
    config: SupervisorConfig,
    state: Arc<Mutex<SupervisorState>>,
}

impl Supervisor {
    pub fn new(bridge: Arc<BridgeClient>, config: SupervisorConfig) -> Self {
        Self {
            bridge,
            config,
            state: Arc::new(Mutex::new(SupervisorState::new())),
        }
    }

    /// Proxy a phase-1 client tool call through the bridge, journaling it
    /// so `lab_crash_report` can show the last N and quarantine the
    /// in-flight one on a crash.
    pub async fn bridge_call(&self, method: &str, params: Value) -> Result<Value, String> {
        let seq = {
            let mut st = self.state.lock().await;
            st.journal.record(method, now_ms())
        };
        let result = self
            .bridge
            .call(method, params)
            .await
            .map_err(|e| format!("bridge: {e}"));
        {
            let mut st = self.state.lock().await;
            st.journal.complete(seq, result.is_ok());
        }
        result
    }

    /// Launch (or relaunch) the client: mint a token, write the session
    /// file, launch+inject, and re-point the bridge. Shared by `start`
    /// and the watchdog's recovery path.
    async fn launch_client(&self, _server_override: Option<String>) -> Result<u32, String> {
        let install_dir = self
            .config
            .install_dir
            .clone()
            .ok_or("CIMMERIA_LAB_INSTALL_DIR is unset")?;
        let dll_path = self
            .config
            .dll_path
            .clone()
            .ok_or("no DLL path (set CIMMERIA_LAB_DLL)")?;

        let token = session_file::generate_token();
        let session = session_file::build_session(
            &token,
            &self.config.bind,
            self.config.port,
            &self.config.upload_endpoint,
        );
        session_file::write_session(&install_dir, &session)?;

        // Native launch runs on a blocking thread.
        let (install2, dll2) = (install_dir.clone(), dll_path.clone());
        let pid = tokio::task::spawn_blocking(move || process::launch(&install2, &dll2))
            .await
            .map_err(|e| format!("launch task: {e}"))??;

        // Point the bridge at the fresh per-launch token.
        let addr = format!("{}:{}", self.config.connect_host(), self.config.port);
        self.bridge.reconfigure(addr, token.clone()).await;

        {
            let mut st = self.state.lock().await;
            st.pid = Some(pid);
            st.started_at = Some(Instant::now());
            st.token = Some(token);
            st.login = LoginState::NotStarted;
            st.watchdog = HeartbeatWatchdog::new(HEARTBEAT_STALE_AFTER);
        }
        Ok(pid)
    }

    /// `lab_client_start` — launch suspended, inject the lab DLL, resume.
    pub async fn start(&self, server_override: Option<String>) -> Result<Value, String> {
        {
            let st = self.state.lock().await;
            if let Some(pid) = st.pid {
                if process::is_alive(pid) {
                    return Err(format!(
                        "a client is already running (pid {pid}); stop it first"
                    ));
                }
            }
        }
        let pid = self.launch_client(server_override).await?;
        self.spawn_watchdog(pid);
        Ok(json!({ "pid": pid, "bridge_port": self.config.port, "started": true }))
    }

    /// `lab_client_stop` — terminate the client.
    pub async fn stop(&self) -> Result<Value, String> {
        let pid = {
            let mut st = self.state.lock().await;
            let pid = st.pid.take();
            st.login = LoginState::NotStarted;
            pid
        };
        match pid {
            Some(pid) => {
                let _ = tokio::task::spawn_blocking(move || process::terminate(pid)).await;
                Ok(json!({ "stopped": true, "pid": pid }))
            }
            None => Ok(json!({ "stopped": false, "reason": "no client running" })),
        }
    }

    /// `lab_client_restart` — stop then start.
    pub async fn restart(&self, server_override: Option<String>) -> Result<Value, String> {
        let _ = self.stop().await;
        // Brief settle so the OS releases the port + the old process.
        tokio::time::sleep(Duration::from_millis(500)).await;
        self.start(server_override).await
    }

    /// `lab_client_status` — pid, uptime, heartbeat age, login state,
    /// crash count. Polls the bridge for the heartbeat and folds it into
    /// the watchdog (also the on-demand staleness check).
    pub async fn status(&self) -> Result<Value, String> {
        let (pid, uptime_ms, login, crash_count) = {
            let mut st = self.state.lock().await;
            let uptime = st
                .started_at
                .map(|t| t.elapsed().as_millis() as i64)
                .unwrap_or(0);
            (
                st.pid,
                uptime,
                st.login,
                st.recovery.recent_crash_count(now_ms()),
            )
        };

        // Heartbeat poll happens without the state lock held.
        let hb = self.bridge.heartbeat().await;
        let (hb_count, hb_state, hb_age): (Option<u64>, &str, i64) = match hb {
            Ok(count) => {
                let mut st = self.state.lock().await;
                let ts = now_ms();
                st.record_heartbeat(count, ts);
                let s = st.watchdog.observe(count, ts);
                let age = st.watchdog.age_ms(ts);
                (
                    Some(count),
                    if s == HeartbeatState::Stale {
                        "stale"
                    } else {
                        "alive"
                    },
                    age,
                )
            }
            Err(_) => (None, "unreachable", 0),
        };

        Ok(json!({
            "pid": pid,
            "running": pid.map(process::is_alive).unwrap_or(false),
            "uptime_ms": uptime_ms,
            "login_state": login.as_str(),
            "crash_count_10min": crash_count,
            "heartbeat": { "tick_count": hb_count, "state": hb_state, "age_ms": hb_age },
        }))
    }

    /// `lab_login` — drive the Lua autologin state machine over the
    /// bridge. See [`autologin_bridge`] for the live-capture caveat.
    pub async fn login(&self, server_override: Option<String>) -> Result<Value, String> {
        let install_dir = self
            .config
            .install_dir
            .clone()
            .ok_or("CIMMERIA_LAB_INSTALL_DIR is unset")?;
        let mut creds = session_file::read_lab_account(&install_dir)?.into_creds();
        if let Some(server) = server_override {
            creds.server = server;
        }

        {
            let mut st = self.state.lock().await;
            st.login = LoginState::LoggingIn;
        }

        let bridge = self.bridge.clone();
        let handle = tokio::runtime::Handle::current();
        let outcome = tokio::task::spawn_blocking(move || {
            let mut screen = autologin_bridge::BridgeScreen { bridge, handle };
            autologin::run(&mut screen, &creds, AUTOLOGIN_MAX_POLLS)
        })
        .await
        .map_err(|e| format!("autologin task: {e}"))?;

        let mut st = self.state.lock().await;
        match &outcome {
            Ok(LoginOutcome::EnteredWorld) => st.login = LoginState::InWorld,
            _ => st.login = LoginState::Failed,
        }
        match outcome {
            Ok(o) => Ok(json!({ "outcome": format!("{o:?}"), "login_state": st.login.as_str() })),
            Err(e) => Err(e),
        }
    }

    /// `lab_screenshot` — capture the client window as PNG bytes +
    /// base64, for the caller to wrap in an MCP image block.
    pub async fn screenshot(&self) -> Result<(String, u32, u32), String> {
        let pid = {
            let st = self.state.lock().await;
            st.pid.ok_or("no client running")?
        };
        let captured = tokio::task::spawn_blocking(move || screenshot::capture_pid(pid))
            .await
            .map_err(|e| format!("screenshot task: {e}"))??;
        let png = screenshot::encode_png(&captured)?;
        Ok((
            screenshot::png_to_base64(&png),
            captured.width,
            captured.height,
        ))
    }

    /// `lab_crash_report` — last minidump, last N commands, quarantined
    /// command, DLL crash marker.
    pub async fn crash_report(&self) -> Result<Value, String> {
        let install_dir = self
            .config
            .install_dir
            .clone()
            .ok_or("CIMMERIA_LAB_INSTALL_DIR is unset")?;
        let dir = session_file::sessions_dir(&install_dir);
        let st = self.state.lock().await;
        Ok(crash_report::build_report(&st.journal, &dir, JOURNAL_CAP))
    }

    /// Snapshot the heartbeat-observation ring for `lab_timeline`. Cloned
    /// out so the timeline builds without holding the state lock.
    pub async fn heartbeat_samples(&self) -> Vec<HeartbeatSample> {
        let st = self.state.lock().await;
        st.heartbeats.iter().copied().collect()
    }

    /// The cached client↔server clock offset, if one has been pinned.
    pub async fn cached_offset(&self) -> Option<ClockOffset> {
        let st = self.state.lock().await;
        st.clock_offset.clone()
    }

    /// Pin a freshly estimated clock offset (from a packet-tap ping).
    pub async fn set_offset(&self, offset: ClockOffset) {
        let mut st = self.state.lock().await;
        st.clock_offset = Some(offset);
    }

    /// Spawn the background heartbeat watchdog for `pid`. It exits once
    /// the current launch's pid changes (a restart), or after it handles
    /// this launch's death.
    fn spawn_watchdog(&self, pid: u32) {
        let this = self.clone();
        tokio::spawn(async move { this.watchdog_loop(pid).await });
    }

    async fn watchdog_loop(&self, my_pid: u32) {
        let mut fails = 0u32;
        loop {
            tokio::time::sleep(WATCHDOG_POLL).await;

            // Stop if this launch has been superseded.
            {
                let st = self.state.lock().await;
                if st.pid != Some(my_pid) {
                    return;
                }
            }

            match self.bridge.heartbeat().await {
                Ok(count) => {
                    fails = 0;
                    let stale = {
                        let mut st = self.state.lock().await;
                        let ts = now_ms();
                        st.record_heartbeat(count, ts);
                        st.watchdog.observe(count, ts)
                    };
                    if stale == HeartbeatState::Stale {
                        tracing::warn!(pid = my_pid, "heartbeat stale; terminating hung client");
                        self.handle_death(my_pid).await;
                        return;
                    }
                }
                Err(_) => {
                    fails += 1;
                    if !process::is_alive(my_pid) || fails >= MAX_HEARTBEAT_FAILS {
                        tracing::warn!(pid = my_pid, fails, "client dead/unreachable");
                        self.handle_death(my_pid).await;
                        return;
                    }
                }
            }
        }
    }

    /// A death/hang was detected: terminate (in case it's hung),
    /// quarantine the in-flight command, record the crash, and — if
    /// under the recovery cap — relaunch and re-autologin.
    async fn handle_death(&self, dead_pid: u32) {
        let _ = tokio::task::spawn_blocking(move || process::terminate(dead_pid)).await;

        let may_relaunch = {
            let mut st = self.state.lock().await;
            st.journal.quarantine_in_flight();
            st.recovery.record_crash(now_ms());
            st.login = LoginState::Crashed;
            st.pid = None;
            st.recovery.should_relaunch(now_ms())
        };

        if !may_relaunch {
            tracing::error!("recovery cap reached (3 crashes / 10 min); not relaunching");
            return;
        }

        match self.launch_client(None).await {
            Ok(new_pid) => {
                tracing::info!(new_pid, "relaunched after crash; re-running autologin");
                self.spawn_watchdog(new_pid);
                if let Err(e) = self.login(None).await {
                    tracing::warn!(error = %e, "post-crash autologin failed");
                }
            }
            Err(e) => tracing::error!(error = %e, "relaunch after crash failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_when_env_absent() {
        // Not reading real env here (other tests/process may have set
        // it); just assert the default constants are what the session
        // writer expects.
        assert_eq!(DEFAULT_BRIDGE_PORT, 8770);
        assert_eq!(DEFAULT_BRIDGE_BIND, "127.0.0.1");
    }

    #[test]
    fn connect_host_maps_wildcard_to_loopback() {
        let c = SupervisorConfig {
            install_dir: None,
            dll_path: None,
            bind: "0.0.0.0".into(),
            port: 8770,
            upload_endpoint: "e".into(),
        };
        assert_eq!(c.connect_host(), "127.0.0.1");
        let c2 = SupervisorConfig {
            bind: "192.168.1.5".into(),
            ..c
        };
        assert_eq!(c2.connect_host(), "192.168.1.5");
    }

    #[test]
    fn login_state_strings_are_stable() {
        assert_eq!(LoginState::InWorld.as_str(), "in_world");
        assert_eq!(LoginState::Crashed.as_str(), "crashed");
    }
}
