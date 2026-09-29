//! Named lab instances: two `SGW.exe` clients side by side on one
//! workstation, each driven by its own `cimmeria-lab` supervisor.
//!
//! The lab was written for one client (`current-session.json`, bridge port
//! 8770, one `lab-account.json`, a guard that refuses to start while any
//! `SGW.exe` runs). An *instance* is a name (`p2`) that moves every one of
//! those single-client assumptions into a per-instance slot, so a second
//! `cimmeria-lab` process (a second MCP server entry in `.mcp.json`) can
//! launch and drive a second client without touching the first:
//!
//! | Single-client thing | Named instance |
//! |---|---|
//! | `sessions/current-session.json` | `sessions/instances/<name>/current-session.json`, found by the DLL through `CIMMERIA_LAB_SESSION_FILE` |
//! | bridge port 8770 | `CIMMERIA_LAB_BRIDGE_PORT` (one per instance) |
//! | crash marker and minidumps beside the session file | the same, in the instance directory |
//! | `sessions/lab-account.json` | `sessions/lab-account.<name>.json` (never the default account: a duplicate login kicks the first client) |
//! | `cimmeria-client-*.log` | `cimmeria-client-*-<name>.log` |
//! | "refuse while any SGW.exe runs" | refuse only for a client the lab does not own, or past the client cap |
//!
//! The default instance (no name) keeps the old layout exactly. Findings
//! and the reasoning: `docs/reverse-engineering/findings/multi-client-lab.md`.
//!
//! Peers find each other through a small registry file per instance
//! (`lab-instance.json`), written at launch and removed at stop, so the
//! start guard can tell "the other lab client" from "a player's client".

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::session_file::{current_session_path, lab_account_path, sessions_dir};

/// Names the instance. Unset or empty = the default instance. The DLLs read
/// the same variable (`crates/client-telemetry/src/session.rs`) to name
/// their log files.
pub const INSTANCE_ENV: &str = "CIMMERIA_LAB_INSTANCE";
/// Points the injected telemetry DLL at this launch's session file. Read by
/// the DLL (`crates/client-telemetry/src/session.rs`, `SESSION_FILE_ENV`).
pub const SESSION_FILE_ENV: &str = "CIMMERIA_LAB_SESSION_FILE";
/// How many `SGW.exe` clients may run at once (default and ceiling below).
pub const MAX_CLIENTS_ENV: &str = "CIMMERIA_LAB_MAX_CLIENTS";
/// Two clients: the second player of a two-player scenario.
pub const DEFAULT_MAX_CLIENTS: usize = 2;
/// Ceiling for `CIMMERIA_LAB_MAX_CLIENTS`. Each client is a 32-bit
/// process with a D3D9 device and its own Mercury session.
pub const CEILING_MAX_CLIENTS: usize = 4;

const REGISTRY_FILE: &str = "lab-instance.json";
/// Registry label of the unnamed instance.
const DEFAULT_LABEL: &str = "default";

/// Validate an instance name: 1 to 16 ASCII letters, digits, `-` or `_`.
/// The name becomes a directory and a file-name fragment, so nothing else
/// (separators, dots, spaces) is accepted.
pub fn validate_name(raw: &str) -> Result<String, String> {
    let ok = (1..=16).contains(&raw.len())
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if !ok {
        return Err(format!(
            "{INSTANCE_ENV} {raw:?} is not a valid instance name: use 1 to 16 letters, digits, '-' or '_'"
        ));
    }
    if raw.eq_ignore_ascii_case(DEFAULT_LABEL) {
        return Err(format!(
            "{INSTANCE_ENV} {raw:?} is reserved for the unnamed instance: leave it unset"
        ));
    }
    Ok(raw.to_string())
}

/// The instance named by [`INSTANCE_ENV`]: `Ok(None)` when unset or empty.
pub fn from_env() -> Result<Option<String>, String> {
    parse(std::env::var(INSTANCE_ENV).ok().as_deref())
}

