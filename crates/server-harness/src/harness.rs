//! Spawn a `cimmeria-server`, wait for it to be ready, reap it on drop.

use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::ports::PortSet;
use crate::process::ChildGuard;
use crate::readiness::{probe, ProbeOutcome, ReadinessCheck};

/// Environment variable naming the server executable explicitly.
pub const SERVER_EXE_ENV: &str = "CIMMERIA_SERVER_EXE";

/// Default ceiling on how long to wait for readiness. Generous because a cold
/// start also connects to Postgres and loads the resource cache.
pub const DEFAULT_READY_TIMEOUT: Duration = Duration::from_secs(90);

/// Gap between readiness probes. Short enough that startup isn't padded,
/// long enough not to spin. This is a *poll interval*, not a sleep-and-hope:
/// the loop exits on the observed condition, never on elapsed time.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Why a harness could not be started.
#[derive(Debug)]
pub enum HarnessError {
    /// The server executable could not be located. Tests should **skip**, not
    /// fail, on this: a developer running `cargo test -p cimmeria-services`
    /// has not necessarily built the server binary, and the same discipline
    /// already applies to the pcap-replay fixtures.
    ServerBinaryNotFound { searched: Vec<PathBuf> },
    /// The process could not be spawned or a port could not be reserved.
    Io(io::Error),
    /// The server exited before reporting ready.
    ExitedDuringStartup { status: String },
    /// The readiness deadline elapsed.
    ReadyTimeout {
        waited: Duration,
        last_outcome: String,
    },
}

impl std::fmt::Display for HarnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServerBinaryNotFound { searched } => write!(
                f,
                "cimmeria-server executable not found (searched: {searched:?}). \
                 Build it with `cargo build -p cimmeria-server`, or set {SERVER_EXE_ENV}."
            ),
            Self::Io(e) => write!(f, "harness I/O error: {e}"),
            Self::ExitedDuringStartup { status } => {
                write!(f, "server exited during startup with status {status}")
            }
            Self::ReadyTimeout {
                waited,
                last_outcome,
            } => write!(
                f,
                "server did not become ready within {waited:?}; last probe: {last_outcome}"
            ),
        }
    }
}

impl std::error::Error for HarnessError {}

impl From<io::Error> for HarnessError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// How to start a server instance.
#[derive(Clone, Debug)]
pub struct HarnessConfig {
    /// Explicit executable path. When `None`, resolved from
    /// [`SERVER_EXE_ENV`] then the ambient target directory.
    pub server_exe: Option<PathBuf>,
    /// Database connection string passed as `DB_URL`. `None` leaves the
    /// server's own default, which is the local test cluster on 5433.
    pub db_url: Option<String>,
    /// Value for `RUST_LOG` in the child.
    pub log_filter: String,
    /// Readiness ceiling.
    pub ready_timeout: Duration,
    /// Which services must be healthy.
    pub readiness: ReadinessCheck,
    /// Extra environment for the child, applied last so it can override
    /// anything the harness sets.
    pub extra_env: Vec<(String, String)>,
    /// Forward the child's stdout/stderr to the parent's.
    ///
    /// `true` by default for two reasons: cargo captures it for failing
    /// tests, and inheriting the console is what makes Ctrl-C reach the child
    /// directly, giving it a *graceful* shutdown rather than the hard kill
    /// `Drop` performs.
    pub inherit_output: bool,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            server_exe: None,
            db_url: None,
            // Quiet by default; the harness's own readiness reporting is
            // what a passing test needs. Raise for debugging.
            log_filter: "info".to_string(),
            ready_timeout: DEFAULT_READY_TIMEOUT,
            readiness: ReadinessCheck::default(),
            extra_env: Vec::new(),
            inherit_output: true,
        }
    }
}

impl HarnessConfig {
    /// Enable server-side autoplay for this instance.
    ///
    /// Sets both required gates: the character id **and** `DEVELOPER_MODE`.
    /// Autoplay refuses to engage without both, so a helper that set only one
    /// would silently produce a server that never auto-enters.
    ///
    /// This does not weaken authentication — the spawned server still
    /// validates credentials, and still refuses the configured id unless the
    /// authenticated account owns it.
    pub fn with_autoplay(mut self, player_id: i32, expected_world: Option<&str>) -> Self {
        self.extra_env.push((
            "CIMMERIA_AUTOPLAY_PLAYER_ID".to_string(),
            player_id.to_string(),
        ));
        self.extra_env
            .push(("DEVELOPER_MODE".to_string(), "true".to_string()));
        if let Some(world) = expected_world {
            self.extra_env
                .push(("CIMMERIA_AUTOPLAY_WORLD".to_string(), world.to_string()));
        }
        self
    }
}

