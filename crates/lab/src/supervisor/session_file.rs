//! Writing `current-session.json` with a `lab` block, and reading the
//! gitignored `lab-account.json` credentials.
//!
//! The supervisor owns the session file for a lab launch (ADR §3.4): it
//! mints a fresh 32-byte bridge token per launch and writes the `lab`
//! block that is the run-time half of the bridge's double activation
//! gate. The `telemetry` block carries a real dev-session token from the
//! server ([`super::telemetry_session`]), which is a different token: the
//! bridge token never leaves the machine. The shape mirrors the
//! launcher's `CurrentSession` plus the `lab` block the DLL's `LabConfig`
//! reads (`crates/client-telemetry/src/session.rs`).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::telemetry_session::{TelemetryGrant, LAB_INSTALL_ID};

/// Bridge default port (mirrors the DLL's `default_lab_port`). 8770 —
/// 8765 is claimed by the SigNoz MCP and the Atrea editor bridge.
pub const DEFAULT_BRIDGE_PORT: u16 = 8770;
/// Bridge default bind (loopback).
pub const DEFAULT_BRIDGE_BIND: &str = "127.0.0.1";

/// The `lab` block written into `current-session.json`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LabBlock {
    pub bind: String,
    pub port: u16,
    pub token: String,
}

/// The `telemetry` block. `enabled` **must** be true or the DLL's boot
/// thread parks before it ever reaches the bridge-start step, so a lab
/// session always writes `enabled: true`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TelemetryBlock {
    pub enabled: bool,
    pub token: String,
    pub upload_endpoint: String,
    pub expires_at_ms: i64,
    pub chunk_max_bytes: u64,
    pub flush_interval_ms: u64,
}

/// `current-session.json` as the supervisor writes it. A superset of
/// what the DLL needs; unknown fields are ignored on the DLL side.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CurrentSession {
    pub schema_version: u32,
    pub install_id: String,
    pub machine_id: String,
    pub session_id: String,
    pub session_started_at_ms: i64,
    pub branch: String,
    pub git_sha: String,
    pub telemetry: TelemetryBlock,
    pub lab: LabBlock,
    pub tags: Vec<String>,
}

/// Lab account credentials, read from `lab-account.json`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct LabAccount {
    /// Server row to pick at server select (`lab_login` falls back to the
    /// preselected row when this name is not in the list).
    #[serde(default)]
    pub server: String,
    pub username: String,
    pub password: String,
    /// The lab character: always protected by `lab_ensure_character_slot`
    /// and played after a crash relaunch.
    #[serde(default)]
    pub character: String,
}

/// `<install>/Binaries/sessions/`.
pub fn sessions_dir(install_dir: &Path) -> PathBuf {
    install_dir.join("Binaries").join("sessions")
}

/// `<install>/Binaries/sessions/current-session.json`.
pub fn current_session_path(install_dir: &Path) -> PathBuf {
    sessions_dir(install_dir).join("current-session.json")
}

/// `<install>/Binaries/sessions/lab-account.json`.
pub fn lab_account_path(install_dir: &Path) -> PathBuf {
    sessions_dir(install_dir).join("lab-account.json")
}

/// Generate a fresh 32-byte lab token as 64 lowercase hex chars. Built
/// from two v4 UUIDs (2 × 16 random bytes) so we reuse the workspace's
/// existing `uuid` dependency rather than adding an RNG crate. The
/// output satisfies the bridge's `valid_token` (64 hex chars).
pub fn generate_token() -> String {
    let a = uuid::Uuid::new_v4().simple().to_string();
    let b = uuid::Uuid::new_v4().simple().to_string();
    format!("{a}{b}")
}

/// Read + parse `lab-account.json`.
pub fn read_lab_account(install_dir: &Path) -> Result<LabAccount, String> {
    let path = lab_account_path(install_dir);
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
}

/// Build the session object for a lab launch. `bridge_token` goes in the
/// `lab` block only; the `telemetry` block is the server's grant.
pub fn build_session(
    bridge_token: &str,
    bind: &str,
    port: u16,
    telemetry: &TelemetryGrant,
) -> CurrentSession {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    CurrentSession {
        schema_version: 1,
        install_id: LAB_INSTALL_ID.to_string(),
        machine_id: LAB_INSTALL_ID.to_string(),
        // The server-minted session id, so the file and the SigNoz rows
        // (`session_id`) name the same run.
        session_id: telemetry.session_id.clone(),
        session_started_at_ms: now_ms,
        branch: "lab".to_string(),
        git_sha: "lab".to_string(),
        telemetry: TelemetryBlock {
            // Must be true or the DLL parks before the bridge starts, even
            // when the mint failed and the token is empty.
            enabled: true,
            token: telemetry.token.clone(),
            upload_endpoint: telemetry.upload_endpoint.clone(),
            expires_at_ms: telemetry.expires_at_ms,
            chunk_max_bytes: telemetry.chunk_max_bytes,
            flush_interval_ms: telemetry.flush_interval_ms,
        },
        lab: LabBlock {
            bind: bind.to_string(),
            port,
            token: bridge_token.to_string(),
        },
        tags: vec!["lab".to_string()],
    }
}

