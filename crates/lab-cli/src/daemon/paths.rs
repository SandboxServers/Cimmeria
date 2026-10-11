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

/// What `labd.pid` says about the running daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonInfo {
    /// The daemon's process id, when the pidfile names one.
    pub pid: Option<u32>,
    /// `host:port` the daemon listens on; `127.0.0.1:8779` when unnamed.
    pub bind: String,
}

/// Reads `labd.pid`. A missing, unreadable or malformed file, or a file with
/// no `bind`, gives `pid: None` and/or the default bind. Port of
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
        .map(str::trim)
        .filter(|bind| !bind.is_empty())
        .unwrap_or(DEFAULT_BIND)
        .to_string();
    DaemonInfo { pid, bind }
}