/// A running server instance, reaped when dropped.
#[derive(Debug)]
pub struct ServerHarness {
    child: ChildGuard,
    ports: PortSet,
    startup: Duration,
}

impl ServerHarness {
    /// Spawn a server on freshly reserved ports and block until it reports
    /// ready.
    ///
    /// On any failure the partially-started child is reaped before returning,
    /// so an error path cannot leak a process.
    pub fn start(config: HarnessConfig) -> Result<Self, HarnessError> {
        let exe = match &config.server_exe {
            Some(p) if p.is_file() => p.clone(),
            Some(p) => {
                return Err(HarnessError::ServerBinaryNotFound {
                    searched: vec![p.clone()],
                })
            }
            None => locate_server_exe()?,
        };

        let ports = PortSet::reserve()?;
        let mut command = Command::new(&exe);

        // Every port the server binds is pinned to a reserved value. Any
        // omission would fall back to a compiled-in default and collide with
        // a concurrently running instance.
        command
            .env("AUTH_PORT", ports.auth.to_string())
            .env("LOGON_PORT", ports.logon.to_string())
            .env("BASE_PORT", ports.base.to_string())
            .env("CELL_PORT", ports.cell.to_string())
            .env("ADMIN_PORT", ports.admin.to_string())
            .env("MINIGAME_PORT", ports.minigame.to_string())
            .env("BASE_EXTERNAL", "127.0.0.1")
            .env("RUST_LOG", &config.log_filter)
            // Discord posts to a real webhook if a config file happens to be
            // present in the working directory. Point it at a path that will
            // not exist so a test server stays silent; a *missing* file is a
            // soft-fail that disables Discord, while an invalid one exits(2).
            .env("DISCORD_CONFIG_PATH", "harness-discord-disabled.toml");

        if let Some(db) = &config.db_url {
            command.env("DB_URL", db);
        }
        for (key, value) in &config.extra_env {
            command.env(key, value);
        }

        if config.inherit_output {
            command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        } else {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
        // Never inherit the parent's stdin: a child reading the test runner's
        // stdin can consume input meant for the runner.
        command.stdin(Stdio::null());

        let started = Instant::now();
        let mut child = ChildGuard::spawn(command, "cimmeria-server")?;

        // `child` is a ChildGuard from here on, so every `?` below reaps it.
        let admin_addr: SocketAddr = format!("127.0.0.1:{}", ports.admin)
            .parse()
            .expect("admin addr is built from a u16 port");

        Self::await_ready(&mut child, admin_addr, &config, started)?;

        let startup = started.elapsed();
        tracing::info!(
            pid = child.pid(),
            admin_port = ports.admin,
            base_port = ports.base,
            startup_ms = startup.as_millis() as u64,
            "Server harness ready"
        );

        Ok(Self {
            child,
            ports,
            startup,
        })
    }

    /// Poll until ready, the server dies, or the deadline passes.
    fn await_ready(
        child: &mut ChildGuard,
        admin_addr: SocketAddr,
        config: &HarnessConfig,
        started: Instant,
    ) -> Result<(), HarnessError> {
        let deadline = started + config.ready_timeout;
        let mut last = "no probe attempted".to_string();

        loop {
            // Fail fast on a dead server rather than polling a port that will
            // never open. Without this the caller waits the full timeout to
            // learn about a bind conflict that was reported instantly.
            if let Some(status) = child.try_exit_status()? {
                return Err(HarnessError::ExitedDuringStartup {
                    status: status.to_string(),
                });
            }

            // Cap the per-probe timeout at the remaining budget so a hung
            // connect cannot overshoot the deadline.
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(HarnessError::ReadyTimeout {
                    waited: started.elapsed(),
                    last_outcome: last,
                });
            }
            let probe_timeout = remaining.min(Duration::from_secs(2));

            match probe(admin_addr, config.readiness, probe_timeout) {
                ProbeOutcome::Ready => return Ok(()),
                ProbeOutcome::NotReady { detail } => last = format!("not ready: {detail}"),
                ProbeOutcome::Unreachable { detail } => last = format!("unreachable: {detail}"),
            }

            // Interval between observations, not a fixed startup delay: the
            // loop exits the moment the condition is observed.
            std::thread::sleep(
                POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())),
            );
        }
    }

    /// Ports this instance was assigned.
    pub fn ports(&self) -> &PortSet {
        &self.ports
    }

    /// OS process id of the server.
    pub fn pid(&self) -> u32 {
        self.child.pid()
    }

    /// How long the server took to become ready.
    pub fn startup_duration(&self) -> Duration {
        self.startup
    }

    /// Admin API address — the readiness endpoint's host.
    pub fn admin_addr(&self) -> SocketAddr {
        format!("127.0.0.1:{}", self.ports.admin)
            .parse()
            .expect("admin addr is built from a u16 port")
    }

    /// SOAP login address a client or wireclient would authenticate against.
    pub fn logon_addr(&self) -> SocketAddr {
        format!("127.0.0.1:{}", self.ports.logon)
            .parse()
            .expect("logon addr is built from a u16 port")
    }

    /// BaseApp Mercury UDP address.
    pub fn base_addr(&self) -> SocketAddr {
        format!("127.0.0.1:{}", self.ports.base)
            .parse()
            .expect("base addr is built from a u16 port")
    }

    /// Kill and reap now instead of at end of scope.
    pub fn shutdown(mut self) {
        self.child.kill_and_reap();
    }
}

