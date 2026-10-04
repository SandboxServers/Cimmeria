//! The Live Research Lab **supervisor**: owns the SGW.exe process for
//! the session, drives the client flows, watches the heartbeat, and recovers
//! from crashes (ADR §3.4, §6; issue #685).
//!
//! Layout (directory from day one — 4+ siblings per the file-org rule):
//! - [`session_file`] — write `current-session.json` + `lab` block +
//!   fresh token; read `lab-account.json`.
//! - [`telemetry_session`] — mint the launch's dev-session telemetry token
//!   (`session_kind = lab`) so the client's events reach SigNoz.
//! - [`login_servers`] — the client's `LoginInternal.lua` server rows,
//!   which name the server to mint from.
//! - [`process`] — native launch/inject/status/terminate + window
//!   resolution by PID.
//! - [`heartbeat`] — the staleness watchdog decision (pure, tested).
//! - [`recovery`] — command journal (+ quarantine) and the 3-in-10-min
//!   relaunch cap (pure, tested).
//! - [`input`] / [`keys`] — native input: clicks, key taps, typing.
//! - [`flows`] — login, character select, play, dialog and logout flows
//!   over the native input, plus UI reads (`client_ui_state`,
//!   `client_wait_for`).
//! - [`entity_table`] — the client's BigWorld entity maps.
//! - [`screenshot`] — GDI window capture → PNG.
//! - [`crash_report`] — assemble `lab_crash_report`.
//! - [`events`] — the seq-numbered client-event history with named
//!   cursors (`client_wait_event`, `client_events_read`).
//! - [`combat`] — hotbar, ability use, combat log, defeat and respawn.

pub mod combat;
pub mod crash_report;
pub mod display;
pub mod entity_table;
pub mod events;
pub mod flows;
pub mod heartbeat;
pub mod input;
pub mod instance;
pub mod keys;
pub mod login_servers;
pub mod process;
pub mod recovery;
pub mod screenshot;
pub mod session_file;
pub mod telemetry_session;
pub mod ui;
mod watchdog;
pub mod world;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::client::BridgeClient;
use crate::timeline::client_events::HeartbeatSample;
use crate::timeline::clock::ClockOffset;

use heartbeat::{HeartbeatState, HeartbeatWatchdog};
use recovery::{CommandJournal, PersistentHooks, RecoveryTracker};
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
/// Heartbeat-observation ring depth fed to `lab_timeline` as the client
/// event source that exists today (ADR §5; the full #686 event ring
/// plugs in later — see `timeline::client_events`).
const HEARTBEAT_RING_CAP: usize = 256;

