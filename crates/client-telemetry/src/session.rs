//! Read `current-session.json` — the launcher's marker file that
//! carries the HMAC token, session id, and upload endpoint the DLL
//! needs to ship events to the server.
//!
//! The launcher writes this file to
//! `<install_dir>/Binaries/sessions/current-session.json` at session
//! start (see `crates/launcher/src/telemetry/session.rs`). The DLL
//! reads it during bootstrap, holds onto the relevant fields for
//! the uploader thread's lifetime, and never re-reads the file in
//! Phase 2 — the uploader runs until process exit. Token expiry
//! and re-read on 401 land in Phase 3 (see
//! `crate::uploader` failure-mode docs).
//!
//! # How the DLL finds the file
//!
//! - Get SGW.exe's own path via `GetModuleFileNameW(NULL, ...)` —
//!   SGW.exe lives in `<install_dir>/Binaries/`.
//! - Parent of that is `<install_dir>/Binaries/`.
//! - Append `sessions/current-session.json`.
//!
//! This avoids needing the launcher to pass the path in via an
//! environment variable or named pipe — the install layout is
//! conventional and the DLL is loaded into a known executable.
//!
//! # Schema discipline
//!
//! The launcher's [`crate::session::TelemetryBlock`] has more fields
//! than the DLL strictly needs (chunk_max_bytes, flush_interval_ms,
//! enabled, expires_at_ms). We deserialize the full block so a future
//! field addition on the launcher side doesn't break the parse — but
//! the DLL only reads `token`, `upload_endpoint`, and `enabled`
//! today. The other fields are reserved for Phase 3+ when token
//! rotation and adaptive flush-cadence land.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("could not derive install_dir from host executable path: {0}")]
    InstallDir(String),
    #[error("could not read current-session.json at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not parse current-session.json at {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("telemetry disabled in current-session.json (telemetry.enabled = false)")]
    Disabled,
}

/// Subset of the launcher's `CurrentSession` that the DLL needs.
/// Unknown fields are ignored by serde so the launcher can add
/// fields without breaking us.
///
/// `install_id`, `machine_id`, `session_id` are the per-session
/// identity that gets stamped onto every event the DLL sends — the
/// server's HMAC verifier checks the token but doesn't directly
/// stamp these onto events, so the DLL has to forward them itself
/// via the `fields` bag of every emit (or, more efficiently, stamp
/// once at session start via a "dll.attached" event and let the
/// server-side join on `sid` from the token claims).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DllSession {
    pub install_id: String,
    pub machine_id: String,
    pub session_id: String,
    pub telemetry: TelemetryBlock,
    /// Optional Live Research Lab bridge config. Written **only** by
    /// the lab supervisor (`cimmeria-lab`), never by a normal
    /// telemetry launch. Its presence is the second half of the
    /// bridge's double activation gate: even a DLL compiled with
    /// `--features lab-bridge` will not open the inbound command
    /// channel unless this block is present. See
    /// [`crate::bridge`] and `docs/architecture/live-research-lab.md`
    /// §3.3.
    ///
    /// Deserialized unconditionally (so the schema is stable and
    /// testable regardless of feature flags); only consumed under
    /// the `lab-bridge` feature.
    #[serde(default)]
    pub lab: Option<LabConfig>,
}

/// The `lab` block of `current-session.json` — bind address, port,
/// and per-launch token for the client bridge's inbound TCP channel.
///
/// Kept here (not under the feature) so the session schema is one
/// stable shape and the "absent block = bridge never starts" guard
/// is unit-testable without the feature compiled in.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct LabConfig {
    /// Interface to bind. Default loopback; overridable for a second
    /// PC on the LAN or VPN (per the ADR, the address is a knob but
    /// the token is mandatory either way).
    #[serde(default = "default_lab_bind")]
    pub bind: String,
    /// TCP port. Default 8770 — 8765 is claimed by both the SigNoz
    /// MCP and the Atrea editor bridge ADR.
    #[serde(default = "default_lab_port")]
    pub port: u16,
    /// 32-byte token as 64 lowercase hex chars, regenerated per
    /// launch by the supervisor. Required on the first framed
    /// message from any client.
    pub token: String,
}

fn default_lab_bind() -> String {
    "127.0.0.1".to_string()
}

