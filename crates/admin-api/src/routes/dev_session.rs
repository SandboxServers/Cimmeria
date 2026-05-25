//! Dev-session telemetry auth endpoint.
//!
//! Mints a short-lived HMAC-SHA256 token the launcher uses to talk to
//! the telemetry ingest Functions app. The Functions side verifies the
//! token independently using the same shared
//! `CIMMERIA_TELEMETRY_HMAC_SECRET`.
//!
//! Token shape:
//!
//! ```text
//! payload = base64url(JSON {iss, sub, sid, iat, exp, scope})
//! sig     = base64url(HMAC-SHA256(secret, payload))
//! token   = payload || "." || sig
//! ```
//!
//! v1 trust model: any caller is trusted. The token only grants
//! `scope: ["telemetry.write"]` and is single-session-scoped, so the
//! worst an attacker can do is upload garbage telemetry.
//!
//! `CIMMERIA_TELEMETRY_KILL_SWITCH=1` makes every mint return 503
//! with `Retry-After: 60`.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use cimmeria_services::orchestrator::Orchestrator;

pub const TOKEN_TTL_SECONDS: i64 = 8 * 60 * 60;

/// Anything shorter is operator misconfiguration; refusing to mint is
/// safer than issuing tokens against a weak key.
pub const MIN_SECRET_BYTES: usize = 32;

/// Default upload endpoint: the cimmeria-server's own admin port,
/// localhost. For deployments where the launcher runs on a different
/// host than the server, operators MUST set
/// `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` to the publicly-reachable
/// URL (e.g. via the Cloudflare Tunnel that exposes the SigNoz UI,
/// or directly via the LAN address).
///
/// Note: this points launcher uploads at cimmeria-server itself,
/// which then replays the events through `tracing` so the OTLP layer
/// ships them to SigNoz. The previous default was the Cosmos-backed
/// Cimmeria-MCP Azure Function endpoint; that path is decommissioned
/// in favor of single-store-of-truth (SigNoz) analytical retrieval.
const DEFAULT_UPLOAD_ENDPOINT: &str = "http://localhost:8443/api/telemetry";
const DEFAULT_CHUNK_MAX_BYTES: u64 = 1_048_576;
const DEFAULT_FLUSH_INTERVAL_MS: u64 = 2_000;

/// Generous vs. a realistic ~400-byte token, low enough to refuse a
/// DoS-via-huge-payload on the public refresh endpoint.
const MAX_TOKEN_LEN: usize = 4096;

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

/// Wire format pinned: any field addition or rename is a breaking
/// change for the Functions-side verifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenClaims {
    pub iss: String,
    pub sub: String, // install_id
    pub sid: String, // session_id
    pub iat: i64,
    pub exp: i64,
    pub scope: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error(
        "Telemetry auth misconfigured: CIMMERIA_TELEMETRY_HMAC_SECRET not set. \
         Set the env var on the server before enabling launcher telemetry."
    )]
    SecretMissing,
    #[error(
        "Telemetry HMAC secret is too short ({got} bytes, need at least {min}). \
         Regenerate with `openssl rand -hex 64`."
    )]
    SecretTooShort { got: usize, min: usize },
    #[error("Kill switch active — telemetry ingest is paused")]
    KillSwitchActive,
    #[error("Token payload decode failed: {0}")]
    BadPayload(String),
    #[error("Token signature invalid")]
    BadSignature,
    #[error("Token expired (exp={exp}, now={now})")]
    Expired { exp: i64, now: i64 },
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            AuthError::SecretMissing | AuthError::SecretTooShort { .. } => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            AuthError::KillSwitchActive => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            AuthError::BadPayload(_) | AuthError::BadSignature => {
                (StatusCode::UNAUTHORIZED, self.to_string())
            }
            AuthError::Expired { .. } => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::Json(_) => (StatusCode::BAD_REQUEST, self.to_string()),
        };
        let mut resp = (status, body).into_response();
        // Cooperative back-off when the kill switch is on — the
        // launcher's chunk uploader respects Retry-After.
        if matches!(self, AuthError::KillSwitchActive) {
            resp.headers_mut().insert(
                axum::http::header::RETRY_AFTER,
                axum::http::HeaderValue::from_static("60"),
            );
        }
        resp
    }
}

pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .route("/dev-session", post(mint))
        .route("/dev-session/refresh", post(refresh))
}

