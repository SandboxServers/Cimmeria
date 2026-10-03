//! A real dev-session telemetry token for each lab launch, so the lab
//! client's DLL events reach SigNoz (`service.name = cimmeria-client`,
//! tagged `cimmeria.session_kind = lab`).
//!
//! Before this, the supervisor wrote a random token and a placeholder
//! endpoint into `current-session.json`; no server accepted either, so
//! every lab client event stayed on the machine. Now each launch asks the
//! server's `/api/auth/dev-session`, the same mint the launcher uses, with
//! `session_kind = "lab"`, and writes the token, session id and upload
//! endpoint it gets back.
//!
//! The telemetry token is **not** the bridge token. The bridge's per-launch
//! token (the `lab` block) is still generated locally and never leaves the
//! machine; the double activation gate is unchanged.
//!
//! The mint goes to the login server the client will log into, read from
//! the install's `LoginInternal.lua` ([`super::login_servers`]): its login
//! port serves `/api/auth/dev-session` too, exactly as for the launcher.
//! The old default, a local admin API on `127.0.0.1:8443`, minted nothing
//! for a lab client playing on the colo, and the launch went ahead with
//! telemetry silently off (2026-10-03).
//!
//! A failed mint **stops the launch** with the reason, because a lab
//! session must reach SigNoz. `CIMMERIA_LAB_TELEMETRY=optional` lets it go
//! ahead without uploads for offline work: the session file then carries
//! an empty token (`telemetry.enabled` stays true, or the DLL would park
//! before starting the bridge), the uploads are refused, and
//! `lab_client_start` reports why.
//!
//! # Environment
//!
//! | Variable | Default | Meaning |
//! |---|---|---|
//! | `CIMMERIA_LAB_SERVER_URL` | unset: the login URL of the client's server row in `LoginInternal.lua` | The server to mint from: a login port or an admin API. A trailing `/api` is accepted. |
//! | `CIMMERIA_LAB_UPLOAD_ENDPOINT` | unset | Overrides the upload endpoint the server returns (e.g. when the lab reaches the server by another address than the one it advertises). |
//! | `CIMMERIA_LAB_TELEMETRY` | `required` | `optional` launches without uploads when the mint fails. |

use std::time::Duration;

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The `install_id` every lab session mints under. The server's per-install
/// mint quota then counts lab relaunches on their own.
pub const LAB_INSTALL_ID: &str = "cimmeria-lab";

/// How long a mint may take before the launch goes ahead without it.
const MINT_TIMEOUT: Duration = Duration::from_secs(5);

/// Where the supervisor mints telemetry tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryConfig {
    /// Server base to mint from (`CIMMERIA_LAB_SERVER_URL`). Empty means
    /// "the client's login server", filled in per launch by
    /// [`TelemetryConfig::for_launch`].
    pub server_url: String,
    /// Upload endpoint override (`CIMMERIA_LAB_UPLOAD_ENDPOINT`).
    pub upload_endpoint_override: Option<String>,
    /// Refuse a launch whose mint failed (`CIMMERIA_LAB_TELEMETRY`,
    /// anything but `optional`).
    pub required: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            upload_endpoint_override: None,
            required: true,
        }
    }
}

impl TelemetryConfig {
    pub fn from_env() -> Self {
        let non_empty = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        Self {
            server_url: non_empty("CIMMERIA_LAB_SERVER_URL").unwrap_or_default(),
            upload_endpoint_override: non_empty("CIMMERIA_LAB_UPLOAD_ENDPOINT"),
            required: !non_empty("CIMMERIA_LAB_TELEMETRY")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("optional")),
        }
    }

    /// This config with `server_url` set for one launch: the override when
    /// there is one, else the login URL of server row `server` in the
    /// install's `LoginInternal.lua`.
    pub fn for_launch(&self, install_dir: &Path, server: Option<&str>) -> Result<Self, String> {
        if !self.server_url.trim().is_empty() {
            return Ok(self.clone());
        }
        let url = super::login_servers::login_url(install_dir, server)?;
        Ok(Self {
            server_url: url,
            ..self.clone()
        })
    }

    /// `Err` with the reason when `grant` has no token and telemetry is
    /// required, so the launch stops instead of running blind.
    pub fn check(&self, grant: &TelemetryGrant) -> Result<(), String> {
        match (&grant.unavailable, self.required) {
            (Some(why), true) => Err(format!(
                "lab telemetry unavailable, not launching: {why}                  (set CIMMERIA_LAB_TELEMETRY=optional to launch without uploads)"
            )),
            _ => Ok(()),
        }
    }
}