pub(crate) fn now_ms() -> i64 {
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
    /// `cimmeria-client-patches.dll`, injected before the bridge DLL as the
    /// launcher does (`CIMMERIA_LAB_PATCHES_DLL`). Unset = bridge only.
    pub patches_dll: Option<PathBuf>,
    /// The i686 `sgw-start32.exe` helper that does the injection (#985):
    /// `CIMMERIA_LAB_START32`, else beside this executable.
    pub helper_path: Option<PathBuf>,
    /// Bridge bind address written into the session file.
    pub bind: String,
    /// Bridge port.
    pub port: u16,
    /// The named lab instance this supervisor drives (`CIMMERIA_LAB_INSTANCE`),
    /// or `None` for the default, single-client layout ([`instance`]).
    pub instance: Option<String>,
    /// Where each launch mints its telemetry token, and an optional
    /// upload endpoint override ([`telemetry_session`]).
    pub telemetry: telemetry_session::TelemetryConfig,
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
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(PathBuf::from));
        let helper_path = process::resolve_helper(
            std::env::var("CIMMERIA_LAB_START32")
                .ok()
                .map(PathBuf::from),
            exe_dir.as_deref(),
        );
        let patches_dll = std::env::var("CIMMERIA_LAB_PATCHES_DLL")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        Self {
            install_dir,
            dll_path,
            patches_dll,
            helper_path,
            bind: std::env::var("CIMMERIA_LAB_BRIDGE_BIND")
                .unwrap_or_else(|_| DEFAULT_BRIDGE_BIND.to_string()),
            port: std::env::var("CIMMERIA_LAB_BRIDGE_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_BRIDGE_PORT),
            // A bad name is reported by `main` before this runs; here it
            // must not silently become the default instance.
            instance: instance::from_env().ok().flatten(),
            telemetry: telemetry_session::TelemetryConfig::from_env(),
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
    /// At character select (after `lab_login` or `lab_logout`).
    CharSelect,
    InWorld,
    Failed,
    Crashed,
}

impl LoginState {
    fn as_str(self) -> &'static str {
        match self {
            LoginState::NotStarted => "not_started",
            LoginState::LoggingIn => "logging_in",
            LoginState::CharSelect => "character_select",
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
    /// The last launch's telemetry grant, as `lab_client_start` reports it.
    telemetry: Option<Value>,
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
    /// Hooks marked `persistent`, re-applied after a crash (ADR §6).
    persistent_hooks: PersistentHooks,
}

impl SupervisorState {
    fn new() -> Self {
        Self {
            pid: None,
            started_at: None,
            token: None,
            telemetry: None,
            login: LoginState::NotStarted,
            watchdog: HeartbeatWatchdog::new(HEARTBEAT_STALE_AFTER),
            recovery: RecoveryTracker::new_default(),
            journal: CommandJournal::new(JOURNAL_CAP),
            heartbeats: VecDeque::with_capacity(HEARTBEAT_RING_CAP),
            clock_offset: None,
            persistent_hooks: PersistentHooks::new(),
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
    /// Client-event history (the bridge ring's only drainer).
    events: Arc<events::store::EventStore>,
    /// The lab lease ([`crate::lease`]): the watchdog relaunches a dead
    /// client only while one is held.
    leases: Arc<crate::lease::LeaseBook>,
}

impl Supervisor {
    pub fn new(bridge: Arc<BridgeClient>, config: SupervisorConfig) -> Self {
        display::spawn_keep_awake();
        Self {
            bridge,
            config,
            state: Arc::new(Mutex::new(SupervisorState::new())),
            events: Arc::new(events::store::EventStore::default()),
            leases: crate::lease::global(),
        }
    }

    /// Use `leases` instead of the process-wide book (tests).
    #[cfg(test)]
    pub fn with_leases(mut self, leases: Arc<crate::lease::LeaseBook>) -> Self {
        self.leases = leases;
        self
    }

    /// The lease book this supervisor's watchdog consults.
    pub fn leases(&self) -> &Arc<crate::lease::LeaseBook> {
        &self.leases
    }

    /// Proxy a phase-1 client tool call through the bridge, journaling it
    /// so `lab_crash_report` can show the last N and quarantine the
    /// in-flight one on a crash.
    pub async fn bridge_call(&self, method: &str, params: Value) -> Result<Value, String> {
        // Every client action of a lease-guarded tool passes here: stop
        // before it when the tool's lease was revoked (lease::permit).
        crate::lease::permit::ensure(&format!("bridge {method}"))?;
        let seq = {
            let mut st = self.state.lock().await;
            st.journal.record(method, now_ms())
        };
        // Keep a copy of the params for persistent-hook bookkeeping; the
        // call consumes `params`.
        let hook_params = matches!(method, "hook_install" | "hook_remove").then(|| params.clone());
        let result = self
            .bridge
            .call(method, params)
            .await
            .map_err(|e| format!("bridge: {e}"));
        {
            let mut st = self.state.lock().await;
            st.journal.complete(seq, result.is_ok());
            // Track persistent hooks so the recovery path can replay them
            // (ADR §6). Only a *successful* install/remove updates the set.
            if let (Ok(res), Some(p)) = (&result, &hook_params) {
                match method {
                    "hook_install" => {
                        if let Some(hid) = res.get("id").and_then(Value::as_u64) {
                            st.persistent_hooks.note_install(hid as u32, p);
                        }
                    }
                    "hook_remove" => {
                        if let Some(hid) = p.get("id").and_then(Value::as_u64) {
                            st.persistent_hooks.note_remove(hid as u32);
                        }
                    }
                    _ => {}
                }
            }
        }
        result
    }

    /// Launch (or relaunch) the client: mint a token, write the session
    /// file, launch+inject, and re-point the bridge. Shared by `start`
    /// and the watchdog's recovery path.
    async fn launch_client(&self, server_override: Option<String>) -> Result<u32, String> {
        // A client launched without a usable display dies on an error box
        // (and the watchdog relaunches it); refuse with the reason instead.
        tokio::task::spawn_blocking(display::ensure_display_for_launch)
            .await
            .map_err(|e| format!("display check: {e}"))??;
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
        let helper = self
            .config
            .helper_path
            .clone()
            .ok_or("no sgw-start32.exe path (set CIMMERIA_LAB_START32)")?;

        let token = session_file::generate_token();
        let inst = self.config.instance.as_deref();
        let session_path = instance::session_path(&install_dir, inst);
        // Mint from the server the client logs into: the server row the
        // login flow will pick (override, else lab-account.json).
        let server = server_override.or_else(|| self.lab_account().map(|a| a.server));
        let telemetry_cfg = self
            .config
            .telemetry
            .for_launch(&install_dir, server.as_deref());
        let telemetry = match &telemetry_cfg {
            Ok(cfg) => {
                telemetry_session::grant_for_launch_cached(
                    cfg,
                    &session_path.with_file_name("lab-telemetry-grant.json"),
                )
                .await
            }
            Err(why) => {
                telemetry_session::TelemetryGrant::unavailable(why.clone(), &self.config.telemetry)
            }
        };
        self.config.telemetry.check(&telemetry)?;
        let mut session =
            session_file::build_session(&token, &self.config.bind, self.config.port, &telemetry);
        if let Some(name) = inst {
            session.tags.push(format!("instance:{name}"));
        }
        session_file::write_session_at(&session_path, &session)?;
        let envs = instance::launch_env(&install_dir, inst);

        // Native launch runs on a blocking thread. SGW.exe lives in
        // `<install>/Binaries`, next to the `sessions/` dir the session file
        // was just written to; the launcher starts it there too.
        let (bin2, dll2) = (install_dir.join("Binaries"), dll_path.clone());
        let patches = self.config.patches_dll.clone();
        // Last check before a process exists: preparation (display probe,
        // telemetry mint) takes seconds, and the lease may be gone by now.
        crate::lease::permit::ensure("launch SGW.exe")?;
        let pid = tokio::task::spawn_blocking(move || {
            process::launch(&bin2, &dll2, &helper, patches.as_deref(), &envs)
        })
        .await
        .map_err(|e| format!("launch task: {e}"))??;
        if let Err(e) = instance::write_entry(&install_dir, inst, pid, self.config.port) {
            tracing::warn!(error = %e, "could not record this client for its peers");
        }

        // Point the bridge at the fresh per-launch token.
        let addr = format!("{}:{}", self.config.connect_host(), self.config.port);
        self.bridge.reconfigure(addr, token.clone()).await;

        {
            let mut st = self.state.lock().await;
            st.pid = Some(pid);
            st.started_at = Some(Instant::now());
            st.token = Some(token);
            st.telemetry = Some(telemetry.status());
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
        // A client the lab did not start (the launcher, a player) is not
        // the lab's to stop or to share a machine with. Another lab
        // instance's client is fine, up to the cap ([`instance::check_launch`]).
        let running = process::running_sgw_pids();
        let peers: Vec<_> = self
            .config
            .install_dir
            .as_deref()
            .map(|d| instance::read_peers(d, self.config.instance.as_deref()))
            .unwrap_or_default()
            .into_iter()
            .filter(|p| process::is_alive(p.pid))
            .collect();
        instance::check_launch(
            &running,
            &peers,
            self.config.port,
            instance::max_clients_from_env(),
        )?;
        let pid = self.launch_client(server_override).await?;
        self.spawn_watchdog(pid);
        let telemetry = self.state.lock().await.telemetry.clone();
        Ok(
            json!({ "pid": pid, "bridge_port": self.config.port, "started": true,
                   "telemetry": telemetry }),
        )
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
                if let Some(d) = self.config.install_dir.as_deref() {
                    instance::remove_entry(d, self.config.instance.as_deref());
                }
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
        let (pid, uptime_ms, login, crash_count, persistent_hooks) = {
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
                st.persistent_hooks.count(),
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
            "persistent_hooks": persistent_hooks,
            "heartbeat": { "tick_count": hb_count, "state": hb_state, "age_ms": hb_age },
        }))
    }

    /// Record login progress for `lab_client_status`.
    pub(crate) async fn set_login_state(&self, login: LoginState) {
        self.state.lock().await.login = login;
    }

    /// This instance's credentials (`lab-account.json`, or
    /// `lab-account.<instance>.json` for a named instance), if the file
    /// exists and parses.
    pub(crate) fn lab_account(&self) -> Option<session_file::LabAccount> {
        let dir = self.config.install_dir.as_deref()?;
        let path = instance::account_path(dir, self.config.instance.as_deref());
        session_file::read_lab_account_at(&path).ok()
    }

    /// `lab_screenshot` — capture the client window as PNG bytes +
    /// base64, for the caller to wrap in an MCP image block.
    pub async fn screenshot(&self) -> Result<(String, u32, u32), String> {
        let captured = self.capture().await?;
        let png = screenshot::encode_png(&captured)?;
        Ok((
            screenshot::png_to_base64(&png),
            captured.width,
            captured.height,
        ))
    }

    /// Capture the client window as RGBA (for screenshots, crops and
    /// pixel probes).
    pub async fn capture(&self) -> Result<screenshot::CapturedImage, String> {
        let pid = {
            let st = self.state.lock().await;
            st.pid.ok_or("no client running")?
        };
        tokio::task::spawn_blocking(move || screenshot::capture_pid(pid))
            .await
            .map_err(|e| format!("screenshot task: {e}"))?
    }

    /// `lab_crash_report` — last minidump, last N commands, quarantined
    /// command, DLL crash marker.
    pub async fn crash_report(&self) -> Result<Value, String> {
        let install_dir = self
            .config
            .install_dir
            .clone()
            .ok_or("CIMMERIA_LAB_INSTALL_DIR is unset")?;
        let dir = instance::instance_dir(&install_dir, self.config.instance.as_deref());
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
            patches_dll: None,
            helper_path: None,
            bind: "0.0.0.0".into(),
            port: 8770,
            instance: None,
            telemetry: Default::default(),
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
