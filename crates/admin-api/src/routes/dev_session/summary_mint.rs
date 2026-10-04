//! The launcher-summary mint: the `session_kind = "launcher_summary"` arm
//! of `/api/auth/dev-session`.
//!
//! The desktop launcher's summary exporter mints one token per export cycle
//! and posts it to `/api/telemetry/launcher-summary`
//! ([`crate::routes::telemetry::launcher_summary_routes`]). The mint needs
//! no credential, like the player one, so what the token can do is bounded
//! the same way: one scope, and a per-address quota of its own.

use std::net::IpAddr;
use std::time::Instant;

use super::handlers::{
    upload_endpoint_env, DevSessionRequest, DevSessionResponse, QuotaPolicy, Tables,
    DEFAULT_CHUNK_MAX_BYTES, DEFAULT_FLUSH_INTERVAL_MS, TOKEN_TTL_SECONDS,
};
use super::quota::{ip_key, validate_install_id};
use super::token::{
    encode_token, load_secret, AuthError, TokenClaims, SCOPE_LAUNCHER_SUMMARY_WRITE,
    SESSION_KIND_LAUNCHER_SUMMARY,
};

/// A launcher version as the summary session sends it: exactly three
/// dot-separated components of one to three ASCII digits. Returns the
/// parsed integers, and callers log and emit those, never the string, so
/// nothing the caller typed reaches a log line or a SigNoz field.
pub(crate) fn parse_version_triple(value: &str) -> Option<(u16, u16, u16)> {
    // `str::parse` alone would take a leading `+`; the byte check also
    // keeps non-ASCII digits out.
    fn component(part: &str) -> Option<u16> {
        let digits = (1..=3).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_digit());
        if !digits {
            return None;
        }
        part.parse().ok()
    }
    let mut parts = value.split('.');
    let triple = (
        component(parts.next()?)?,
        component(parts.next()?)?,
        component(parts.next()?)?,
    );
    parts.next().is_none().then_some(triple)
}

/// Mint for the desktop launcher's summary exporter. The token carries
/// nothing the caller chose: the scope, the kind and the `sub` are server
/// constants, so a summary row cannot be tied to an installation through
/// its token. The identifier fields a player's launcher fills in must be
/// empty, which keeps a build that sends them from looking accepted.
pub(super) fn mint_summary(
    tables: &Tables,
    policy: &QuotaPolicy,
    peer_ip: IpAddr,
    req: DevSessionRequest,
    now: Instant,
    now_unix: i64,
) -> Result<DevSessionResponse, AuthError> {
    // Its own table with the per-IP mint limit, and no per-install charge:
    // the exporter sends a fresh random `install_id` with every mint.
    tables.mint_summary_ip.check_and_record(
        ip_key(peer_ip),
        policy.mint_per_ip,
        policy.window,
        "mint/summary_ip",
        now,
    )?;
    validate_install_id(&req.install_id).map_err(AuthError::BadInstallId)?;
    const MUST_BE_EMPTY: &str = "must be empty for a launcher_summary session";
    for (field, value) in [
        ("machine_id", &req.machine_id),
        ("branch", &req.branch),
        ("git_sha", &req.git_sha),
    ] {
        if !value.is_empty() {
            return Err(AuthError::BadField {
                field,
                reason: MUST_BE_EMPTY,
            });
        }
    }
    if !req.tags.is_empty() {
        return Err(AuthError::BadField {
            field: "tags",
            reason: MUST_BE_EMPTY,
        });
    }
    let (major, minor, patch) =
        parse_version_triple(&req.launcher_version).ok_or(AuthError::BadField {
            field: "launcher_version",
            reason: "must be three dot-separated components of 1 to 3 digits",
        })?;
    let secret = load_secret()?;
    let session_id = uuid::Uuid::new_v4().to_string();
    let exp = now_unix + TOKEN_TTL_SECONDS;
    let claims = TokenClaims {
        iss: "cimmeria-server".into(),
        sub: SESSION_KIND_LAUNCHER_SUMMARY.into(),
        sid: session_id.clone(),
        iat: now_unix,
        exp,
        scope: vec![SCOPE_LAUNCHER_SUMMARY_WRITE.into()],
        kind: Some(SESSION_KIND_LAUNCHER_SUMMARY.into()),
    };
    let token = encode_token(&claims, &secret)?;
    // No identifiers row: `install_id` is random per mint and is dropped
    // here. The version is re-formatted from the parsed integers.
    tracing::info!(
        session_id = %session_id,
        session_kind = claims.session_kind(),
        launcher_version = %format_args!("{major}.{minor}.{patch}"),
        exp,
        "Minted launcher-summary token"
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
