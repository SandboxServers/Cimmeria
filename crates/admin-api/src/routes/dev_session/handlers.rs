//! `/api/auth/dev-session` mint and refresh handlers.

use std::net::{IpAddr, SocketAddr};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use axum::extract::ConnectInfo;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::launcher_kind;
use super::quota::{ip_key, validate_install_id, WindowTable};
use super::token::{
    encode_token, load_secret, AuthError, TokenClaims, SESSION_KIND_LAB, SESSION_KIND_LAUNCHER,
    SESSION_KIND_PLAYER,
};

pub const TOKEN_TTL_SECONDS: i64 = 8 * 60 * 60;

/// Default upload endpoint: the cimmeria-server's own admin port,
/// localhost. For deployments where the launcher runs on a different
/// host than the server, operators MUST set
/// `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` to the publicly-reachable
/// URL, normally the public login port, which serves the same routes
/// (`http://<host>:8081/api/telemetry`, see
/// [`crate::login_port_telemetry_router`]).
///
/// Note: this points launcher uploads at cimmeria-server itself,
/// which then replays the events through `tracing` so the OTLP layer
/// ships them to SigNoz.
const DEFAULT_UPLOAD_ENDPOINT: &str = "http://localhost:8443/api/telemetry";
pub(super) const DEFAULT_CHUNK_MAX_BYTES: u64 = 1_048_576;
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
/// Refresh calls per address whose token fails signature or payload
/// checks. A working launcher never sends one, so this is tight.
const DEFAULT_REFRESH_BAD_PER_IP: u32 = 30;
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
    /// `"lab"` from the Live Research Lab supervisor; absent or
    /// `"player"` from a player's launcher. Becomes the token's `kind`
    /// claim, and every row the session uploads is tagged with it. Any
    /// other value is refused with 400 so a typo cannot silently file lab
    /// rows as player rows.
    #[serde(default)]
    pub session_kind: Option<String>,
    /// The game account the player logs in with (TG-NET-18, owner decision
    /// D-TG14), when the launcher knows it. Signed into the token as `acct`;
    /// a stream naming the account wins world entry's join over the machine
    /// and address match. No launcher sends it today (they mint before the
    /// game logs in). Blank counts as absent.
    #[serde(default)]
    pub account_name: Option<String>,
}

/// The longest `account_name` a mint takes: the game login's own cap
/// (`cimmeria_auth::auth::handlers`, 3 to 20 bytes), so a name the game
/// would refuse is never stored.
pub const MAX_ACCOUNT_NAME_LEN: usize = 20;

/// The `acct` claim for a requested account name: `None` when absent or
/// blank (the token then has the bytes it had before the claim existed),
/// the trimmed name otherwise. A name that is too long or carries control
/// characters (it reaches the mint log line) is refused with 400.
pub(super) fn account_claim(requested: Option<&str>) -> Result<Option<String>, AuthError> {
    let Some(name) = requested.map(str::trim).filter(|n| !n.is_empty()) else {
        return Ok(None);
    };
    if name.len() > MAX_ACCOUNT_NAME_LEN {
        return Err(AuthError::BadField {
            field: "account_name",
            reason: "exceeds 20 bytes",
        });
    }
    if name.chars().any(char::is_control) {
        return Err(AuthError::BadField {
            field: "account_name",
            reason: "must not contain control characters",
        });
    }
    Ok(Some(name.to_string()))
}

/// The `kind` claim for a requested session kind: `None` for a player
/// session (so the token is byte-identical to one minted before the claim
/// existed), `Some("lab")` for a lab one.
pub(super) fn kind_claim(requested: Option<&str>) -> Result<Option<String>, AuthError> {
    match requested {
        None => Ok(None),
        Some(k) if k == SESSION_KIND_PLAYER => Ok(None),
        Some(k) if k == SESSION_KIND_LAB => Ok(Some(SESSION_KIND_LAB.to_string())),
        Some(k) if k == SESSION_KIND_LAUNCHER => Ok(Some(SESSION_KIND_LAUNCHER.to_string())),
        Some(_) => Err(AuthError::BadField {
            field: "session_kind",
            reason: "must be \"player\", \"lab\" or \"launcher\"",
        }),
    }
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
    pub refresh_bad_per_ip: u32,
    pub max_session_secs: i64,
    pub mint_launcher_per_ip: u32,
    pub mint_launcher_per_install: u32,
    pub launcher_disabled: bool,
}