fn default_lab_port() -> u16 {
    8770
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TelemetryBlock {
    pub enabled: bool,
    pub token: String,
    pub upload_endpoint: String,
    /// Reserved for Phase 3 — when the DLL rotates the token at
    /// 75% TTL elapsed (mirrors launcher policy). Today: read but
    /// not acted on.
    #[serde(default)]
    pub expires_at_ms: i64,
    /// Reserved — see fn doc.
    #[serde(default)]
    pub chunk_max_bytes: u64,
    /// Reserved — see fn doc.
    #[serde(default)]
    pub flush_interval_ms: u64,
}

/// Locate `current-session.json` relative to `host_exe_path`.
///
/// `host_exe_path` is SGW.exe's full path from
/// `GetModuleFileNameW(NULL, ...)`. The install layout is:
///
/// ```text
/// <install_dir>/
///   Binaries/
///     SGW.exe                     <- host_exe_path
///     sessions/
///       current-session.json      <- what we want
/// ```
///
/// Pure function — testable without any filesystem access.
pub fn session_path_for_host(host_exe_path: &Path) -> Result<PathBuf, SessionError> {
    let binaries = host_exe_path
        .parent()
        .ok_or_else(|| SessionError::InstallDir(host_exe_path.display().to_string()))?;
    Ok(binaries.join("sessions").join("current-session.json"))
}

/// Load + parse `current-session.json` from disk.
///
/// Returns `Err(SessionError::Disabled)` when the parsed block has
/// `telemetry.enabled = false` — the DLL's bootstrap thread reads
/// this as "park, do nothing else." Anything else (IO, parse) is a
/// hard error worth surfacing to the bootstrap thread's outer
/// `catch_unwind`.
pub fn load_session(path: &Path) -> Result<DllSession, SessionError> {
    let text = fs::read_to_string(path).map_err(|source| SessionError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let session: DllSession =
        serde_json::from_str(&text).map_err(|source| SessionError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
    if !session.telemetry.enabled {
        return Err(SessionError::Disabled);
    }
    Ok(session)
}

/// Convenience: the identity bag every DLL event should carry as
/// `fields` so server-side queries can pivot on session_id /
/// install_id without parsing the JWT-style HMAC token. Kept as a
/// helper so the DLL doesn't duplicate this construction at every
/// emit site.
pub fn identity_fields(session: &DllSession) -> HashMap<String, serde_json::Value> {
    let mut h = HashMap::with_capacity(3);
    h.insert(
        "install_id".to_string(),
        serde_json::Value::String(session.install_id.clone()),
    );
    h.insert(
        "machine_id".to_string(),
        serde_json::Value::String(session.machine_id.clone()),
    );
    h.insert(
        "session_id".to_string(),
        serde_json::Value::String(session.session_id.clone()),
    );
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Standard install layout: SGW.exe under Binaries/, sessions/
    /// alongside.
    #[test]
    fn session_path_for_standard_install() {
        let host = PathBuf::from("/opt/SGW/Binaries/SGW.exe");
        let p = session_path_for_host(&host).unwrap();
        assert_eq!(
            p,
            PathBuf::from("/opt/SGW/Binaries/sessions/current-session.json")
        );
    }

    /// Empty path → parent() returns None → InstallDir error.
    /// Defensive: this can only happen if GetModuleFileNameW
    /// returned a zero-length string, which Windows shouldn't but
    /// the bootstrap thread shouldn't panic on it either.
    #[test]
    fn session_path_no_parent_errors() {
        let host = PathBuf::from("");
        let err = session_path_for_host(&host).unwrap_err();
        match err {
            SessionError::InstallDir(_) => {}
            other => panic!("expected InstallDir, got {other:?}"),
        }
    }

    /// Happy-path load: write a launcher-shaped JSON, parse it back.
    /// Mirrors the launcher's `write_current_session` output.
    #[test]
    fn load_session_happy_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("current-session.json");
        fs::write(
            &path,
            r#"{
                "schema_version": 1,
                "install_id": "i-1",
                "machine_id": "m-1",
                "session_id": "s-1",
                "session_started_at_ms": 100,
                "branch": "main",
                "git_sha": "deadbeef",
                "telemetry": {
                    "enabled": true,
                    "token": "payload.sig",
                    "expires_at_ms": 200,
                    "upload_endpoint": "https://x.example/api/telemetry/upload-chunk",
                    "chunk_max_bytes": 1048576,
                    "flush_interval_ms": 2000
                },
                "tags": []
            }"#,
        )
        .unwrap();
        let s = load_session(&path).unwrap();
        assert_eq!(s.install_id, "i-1");
        assert_eq!(s.machine_id, "m-1");
        assert_eq!(s.session_id, "s-1");
        assert!(s.telemetry.enabled);
        assert_eq!(s.telemetry.token, "payload.sig");
        assert_eq!(
            s.telemetry.upload_endpoint,
            "https://x.example/api/telemetry/upload-chunk"
        );
    }

    /// `telemetry.enabled = false` → `SessionError::Disabled`. The
    /// bootstrap thread treats this as "park, do nothing" — same
    /// as the launcher's kill-switch path.
    #[test]
    fn load_session_disabled_telemetry_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("current-session.json");
        fs::write(
            &path,
            r#"{
                "schema_version": 1,
                "install_id": "i",
                "machine_id": "m",
                "session_id": "s",
                "session_started_at_ms": 0,
                "branch": "b",
                "git_sha": "g",
                "telemetry": {
                    "enabled": false,
                    "token": "",
                    "expires_at_ms": 0,
                    "upload_endpoint": "",
                    "chunk_max_bytes": 0,
                    "flush_interval_ms": 0
                }
            }"#,
        )
        .unwrap();
        let err = load_session(&path).unwrap_err();
        match err {
            SessionError::Disabled => {}
            other => panic!("expected Disabled, got {other:?}"),
        }
    }

    /// Missing file → IO error with the path attached so the
    /// bootstrap-thread log line carries it.
    #[test]
    fn load_session_missing_file_errors_with_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nope.json");
        let err = load_session(&path).unwrap_err();
        match err {
            SessionError::Io { path: p, .. } => assert_eq!(p, path),
            other => panic!("expected Io, got {other:?}"),
        }
    }

    /// No `lab` block → `lab` deserializes to `None`. This is the
    /// schema half of the bridge's double gate: a normal telemetry
    /// launch (which never writes a `lab` block) leaves the field
    /// absent, and `bridge::maybe_start` reads that as "do not start."
    #[test]
    fn load_session_without_lab_block_leaves_lab_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("current-session.json");
        fs::write(
            &path,
            r#"{
                "install_id": "i", "machine_id": "m", "session_id": "s",
                "telemetry": {
                    "enabled": true, "token": "t",
                    "upload_endpoint": "https://x/api", "expires_at_ms": 0,
                    "chunk_max_bytes": 0, "flush_interval_ms": 0
                }
            }"#,
        )
        .unwrap();
        let s = load_session(&path).unwrap();
        assert_eq!(s.lab, None, "absent lab block must parse to None");
    }

    /// A `lab` block with only `token` fills `bind`/`port` from the
    /// serde defaults (loopback / 8770).
    #[test]
    fn load_session_lab_block_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("current-session.json");
        fs::write(
            &path,
            r#"{
                "install_id": "i", "machine_id": "m", "session_id": "s",
                "telemetry": {
                    "enabled": true, "token": "t",
                    "upload_endpoint": "https://x/api", "expires_at_ms": 0,
                    "chunk_max_bytes": 0, "flush_interval_ms": 0
                },
                "lab": { "token": "abc123" }
            }"#,
        )
        .unwrap();
        let s = load_session(&path).unwrap();
        let lab = s.lab.expect("lab block present");
        assert_eq!(lab.bind, "127.0.0.1");
        assert_eq!(lab.port, 8770);
        assert_eq!(lab.token, "abc123");
    }

    /// Identity fields helper produces the canonical 3-entry bag
    /// that every DLL event should carry.
    #[test]
    fn identity_fields_carries_three_keys() {
        let s = DllSession {
            install_id: "i-X".into(),
            machine_id: "m-X".into(),
            session_id: "s-X".into(),
            telemetry: TelemetryBlock {
                enabled: true,
                token: "t".into(),
                expires_at_ms: 0,
                upload_endpoint: "u".into(),
                chunk_max_bytes: 0,
                flush_interval_ms: 0,
            },
            lab: None,
        };
        let f = identity_fields(&s);
        assert_eq!(f.len(), 3);
        assert_eq!(f["install_id"], serde_json::json!("i-X"));
        assert_eq!(f["machine_id"], serde_json::json!("m-X"));
        assert_eq!(f["session_id"], serde_json::json!("s-X"));
    }
}
