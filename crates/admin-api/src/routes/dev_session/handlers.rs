//! `/api/auth/dev-session` mint and refresh handlers.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use cimmeria_services::orchestrator::Orchestrator;

use super::quota::{install_key, ip_key, validate_install_id, validate_metadata, WindowTable};
use super::token::{encode_token, load_secret, AuthError, TokenClaims, SCOPE_TELEMETRY_WRITE};

pub const TOKEN_TTL_SECONDS: i64 = 8 * 60 * 60;

/// Default upload endpoint: the cimmeria-server's own admin port,
/// localhost. For deployments where the launcher runs on a different
/// host than the server, operators MUST set
/// `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` to the publicly-reachable
/// URL (e.g. via the Cloudflare Tunnel that exposes the SigNoz UI,
/// or directly via the LAN address).
///
/// Note: this points launcher uploads at cimmeria-server itself,
/// which then replays the events through `tracing` so the OTLP layer
/// ships them to SigNoz.
const DEFAULT_UPLOAD_ENDPOINT: &str = "http://localhost:8443/api/telemetry";
const DEFAULT_CHUNK_MAX_BYTES: u64 = 1_048_576;
const DEFAULT_FLUSH_INTERVAL_MS: u64 = 2_000;

const DEFAULT_QUOTA_WINDOW_SECS: u64 = 3_600;
/// Sized for a shared egress address, not for one developer: a team
/// behind one NAT or one Cloudflare Tunnel shares this bucket, and a
/// wrongly-refused mint costs a session's telemetry.
const DEFAULT_MINT_PER_IP: u32 = 120;
/// One mint per game launch; this covers a crash-loop debugging
/// session on a single machine with room to spare.
const DEFAULT_MINT_PER_INSTALL: u32 = 30;
/// The launcher refreshes once per token lifetime, but retries on
/// transport errors; generous enough that only a loop trips it.
const DEFAULT_REFRESH_PER_IP: u32 = 480;
/// Ceiling on how long one minted session can be extended by
/// chained refreshes. Well past any real play session, so the
/// launcher only meets it after a token has leaked or a process has
/// been left running for a day.
const DEFAULT_MAX_SESSION_SECS: i64 = 24 * 60 * 60;