impl QuotaPolicy {
    pub fn from_env() -> Self {
        let (launcher_ip, launcher_install, launcher_disabled) = launcher_kind::quota_from_env();
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
            refresh_bad_per_ip: env_u32(
                "CIMMERIA_TELEMETRY_REFRESH_BAD_QUOTA_PER_IP",
                DEFAULT_REFRESH_BAD_PER_IP,
            ),
            max_session_secs: env_i64(
                "CIMMERIA_TELEMETRY_MAX_SESSION_SECS",
                DEFAULT_MAX_SESSION_SECS,
            ),
            mint_launcher_per_ip: launcher_ip,
            mint_launcher_per_install: launcher_install,
            launcher_disabled,
        }
    }
}

/// The counter tables live for the process: a per-request table would
/// count nothing.
pub struct Tables {
    pub mint_ip: WindowTable,
    pub mint_install: WindowTable,
    pub mint_launcher_ip: WindowTable,
    pub mint_launcher_install: WindowTable,
    pub refresh_ip: WindowTable,
    pub refresh_bad: WindowTable,
}

impl Tables {
    pub(super) fn new() -> Self {
        Self {
            mint_ip: WindowTable::new(),
            mint_install: WindowTable::new(),
            mint_launcher_ip: WindowTable::new(),
            mint_launcher_install: WindowTable::new(),
            refresh_ip: WindowTable::new(),
            refresh_bad: WindowTable::new(),
        }
    }
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(Tables::new)
}