/// `…/api/auth/dev-session` for a server base URL, with or without a
/// trailing `/api` (the launcher's `auth_url` convention includes it).
pub fn dev_session_url(server_url: &str) -> String {
    let base = server_url.trim().trim_end_matches('/');
    let base = base.strip_suffix("/api").unwrap_or(base);
    format!("{base}/api/auth/dev-session")
}

/// The mint request. Field names are the server's `DevSessionRequest`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MintRequest {
    pub install_id: String,
    pub machine_id: String,
    pub branch: String,
    pub git_sha: String,
    pub launcher_version: String,
    pub tags: Vec<String>,
    /// Always `"lab"`: every row this session uploads is tagged lab.
    pub session_kind: String,
}

impl MintRequest {
    pub fn lab() -> Self {
        Self {
            install_id: LAB_INSTALL_ID.to_string(),
            machine_id: LAB_INSTALL_ID.to_string(),
            branch: "lab".to_string(),
            git_sha: "lab".to_string(),
            launcher_version: concat!("cimmeria-lab ", env!("CARGO_PKG_VERSION")).to_string(),
            tags: vec!["lab".to_string()],
            session_kind: "lab".to_string(),
        }
    }
}

/// The server's `DevSessionResponse`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MintResponse {
    pub session_id: String,
    pub token: String,
    pub expires_at_ms: i64,
    pub upload_endpoint: String,
    pub chunk_max_bytes: u64,
    pub flush_interval_ms: u64,
}

/// What one launch writes into the session file's `telemetry` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryGrant {
    /// The server-minted session id, so the SigNoz rows and the session
    /// file agree; a local one when the mint failed.
    pub session_id: String,
    /// The dev-session token; empty when the mint failed.
    pub token: String,
    pub upload_endpoint: String,
    pub expires_at_ms: i64,
    pub chunk_max_bytes: u64,
    pub flush_interval_ms: u64,
    /// `None` when minted; why not otherwise.
    pub unavailable: Option<String>,
}

impl TelemetryGrant {
    /// A minted grant, with the endpoint override applied.
    pub fn minted(resp: MintResponse, cfg: &TelemetryConfig) -> Self {
        Self {
            session_id: resp.session_id,
            token: resp.token,
            upload_endpoint: cfg
                .upload_endpoint_override
                .clone()
                .unwrap_or(resp.upload_endpoint),
            expires_at_ms: resp.expires_at_ms,
            chunk_max_bytes: resp.chunk_max_bytes,
            flush_interval_ms: resp.flush_interval_ms,
            unavailable: None,
        }
    }

    /// No token: the launch goes ahead, uploads are refused.
    pub fn unavailable(reason: String, cfg: &TelemetryConfig) -> Self {
        let base = cfg.server_url.trim().trim_end_matches('/');
        let base = base.strip_suffix("/api").unwrap_or(base);
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            token: String::new(),
            upload_endpoint: cfg
                .upload_endpoint_override
                .clone()
                .unwrap_or_else(|| format!("{base}/api/telemetry")),
            expires_at_ms: 0,
            chunk_max_bytes: 1 << 20,
            flush_interval_ms: 2000,
            unavailable: Some(reason),
        }
    }

    /// What `lab_client_start` and `lab_client_status` report.
    pub fn status(&self) -> Value {
        match &self.unavailable {
            None => json!({
                "enabled": true,
                "session_id": self.session_id,
                "upload_endpoint": self.upload_endpoint,
                "signoz": "service.name = cimmeria-client, cimmeria.session_kind = lab",
            }),
            Some(why) => json!({ "enabled": false, "reason": why }),
        }
    }
}

