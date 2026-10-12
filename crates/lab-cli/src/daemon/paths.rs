//! Where the lab daemon keeps its files.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::env_file;

/// The bind address `Get-LabStatus` uses when `labd.pid` names none.
const DEFAULT_BIND: &str = "127.0.0.1:8779";

/// `%LOCALAPPDATA%`, or the temp directory where that variable is unset
/// (the Linux CI runners).
fn local_app_data() -> PathBuf {
    match std::env::var_os("LOCALAPPDATA") {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => std::env::temp_dir(),
    }
}

/// The lab home: `CIMMERIA_LAB_HOME` when set and non-empty (tests only),
/// else `%LOCALAPPDATA%\cimmeria-lab`. Port of `Get-LabHome`.
pub fn lab_home() -> PathBuf {
    match std::env::var_os("CIMMERIA_LAB_HOME") {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => local_app_data().join("cimmeria-lab"),
    }
}

/// True when `CIMMERIA_LAB_HOME` is set and non-empty, so [`lab_home`] is the
/// test override and not the real `%LOCALAPPDATA%\cimmeria-lab`.
pub fn lab_home_overridden() -> bool {
    std::env::var_os("CIMMERIA_LAB_HOME").is_some_and(|value| !value.is_empty())
}

/// `<home>\labd.env`. Port of `Get-LabdEnvPath`.
pub fn labd_env_path() -> PathBuf {
    lab_home().join("labd.env")
}

/// `<home>\labd.pid`.
pub fn labd_pid_path() -> PathBuf {
    lab_home().join("labd.pid")
}

/// `<home>\labd.log`.
pub fn labd_log_path() -> PathBuf {
    lab_home().join("labd.log")
}

/// `<home>\cli`, where the installed PowerShell CLI lives.
pub fn installed_cli_dir() -> PathBuf {
    lab_home().join("cli")
}

/// The first non-blank value of `key`: labd.env wins, then the process
/// environment. Values are trimmed. The rule of `Get-UatRoot` and
/// `Get-ProfileRoot`.
fn setting(key: &str) -> Option<String> {
    let lines = env_file::read_or_empty(&labd_env_path());
    let from_file = env_file::get(&lines, key).map(str::to_owned);
    let from_env = std::env::var(key).ok();
    [from_file, from_env]
        .into_iter()
        .flatten()
        .map(|value| value.trim().to_string())
        .find(|value| !value.is_empty())
}

/// The UAT folder for batches and runs: `CIMMERIA_LAB_UAT_DIR` (labd.env,
/// then the environment), else `<home>\uat-runs`. Port of `Get-UatRoot`.
pub fn uat_root() -> PathBuf {
    match setting("CIMMERIA_LAB_UAT_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => lab_home().join("uat-runs"),
    }
}

/// The game's profile root: `CIMMERIA_LAB_PROFILE_ROOT` (labd.env, then the
/// environment), else `%LOCALAPPDATA%\cimmeria-lab\instances` (not under
/// `CIMMERIA_LAB_HOME`, as in `Get-ProfileRoot`).
pub fn profile_root() -> PathBuf {
    match setting("CIMMERIA_LAB_PROFILE_ROOT") {
        Some(dir) => PathBuf::from(dir),
        None => local_app_data().join("cimmeria-lab").join("instances"),
    }
}

/// `docs/guides/uat-specs` of the checkout that `start` sits in or under
/// (`start` and each of its ancestors are tried, nearest first).
pub fn default_specs_dir_from(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|dir| dir.join("docs").join("guides").join("uat-specs"))
        .find(|candidate| candidate.is_dir())
}

/// The repo's spec folder when `lab.exe` runs from a checkout (its own
/// folder or the current directory sits inside one), else `None` and the
/// daemon uses its own default. Port of `Get-DefaultSpecsDir`.
pub fn default_specs_dir() -> Option<PathBuf> {
    let from_exe = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().and_then(default_specs_dir_from));
    from_exe.or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|cwd| default_specs_dir_from(&cwd))
    })
}

/// `dir` made absolute against `cwd`, as text. A path that is already rooted
/// (a drive letter or a UNC prefix with either slash, or a leading slash) is
/// returned as it is, on every platform; anything else is joined to `cwd`.
/// The folder need not exist. The daemon runs in another directory, so a
/// relative spec folder must be sent as the folder the user meant.
pub fn absolute_in(dir: &str, cwd: &Path) -> String {
    if dir.is_empty() || is_rooted(dir) {
        return dir.to_string();
    }
    cwd.join(dir).display().to_string()
}

/// [`absolute_in`] against this process's current directory (`dir` unchanged
/// when that cannot be read). Every `--specs-dir` goes through it before any
/// daemon call.
pub fn absolute_specs_dir(dir: &str) -> String {
    match std::env::current_dir() {
        Ok(cwd) => absolute_in(dir, &cwd),
        Err(_) => dir.to_string(),
    }
}

fn is_rooted(dir: &str) -> bool {
    let b = dir.as_bytes();
    let sep = |c: u8| c == b'\\' || c == b'/';
    Path::new(dir).is_absolute()
        || (b.len() >= 2 && sep(b[0]) && sep(b[1]))
        || (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && sep(b[2]))
}

/// What `labd.pid` says about the running daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonInfo {
    /// The daemon's process id, when the pidfile names one.
    pub pid: Option<u32>,
    /// `host:port` the daemon listens on; `127.0.0.1:8779` when unnamed.
    pub bind: String,
}

/// Reads `labd.pid`. A missing, unreadable or malformed file, or a file with
/// no `bind` or a `bind` that is not loopback (see [`loopback_bind`]), gives
/// `pid: None` and/or the default bind. Port of
/// `Get-DaemonInfo` plus the default-bind rule of `Get-LabStatus`.
pub fn daemon_info() -> DaemonInfo {
    let none = DaemonInfo {
        pid: None,
        bind: DEFAULT_BIND.to_string(),
    };
    let Ok(text) = std::fs::read_to_string(labd_pid_path()) else {
        return none;
    };
    let Ok(value) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) else {
        return none;
    };
    let pid = value
        .get("pid")
        .and_then(Value::as_u64)
        .and_then(|pid| u32::try_from(pid).ok());
    let bind = value
        .get("bind")
        .and_then(Value::as_str)
        .and_then(loopback_bind)
        .unwrap_or_else(|| DEFAULT_BIND.to_string());
    DaemonInfo { pid, bind }
}

/// `text` as a loopback `host:port`, or `None`. Accepted: an IP socket address
/// whose IP is loopback (`127.0.0.1:8779`, `[::1]:8779`) and `localhost:port`.
/// Everything else is refused: other IPs (`0.0.0.0` included), host names,
/// userinfo (`127.0.0.1:8779@evil.example`), paths and port 0. The bearer token
/// is sent to this address, and `labd.pid` is a file anyone with write access
/// to the lab home can change.
pub fn loopback_bind(text: &str) -> Option<String> {
    let text = text.trim();
    if let Ok(addr) = text.parse::<std::net::SocketAddr>() {
        return (addr.ip().is_loopback() && addr.port() != 0).then(|| addr.to_string());
    }
    let (host, port) = text.rsplit_once(':')?;
    let port: u16 = port.parse().ok().filter(|port| *port != 0)?;
    if !host.eq_ignore_ascii_case("localhost") {
        return None;
    }
    Some(format!("localhost:{port}"))
}