/// Find the server executable.
///
/// Order: [`SERVER_EXE_ENV`], then alongside the running test executable
/// (`target/<profile>/deps/test-hash` → `target/<profile>/cimmeria-server`),
/// then the repo-root copy that the documented build step produces.
fn locate_server_exe() -> Result<PathBuf, HarnessError> {
    let exe_name = if cfg!(windows) {
        "cimmeria-server.exe"
    } else {
        "cimmeria-server"
    };

    let mut searched = Vec::new();

    if let Some(from_env) = std::env::var_os(SERVER_EXE_ENV) {
        let path = PathBuf::from(from_env);
        if path.is_file() {
            return Ok(path);
        }
        searched.push(path);
    }

    if let Ok(test_exe) = std::env::current_exe() {
        // Test binaries live in `<target>/<profile>/deps/`; the server binary
        // is one level up. Also check the same directory for the case where a
        // consumer runs from the profile directory directly.
        for dir in test_exe
            .parent()
            .into_iter()
            .chain(test_exe.parent().and_then(Path::parent).into_iter())
        {
            let candidate = dir.join(exe_name);
            if candidate.is_file() {
                return Ok(candidate);
            }
            searched.push(candidate);
        }
    }

    // The project's build instructions copy the exe to the repo root.
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join(exe_name);
        if candidate.is_file() {
            return Ok(candidate);
        }
        searched.push(candidate);
    }

    Err(HarnessError::ServerBinaryNotFound { searched })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A missing binary must produce the skippable error variant, not a
    /// panic and not a generic I/O error — tests key on this to skip rather
    /// than fail when the server hasn't been built.
    #[test]
    fn explicit_missing_exe_reports_binary_not_found() {
        let config = HarnessConfig {
            server_exe: Some(PathBuf::from("definitely-not-a-real-server-binary")),
            ..Default::default()
        };
        match ServerHarness::start(config) {
            Err(HarnessError::ServerBinaryNotFound { searched }) => {
                assert!(
                    !searched.is_empty(),
                    "error should report what it looked for"
                );
            }
            other => panic!("expected ServerBinaryNotFound, got {other:?}"),
        }
    }

    /// `with_autoplay` must set BOTH gates. Setting only the id yields a
    /// server that refuses autoplay with `developer_mode_required`, which
    /// would look like a broken harness rather than a config error.
    #[test]
    fn with_autoplay_sets_both_required_gates() {
        let config = HarnessConfig::default().with_autoplay(42, Some("Castle_CellBlock"));
        let env: std::collections::HashMap<&str, &str> = config
            .extra_env
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        assert_eq!(env.get("CIMMERIA_AUTOPLAY_PLAYER_ID"), Some(&"42"));
        assert_eq!(
            env.get("DEVELOPER_MODE"),
            Some(&"true"),
            "developer_mode is autoplay's second gate; without it the server refuses to engage"
        );
        assert_eq!(
            env.get("CIMMERIA_AUTOPLAY_WORLD"),
            Some(&"Castle_CellBlock")
        );
    }

    #[test]
    fn with_autoplay_omits_world_guard_when_unset() {
        let config = HarnessConfig::default().with_autoplay(7, None);
        assert!(
            !config
                .extra_env
                .iter()
                .any(|(k, _)| k == "CIMMERIA_AUTOPLAY_WORLD"),
            "no world guard should be set when none was requested"
        );
    }

    /// Autoplay must be absent unless explicitly requested — the harness must
    /// not be a back door that arms it for every spawned server.
    #[test]
    fn default_config_sets_no_autoplay_env() {
        let config = HarnessConfig::default();
        assert!(
            config.extra_env.is_empty(),
            "default harness config must not set any autoplay/developer env"
        );
    }
}