/// Write `current-session.json` for a lab launch. Creates the sessions
/// directory if missing. Returns the path written.
pub fn write_session(install_dir: &Path, session: &CurrentSession) -> Result<PathBuf, String> {
    let dir = sessions_dir(install_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let path = current_session_path(install_dir);
    let bytes =
        serde_json::to_vec_pretty(session).map_err(|e| format!("serialize session: {e}"))?;
    std::fs::write(&path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::super::telemetry_session::{MintResponse, TelemetryConfig};
    use super::*;

    /// A grant as the server's mint returns it.
    fn minted() -> TelemetryGrant {
        TelemetryGrant::minted(
            MintResponse {
                session_id: "sid-from-server".into(),
                token: "payload.sig".into(),
                expires_at_ms: 1_700_000_000_000,
                upload_endpoint: "http://127.0.0.1:8443/api/telemetry".into(),
                chunk_max_bytes: 1 << 20,
                flush_interval_ms: 2000,
            },
            &TelemetryConfig::default(),
        )
    }

    #[test]
    fn token_is_64_lowercase_hex() {
        let t = generate_token();
        assert_eq!(t.len(), 64, "32 bytes = 64 hex chars");
        assert!(t.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(t.chars().all(|c| !c.is_ascii_uppercase()));
        // Fresh each call.
        assert_ne!(generate_token(), generate_token());
    }

    #[test]
    fn session_paths_follow_install_layout() {
        let install = Path::new("C:/Games/SGW");
        assert!(current_session_path(install).ends_with("Binaries/sessions/current-session.json"));
        assert!(lab_account_path(install).ends_with("Binaries/sessions/lab-account.json"));
    }

    /// The written session carries enabled telemetry and the lab block
    /// with the fresh token — the two halves the DLL needs to start the
    /// bridge. Round-trips through JSON to the DLL's expected shape.
    #[test]
    fn built_session_has_enabled_telemetry_and_lab_token() {
        let s = build_session(
            &"a".repeat(64),
            DEFAULT_BRIDGE_BIND,
            DEFAULT_BRIDGE_PORT,
            &minted(),
        );
        assert!(s.telemetry.enabled, "must be true or the DLL parks");
        assert_eq!(s.lab.token, "a".repeat(64));
        assert_eq!(s.lab.port, 8770);

        let json = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["lab"]["token"], "a".repeat(64));
        assert_eq!(v["telemetry"]["enabled"], true);
    }

    /// The telemetry block is the server's grant: its token, its session
    /// id and its upload endpoint, not the placeholder endpoint and random
    /// token no server accepted. The bridge token stays in the `lab` block
    /// only, so it never travels to the server in an upload header.
    #[test]
    fn built_session_writes_the_minted_token_and_endpoint() {
        let bridge = "b".repeat(64);
        let s = build_session(&bridge, DEFAULT_BRIDGE_BIND, DEFAULT_BRIDGE_PORT, &minted());
        assert_eq!(s.telemetry.token, "payload.sig");
        assert_eq!(
            s.telemetry.upload_endpoint,
            "http://127.0.0.1:8443/api/telemetry"
        );
        assert_eq!(s.telemetry.expires_at_ms, 1_700_000_000_000);
        assert_eq!(s.session_id, "sid-from-server");
        assert_ne!(
            s.telemetry.token, bridge,
            "bridge token must not be the upload token"
        );
        assert_eq!(s.lab.token, bridge);
        assert_eq!(s.install_id, "cimmeria-lab");
    }

    /// A failed mint still writes `enabled: true` (or the DLL parks and
    /// the bridge never starts), with an empty token the server refuses.
    #[test]
    fn a_failed_mint_keeps_the_bridge_reachable() {
        let grant = TelemetryGrant::unavailable("down".into(), &TelemetryConfig::default());
        let s = build_session(
            &"c".repeat(64),
            DEFAULT_BRIDGE_BIND,
            DEFAULT_BRIDGE_PORT,
            &grant,
        );
        assert!(s.telemetry.enabled);
        assert!(s.telemetry.token.is_empty());
        assert_eq!(s.lab.token, "c".repeat(64));
        assert_eq!(
            s.telemetry.upload_endpoint,
            "http://127.0.0.1:8443/api/telemetry"
        );
    }

    #[test]
    fn write_and_reread_lab_account_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let install = dir.path();
        std::fs::create_dir_all(sessions_dir(install)).unwrap();
        std::fs::write(
            lab_account_path(install),
            r#"{"server":"Cimmeria","username":"lab","password":"pw","character":"LabRat"}"#,
        )
        .unwrap();
        let acct = read_lab_account(install).unwrap();
        assert_eq!(acct.server, "Cimmeria");
        assert_eq!(acct.character, "LabRat");
        assert_eq!(acct.username, "lab");
    }

    #[test]
    fn write_session_creates_dir_and_file() {
        let dir = tempfile::tempdir().unwrap();
        let install = dir.path();
        let s = build_session(
            &"b".repeat(64),
            DEFAULT_BRIDGE_BIND,
            DEFAULT_BRIDGE_PORT,
            &minted(),
        );
        let path = write_session(install, &s).unwrap();
        assert!(path.exists());
        let back: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(back["lab"]["port"], 8770);
    }
}
