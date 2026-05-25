//! `current-session.json` — per-session marker file written under
//! `<install_dir>/Binaries/sessions/`. Reserved for a future Lua-side
//! hook in the client; the launcher writes it at session start and
//! overwrites on the next session.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::state::{atomic_write, StateError};

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("State error: {0}")]
    State(#[from] StateError),
}

pub const CURRENT_SESSION_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurrentSession {
    pub schema_version: u32,
    pub install_id: String,
    pub machine_id: String,
    pub session_id: String,
    pub session_started_at_ms: i64,
    pub branch: String,
    pub git_sha: String,
    pub telemetry: TelemetryBlock,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelemetryBlock {
    pub enabled: bool,
    pub token: String,
    pub expires_at_ms: i64,
    pub upload_endpoint: String,
    pub chunk_max_bytes: u64,
    pub flush_interval_ms: u64,
}

pub fn current_session_path(install_dir: &Path) -> PathBuf {
    install_dir
        .join("Binaries")
        .join("sessions")
        .join("current-session.json")
}

pub fn write_current_session(
    install_dir: &Path,
    session: &CurrentSession,
) -> Result<PathBuf, SessionError> {
    let path = current_session_path(install_dir);
    let bytes = serde_json::to_vec_pretty(session)?;
    atomic_write(&path, &bytes)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> CurrentSession {
        CurrentSession {
            schema_version: CURRENT_SESSION_SCHEMA,
            install_id: "11111111-2222-3333-4444-555555555555".into(),
            machine_id: "abcdef0123456789".into(),
            session_id: "66666666-7777-8888-9999-aaaaaaaaaaaa".into(),
            session_started_at_ms: 1_700_000_000_000,
            branch: "main".into(),
            git_sha: "deadbeef".into(),
            telemetry: TelemetryBlock {
                enabled: true,
                token: "payload.sig".into(),
                expires_at_ms: 1_700_028_800_000,
                upload_endpoint: "https://x.example/api".into(),
                chunk_max_bytes: 1_048_576,
                flush_interval_ms: 2_000,
            },
            tags: vec!["smoke".into()],
        }
    }

    #[test]
    fn write_creates_nested_dirs_and_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let session = sample();
        let path = write_current_session(dir.path(), &session).unwrap();
        assert!(path.ends_with("Binaries/sessions/current-session.json"));
        let text = std::fs::read_to_string(&path).unwrap();
        let loaded: CurrentSession = serde_json::from_str(&text).unwrap();
        assert_eq!(loaded, session);
    }

    #[test]
    fn write_overwrites_previous_file() {
        let dir = tempfile::tempdir().unwrap();
        let s1 = sample();
        write_current_session(dir.path(), &s1).unwrap();
        let mut s2 = s1.clone();
        s2.session_id = "new-session".into();
        write_current_session(dir.path(), &s2).unwrap();
        let text = std::fs::read_to_string(current_session_path(dir.path())).unwrap();
        let loaded: CurrentSession = serde_json::from_str(&text).unwrap();
        assert_eq!(loaded.session_id, "new-session");
    }

    #[test]
    fn load_ignores_unknown_future_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = current_session_path(dir.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{
                "schema_version": 1,
                "install_id": "i",
                "machine_id": "m",
                "session_id": "s",
                "session_started_at_ms": 1,
                "branch": "b",
                "git_sha": "g",
                "telemetry": {
                    "enabled": true,
                    "token": "t",
                    "expires_at_ms": 2,
                    "upload_endpoint": "e",
                    "chunk_max_bytes": 3,
                    "flush_interval_ms": 4
                },
                "future_field_v2": "ignored"
            }"#,
        )
        .unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let loaded: CurrentSession = serde_json::from_str(&text).unwrap();
        assert_eq!(loaded.install_id, "i");
    }
}