/// [`from_env`] over an explicit value, for tests.
pub fn parse(raw: Option<&str>) -> Result<Option<String>, String> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(name) => validate_name(name).map(Some),
    }
}

/// The client cap from [`MAX_CLIENTS_ENV`], clamped to `1..=CEILING`.
pub fn max_clients_from_env() -> usize {
    clamp_max(std::env::var(MAX_CLIENTS_ENV).ok().as_deref())
}

/// [`max_clients_from_env`] over an explicit value, for tests.
pub fn clamp_max(raw: Option<&str>) -> usize {
    raw.and_then(|s| s.trim().parse::<usize>().ok())
        .map_or(DEFAULT_MAX_CLIENTS, |n| n.clamp(1, CEILING_MAX_CLIENTS))
}

/// Where this instance's session file, crash marker and minidumps live:
/// `sessions/` for the default instance (the old layout),
/// `sessions/instances/<name>/` for a named one.
pub fn instance_dir(install_dir: &Path, instance: Option<&str>) -> PathBuf {
    match instance {
        None => sessions_dir(install_dir),
        Some(name) => sessions_dir(install_dir).join("instances").join(name),
    }
}

/// `<instance dir>/current-session.json`.
pub fn session_path(install_dir: &Path, instance: Option<&str>) -> PathBuf {
    match instance {
        None => current_session_path(install_dir),
        Some(_) => instance_dir(install_dir, instance).join("current-session.json"),
    }
}

/// The credentials file: `lab-account.json` for the default instance,
/// `lab-account.<name>.json` for a named one. A named instance never falls
/// back to the default file: two clients logged into one account kick each
/// other (`duplicate_login`), which defeats a two-player scenario.
pub fn account_path(install_dir: &Path, instance: Option<&str>) -> PathBuf {
    match instance {
        None => lab_account_path(install_dir),
        Some(name) => sessions_dir(install_dir).join(format!("lab-account.{name}.json")),
    }
}

/// The environment the helper gives the game: nothing for the default
/// instance (the DLL reads the conventional path), the session file and
/// the instance name for a named one.
pub fn launch_env(install_dir: &Path, instance: Option<&str>) -> Vec<(String, String)> {
    match instance {
        None => Vec::new(),
        Some(name) => vec![
            (
                SESSION_FILE_ENV.to_string(),
                session_path(install_dir, Some(name))
                    .to_string_lossy()
                    .into_owned(),
            ),
            (INSTANCE_ENV.to_string(), name.to_string()),
        ],
    }
}

/// One launched client, as recorded for its peers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerEntry {
    pub instance: String,
    pub pid: u32,
    pub bridge_port: u16,
}

fn registry_path(install_dir: &Path, label: &str) -> PathBuf {
    sessions_dir(install_dir)
        .join("instances")
        .join(label)
        .join(REGISTRY_FILE)
}

fn label(instance: Option<&str>) -> &str {
    instance.unwrap_or(DEFAULT_LABEL)
}

/// Record this instance's client for its peers.
pub fn write_entry(
    install_dir: &Path,
    instance: Option<&str>,
    pid: u32,
    bridge_port: u16,
) -> Result<(), String> {
    let label = label(instance);
    let path = registry_path(install_dir, label);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    }
    let entry = PeerEntry {
        instance: label.to_string(),
        pid,
        bridge_port,
    };
    let bytes = serde_json::to_vec_pretty(&entry).map_err(|e| format!("serialize entry: {e}"))?;
    std::fs::write(&path, bytes).map_err(|e| format!("write {}: {e}", path.display()))
}

/// Forget this instance's client (stop). Best effort.
pub fn remove_entry(install_dir: &Path, instance: Option<&str>) {
    let _ = std::fs::remove_file(registry_path(install_dir, label(instance)));
}

