//! The launcher's share of the upload gate. A launcher token posts chunks
//! and nothing else, under its own session, install and address quotas,
//! its own slot pool and smaller size caps. The launcher kill switch refuses
//! only launcher tokens. Player and lab tokens pass through the same checks
//! and keep their own tables, slots and limits.

use std::net::IpAddr;
use std::time::Instant;

use axum::http::header::AUTHORIZATION;
use axum::http::HeaderMap;

use crate::routes::dev_session::quota::{install_key, ip_key};
use crate::routes::dev_session::token::SCOPE_LAUNCHER_WRITE;
use crate::routes::dev_session::{
    decode_token, load_secret, AuthError, TokenClaims, SCOPE_TELEMETRY_WRITE,
};

use super::dto::IngestError;
use super::upload_gate::{Route, UploadLimits, UploadPolicy, UploadState, CHUNK_QUOTA_WINDOW};

/// The most a launcher chunk body may be, whatever the server's limit.
const LAUNCHER_CHUNK_BODY_BYTES: usize = 65_536;

/// The most a launcher chunk may expand to.
const LAUNCHER_CHUNK_DECOMPRESSED_BYTES: u64 = 262_144;

/// The most one launcher chunk row may be.
const LAUNCHER_CHUNK_ROW_BYTES: usize = 4_096;

/// The most rows one launcher chunk may carry.
const LAUNCHER_CHUNK_ROWS: usize = 200;

/// The bearer check for one route. The token is decoded and checked as it
/// always was; then its kind must agree with its scope. A launcher token is
/// refused on the bundle route, and a chunk accepts either scope.
pub(super) fn verify_bearer_for(
    headers: &HeaderMap,
    route: Route,
) -> Result<TokenClaims, IngestError> {
    let raw = headers
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(IngestError::MissingAuth)?;
    let token = raw
        .strip_prefix("Bearer ")
        .ok_or(IngestError::MissingAuth)?
        .trim();
    if token.is_empty() {
        return Err(IngestError::MissingAuth);
    }
    // Single source of truth for the HMAC secret — `dev_session::mint`
    // signs with this same loader, so any drift between the two paths
    // would cause every launcher upload to fail HMAC verification.
    let secret = load_secret().map_err(IngestError::Auth)?;
    let claims = decode_token(token, &secret).map_err(IngestError::Auth)?;
    let now = chrono::Utc::now().timestamp();
    if claims.exp <= now {
        return Err(IngestError::Auth(AuthError::Expired {
            exp: claims.exp,
            now,
        }));
    }
    // The scope is the only thing that keeps a minted token from
    // being a general-purpose credential, so it has to be checked
    // here rather than assumed from the mint path.
    if !claims.kind_matches_scope() {
        return Err(IngestError::Auth(AuthError::MissingScope {
            wanted: SCOPE_TELEMETRY_WRITE,
        }));
    }
    if route == Route::Bundle && claims.is_launcher() {
        return Err(IngestError::Forbidden(
            "a launcher session cannot upload a bundle",
        ));
    }
    let accepted = match route {
        Route::Chunk => {
            claims.has_scope(SCOPE_TELEMETRY_WRITE) || claims.has_scope(SCOPE_LAUNCHER_WRITE)
        }
        Route::Bundle => claims.has_scope(SCOPE_TELEMETRY_WRITE),
    };
    if !accepted {
        return Err(IngestError::Auth(AuthError::MissingScope {
            wanted: SCOPE_TELEMETRY_WRITE,
        }));
    }
    Ok(claims)
}

/// Charge a launcher chunk to the launcher's own windows, in order: the
/// session, the install, then the address. A launcher chunk is charged here
/// instead of on the player's chunk windows. The first full window refuses,
/// and the windows before it keep the charge.
pub(super) fn charge_launcher_chunk(
    state: &UploadState,
    policy: &UploadPolicy,
    claims: &TokenClaims,
    peer: IpAddr,
    now: Instant,
) -> Result<(), IngestError> {
    state
        .launcher_chunk_session
        .check_and_record(
            install_key(&claims.sid),
            policy.launcher_chunk_per_session,
            CHUNK_QUOTA_WINDOW,
            "upload/launcher_session",
            now,
        )
        .map_err(|q| IngestError::Auth(AuthError::QuotaExceeded(q)))?;
    state
        .launcher_chunk_install
        .check_and_record(
            install_key(&claims.sub),
            policy.launcher_chunk_per_install,
            CHUNK_QUOTA_WINDOW,
            "upload/launcher_install",
            now,
        )
        .map_err(|q| IngestError::Auth(AuthError::QuotaExceeded(q)))?;
    state
        .launcher_chunk_ip
        .check_and_record(
            ip_key(peer),
            policy.launcher_chunk_per_ip,
            CHUNK_QUOTA_WINDOW,
            "upload/launcher_ip",
            now,
        )
        .map_err(|q| IngestError::Auth(AuthError::QuotaExceeded(q)))?;
    Ok(())
}

impl UploadLimits {
    /// The limits a launcher chunk runs under: each chunk size is the smaller
    /// of this limit and the launcher's cap. Player and lab limits are not
    /// changed by it.
    pub(super) fn launcher(&self) -> UploadLimits {
        UploadLimits {
            chunk_body_bytes: self.chunk_body_bytes.min(LAUNCHER_CHUNK_BODY_BYTES),
            chunk_decompressed_bytes: self
                .chunk_decompressed_bytes
                .min(LAUNCHER_CHUNK_DECOMPRESSED_BYTES),
            chunk_rows: self.chunk_rows.min(LAUNCHER_CHUNK_ROWS),
            chunk_row_bytes: self.chunk_row_bytes.min(LAUNCHER_CHUNK_ROW_BYTES),
            ..self.clone()
        }
    }
}