pub async fn mint(
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
    let launcher = launcher_kind::admit(policy, &req)?;
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
    launcher_kind::charge_ip(tables, policy, peer_ip, launcher, now)?;
    validate_install_id(&req.install_id).map_err(AuthError::BadInstallId)?;
    launcher_kind::check_request(&req, launcher)?;
    let kind = kind_claim(req.session_kind.as_deref())?;
    let acct = account_claim(req.account_name.as_deref())?;
    launcher_kind::charge_install(tables, policy, &req.install_id, launcher, now)?;
    let secret = load_secret()?;
    let session_id = uuid::Uuid::new_v4().to_string();
    let exp = now_unix + launcher_kind::token_ttl(launcher);
    let claims = TokenClaims {
        iss: "cimmeria-server".into(),
        sub: req.install_id,
        sid: session_id.clone(),
        iat: now_unix,
        exp,
        scope: launcher_kind::scope_for(launcher),
        kind,
        acct,
        // Signed so a refresh, which sees only the token, can re-record the stream.
        mid: launcher_kind::machine_claim(launcher, &req.machine_id),
    };
    let token = encode_token(&claims, &secret)?;
    launcher_kind::record_unless_launcher(launcher, &claims, peer_ip);
    // install_id / machine_id stay at debug to avoid leaking
    // persistent fingerprints into info-level pipelines. The token itself
    // is never logged.
    tracing::info!(
        session_id = %session_id, // nt:id-only generated UUID, nothing to name
        account_name = claims.acct.as_deref(),
        session_kind = claims.session_kind(),
        branch = %req.branch,
        git_sha = %req.git_sha,
        launcher_version = %req.launcher_version,
        exp,
        "Minted dev-session telemetry token"
    );
    tracing::debug!(
        session_id = %session_id, // nt:id-only generated UUID, nothing to name
        install_id = %claims.sub, // nt:id-only a generated install UUID, nothing to name
        machine_id = %launcher_kind::machine_id_for_log(launcher, &req.machine_id), // nt:id-only an opaque machine fingerprint, nothing to name
        "dev-session caller identifiers (debug-only)"
    );
    Ok(DevSessionResponse {
        session_id,
        token,
        expires_at_ms: exp * 1000,
        upload_endpoint: upload_endpoint_env(),
        chunk_max_bytes: launcher_kind::chunk_max_bytes(launcher),
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
    // Verify before charging the refresh quota. Charging first let
    // anyone who shares an egress address with real launchers (a
    // tunnel, an office NAT) burn the shared bucket with junk tokens
    // and lock every launcher out of refresh. Junk is counted on its
    // own, tighter table instead, which never touches the valid-token
    // allowance. The HMAC check is cheap and the body is capped.
    let secret = load_secret()?;
    let claims = match super::token::decode_token(token, &secret) {
        Ok(claims) => claims,
        Err(e) => {
            tables.refresh_bad.check_and_record(
                ip_key(peer_ip),
                policy.refresh_bad_per_ip,
                policy.window,
                "refresh/bad_token",
                now,
            )?;
            return Err(e);
        }
    };
    launcher_kind::refuse_refresh(&claims)?;
    tables.refresh_ip.check_and_record(
        ip_key(peer_ip),
        policy.refresh_per_ip,
        policy.window,
        "refresh/ip",
        now,
    )?;
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
        // A refresh keeps the session's kind: a lab session stays lab.
        kind: claims.kind,
        // ...and keeps its account, so world entry still finds the stream.
        acct: claims.acct,
        // ...and its machine, so the join still finds it by machine.
        mid: claims.mid,
    };
    let token = encode_token(&new_claims, &secret)?;
    record_stream(&new_claims, peer_ip);
    tracing::info!(
        session_id = %new_claims.sid, // nt:id-only generated UUID, nothing to name
        account_name = new_claims.acct.as_deref(),
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

/// Record the stream under its `mid` and `acct` claims and the request's
/// address, for world entry's `client_telemetry` field (TG-NET-18). A
/// refresh records the address it came from, the launcher's latest. Called
/// after the token is signed, so a refused mint or refresh leaves no stream
/// behind.
pub(super) fn record_stream(claims: &TokenClaims, peer_ip: IpAddr) {
    use cimmeria_services::auth::client_telemetry::{record_client_telemetry_stream, StreamRecord};
    record_client_telemetry_stream(StreamRecord {
        session_id: &claims.sid,
        expires_at_unix: claims.exp,
        machine_id: claims.mid.as_deref(),
        account_name: claims.acct.as_deref(),
        peer_ip: Some(peer_ip),
    });
}

/// Every refusal the operator can act on leaves a row with its `reason`
/// (see docs/operations/telemetry.md): quota limits need tuning, a paused
/// ingest or an expired token explains a launcher's telemetry stream
/// going quiet. Refusals a caller can't fix are logged at INFO or below.
pub(super) fn log_refusal(route: &'static str, peer: IpAddr, err: &AuthError) {
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
        // The operator's misconfiguration, not the caller's: every launcher
        // and lab session is refused until it is fixed, and before this row
        // the only trace of it was a 500 on the caller's side.
        AuthError::SecretMissing | AuthError::SecretTooShort { .. } => tracing::error!(
            route,
            peer = %peer,
            reason = "dev_session_secret_unusable",
            "dev-session request refused: {err}"
        ),
        AuthError::KillSwitchActive => tracing::warn!(
            route,
            peer = %peer,
            reason = "dev_session_kill_switch",
            "dev-session request refused: {err}"
        ),
        AuthError::SessionLifetimeExceeded { elapsed, cap } => tracing::info!(
            route,
            peer = %peer,
            elapsed,
            cap,
            reason = "dev_session_lifetime_exceeded",
            "dev-session request refused: {err}"
        ),
        AuthError::Expired { exp, now } => tracing::info!(
            route,
            peer = %peer,
            exp,
            now,
            reason = "dev_session_expired",
            "dev-session request refused: {err}"
        ),
        AuthError::BadPayload(_) | AuthError::BadSignature => tracing::info!(
            route,
            peer = %peer,
            reason = "dev_session_bad_token",
            "dev-session request refused: {err}"
        ),
        _ => {}
    }
}

pub(crate) fn kill_switch_active() -> bool {
    matches!(
        std::env::var("CIMMERIA_TELEMETRY_KILL_SWITCH"),
        Ok(v) if v == "1"
    )
}

/// `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT`, or the local default when it is
/// unset **or blank**. `docker/compose.yml` passes it as
/// `${CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT:-}`, which sets it to `""` when
/// the operator's `.env` has no value; handing callers an empty endpoint
/// would make every upload fail with a URL error instead.
pub(super) fn upload_endpoint_env() -> String {
    upload_endpoint_from(std::env::var("CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT").ok())
}

pub(super) fn upload_endpoint_from(value: Option<String>) -> String {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_UPLOAD_ENDPOINT.to_string())
}

/// A malformed value falls back to the default rather than refusing
/// to serve: an operator typo must not take telemetry offline.
fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

pub(crate) fn env_u32(name: &str, default: u32) -> u32 {
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