pub async fn mint(
    State(_orchestrator): State<Arc<Orchestrator>>,
    Json(req): Json<DevSessionRequest>,
) -> Result<Json<DevSessionResponse>, AuthError> {
    if kill_switch_active() {
        return Err(AuthError::KillSwitchActive);
    }
    let secret = load_secret()?;
    let session_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();
    let exp = now + TOKEN_TTL_SECONDS;
    let claims = TokenClaims {
        iss: "cimmeria-server".into(),
        sub: req.install_id,
        sid: session_id.clone(),
        iat: now,
        exp,
        scope: vec!["telemetry.write".into()],
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
    Ok(Json(DevSessionResponse {
        session_id,
        token,
        expires_at_ms: exp * 1000,
        upload_endpoint: upload_endpoint_env(),
        chunk_max_bytes: DEFAULT_CHUNK_MAX_BYTES,
        flush_interval_ms: DEFAULT_FLUSH_INTERVAL_MS,
    }))
}

pub async fn refresh(
    State(_orchestrator): State<Arc<Orchestrator>>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<DevSessionResponse>, AuthError> {
    if kill_switch_active() {
        return Err(AuthError::KillSwitchActive);
    }
    let secret = load_secret()?;
    let claims = decode_token(&req.token, &secret)?;
    let now = chrono::Utc::now().timestamp();
    // Refresh of an already-expired token: forbid. The launcher's
    // 401-on-expiry path mints a fresh session instead — refresh is
    // only for "almost-expired but still valid."
    if claims.exp <= now {
        return Err(AuthError::Expired {
            exp: claims.exp,
            now,
        });
    }
    let new_exp = now + TOKEN_TTL_SECONDS;
    let new_claims = TokenClaims {
        iss: claims.iss,
        sub: claims.sub.clone(),
        sid: claims.sid.clone(),
        iat: now,
        exp: new_exp,
        scope: claims.scope,
    };
    let token = encode_token(&new_claims, &secret)?;
    tracing::info!(
        session_id = %new_claims.sid,
        old_exp = claims.exp,
        new_exp,
        "Refreshed dev-session telemetry token"
    );
    Ok(Json(DevSessionResponse {
        session_id: new_claims.sid,
        token,
        expires_at_ms: new_exp * 1000,
        upload_endpoint: upload_endpoint_env(),
        chunk_max_bytes: DEFAULT_CHUNK_MAX_BYTES,
        flush_interval_ms: DEFAULT_FLUSH_INTERVAL_MS,
    }))
}

fn kill_switch_active() -> bool {
    matches!(
        std::env::var("CIMMERIA_TELEMETRY_KILL_SWITCH"),
        Ok(v) if v == "1"
    )
}

fn upload_endpoint_env() -> String {
    std::env::var("CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT")
        .unwrap_or_else(|_| DEFAULT_UPLOAD_ENDPOINT.to_string())
}

/// Shared HMAC secret loader. Exposed at module visibility so the
/// telemetry ingest endpoints (which verify tokens minted here) reuse
/// the same parsing rules — drift between mint and verify would cause
/// every launcher upload to fail validation.
pub(super) fn load_secret() -> Result<Vec<u8>, AuthError> {
    let raw =
        std::env::var("CIMMERIA_TELEMETRY_HMAC_SECRET").map_err(|_| AuthError::SecretMissing)?;
    let raw_trimmed = raw.trim();
    if raw_trimmed.is_empty() {
        return Err(AuthError::SecretMissing);
    }
    // Accept both raw-bytes (just the env value) and hex-encoded
    // forms. `openssl rand -hex 64` produces 128 hex chars → 64 raw
    // bytes; operators sometimes paste the hex form directly into
    // the GitHub Secret. Try hex first, fall back to UTF-8 bytes.
    let bytes = match hex_decode_lenient(raw_trimmed) {
        Some(bytes) => bytes,
        None => raw_trimmed.as_bytes().to_vec(),
    };
    if bytes.len() < MIN_SECRET_BYTES {
        return Err(AuthError::SecretTooShort {
            got: bytes.len(),
            min: MIN_SECRET_BYTES,
        });
    }
    Ok(bytes)
}

/// Decode an even-length all-hex string to bytes. Returns `None` on
/// any non-hex character or odd length — caller falls back to raw-
/// bytes interpretation.
fn hex_decode_lenient(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    for chunk in bytes.chunks(2) {
        let hi = (chunk[0] as char).to_digit(16)?;
        let lo = (chunk[1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
    }
    Some(out)
}

pub fn encode_token(claims: &TokenClaims, secret: &[u8]) -> Result<String, AuthError> {
    let payload_json = serde_json::to_vec(claims)?;
    let payload_b64 = URL_SAFE_NO_PAD.encode(&payload_json);
    let mut mac =
        <Hmac<Sha256> as Mac>::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(payload_b64.as_bytes());
    let sig = mac.finalize().into_bytes();
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig);
    Ok(format!("{payload_b64}.{sig_b64}"))
}

pub fn decode_token(token: &str, secret: &[u8]) -> Result<TokenClaims, AuthError> {
    if token.len() > MAX_TOKEN_LEN {
        return Err(AuthError::BadPayload(format!(
            "token exceeds {MAX_TOKEN_LEN}-byte cap"
        )));
    }
    let (payload_b64, sig_b64) = token
        .split_once('.')
        .ok_or_else(|| AuthError::BadPayload("missing '.' separator".into()))?;
    let expected_sig = URL_SAFE_NO_PAD
        .decode(sig_b64)
        .map_err(|e| AuthError::BadPayload(format!("signature base64: {e}")))?;
    let mut mac =
        <Hmac<Sha256> as Mac>::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(payload_b64.as_bytes());
    // verify_slice is constant-time — defends against timing oracle
    // attacks on the signature comparison.
    mac.verify_slice(&expected_sig)
        .map_err(|_| AuthError::BadSignature)?;
    let payload_json = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|e| AuthError::BadPayload(format!("payload base64: {e}")))?;
    let claims: TokenClaims = serde_json::from_slice(&payload_json)?;
    Ok(claims)
}

/// Process-wide serialization for tests that mutate
/// `CIMMERIA_TELEMETRY_*` env vars. `cargo test` is multi-threaded by
/// default; any module that reads/writes these vars in `#[test]`
/// scopes must lock this before doing so. Exposed at module
/// visibility so the sibling `telemetry` tests share the same lock —
/// a per-module lock would still race against this one.
#[cfg(test)]
pub(super) fn env_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_secret() -> Vec<u8> {
        vec![0x42; 64]
    }

    fn fake_claims() -> TokenClaims {
        TokenClaims {
            iss: "cimmeria-server".into(),
            sub: "install-abc".into(),
            sid: "session-123".into(),
            iat: 1_700_000_000,
            exp: 1_700_028_800,
            scope: vec!["telemetry.write".into()],
        }
    }

    // Roundtrip: encode → decode produces the same claims. Pins the
    // wire-format contract so a refactor that changes JSON field order
    // (which would change the b64) is still byte-stable.
    #[test]
    fn encode_decode_roundtrip_preserves_claims() {
        let secret = test_secret();
        let claims = fake_claims();
        let token = encode_token(&claims, &secret).unwrap();
        // Token shape: two base64-url-no-pad segments joined by '.'.
        let (p, s) = token.split_once('.').unwrap();
        assert!(!p.is_empty() && !s.is_empty());
        let decoded = decode_token(&token, &secret).unwrap();
        assert_eq!(decoded, claims);
    }

    // Tampered payload (even if re-base64'd cleanly) must fail
    // signature verification — defends against an attacker swapping
    // `sub` from their install_id to someone else's after intercepting
    // a token.
    #[test]
    fn decode_rejects_tampered_payload() {
        let secret = test_secret();
        let claims = fake_claims();
        let token = encode_token(&claims, &secret).unwrap();
        let (_, sig) = token.split_once('.').unwrap();
        let tampered_claims = TokenClaims {
            sub: "evil-install".into(),
            ..fake_claims()
        };
        let tampered_payload =
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&tampered_claims).unwrap());
        let tampered_token = format!("{tampered_payload}.{sig}");
        let err = decode_token(&tampered_token, &secret).unwrap_err();
        assert!(matches!(err, AuthError::BadSignature));
    }

    // Wrong key cannot verify a valid signature — defends against an
    // attacker forging tokens with a guessed secret.
    #[test]
    fn decode_rejects_wrong_secret() {
        let claims = fake_claims();
        let token = encode_token(&claims, &test_secret()).unwrap();
        let other_secret = vec![0x77; 64];
        let err = decode_token(&token, &other_secret).unwrap_err();
        assert!(matches!(err, AuthError::BadSignature));
    }

    // Malformed token without the dot separator surfaces as
    // BadPayload, NOT BadSignature — easier ops triage when an upload
    // proxy strips fragments or someone pastes only half the token.
    #[test]
    fn decode_rejects_token_missing_separator() {
        let err = decode_token("not-a-token", &test_secret()).unwrap_err();
        assert!(matches!(err, AuthError::BadPayload(_)));
    }

    // Reject tokens larger than MAX_TOKEN_LEN before the base64 decode
    // path can allocate. Defends the public refresh endpoint from
    // memory/CPU exhaustion via an oversized payload.
    #[test]
    fn decode_rejects_oversized_token() {
        let huge = "a".repeat(MAX_TOKEN_LEN + 1);
        let err = decode_token(&huge, &test_secret()).unwrap_err();
        match err {
            AuthError::BadPayload(msg) => {
                assert!(msg.contains("cap"), "should explain the cap, got: {msg}");
            }
            other => panic!("expected BadPayload, got {other:?}"),
        }
    }

    // load_secret accepts both hex-encoded and raw-bytes secret forms.
    // GitHub Secrets stores both as strings; operators using
    // `openssl rand -hex 64` produce hex while `openssl rand -base64 48`
    // produces raw bytes interpreted as UTF-8.
    #[test]
    fn load_secret_accepts_hex_encoded() {
        let _g = env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let prev = std::env::var("CIMMERIA_TELEMETRY_HMAC_SECRET").ok();
        // 128 hex chars = 64 raw bytes, well above MIN_SECRET_BYTES.
        let hex_secret = "a".repeat(128);
        std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", &hex_secret);
        let bytes = load_secret().unwrap();
        assert_eq!(bytes.len(), 64, "hex should decode to 64 raw bytes");
        match prev {
            Some(v) => std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", v),
            None => std::env::remove_var("CIMMERIA_TELEMETRY_HMAC_SECRET"),
        }
    }

    #[test]
    fn load_secret_accepts_raw_bytes_via_utf8() {
        let _g = env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let prev = std::env::var("CIMMERIA_TELEMETRY_HMAC_SECRET").ok();
        let raw = "this-is-a-32-byte-utf8-secret!!!";
        assert_eq!(raw.len(), 32);
        std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", raw);
        let bytes = load_secret().unwrap();
        assert_eq!(bytes.len(), 32);
        match prev {
            Some(v) => std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", v),
            None => std::env::remove_var("CIMMERIA_TELEMETRY_HMAC_SECRET"),
        }
    }

    // Missing env → SecretMissing, with an actionable error message.
    #[test]
    fn load_secret_missing_env_returns_secret_missing() {
        let _g = env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let prev = std::env::var("CIMMERIA_TELEMETRY_HMAC_SECRET").ok();
        std::env::remove_var("CIMMERIA_TELEMETRY_HMAC_SECRET");
        let err = load_secret().unwrap_err();
        assert!(matches!(err, AuthError::SecretMissing));
        if let Some(v) = prev {
            std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", v);
        }
    }

    // Empty env (GH Actions resolves missing secrets to empty strings)
    // — must surface as SecretMissing, NOT a 0-byte secret that gets
    // rejected later as too short.
    #[test]
    fn load_secret_empty_env_returns_secret_missing() {
        let _g = env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let prev = std::env::var("CIMMERIA_TELEMETRY_HMAC_SECRET").ok();
        std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", "   ");
        let err = load_secret().unwrap_err();
        assert!(matches!(err, AuthError::SecretMissing));
        match prev {
            Some(v) => std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", v),
            None => std::env::remove_var("CIMMERIA_TELEMETRY_HMAC_SECRET"),
        }
    }

    // Too-short secret rejected: refusing to mint is safer than
    // silently issuing tokens against a 4-byte "secret."
    #[test]
    fn load_secret_too_short_returns_secret_too_short() {
        let _g = env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let prev = std::env::var("CIMMERIA_TELEMETRY_HMAC_SECRET").ok();
        std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", "short");
        let err = load_secret().unwrap_err();
        assert!(matches!(
            err,
            AuthError::SecretTooShort {
                min: MIN_SECRET_BYTES,
                ..
            }
        ));
        match prev {
            Some(v) => std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", v),
            None => std::env::remove_var("CIMMERIA_TELEMETRY_HMAC_SECRET"),
        }
    }

    // Kill switch flips on with CIMMERIA_TELEMETRY_KILL_SWITCH=1; any
    // other value treats it as off.
    #[test]
    fn kill_switch_respects_env() {
        let _g = env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let prev = std::env::var("CIMMERIA_TELEMETRY_KILL_SWITCH").ok();

        std::env::set_var("CIMMERIA_TELEMETRY_KILL_SWITCH", "1");
        assert!(kill_switch_active());
        std::env::set_var("CIMMERIA_TELEMETRY_KILL_SWITCH", "0");
        assert!(!kill_switch_active());
        std::env::set_var("CIMMERIA_TELEMETRY_KILL_SWITCH", "true");
        assert!(
            !kill_switch_active(),
            "only literal '1' counts as on, not 'true' — keeps the contract crisp"
        );
        std::env::remove_var("CIMMERIA_TELEMETRY_KILL_SWITCH");
        assert!(!kill_switch_active());

        if let Some(v) = prev {
            std::env::set_var("CIMMERIA_TELEMETRY_KILL_SWITCH", v);
        }
    }

    // hex_decode_lenient: round-trips valid hex, rejects non-hex,
    // rejects odd-length.
    #[test]
    fn hex_decode_lenient_round_trips() {
        let bytes = vec![0x00, 0x42, 0xff, 0xab];
        let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let decoded = hex_decode_lenient(&hex).unwrap();
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn hex_decode_lenient_rejects_non_hex_chars() {
        assert!(hex_decode_lenient("xyz0").is_none());
    }

    #[test]
    fn hex_decode_lenient_rejects_odd_length() {
        assert!(hex_decode_lenient("abc").is_none());
    }
}
