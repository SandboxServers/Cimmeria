//! End-of-session bundle POST.
//!
//! At game-exit time the launcher zips the full
//! `Binaries/sgwdebuglog*` + `Binaries/sessions/**` contents and POSTs
//! a multipart payload (JSON metadata + zip file) to
//! `<upload_endpoint>/upload-bundle`. Idempotent on `(session_id,
//! sha256(zip))` server-side.

use std::path::Path;

use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::logs::{build_log_zip, LogError};

#[derive(Debug, Error)]
pub enum BundleError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Log scan error: {0}")]
    Logs(#[from] LogError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Server returned {status}: {body}")]
    Status { status: u16, body: String },
    #[error("Token rejected (401)")]
    TokenRejected,
    #[error("Kill switch active (503) — retry after {retry_after_secs}s")]
    KillSwitch { retry_after_secs: u64 },
    #[error("No log content to bundle")]
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMetadata {
    pub session_id: String,
    pub install_id: String,
    pub machine_id: String,
    pub launcher_version: String,
    pub session_started_at_ms: i64,
    pub session_ended_at_ms: i64,
    pub event_count: u64,
    pub dropped_lines: u64,
    pub zip_sha256: String,
    pub zip_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct BundleOutcome {
    pub zip_sha256: String,
    pub zip_bytes: u64,
}

/// Build the bundle metadata + zip from `install_dir`'s session logs
/// and POST as multipart. Returns Empty if no logs are present (no
/// network call fires).
pub async fn upload_bundle(
    http: &reqwest::Client,
    upload_endpoint: &str,
    token: &str,
    install_dir: &Path,
    mut metadata: BundleMetadata,
) -> Result<BundleOutcome, BundleError> {
    let Some(zip) = build_log_zip(install_dir)? else {
        return Err(BundleError::Empty);
    };
    let mut hasher = Sha256::new();
    hasher.update(&zip);
    let digest = hex_lowercase(&hasher.finalize());
    metadata.zip_sha256.clone_from(&digest);
    metadata.zip_bytes = zip.len() as u64;

    let url = format!("{}/upload-bundle", upload_endpoint.trim_end_matches('/'));
    let metadata_part = Part::text(serde_json::to_string(&metadata)?)
        .mime_str("application/json")?;
    let zip_part = Part::bytes(zip.clone())
        .file_name("session.zip")
        .mime_str("application/zip")?;
    let form = Form::new()
        .part("metadata", metadata_part)
        .part("zip", zip_part);
    let resp = http
        .post(&url)
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await?;
    let status = resp.status();
    if status.is_success() {
        return Ok(BundleOutcome {
            zip_sha256: digest,
            zip_bytes: zip.len() as u64,
        });
    }
    if status.as_u16() == 401 {
        return Err(BundleError::TokenRejected);
    }
    if status.as_u16() == 503 {
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60);
        return Err(BundleError::KillSwitch {
            retry_after_secs: retry_after,
        });
    }
    let body = resp.text().await.unwrap_or_default();
    Err(BundleError::Status {
        status: status.as_u16(),
        body,
    })
}

fn hex_lowercase(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn meta() -> BundleMetadata {
        BundleMetadata {
            session_id: "s".into(),
            install_id: "i".into(),
            machine_id: "m".into(),
            launcher_version: "0.1.0".into(),
            session_started_at_ms: 1,
            session_ended_at_ms: 2,
            event_count: 0,
            dropped_lines: 0,
            zip_sha256: String::new(),
            zip_bytes: 0,
        }
    }

    fn seed_log(dir: &Path) {
        let bin = dir.join("Binaries");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("sgwdebuglog"), b"hello\nworld\n").unwrap();
    }

    // Empty install dir → Empty, no network call. The mock server has
    // no expectations so the test fails if we accidentally fire one.
    #[tokio::test]
    async fn upload_bundle_returns_empty_when_no_logs() {
        let dir = tempfile::tempdir().unwrap();
        let http = reqwest::Client::new();
        let err = upload_bundle(
            &http,
            "https://no.such.host.invalid./api",
            "t",
            dir.path(),
            meta(),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, BundleError::Empty));
    }

    #[tokio::test]
    async fn upload_bundle_posts_multipart_and_returns_outcome() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/upload-bundle"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        seed_log(dir.path());
        let http = reqwest::Client::new();
        let outcome = upload_bundle(
            &http,
            &format!("{}/api", server.uri()),
            "t",
            dir.path(),
            meta(),
        )
        .await
        .unwrap();
        assert_eq!(outcome.zip_sha256.len(), 64);
        assert!(outcome.zip_bytes > 0);
    }

    #[tokio::test]
    async fn upload_bundle_401_surfaces_token_rejected() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/upload-bundle"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        seed_log(dir.path());
        let http = reqwest::Client::new();
        let err = upload_bundle(
            &http,
            &format!("{}/api", server.uri()),
            "t",
            dir.path(),
            meta(),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, BundleError::TokenRejected));
    }

    #[tokio::test]
    async fn upload_bundle_503_surfaces_kill_switch_with_retry_after() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/upload-bundle"))
            .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "90"))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        seed_log(dir.path());
        let http = reqwest::Client::new();
        let err = upload_bundle(
            &http,
            &format!("{}/api", server.uri()),
            "t",
            dir.path(),
            meta(),
        )
        .await
        .unwrap_err();
        match err {
            BundleError::KillSwitch { retry_after_secs } => assert_eq!(retry_after_secs, 90),
            other => panic!("expected KillSwitch, got {other:?}"),
        }
    }
}