/// POST the lab mint request to `cfg.server_url`.
pub async fn mint(cfg: &TelemetryConfig) -> Result<MintResponse, String> {
    let url = dev_session_url(&cfg.server_url);
    let http = reqwest::Client::builder()
        .timeout(MINT_TIMEOUT)
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let resp = http
        .post(&url)
        .json(&MintRequest::lab())
        .send()
        .await
        .map_err(|e| format!("POST {url}: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("POST {url}: {status}: {}", body.trim()));
    }
    resp.json::<MintResponse>()
        .await
        .map_err(|e| format!("POST {url}: bad response: {e}"))
}

/// Mint for one launch, falling back to [`TelemetryGrant::unavailable`]
/// (uncached; launches use [`grant_for_launch_cached`]).
#[cfg(test)]
pub async fn grant_for_launch(cfg: &TelemetryConfig) -> TelemetryGrant {
    match mint(cfg).await {
        Ok(resp) => {
            tracing::info!(
                session_id = %resp.session_id,
                server = %cfg.server_url,
                "lab telemetry session minted"
            );
            TelemetryGrant::minted(resp, cfg)
        }
        Err(why) => {
            tracing::warn!(
                server = %cfg.server_url,
                reason = %why,
                "lab telemetry unavailable"
            );
            TelemetryGrant::unavailable(why, cfg)
        }
    }
}

/// How long a cached token must still be valid to be reused for a launch.
const REUSE_MIN_REMAINING_MS: i64 = 30 * 60 * 1000;

/// A minted response remembered on disk, so relaunches reuse it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CachedMint {
    server_url: String,
    mint: MintResponse,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// The cached mint for `cfg.server_url`, if it has at least
/// [`REUSE_MIN_REMAINING_MS`] left at `now`.
fn cached_mint(path: &Path, cfg: &TelemetryConfig, now: i64) -> Option<MintResponse> {
    let c: CachedMint = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    (c.server_url == cfg.server_url && c.mint.expires_at_ms - now >= REUSE_MIN_REMAINING_MS)
        .then_some(c.mint)
}

/// A grant for one launch that reuses a still-valid token from `cache`
/// instead of minting again. Tokens last hours and the server limits mints
/// per install id (30 per window), so minting on every relaunch ran a repro
/// campaign out of tokens and its clients uploaded nothing (2026-09-29).
pub async fn grant_for_launch_cached(cfg: &TelemetryConfig, cache: &Path) -> TelemetryGrant {
    if let Some(mint) = cached_mint(cache, cfg, now_ms()) {
        tracing::info!(
            session_id = %mint.session_id,
            server = %cfg.server_url,
            expires_at_ms = mint.expires_at_ms,
            "lab telemetry session reused"
        );
        return TelemetryGrant::minted(mint, cfg);
    }
    match mint(cfg).await {
        Ok(resp) => {
            tracing::info!(
                session_id = %resp.session_id,
                server = %cfg.server_url,
                "lab telemetry session minted"
            );
            let cached = CachedMint {
                server_url: cfg.server_url.clone(),
                mint: resp.clone(),
            };
            if let Err(e) = serde_json::to_vec(&cached)
                .map_err(|e| e.to_string())
                .and_then(|b| std::fs::write(cache, b).map_err(|e| e.to_string()))
            {
                tracing::warn!(path = %cache.display(), error = %e, "lab telemetry grant not cached");
            }
            TelemetryGrant::minted(resp, cfg)
        }
        Err(why) => {
            tracing::warn!(
                server = %cfg.server_url,
                reason = %why,
                "lab telemetry unavailable"
            );
            TelemetryGrant::unavailable(why, cfg)
        }
    }
}

#[cfg(test)]
#[path = "telemetry_session_tests.rs"]
mod tests;