/// Every other instance's recorded client. The caller filters out the ones
/// whose process is gone.
pub fn read_peers(install_dir: &Path, instance: Option<&str>) -> Vec<PeerEntry> {
    let own = label(instance);
    let Ok(dir) = std::fs::read_dir(sessions_dir(install_dir).join("instances")) else {
        return Vec::new();
    };
    dir.filter_map(Result::ok)
        .filter_map(|e| std::fs::read(e.path().join(REGISTRY_FILE)).ok())
        .filter_map(|bytes| serde_json::from_slice::<PeerEntry>(&bytes).ok())
        .filter(|p| p.instance != own)
        .collect()
}

/// Whether a new client may start. `running` is every `SGW.exe` with a
/// window, `peers` the other instances' clients that are still alive, and
/// `max` the client cap. Refuses when
///
/// - a peer already holds this instance's bridge port (a config error that
///   would otherwise surface as the second DLL failing to bind, or worse,
///   as this supervisor talking to the peer's bridge),
/// - an `SGW.exe` runs that is not a peer's (a player's client or the
///   launcher's: not the lab's to share a machine with), or
/// - the peers plus other clients already fill the cap.
///
/// With no peers and no cap pressure this is the old rule: any running
/// `SGW.exe` blocks the start.
pub fn check_launch(
    running: &[u32],
    peers: &[PeerEntry],
    own_port: u16,
    max: usize,
) -> Result<(), String> {
    if let Some(p) = peers.iter().find(|p| p.bridge_port == own_port) {
        return Err(format!(
            "instance {:?} already uses bridge port {own_port}; give this instance its own \
             CIMMERIA_LAB_BRIDGE_PORT (and CIMMERIA_LAB_BRIDGE)",
            p.instance
        ));
    }
    let outside: Vec<u32> = running
        .iter()
        .copied()
        .filter(|pid| !peers.iter().any(|p| p.pid == *pid))
        .collect();
    if !outside.is_empty() {
        return Err(format!(
            "SGW.exe is already running (pid {outside:?}) outside the lab; close it before \
             starting a lab client"
        ));
    }
    if peers.len() >= max {
        return Err(format!(
            "{} lab client(s) already running and the cap is {max} \
             ({MAX_CLIENTS_ENV}, ceiling {CEILING_MAX_CLIENTS}); stop one first",
            peers.len()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(instance: &str, pid: u32, port: u16) -> PeerEntry {
        PeerEntry {
            instance: instance.into(),
            pid,
            bridge_port: port,
        }
    }

    #[test]
    fn unset_or_blank_is_the_default_instance() {
        assert_eq!(parse(None), Ok(None));
        assert_eq!(parse(Some("")), Ok(None));
        assert_eq!(parse(Some("  ")), Ok(None));
        assert_eq!(parse(Some(" p2 ")), Ok(Some("p2".into())));
    }

    #[test]
    fn names_that_could_escape_the_instances_dir_are_refused() {
        for bad in [
            "..",
            "a/b",
            "a\\b",
            "p 2",
            "p.2",
            "seventeen-chars-xx",
            "DEFAULT",
        ] {
            assert!(parse(Some(bad)).is_err(), "{bad:?} must be refused");
        }
        assert!(parse(Some("p2")).is_ok());
        assert!(parse(Some("Player_2-b")).is_ok());
    }

    #[test]
    fn default_instance_keeps_the_single_client_layout() {
        let install = Path::new("C:/Games/SGW");
        assert!(session_path(install, None).ends_with("Binaries/sessions/current-session.json"));
        assert!(account_path(install, None).ends_with("Binaries/sessions/lab-account.json"));
        assert!(launch_env(install, None).is_empty());
    }

    #[test]
    fn named_instance_gets_its_own_session_account_and_env() {
        let install = Path::new("C:/Games/SGW");
        let session = session_path(install, Some("p2"));
        assert!(session.ends_with("Binaries/sessions/instances/p2/current-session.json"));
        assert!(
            account_path(install, Some("p2")).ends_with("Binaries/sessions/lab-account.p2.json")
        );
        let env = launch_env(install, Some("p2"));
        assert_eq!(
            env,
            vec![
                (
                    SESSION_FILE_ENV.to_string(),
                    session.to_string_lossy().into_owned()
                ),
                (INSTANCE_ENV.to_string(), "p2".to_string()),
            ]
        );
    }

    #[test]
    fn env_names_are_the_contract_with_the_dll() {
        assert_eq!(INSTANCE_ENV, "CIMMERIA_LAB_INSTANCE");
        assert_eq!(SESSION_FILE_ENV, "CIMMERIA_LAB_SESSION_FILE");
    }

    #[test]
    fn cap_defaults_to_two_and_is_clamped() {
        assert_eq!(clamp_max(None), 2);
        assert_eq!(clamp_max(Some("junk")), 2);
        assert_eq!(clamp_max(Some("0")), 1);
        assert_eq!(clamp_max(Some("3")), 3);
        assert_eq!(clamp_max(Some("99")), CEILING_MAX_CLIENTS);
    }

    #[test]
    fn nothing_running_starts() {
        assert!(check_launch(&[], &[], 8770, 2).is_ok());
    }

    #[test]
    fn a_client_outside_the_lab_still_blocks_the_start() {
        // The old rule: a player's or the launcher's client is not the
        // lab's to share a machine with.
        let err = check_launch(&[4242], &[], 8770, 2).unwrap_err();
        assert!(err.contains("outside the lab"), "{err}");
    }

    #[test]
    fn a_peers_client_does_not_block_the_second_start() {
        let peers = [peer("default", 1000, 8770)];
        assert!(check_launch(&[1000], &peers, 8771, 2).is_ok());
    }

    #[test]
    fn a_stranger_next_to_a_peer_still_blocks() {
        let peers = [peer("default", 1000, 8770)];
        let err = check_launch(&[1000, 4242], &peers, 8771, 3).unwrap_err();
        assert!(
            err.contains("4242") && err.contains("outside the lab"),
            "{err}"
        );
    }

    #[test]
    fn a_third_client_is_refused_at_the_default_cap() {
        let peers = [peer("default", 1000, 8770), peer("p2", 2000, 8771)];
        let err = check_launch(&[1000, 2000], &peers, 8772, 2).unwrap_err();
        assert!(err.contains("cap is 2"), "{err}");
    }

    #[test]
    fn a_peer_still_launching_counts_toward_the_cap() {
        // It has no window yet, so `running` misses it; the registry does not.
        let peers = [peer("default", 1000, 8770)];
        let err = check_launch(&[], &peers, 8771, 1).unwrap_err();
        assert!(err.contains("cap is 1"), "{err}");
    }

    #[test]
    fn two_instances_on_one_bridge_port_are_refused() {
        let peers = [peer("default", 1000, 8770)];
        let err = check_launch(&[1000], &peers, 8770, 2).unwrap_err();
        assert!(err.contains("bridge port 8770"), "{err}");
    }

    #[test]
    fn registry_round_trips_and_a_peer_never_sees_itself() {
        let dir = tempfile::tempdir().unwrap();
        write_entry(dir.path(), None, 1000, 8770).unwrap();
        write_entry(dir.path(), Some("p2"), 2000, 8771).unwrap();

        let seen_by_default = read_peers(dir.path(), None);
        assert_eq!(seen_by_default, vec![peer("p2", 2000, 8771)]);
        let seen_by_p2 = read_peers(dir.path(), Some("p2"));
        assert_eq!(seen_by_p2, vec![peer("default", 1000, 8770)]);

        remove_entry(dir.path(), Some("p2"));
        assert!(read_peers(dir.path(), None).is_empty());
    }

    #[test]
    fn a_corrupt_registry_file_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let bad = registry_path(dir.path(), "junk");
        std::fs::create_dir_all(bad.parent().unwrap()).unwrap();
        std::fs::write(&bad, b"not json").unwrap();
        assert!(read_peers(dir.path(), None).is_empty());
    }
}