#[derive(Debug, Deserialize)]
pub struct DevSessionRequest {
    pub install_id: String,
    pub machine_id: String,
    pub branch: String,
    pub git_sha: String,
    pub launcher_version: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DevSessionResponse {
    pub session_id: String,
    pub token: String,
    pub expires_at_ms: i64,
    pub upload_endpoint: String,
    pub chunk_max_bytes: u64,
    pub flush_interval_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub token: String,
}

/// Operator-tunable limits. Read per request so the kill-switch-style
/// "change the env and restart" operating model applies here too.
pub struct QuotaPolicy {
    pub window: Duration,
    pub mint_per_ip: u32,
    pub mint_per_install: u32,
    pub refresh_per_ip: u32,
    pub max_session_secs: i64,
}

impl QuotaPolicy {
    pub fn from_env() -> Self {
        Self {
            window: Duration::from_secs(env_u64(
                "CIMMERIA_TELEMETRY_QUOTA_WINDOW_SECS",
                DEFAULT_QUOTA_WINDOW_SECS,
            )),
            mint_per_ip: env_u32("CIMMERIA_TELEMETRY_MINT_QUOTA_PER_IP", DEFAULT_MINT_PER_IP),
            mint_per_install: env_u32(
                "CIMMERIA_TELEMETRY_MINT_QUOTA_PER_INSTALL",
                DEFAULT_MINT_PER_INSTALL,
            ),
            refresh_per_ip: env_u32(
                "CIMMERIA_TELEMETRY_REFRESH_QUOTA_PER_IP",
                DEFAULT_REFRESH_PER_IP,
            ),
            max_session_secs: env_i64(
                "CIMMERIA_TELEMETRY_MAX_SESSION_SECS",
                DEFAULT_MAX_SESSION_SECS,
            ),
        }
    }
}

/// The counter tables live for the process: a per-request table would
/// count nothing.
pub struct Tables {
    pub mint_ip: WindowTable,
    pub mint_install: WindowTable,
    pub refresh_ip: WindowTable,
}

impl Tables {
    pub(super) fn new() -> Self {
        Self {
            mint_ip: WindowTable::new(),
            mint_install: WindowTable::new(),
            refresh_ip: WindowTable::new(),
        }
    }
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(Tables::new)
}

pub async fn mint(
    State(_orchestrator): State<Arc<Orchestrator>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(req): Json<DevSessionRequest>,
) -> Result<Json<DevSessionResponse>, AuthError> {
    mint_inner(
        tables(),
        &QuotaPolicy::from_env(),
        peer.ip(),
        req,
        Instant::now(),
        chrono::Utc::now().timestamp(),
    )
    .inspect_err(|e| log_refusal("mint", peer.ip(), e))
    .map(Json)
}

pub async fn refresh(
    State(_orchestrator): State<Arc<Orchestrator>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<DevSessionResponse>, AuthError> {
    refresh_inner(
        tables(),
        &QuotaPolicy::from_env(),
        peer.ip(),
        &req.token,
        Instant::now(),
        chrono::Utc::now().timestamp(),
    )
    .inspect_err(|e| log_refusal("refresh", peer.ip(), e))
    .map(Json)
}

pub(super) fn mint_inner(
    tables: &Tables,
    policy: &QuotaPolicy,
    peer_ip: IpAddr,
    req: DevSessionRequest,
    now: Instant,
    now_unix: i64,
) -> Result<DevSessionResponse, AuthError> {
    if kill_switch_active() {
        return Err(AuthError::KillSwitchActive);
    }
    // Charged before `install_id` is validated, so rotating or
    // malforming it buys nothing from one address. A body that fails
    // JSON extraction never reaches this function (axum rejects it in
    // the `Json` extractor) and is not charged; no token is issued on
    // that path either.
    tables.mint_ip.check_and_record(
        ip_key(peer_ip),
        policy.mint_per_ip,
        policy.window,
        "mint/ip",
        now,
    )?;
    validate_install_id(&req.install_id).map_err(AuthError::BadInstallId)?;
    for (field, value) in [
        ("machine_id", &req.machine_id),
        ("branch", &req.branch),
        ("git_sha", &req.git_sha),
        ("launcher_version", &req.launcher_version),
    ] {
        validate_metadata(value).map_err(|reason| AuthError::BadField { field, reason })?;
    }
    tables.mint_install.check_and_record(
        install_key(&req.install_id),
        policy.mint_per_install,
        policy.window,
        "mint/install_id",
        now,
    )?;
    let secret = load_secret()?;
    let session_id = uuid::Uuid::new_v4().to_string();
    let exp = now_unix + TOKEN_TTL_SECONDS;
    let claims = TokenClaims {
        iss: "cimmeria-server".into(),
        sub: req.install_id,
        sid: session_id.clone(),
        iat: now_unix,
        exp,
        scope: vec![SCOPE_TELEMETRY_WRITE.into()],
    };
    let token = encode_token(&claims, &secret)?;
    // install_id / machine_id stay at debug to avoid leaking
    // persistent fingerprints into info-level pipelines.
    tracing::info!(
        session_id = %session_id,
        branch = %req.branch,
        git_sha = %req.git_sha,
        launcher_version = %req.launcher_version,
        exp,
        "Minted dev-session telemetry token"
    );
    tracing::debug!(
        session_id = %session_id,
        install_id = %claims.sub,
        machine_id = %req.machine_id,
        "dev-session caller identifiers (debug-only)"
    );
    Ok(DevSessionResponse {
        session_id,
        token,
        expires_at_ms: exp * 1000,
        upload_endpoint: upload_endpoint_env(),
        chunk_max_bytes: DEFAULT_CHUNK_MAX_BYTES,
        flush_interval_ms: DEFAULT_FLUSH_INTERVAL_MS,
    })
}

pub(super) fn refresh_inner(
    tables: &Tables,
    policy: &QuotaPolicy,
    peer_ip: IpAddr,
    token: &str,
    now: Instant,
    now_unix: i64,
) -> Result<DevSessionResponse, AuthError> {
    if kill_switch_active() {
        return Err(AuthError::KillSwitchActive);
    }
    tables.refresh_ip.check_and_record(
        ip_key(peer_ip),
        policy.refresh_per_ip,
        policy.window,
        "refresh/ip",
        now,
    )?;
    let secret = load_secret()?;
    let claims = super::token::decode_token(token, &secret)?;
    // `iat` is the original mint time and is deliberately NOT reset
    // below, so a chain of refreshes is bounded rather than able to
    // extend one minted session forever. The launcher tracks its own
    // issue time for the proactive-refresh policy and never reads
    // this claim.
    //
    // Checked before the expiry test on purpose: once the expiry is
    // clamped to the session deadline the two coincide, and
    // "mint a fresh session" is the actionable message of the two.
    let elapsed = now_unix.saturating_sub(claims.iat).max(0);
    if elapsed >= policy.max_session_secs {
        return Err(AuthError::SessionLifetimeExceeded {
            elapsed,
            cap: policy.max_session_secs,
        });
    }
    // Refresh of an already-expired token: forbid. The launcher's
    // 401-on-expiry path mints a fresh session instead — refresh is
    // only for "almost-expired but still valid."
    if claims.exp <= now_unix {
        return Err(AuthError::Expired {
            exp: claims.exp,
            now: now_unix,
        });
    }
    let session_deadline = claims.iat.saturating_add(policy.max_session_secs);
    let new_exp = (now_unix + TOKEN_TTL_SECONDS).min(session_deadline);
    let new_claims = TokenClaims {
        iss: claims.iss,
        sub: claims.sub,
        sid: claims.sid,
        iat: claims.iat,
        exp: new_exp,
        scope: claims.scope,
    };
    let token = encode_token(&new_claims, &secret)?;
    tracing::info!(
        session_id = %new_claims.sid,
        old_exp = claims.exp,
        new_exp,
        session_age_secs = elapsed,
        "Refreshed dev-session telemetry token"
    );
    Ok(DevSessionResponse {
        session_id: new_claims.sid,
        token,
        expires_at_ms: new_exp * 1000,
        upload_endpoint: upload_endpoint_env(),
        chunk_max_bytes: DEFAULT_CHUNK_MAX_BYTES,
        flush_interval_ms: DEFAULT_FLUSH_INTERVAL_MS,
    })
}

/// Quota and validation refusals are the operator's signal that a
/// limit needs tuning (see docs/operations/telemetry.md), so they are
/// logged; the rest of the error family already surfaces elsewhere.
fn log_refusal(route: &'static str, peer: IpAddr, err: &AuthError) {
    match err {
        AuthError::QuotaExceeded(q) => tracing::warn!(
            route,
            peer = %peer,
            scope = q.scope,
            retry_after_secs = q.retry_after_secs,
            reason = "dev_session_quota_exceeded",
            "dev-session request refused: {}",
            q
        ),
        AuthError::BadInstallId(_) | AuthError::BadField { .. } => tracing::debug!(
            route,
            peer = %peer,
            reason = "dev_session_bad_request",
            "dev-session request refused: {err}"
        ),
        _ => {}
    }
}

pub(super) fn kill_switch_active() -> bool {
    matches!(
        std::env::var("CIMMERIA_TELEMETRY_KILL_SWITCH"),
        Ok(v) if v == "1"
    )
}

fn upload_endpoint_env() -> String {
    std::env::var("CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT")
        .unwrap_or_else(|_| DEFAULT_UPLOAD_ENDPOINT.to_string())
}

/// A malformed value falls back to the default rather than refusing
/// to serve: an operator typo must not take telemetry offline.
fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

fn env_i64(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}
